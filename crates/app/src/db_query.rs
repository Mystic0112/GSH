//! Aba de um `.dbq` (`db_query_view.dart`): editor SQL em cima, grade de resultado embaixo.
//!
//! O arquivo persiste a conexão e o limite como frontmatter (`-- db:` / `-- limit:`); o editor
//! mostra só o SQL. "Untitled" nasce em memória e só vira arquivo no primeiro salvar. Mudança
//! externa no arquivo (um agente salvou) re-executa a query sozinha; abrir um arquivo nunca roda SQL.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use gsh_db::statements::{self, SqlStatement};
use gsh_db::{Connection, DbError, DbqDocument, Engine, QueryResult, SecretStore, DEFAULT_LIMIT};
use iced::widget::text_editor::{self, Position};
use iced::Task;

use crate::db_state::run_on;
use crate::editor_pane::{EditorMsg, EditorPane};
use crate::pane_model::QueryId;
use crate::{labels, theme};

/// Falhas de "Salvar como" (`FileOperationError` do Flutter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    EmptyName,
    NameHasSlash,
    InvalidName,
    NoWorkspace,
    AlreadyExists(String),
    WriteFailed,
}

impl SaveError {
    pub fn message(&self) -> String {
        match self {
            Self::EmptyName => labels::DB_ERR_SAVE_EMPTY_NAME.to_string(),
            Self::NameHasSlash => labels::DB_ERR_SAVE_NAME_HAS_SLASH.to_string(),
            Self::InvalidName => labels::DB_ERR_SAVE_INVALID_NAME.to_string(),
            Self::NoWorkspace => labels::DB_ERR_SAVE_NO_WORKSPACE.to_string(),
            Self::AlreadyExists(name) => labels::db_err_save_already_exists(name),
            Self::WriteFailed => labels::DB_ERR_SAVE_WRITE_FAILED.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum QueryMsg {
    Editor(EditorMsg),
    Run,
    Done(Result<QueryResult, DbError>),
    OpenPicker,
    ClosePicker,
    PickConnection(String),
    Save,
    /// "Salvar e fechar" do diálogo de alterações não salvas.
    SaveAndClose,
    SaveName(String),
    SaveSubmit,
    SaveCancel,
    DismissSaveError,
    SelectRow(usize),
    DragSplit(f32),
    GridScroll(f32),
    ResizeStart(Vec<f32>),
    ResizeColumn(usize, f32),
    ShowJson(bool),
    JsonAction(text_editor::Action),
    ErrorAction(text_editor::Action),
    CopyJson,
    CopiedReset,
    CheckDisk,
    Noop,
}

/// Contexto que o shell empresta a cada atualização.
pub struct Ctx<'a> {
    pub secrets: &'a SecretStore,
}

/// Estado de visualização (`DbTabViewState`).
#[derive(Debug)]
pub struct ResultView {
    pub result: Option<QueryResult>,
    pub error: Option<DbError>,
    pub error_content: Option<text_editor::Content>,
    pub split: f32,
    pub base_widths: Vec<f32>,
    pub manual_widths: Option<Vec<f32>>,
    pub selected_row: Option<usize>,
    pub as_json: bool,
    pub json: Option<(text_editor::Content, Arc<str>)>,
    pub grid_scroll: f32,
}

impl Default for ResultView {
    fn default() -> Self {
        Self {
            result: None,
            error: None,
            error_content: None,
            split: theme::DB_QUERY_SPLIT_DEFAULT,
            base_widths: Vec::new(),
            manual_widths: None,
            selected_row: None,
            as_json: false,
            json: None,
            grid_scroll: 0.0,
        }
    }
}

/// Diálogo "Salvar query como".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveDialog {
    pub name: String,
}

/// O que fazer depois que o "Salvar como" concluir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AfterSave {
    Nothing,
    Run,
    Close,
}

#[derive(Debug)]
pub struct DbQuery {
    pub id: QueryId,
    pub editor: EditorPane,
    /// `None` = untitled.
    pub path: Option<PathBuf>,
    scratch_title: String,
    /// Conteúdo do disco como o conhecemos (base de dirty).
    baseline: String,
    pub conn: Option<String>,
    pub limit: Option<i64>,
    pub running: bool,
    pub view: ResultView,
    pub picker_open: bool,
    pub save_dialog: Option<SaveDialog>,
    pub save_error: Option<String>,
    pub copied: bool,
    after_save: AfterSave,
    /// Pasta do workspace da aba (resolve SQLite relativo, o cofre e onde o untitled é salvo).
    root: PathBuf,
    /// "Salvar e fechar" concluiu: o shell fecha a aba.
    wants_close: bool,
    fingerprint: Option<(SystemTime, u64)>,
    /// Altura útil da área do editor + resultado, medida no último `view`.
    pub(crate) height: std::cell::Cell<f32>,
}

impl DbQuery {
    /// Aba untitled (`openScratchDbq`): buffer em memória, sem arquivo.
    pub fn scratch(id: QueryId, number: usize, content: &str, root: PathBuf) -> Self {
        let doc = DbqDocument::parse(content);
        Self::build(
            id,
            None,
            format!("Untitled-{number}.dbq"),
            content.to_string(),
            doc,
            root,
        )
    }

    /// Aba de um arquivo `.dbq` já lido.
    pub fn from_file(id: QueryId, path: PathBuf, content: String, root: PathBuf) -> Self {
        let doc = DbqDocument::parse(&content);
        let mut query = Self::build(id, Some(path), String::new(), content, doc, root);
        query.fingerprint = query.path.as_deref().and_then(fingerprint);
        query
    }

    fn build(
        id: QueryId,
        path: Option<PathBuf>,
        scratch_title: String,
        baseline: String,
        doc: DbqDocument,
        root: PathBuf,
    ) -> Self {
        Self {
            id,
            // o caminho do editor só escolhe o realce (`.sql`); quem salva é esta aba
            editor: EditorPane::new(PathBuf::from("query.sql"), doc.sql).with_submit_shortcut(),
            path,
            scratch_title,
            baseline,
            conn: doc.db,
            limit: doc.limit,
            running: false,
            view: ResultView::default(),
            picker_open: false,
            save_dialog: None,
            save_error: None,
            copied: false,
            after_save: AfterSave::Nothing,
            root,
            wants_close: false,
            fingerprint: None,
            height: std::cell::Cell::new(0.0),
        }
    }

    /// Id do scrollable vertical da grade.
    pub fn grid_id(&self) -> iced::widget::Id {
        iced::widget::Id::from(format!("dbq-grid-{}", self.id))
    }

    pub fn is_scratch(&self) -> bool {
        self.path.is_none()
    }

    /// Título da aba: o basename do arquivo ou `Untitled-N.dbq`.
    pub fn title(&self) -> String {
        match &self.path {
            Some(path) => crate::pane_model::file_display_name(path),
            None => self.scratch_title.clone(),
        }
    }

    pub fn sql(&self) -> &str {
        self.editor.state.buffer.text()
    }

    fn current(&self) -> DbqDocument {
        DbqDocument {
            db: self.conn.clone(),
            limit: self.limit,
            sql: self.sql().to_string(),
        }
    }

    /// Untitled está sempre "não salvo"; arquivo, quando algo difere do disco.
    pub fn dirty(&self) -> bool {
        if self.is_scratch() {
            return true;
        }
        let disk = DbqDocument::parse(&self.baseline);
        disk.sql != self.sql() || disk.db != self.conn || disk.limit != self.limit
    }

    /// Pasta do workspace da aba.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// "Salvar e fechar" terminou: o shell deve fechar a aba. Consome o pedido.
    pub fn take_close_request(&mut self) -> bool {
        std::mem::take(&mut self.wants_close)
    }

    pub fn connection<'a>(&self, connections: &'a [Connection]) -> Option<&'a Connection> {
        let name = self.conn.as_deref()?;
        connections.iter().find(|c| c.name == name)
    }

    pub fn update(&mut self, message: QueryMsg, ctx: &Ctx<'_>) -> Task<QueryMsg> {
        match message {
            QueryMsg::Editor(EditorMsg::Save) | QueryMsg::Save => {
                return self.save(ctx, AfterSave::Nothing)
            }
            QueryMsg::SaveAndClose => return self.save(ctx, AfterSave::Close),
            QueryMsg::Editor(EditorMsg::Submit) | QueryMsg::Run => return self.run(false, ctx),
            QueryMsg::Editor(message) => return self.editor.update(message).map(QueryMsg::Editor),
            QueryMsg::Done(result) => {
                self.finish(result);
                // resultado novo começa no topo (o scrollable guarda o próprio deslocamento)
                return iced::widget::operation::scroll_to(
                    self.grid_id(),
                    iced::widget::scrollable::AbsoluteOffset {
                        x: Some(0.0),
                        y: Some(0.0),
                    },
                );
            }
            QueryMsg::OpenPicker => self.picker_open = true,
            QueryMsg::ClosePicker => self.picker_open = false,
            QueryMsg::PickConnection(name) => {
                self.picker_open = false;
                self.conn = Some(name);
                // persiste a escolha no frontmatter e já executa (um arquivo novo pede o nome
                // antes: é o `_save()` do Flutter, que num untitled abre o "Salvar como")
                return self.save(ctx, AfterSave::Run);
            }
            QueryMsg::SaveName(name) => {
                if let Some(dialog) = self.save_dialog.as_mut() {
                    dialog.name = name;
                }
            }
            QueryMsg::SaveSubmit => return self.save_as(ctx),
            QueryMsg::SaveCancel => {
                self.save_dialog = None;
                self.after_save = AfterSave::Nothing;
            }
            QueryMsg::DismissSaveError => self.save_error = None,
            QueryMsg::SelectRow(row) => self.view.selected_row = Some(row),
            QueryMsg::DragSplit(delta) => self.drag_split(delta),
            QueryMsg::GridScroll(offset) => self.view.grid_scroll = offset,
            QueryMsg::ResizeStart(effective) => {
                // o primeiro arrasto congela as larguras visuais: o resto vira 1:1 com o mouse
                self.view.manual_widths.get_or_insert(effective);
            }
            QueryMsg::ResizeColumn(column, delta) => {
                if let Some(width) = self
                    .view
                    .manual_widths
                    .as_mut()
                    .and_then(|widths| widths.get_mut(column))
                {
                    *width = (*width + delta).clamp(theme::DB_GRID_COL_MIN, theme::DB_GRID_COL_MAX);
                }
            }
            QueryMsg::ShowJson(on) => {
                self.view.as_json = on;
                if on {
                    self.build_json();
                }
            }
            QueryMsg::JsonAction(action) => {
                if !action.is_edit() {
                    if let Some((content, _)) = self.view.json.as_mut() {
                        content.perform(action);
                    }
                }
            }
            QueryMsg::ErrorAction(action) => {
                if !action.is_edit() {
                    if let Some(content) = self.view.error_content.as_mut() {
                        content.perform(action);
                    }
                }
            }
            QueryMsg::CopyJson => {
                let Some(result) = self.view.result.as_ref() else {
                    return Task::none();
                };
                self.copied = true;
                return Task::batch([
                    iced::clipboard::write(result.to_json(None)),
                    Task::perform(
                        async {
                            let _ = tokio::task::spawn_blocking(|| {
                                std::thread::sleep(std::time::Duration::from_millis(
                                    theme::DB_COPIED_MILLIS,
                                ));
                            })
                            .await;
                        },
                        |()| QueryMsg::CopiedReset,
                    ),
                ]);
            }
            QueryMsg::CopiedReset => self.copied = false,
            QueryMsg::CheckDisk => return self.check_disk(ctx),
            QueryMsg::Noop => {}
        }
        Task::none()
    }

    // ----------------------------------------------------------------------------- executar

    /// Statements que um "Run" executa: o arquivo todo no automático; a seleção expandida aos
    /// statements que ela toca; senão o statement sob o cursor.
    fn statements_to_run(&self, auto: bool) -> Vec<SqlStatement> {
        let sql = self.sql();
        let all = statements::split(sql);
        if all.is_empty() {
            return Vec::new();
        }
        if auto {
            return all;
        }
        let cursor = self.editor.content.cursor();
        let caret = offset_of(sql, cursor.position);
        match cursor.selection {
            Some(anchor) if anchor != cursor.position => {
                let anchor = offset_of(sql, anchor);
                statements::statements_in_range(&all, anchor.min(caret), anchor.max(caret))
                    .into_iter()
                    .cloned()
                    .collect()
            }
            _ => statements::statement_at(&all, caret)
                .cloned()
                .into_iter()
                .collect(),
        }
    }

    fn run(&mut self, auto: bool, ctx: &Ctx<'_>) -> Task<QueryMsg> {
        if self.running {
            return Task::none();
        }
        let Some(conn) = self.conn.clone() else {
            return Task::none();
        };
        // arquivo real: Run salva antes (o arquivo é a fonte de verdade que o agente lê);
        // untitled executa o buffer direto, sem forçar save
        if !self.is_scratch() && self.dirty() && !self.write_file() {
            return Task::none();
        }
        let to_run = self.statements_to_run(auto);
        if to_run.is_empty() {
            return Task::none();
        }
        self.running = true;
        self.view.error = None;
        self.view.error_content = None;
        let texts: Vec<String> = to_run.into_iter().map(|statement| statement.text).collect();
        let limit = self.limit.map(|limit| usize::try_from(limit).unwrap_or(0));
        let (root, secrets) = (self.root.clone(), ctx.secrets.clone());
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || execute(&root, &secrets, &conn, &texts, limit))
                    .await
                    .unwrap_or_else(|error| Err(DbError::QueryFailed(error.to_string())))
            },
            QueryMsg::Done,
        )
    }

    fn finish(&mut self, result: Result<QueryResult, DbError>) {
        self.running = false;
        match result {
            Ok(result) => {
                self.view.base_widths = base_widths(&result);
                self.view.manual_widths = None;
                self.view.selected_row = None;
                self.view.grid_scroll = 0.0;
                self.view.error = None;
                self.view.error_content = None;
                self.view.result = Some(result);
                self.view.json = None;
                if self.view.as_json {
                    self.build_json();
                }
            }
            Err(error) => {
                self.view.error_content = Some(text_editor::Content::with_text(
                    &labels::db_error_message(&error),
                ));
                self.view.error = Some(error);
            }
        }
    }

    fn build_json(&mut self) {
        if let Some(result) = self
            .view
            .result
            .as_ref()
            .filter(|r| r.affected_rows.is_none())
        {
            let text = result.to_json(Some(theme::DB_JSON_PREVIEW_ROWS));
            self.view.json = Some((text_editor::Content::with_text(&text), text.into()));
        }
    }

    fn drag_split(&mut self, delta: f32) {
        let height = self.height.get();
        if height <= 0.0 {
            return;
        }
        let editor = editor_height(height, self.view.split);
        self.view.split =
            ((editor + delta) / height).clamp(theme::DB_QUERY_SPLIT_MIN, theme::DB_QUERY_SPLIT_MAX);
    }

    // -------------------------------------------------------------------------------- salvar

    /// Salva (`_save`). Arquivo grava direto; untitled abre o "Salvar como" e `after` roda depois.
    fn save(&mut self, ctx: &Ctx<'_>, after: AfterSave) -> Task<QueryMsg> {
        if !self.dirty() {
            return self.after_save_task(after, ctx);
        }
        if self.is_scratch() {
            self.after_save = after;
            self.save_dialog = Some(SaveDialog {
                name: labels::DB_QUERY_DEFAULT_FILE_NAME.to_string(),
            });
            return iced::widget::operation::focus(SAVE_NAME_ID);
        }
        if self.write_file() {
            self.after_save_task(after, ctx)
        } else {
            self.save_error = Some(SaveError::WriteFailed.message());
            Task::none()
        }
    }

    fn after_save_task(&mut self, after: AfterSave, ctx: &Ctx<'_>) -> Task<QueryMsg> {
        match after {
            AfterSave::Nothing => Task::none(),
            AfterSave::Run => self.run(false, ctx),
            AfterSave::Close => {
                self.wants_close = true;
                Task::none()
            }
        }
    }

    /// Grava o conteúdo no arquivo da aba; `baseline` passa a ser o gravado.
    fn write_file(&mut self) -> bool {
        let Some(path) = self.path.clone() else {
            return false;
        };
        let content = self.current().serialize();
        if std::fs::write(&path, &content).is_err() {
            return false;
        }
        self.baseline = content;
        self.fingerprint = fingerprint(&path);
        true
    }

    /// Confirma o nome: valida, cria o arquivo na raiz do workspace e a aba vira de arquivo.
    fn save_as(&mut self, ctx: &Ctx<'_>) -> Task<QueryMsg> {
        let Some(dialog) = self.save_dialog.take() else {
            return Task::none();
        };
        let after = std::mem::replace(&mut self.after_save, AfterSave::Nothing);
        let content = self.current().serialize();
        match create_file(&self.root, &dialog.name, &content) {
            Ok(path) => {
                self.fingerprint = fingerprint(&path);
                self.path = Some(path);
                self.baseline = content;
                self.after_save_task(after, ctx)
            }
            Err(error) => {
                self.save_error = Some(error.message());
                Task::none()
            }
        }
    }

    // ----------------------------------------------------------------------- mudança externa

    /// Arquivo mudou no disco (um agente salvou): sem edição local adota o novo conteúdo e
    /// re-executa o arquivo inteiro. Edição local vence.
    fn check_disk(&mut self, ctx: &Ctx<'_>) -> Task<QueryMsg> {
        let Some(path) = self.path.clone() else {
            return Task::none();
        };
        let now = fingerprint(&path);
        if now == self.fingerprint || now.is_none() {
            return Task::none();
        }
        self.fingerprint = now;
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Task::none();
        };
        if text == self.baseline || self.dirty() {
            return Task::none();
        }
        let doc = DbqDocument::parse(&text);
        self.baseline = text;
        self.conn = doc.db;
        self.limit = doc.limit;
        let config = self.editor.config.clone();
        self.editor = EditorPane::new(PathBuf::from("query.sql"), doc.sql)
            .with_submit_shortcut()
            .with_config(config);
        if self.conn.is_some() {
            return self.run(true, ctx);
        }
        Task::none()
    }
}

/// Id do campo de nome do "Salvar como".
pub const SAVE_NAME_ID: &str = "dbq-save-name";

/// Executa os statements na conexão `conn` (relida do disco) e devolve o último resultado.
pub fn execute(
    root: &Path,
    secrets: &SecretStore,
    conn: &str,
    texts: &[String],
    limit: Option<usize>,
) -> Result<QueryResult, DbError> {
    run_on(root, secrets, conn, |endpoint| {
        gsh_db::driver::run_statements(&endpoint, texts, limit)
    })
}

/// Largura base por coluna: nome mais amostra das primeiras linhas (`_computeWidths`).
pub fn base_widths(result: &QueryResult) -> Vec<f32> {
    (0..result.columns.len())
        .map(|column| {
            let header = result
                .columns
                .get(column)
                .map_or(0, |c| c.name.chars().count());
            let sample = result
                .rows
                .iter()
                .take(theme::DB_GRID_SAMPLE_ROWS)
                .filter_map(|row| row.get(column))
                .map(|cell| cell.display().chars().count())
                .max()
                .unwrap_or(0);
            let chars = header.max(sample) as f32;
            (chars * theme::DB_GRID_BASE_CHAR + theme::DB_GRID_BASE_PAD)
                .clamp(theme::DB_GRID_BASE_MIN, theme::DB_GRID_BASE_MAX)
        })
        .collect()
}

/// Altura do editor para uma área de `height` e a proporção `split`.
pub fn editor_height(height: f32, split: f32) -> f32 {
    let max = (height - theme::DB_QUERY_RESULT_MIN).max(theme::DB_QUERY_EDITOR_MIN);
    (height * split).clamp(theme::DB_QUERY_EDITOR_MIN, max)
}

/// Offset em bytes de uma posição linha/coluna do editor dentro de `text`.
fn offset_of(text: &str, position: Position) -> usize {
    let mut offset = 0;
    for (index, line) in text.split('\n').enumerate() {
        if index == position.line {
            let visible = line.strip_suffix('\r').unwrap_or(line);
            let mut column = position.column.min(visible.len());
            while !visible.is_char_boundary(column) {
                column -= 1;
            }
            return offset + column;
        }
        offset += line.len() + 1;
    }
    text.len()
}

fn fingerprint(path: &Path) -> Option<(SystemTime, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// Validação de nome do `files.validateName` (só o basename; `.dbq` é anexado).
fn validate_name(name: &str) -> Result<(), SaveError> {
    if name.contains(['/', '\\']) {
        return Err(SaveError::NameHasSlash);
    }
    if name == "." || name == ".." || name.contains('\0') {
        return Err(SaveError::InvalidName);
    }
    Ok(())
}

/// `saveScratchAs`: cria `<raiz>/<nome>.dbq` sem sobrescrever.
fn create_file(root: &Path, name: &str, content: &str) -> Result<PathBuf, SaveError> {
    let mut name = name.trim().to_string();
    if name.is_empty() {
        return Err(SaveError::EmptyName);
    }
    if !name.to_lowercase().ends_with(".dbq") {
        name.push_str(".dbq");
    }
    validate_name(&name)?;
    if root.as_os_str().is_empty() {
        return Err(SaveError::NoWorkspace);
    }
    let path = root.join(&name);
    if path.exists() {
        return Err(SaveError::AlreadyExists(name));
    }
    std::fs::write(&path, content).map_err(|_| SaveError::WriteFailed)?;
    Ok(path)
}

/// SQL que o "New query" de uma tabela pré-preenche.
pub fn select_all(engine: Engine, schema: Option<&str>, table: &str) -> String {
    format!(
        "SELECT * FROM {} LIMIT 100;",
        gsh_db::schema::qualified_ident(engine, schema, table)
    )
}

/// Conteúdo inicial de um untitled (`openScratchDbq`): `-- db:` quando há conexão.
pub fn scratch_content(conn: Option<&str>, sql: Option<&str>) -> String {
    let body = sql.unwrap_or("SELECT 1;");
    match conn {
        Some(conn) => format!("-- db: {conn}\n{body}\n"),
        None => format!("{body}\n"),
    }
}

pub const DEFAULT_QUERY_LIMIT: usize = DEFAULT_LIMIT;

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_db::{Access, Cell, Origin};
    use iced::widget::text_editor::{Action, Motion};

    struct Fixture {
        dir: tempfile::TempDir,
        secrets: SecretStore,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let db = rusqlite::Connection::open(dir.path().join("app.db")).unwrap();
            db.execute_batch(
                "CREATE TABLE t (n INTEGER, s TEXT); INSERT INTO t VALUES (1,'a'),(2,'b'),(3,'c');",
            )
            .unwrap();
            let connections = [Connection::sqlite(
                "app",
                "app.db",
                Origin::Registered,
                Access::Read,
                true,
            )];
            gsh_db::store::save(dir.path(), &connections).unwrap();
            let secrets = SecretStore::new(dir.path().join("secrets.json"));
            Self { dir, secrets }
        }

        fn ctx(&self) -> Ctx<'_> {
            Ctx {
                secrets: &self.secrets,
            }
        }

        fn root(&self) -> PathBuf {
            self.dir.path().to_path_buf()
        }
    }

    #[test]
    fn scratch_content_and_title() {
        assert_eq!(scratch_content(None, None), "SELECT 1;\n");
        assert_eq!(
            scratch_content(Some("pg"), Some("select 2;")),
            "-- db: pg\nselect 2;\n"
        );
        let query = DbQuery::scratch(1, 3, &scratch_content(Some("pg"), None), PathBuf::new());
        assert_eq!(query.title(), "Untitled-3.dbq");
        assert_eq!(query.conn.as_deref(), Some("pg"));
        assert_eq!(query.sql(), "SELECT 1;\n");
        assert!(query.dirty(), "untitled sempre está não salvo");
    }

    #[test]
    fn select_all_quotes_and_qualifies() {
        assert_eq!(
            select_all(Engine::Postgres, Some("sales"), "Orders"),
            "SELECT * FROM \"sales\".\"Orders\" LIMIT 100;"
        );
    }

    #[test]
    fn offsets_follow_lines_columns_and_char_boundaries() {
        let text = "ab\r\ncé\nxyz";
        assert_eq!(offset_of(text, Position { line: 0, column: 1 }), 1);
        assert_eq!(
            offset_of(
                text,
                Position {
                    line: 0,
                    column: 99
                }
            ),
            2
        );
        assert_eq!(offset_of(text, Position { line: 1, column: 1 }), 5);
        // no meio do `é` (2 bytes) recua para o início do caractere
        assert_eq!(offset_of(text, Position { line: 1, column: 2 }), 5);
        assert_eq!(offset_of(text, Position { line: 2, column: 2 }), 10);
        assert_eq!(offset_of(text, Position { line: 9, column: 0 }), text.len());
    }

    #[test]
    fn run_picks_the_statement_under_the_cursor_or_the_selection() {
        let mut query = DbQuery::scratch(
            1,
            1,
            "-- db: app\nselect 1;\nselect 2;\nselect 3;\n",
            PathBuf::new(),
        );
        let texts = |q: &DbQuery, auto: bool| -> Vec<String> {
            q.statements_to_run(auto)
                .into_iter()
                .map(|s| s.text)
                .collect()
        };
        // cursor no início: primeiro statement do corpo
        assert_eq!(texts(&query, false), ["select 1"]);
        let _ = query
            .editor
            .update(EditorMsg::Action(Action::Move(Motion::Down)));
        assert_eq!(texts(&query, false), ["select 2"]);
        // seleção que toca dois statements executa os dois, inteiros
        let _ = query
            .editor
            .update(EditorMsg::Action(Action::Select(Motion::Down)));
        assert_eq!(texts(&query, false), ["select 2", "select 3"]);
        // automático: o arquivo todo
        assert_eq!(texts(&query, true), ["select 1", "select 2", "select 3"]);
    }

    #[test]
    fn execute_returns_the_last_statement_and_keeps_driver_errors_typed() {
        let fx = Fixture::new();
        let ok = execute(
            fx.dir.path(),
            &fx.secrets,
            "app",
            &["select 1".into(), "select n, s from t order by n".into()],
            Some(2),
        )
        .unwrap();
        assert_eq!(ok.rows.len(), 2);
        assert!(ok.truncated);
        assert_eq!(ok.rows[0][0], Cell::Int(1));
        let missing = execute(
            fx.dir.path(),
            &fx.secrets,
            "nope",
            &["select 1".into()],
            None,
        )
        .unwrap_err();
        assert!(matches!(missing, DbError::UnknownConnection { .. }));
        let broken =
            execute(fx.dir.path(), &fx.secrets, "app", &["selec".into()], None).unwrap_err();
        assert_eq!(broken.kind(), "query_failed");
    }

    #[test]
    fn finishing_a_run_resets_the_grid_and_an_error_keeps_the_last_result() {
        let fx = Fixture::new();
        let mut query = DbQuery::scratch(1, 1, "-- db: app\nselect n, s from t;\n", fx.root());
        let result = execute(
            fx.dir.path(),
            &fx.secrets,
            "app",
            &["select n, s from t".into()],
            None,
        )
        .unwrap();
        query.running = true;
        query.view.selected_row = Some(1);
        let _ = query.update(QueryMsg::Done(Ok(result)), &fx.ctx());
        assert!(!query.running);
        assert_eq!(query.view.selected_row, None);
        assert_eq!(query.view.base_widths.len(), 2);
        assert!(query.view.manual_widths.is_none());

        let _ = query.update(QueryMsg::Done(Err(DbError::Timeout(30))), &fx.ctx());
        assert!(query.view.error.is_some() && query.view.result.is_some());
        assert!(query.view.error_content.is_some());
    }

    #[test]
    fn column_resize_freezes_widths_then_clamps() {
        let fx = Fixture::new();
        let mut query = DbQuery::scratch(1, 1, "x", fx.root());
        let ctx = fx.ctx();
        let _ = query.update(QueryMsg::ResizeStart(vec![100.0, 200.0]), &ctx);
        let _ = query.update(QueryMsg::ResizeStart(vec![1.0, 1.0]), &ctx);
        assert_eq!(query.view.manual_widths, Some(vec![100.0, 200.0]));
        let _ = query.update(QueryMsg::ResizeColumn(0, 30.0), &ctx);
        let _ = query.update(QueryMsg::ResizeColumn(1, -500.0), &ctx);
        assert_eq!(query.view.manual_widths, Some(vec![130.0, 60.0]));
        let _ = query.update(QueryMsg::ResizeColumn(0, 99_999.0), &ctx);
        assert_eq!(
            query.view.manual_widths.as_ref().map(|w| w[0]),
            Some(2000.0)
        );
    }

    #[test]
    fn base_widths_follow_name_and_sample() {
        let result = QueryResult::from_rows(
            vec!["id".into(), "a_much_longer_column_name".into()],
            vec![vec![Cell::Int(1), Cell::Text("x".into())]],
            false,
            std::time::Duration::ZERO,
        );
        let widths = base_widths(&result);
        assert_eq!(widths[0], 64.0);
        assert_eq!(widths[1], 25.0 * 7.5 + 26.0);
    }

    #[test]
    fn split_drag_is_clamped_and_needs_a_measured_height() {
        let fx = Fixture::new();
        let mut query = DbQuery::scratch(1, 1, "x", fx.root());
        let ctx = fx.ctx();
        let _ = query.update(QueryMsg::DragSplit(50.0), &ctx);
        assert_eq!(query.view.split, 0.5, "sem altura medida não mexe");
        query.height.set(600.0);
        let _ = query.update(QueryMsg::DragSplit(60.0), &ctx);
        assert!((query.view.split - 0.6).abs() < 1e-6);
        let _ = query.update(QueryMsg::DragSplit(10_000.0), &ctx);
        assert_eq!(query.view.split, 0.85);
        let _ = query.update(QueryMsg::DragSplit(-10_000.0), &ctx);
        assert_eq!(query.view.split, 0.12);
        assert_eq!(editor_height(600.0, 0.5), 300.0);
        assert_eq!(editor_height(600.0, 0.99), 490.0);
        assert_eq!(editor_height(600.0, 0.01), 90.0);
    }

    #[test]
    fn untitled_save_asks_for_a_name_then_becomes_a_file_and_runs() {
        let fx = Fixture::new();
        let mut query = DbQuery::scratch(
            1,
            1,
            &scratch_content(Some("app"), Some("select n from t;")),
            fx.root(),
        );
        let ctx = fx.ctx();
        let _ = query.update(QueryMsg::Save, &ctx);
        assert_eq!(
            query.save_dialog.as_ref().map(|d| d.name.as_str()),
            Some("query.dbq")
        );

        let _ = query.update(QueryMsg::SaveName("minha".into()), &ctx);
        let _ = query.update(QueryMsg::SaveSubmit, &ctx);
        let path = fx.dir.path().join("minha.dbq");
        assert_eq!(query.path.as_deref(), Some(path.as_path()));
        assert_eq!(query.title(), "minha.dbq");
        assert!(!query.dirty());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "-- db: app\n\nselect n from t;\n"
        );
    }

    #[test]
    fn save_as_refuses_bad_names_and_existing_files() {
        let fx = Fixture::new();
        let root = fx.dir.path();
        assert_eq!(create_file(root, "  ", "x"), Err(SaveError::EmptyName));
        assert_eq!(create_file(root, "a/b", "x"), Err(SaveError::NameHasSlash));
        assert_eq!(
            create_file(Path::new(""), "q", "x"),
            Err(SaveError::NoWorkspace)
        );
        create_file(root, "q.dbq", "one").unwrap();
        assert_eq!(
            create_file(root, "q", "two"),
            Err(SaveError::AlreadyExists("q.dbq".into()))
        );
        assert_eq!(std::fs::read_to_string(root.join("q.dbq")).unwrap(), "one");
        // a extensão em maiúsculas conta como já presente: não anexa outra
        assert!(create_file(root, "Outra.DBQ", "x")
            .unwrap()
            .ends_with("Outra.DBQ"));
    }

    #[test]
    fn file_tabs_write_through_and_pick_connection_saves_the_choice() {
        let fx = Fixture::new();
        let path = fx.dir.path().join("q.dbq");
        std::fs::write(&path, "select n from t;\n").unwrap();
        let mut query = DbQuery::from_file(1, path.clone(), "select n from t;\n".into(), fx.root());
        assert!(!query.dirty());
        let _ = query.update(QueryMsg::PickConnection("app".into()), &fx.ctx());
        assert!(query.running, "escolher a conexão já executa");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "-- db: app\n\nselect n from t;\n"
        );
        assert!(!query.dirty());
    }

    #[test]
    fn external_change_adopts_the_new_content_without_running_when_no_connection() {
        let fx = Fixture::new();
        let path = fx.dir.path().join("q.dbq");
        std::fs::write(&path, "select 1;\n").unwrap();
        let mut query = DbQuery::from_file(1, path.clone(), "select 1;\n".into(), fx.root());
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "select 22222;\n").unwrap();
        let _ = query.update(QueryMsg::CheckDisk, &fx.ctx());
        assert_eq!(query.sql(), "select 22222;\n");
        assert!(!query.running);

        // com conexão, a mudança externa executa o arquivo inteiro
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "-- db: app\n\nselect n from t;\n").unwrap();
        let _ = query.update(QueryMsg::CheckDisk, &fx.ctx());
        assert_eq!(query.conn.as_deref(), Some("app"));
        assert!(query.running);
    }

    #[test]
    fn local_edits_win_over_an_external_change() {
        let fx = Fixture::new();
        let path = fx.dir.path().join("q.dbq");
        std::fs::write(&path, "select 1;\n").unwrap();
        let mut query = DbQuery::from_file(1, path.clone(), "select 1;\n".into(), fx.root());
        let _ = query.update(
            QueryMsg::Editor(EditorMsg::Action(Action::Edit(text_editor::Edit::Insert(
                'x',
            )))),
            &fx.ctx(),
        );
        assert!(query.dirty());
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "select 999;\n").unwrap();
        let _ = query.update(QueryMsg::CheckDisk, &fx.ctx());
        assert!(query.sql().starts_with('x'));
    }

    #[test]
    fn run_without_a_connection_or_while_running_does_nothing() {
        let fx = Fixture::new();
        let mut query = DbQuery::scratch(1, 1, "select 1;", fx.root());
        let _ = query.update(QueryMsg::Run, &fx.ctx());
        assert!(!query.running);
        query.conn = Some("app".into());
        let _ = query.update(QueryMsg::Run, &fx.ctx());
        assert!(query.running);
        query.view.error = Some(DbError::Timeout(1));
        let _ = query.update(QueryMsg::Run, &fx.ctx());
        assert!(query.view.error.is_some(), "segundo Run em voo é ignorado");
    }
}
