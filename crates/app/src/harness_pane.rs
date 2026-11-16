//! Aba Harness (`harness_view.dart` + `harness_viewmodel.dart` do Flutter): o que o
//! Claude Code carrega ao rodar na pasta do workspace, em cartões por seção.
//!
//! A leitura (`gsh_harness::read`) roda numa thread própria e é refeita ao abrir a aba e
//! quando ela volta a ficar ativa. Clicar numa linha abre o arquivo no editor; "Novo" cria
//! o item pelo modelo (pedindo o nome em skills, subagentes, comandos e hooks).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use gsh_harness::{Entry, Kind, ProjectHarness, Scope};
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, responsive, row, scrollable, text, text_input, tooltip, Space,
};
use iced::{alignment, Alignment, Background, Border, Color, Element, Fill, Length, Task, Theme};

use crate::{content_search_panel, labels, pane_chrome, pane_close_dialog, panel_widgets, theme};

/// Id do campo de nome, para focá-lo ao abrir o diálogo (`autofocus`).
const NAME_INPUT_ID: &str = "harness-name";

/// Quadro do spinner (~30 fps), igual ao da busca.
pub const SPIN_TICK_MS: u64 = 33;

/// Largura média de um caractere em unidades de corpo (aproximação para o "…" das linhas).
const BODY_CHAR_RATIO: f32 = 0.52;
/// JetBrains Mono: avanço de 0,6 em.
const MONO_CHAR_RATIO: f32 = 0.6;

#[derive(Debug, Clone)]
pub enum Msg {
    /// Fim da leitura do workspace (`None` = a leitura falhou).
    Loaded(PathBuf, Option<ProjectHarness>),
    Reload(PathBuf),
    /// Clique numa linha: abre o arquivo no editor.
    Open(PathBuf),
    /// "Novo" / "Editar arquivo" de uma seção.
    Create(PathBuf, Kind),
    /// "Mostrar todos" / "Mostrar menos" de uma seção.
    ToggleAll(PathBuf, Kind),
    NameChanged(String),
    NameSubmit,
    NameCancel,
    /// Item criado (caminho do arquivo) ou falha (`None`).
    Created(PathBuf, Option<PathBuf>),
    /// Quadro do spinner.
    Spin,
    Noop,
}

/// Formulário de nome do item novo (`showRealmNameDialog`).
#[derive(Debug, Clone)]
struct NameForm {
    workspace: PathBuf,
    kind: Kind,
    name: String,
    /// Nomes já usados na seção, em minúsculas.
    taken: BTreeSet<String>,
}

impl NameForm {
    fn trimmed(&self) -> &str {
        self.name.trim()
    }

    fn valid(&self) -> bool {
        let name = self.trimmed();
        !name.is_empty() && !self.taken.contains(&name.to_lowercase())
    }
}

/// Estado das abas Harness: o harness lido de cada workspace. Recarrega quando a aba
/// aparece ou o usuário pede (não fica vigiando arquivos).
#[derive(Debug, Default)]
pub struct HarnessState {
    loaded: BTreeMap<PathBuf, ProjectHarness>,
    loading: BTreeSet<PathBuf>,
    /// Seções com "mostrar todos" ligado.
    expanded: BTreeSet<(PathBuf, Kind)>,
    name_form: Option<NameForm>,
    pub spin_ms: u64,
}

impl HarnessState {
    pub fn of(&self, workspace: &Path) -> Option<&ProjectHarness> {
        self.loaded.get(workspace)
    }

    pub fn is_loading(&self, workspace: &Path) -> bool {
        self.loading.contains(workspace)
    }

    pub fn any_loading(&self) -> bool {
        !self.loading.is_empty()
    }

    /// Lê o workspace fora da thread da UI; uma leitura por vez em cada workspace.
    pub fn load(&mut self, workspace: &Path) -> Task<Msg> {
        if !self.loading.insert(workspace.to_path_buf()) {
            return Task::none();
        }
        let owned = workspace.to_path_buf();
        let target = owned.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || gsh_harness::read(&target, &home_dir())).await
            },
            move |result| Msg::Loaded(owned.clone(), result.ok()),
        )
    }

    /// Aplica a mensagem; o segundo valor é um arquivo para abrir no editor.
    pub fn update(&mut self, msg: Msg) -> (Task<Msg>, Option<PathBuf>) {
        match msg {
            Msg::Loaded(workspace, harness) => {
                self.loading.remove(&workspace);
                if let Some(harness) = harness {
                    self.loaded.insert(workspace, harness);
                }
                (Task::none(), None)
            }
            Msg::Reload(workspace) => (self.load(&workspace), None),
            Msg::Open(path) => (Task::none(), Some(path)),
            Msg::Create(workspace, kind) => (self.start_create(workspace, kind), None),
            Msg::ToggleAll(workspace, kind) => {
                let key = (workspace, kind);
                if !self.expanded.remove(&key) {
                    self.expanded.insert(key);
                }
                (Task::none(), None)
            }
            Msg::NameChanged(name) => {
                if let Some(form) = &mut self.name_form {
                    form.name = name;
                }
                (Task::none(), None)
            }
            Msg::NameSubmit => (self.submit_name(), None),
            Msg::NameCancel => {
                self.name_form = None;
                (Task::none(), None)
            }
            Msg::Created(workspace, Some(path)) => (self.load(&workspace), Some(path)),
            Msg::Created(_, None) => (Task::none(), None),
            Msg::Spin => {
                self.spin_ms = self.spin_ms.wrapping_add(SPIN_TICK_MS);
                (Task::none(), None)
            }
            Msg::Noop => (Task::none(), None),
        }
    }

    /// Skills, subagentes, comandos e hooks pedem o nome; o resto cria direto.
    fn start_create(&mut self, workspace: PathBuf, kind: Kind) -> Task<Msg> {
        if !matches!(
            kind,
            Kind::Skills | Kind::Agents | Kind::Commands | Kind::Hooks
        ) {
            return create_task(workspace, kind, None);
        }
        let taken = self
            .of(&workspace)
            .map(|harness| harness.of(kind))
            .unwrap_or_default()
            .into_iter()
            .map(|entry| entry.name.replacen('/', "", 1).to_lowercase())
            .collect();
        self.name_form = Some(NameForm {
            workspace,
            kind,
            name: String::new(),
            taken,
        });
        iced::widget::operation::focus(NAME_INPUT_ID)
    }

    fn submit_name(&mut self) -> Task<Msg> {
        match self.name_form.take() {
            Some(form) if form.valid() => {
                let name = form.trimmed().to_owned();
                create_task(form.workspace, form.kind, Some(name))
            }
            other => {
                self.name_form = other;
                Task::none()
            }
        }
    }

    /// Diálogo do nome do item novo, quando aberto.
    pub fn name_dialog(&self) -> Option<Element<'_, Msg>> {
        let form = self.name_form.as_ref()?;
        let mut body = column![text_input(labels::HARNESS_NAME_PLACEHOLDER, &form.name)
            .id(NAME_INPUT_ID)
            .on_input(Msg::NameChanged)
            .on_submit(Msg::NameSubmit)
            .style(panel_widgets::input_style)]
        .width(Length::Fixed(theme::HARNESS_NAME_DIALOG_WIDTH));
        if !form.trimmed().is_empty() && !form.valid() {
            body = body
                .push(Space::new().height(theme::HARNESS_NAME_ERROR_GAP))
                .push(label(
                    labels::HARNESS_DUPLICATE_NAME,
                    theme::SIZE_LABEL,
                    theme::palette().error,
                ));
        }
        Some(pane_close_dialog::custom(
            labels::harness_new_title(labels::harness_title(form.kind)),
            body.into(),
            labels::HARNESS_CREATE,
            Msg::NameCancel,
            form.valid().then_some(Msg::NameSubmit),
            Msg::Noop,
        ))
    }
}

/// `$HOME` (ou `%USERPROFILE%`); vazio = sem nada do usuário.
fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

fn create_task(workspace: PathBuf, kind: Kind, name: Option<String>) -> Task<Msg> {
    let owned = workspace.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                gsh_harness::create(&workspace, kind, name.as_deref())
            })
            .await
        },
        move |result| {
            let path = match result {
                Ok(Ok(path)) => Some(path),
                // o Flutter só registra no log e não abre nada
                Ok(Err(error)) => {
                    eprintln!("[harness] criar {kind:?} falhou: {error:?}");
                    None
                }
                Err(_) => None,
            };
            Msg::Created(owned.clone(), path)
        },
    )
}

// ---------------------------------------------------------------------------------------
// Tela

/// Conteúdo da aba Harness do workspace `workspace`.
pub fn view<'a>(state: &'a HarnessState, workspace: &'a Path) -> Element<'a, Msg> {
    responsive(move |size| page(state, workspace, size.width)).into()
}

fn page<'a>(state: &'a HarnessState, workspace: &'a Path, width: f32) -> Element<'a, Msg> {
    let palette = theme::palette();
    let loading = state.is_loading(workspace);
    let title = text(labels::HARNESS_TAB_TITLE)
        .font(theme::FONT_DISPLAY)
        .size(theme::SIZE_DISPLAY)
        .line_height(theme::LINE_HEIGHT_DISPLAY)
        .color(palette.text);
    let subtitle = label(
        labels::harness_subtitle(&workspace.to_string_lossy()),
        theme::SIZE_LABEL,
        palette.text3,
    );
    let header = row![
        column![
            title,
            Space::new().height(theme::HARNESS_SUBTITLE_GAP),
            subtitle
        ]
        .width(Fill),
        refresh_button(workspace, loading, state.spin_ms),
    ]
    .align_y(Alignment::Center);

    let mut body = column![header, Space::new().height(theme::HARNESS_HEADER_GAP)];
    match state.of(workspace) {
        None => {
            body = body.push(
                container(content_search_panel::spinner(
                    theme::HARNESS_LOADING_SPINNER,
                    state.spin_ms,
                ))
                .center_x(Fill),
            );
        }
        Some(harness) => {
            let inner = (width - 2.0 * theme::HARNESS_PADDING_SIDE).max(0.0);
            body = body
                .push(cost_card(harness))
                .push(Space::new().height(theme::HARNESS_GRID_GAP))
                .push(grid(state, workspace, harness, inner));
        }
    }

    container(
        scrollable(container(body).width(Fill).padding(iced::Padding {
            top: theme::HARNESS_PADDING_TOP,
            right: theme::HARNESS_PADDING_SIDE,
            bottom: theme::HARNESS_PADDING_BOTTOM,
            left: theme::HARNESS_PADDING_SIDE,
        }))
        .width(Fill)
        .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        ..Default::default()
    })
    .into()
}

/// Recarregar, ou o spinner enquanto lê.
fn refresh_button<'a>(workspace: &Path, loading: bool, spin_ms: u64) -> Element<'a, Msg> {
    let content: Element<'a, Msg> = if loading {
        content_search_panel::spinner(theme::HARNESS_REFRESH_SPINNER, spin_ms)
    } else {
        icon(
            labels::ICON_REFRESH,
            theme::HARNESS_REFRESH_ICON,
            theme::palette().text2,
        )
    };
    tooltip(
        button(content)
            .padding(theme::HARNESS_REFRESH_PADDING)
            .on_press_maybe((!loading).then(|| Msg::Reload(workspace.to_path_buf())))
            .style(hover_style(theme::HARNESS_ROW_RADIUS)),
        pane_chrome::tip_bubble(labels::HARNESS_REFRESH),
        tooltip::Position::Bottom,
    )
    .into()
}

/// Cartões em 1 coluna, ou 2 a partir de 980px (o `Wrap` do Flutter).
fn grid<'a>(
    state: &HarnessState,
    workspace: &Path,
    harness: &ProjectHarness,
    inner: f32,
) -> Element<'a, Msg> {
    let columns = if inner >= theme::HARNESS_TWO_COLUMNS_MIN_WIDTH {
        2
    } else {
        1
    };
    let card_width = (inner - (columns - 1) as f32 * theme::HARNESS_GRID_GAP) / columns as f32;
    let mut rows = column![].spacing(theme::HARNESS_GRID_GAP);
    for kinds in Kind::ALL.chunks(columns) {
        let mut line = row![].spacing(theme::HARNESS_GRID_GAP);
        for &kind in kinds {
            line = line.push(section_card(state, workspace, harness, kind, card_width));
        }
        rows = rows.push(line);
    }
    rows.into()
}

/// Resumo do topo: quanto o harness custa em toda mensagem e quantos avisos.
fn cost_card<'a>(harness: &ProjectHarness) -> Element<'a, Msg> {
    let palette = theme::palette();
    let issues = harness.issue_count();
    let parts: Vec<String> = [Kind::Memory, Kind::Skills, Kind::Agents, Kind::Commands]
        .into_iter()
        .filter_map(|kind| {
            let tokens = harness.always_on_tokens(Some(kind));
            (tokens > 0)
                .then(|| format!("{} {}", labels::harness_title(kind), format_tokens(tokens)))
        })
        .collect();

    let mut heading = row![
        title_text(labels::HARNESS_COST_TITLE),
        Space::new().width(theme::HARNESS_COST_TITLE_GAP),
        text(labels::harness_cost_value(&format_tokens(
            harness.always_on_tokens(None)
        )))
        .font(theme::FONT_DISPLAY)
        .size(theme::HARNESS_COST_VALUE_SIZE)
        .line_height(theme::LINE_HEIGHT_DISPLAY)
        .color(palette.text),
        Space::new().width(Fill),
    ]
    .align_y(Alignment::End);
    if issues > 0 {
        heading = heading.push(
            row![
                icon(
                    labels::ICON_WARNING_AMBER,
                    theme::HARNESS_COST_ISSUE_ICON,
                    palette.warn
                ),
                Space::new().width(theme::HARNESS_COST_ISSUE_GAP),
                label(
                    labels::harness_issues(issues),
                    theme::SIZE_LABEL,
                    palette.warn
                ),
            ]
            .align_y(Alignment::Center),
        );
    }
    let mut content = column![heading];
    if !parts.is_empty() {
        content = content
            .push(Space::new().height(theme::HARNESS_COST_PARTS_GAP))
            .push(label(parts.join("  ·  "), theme::SIZE_LABEL, palette.text2));
    }
    content = content
        .push(Space::new().height(theme::HARNESS_COST_HINT_GAP))
        .push(label(
            labels::HARNESS_COST_HINT,
            theme::SIZE_LABEL,
            palette.text3,
        ))
        .push(Space::new().height(theme::HARNESS_COST_TIP_GAP))
        .push(label(
            labels::HARNESS_COST_TIP,
            theme::SIZE_LABEL,
            palette.text3,
        ));
    container(content)
        .width(Fill)
        .padding(theme::HARNESS_COST_PADDING)
        .style(card_style)
        .into()
}

fn section_icon(kind: Kind) -> &'static str {
    match kind {
        Kind::Memory => labels::ICON_DESCRIPTION,
        Kind::Skills => labels::ICON_AUTO_AWESOME,
        Kind::Agents => labels::ICON_SMART_TOY,
        Kind::Commands => labels::ICON_TERMINAL,
        Kind::Hooks => labels::ICON_BOLT,
        Kind::Permissions => labels::ICON_SHIELD,
        Kind::Mcp => labels::ICON_EXTENSION,
    }
}

/// Permissões e MCP só abrem o arquivo; as outras seções criam item.
fn section_action(kind: Kind) -> (&'static str, &'static str) {
    match kind {
        Kind::Memory => (labels::ICON_PLUS, labels::HARNESS_MEMORY_NEW),
        Kind::Permissions | Kind::Mcp => (labels::ICON_EDIT, labels::HARNESS_EDIT_FILE),
        _ => (labels::ICON_PLUS, labels::HARNESS_NEW_ITEM),
    }
}

/// Onde avisar de um JSON inválido: `.mcp.json` no MCP; `settings*.json` em Hooks (o
/// arquivo também tem as permissões).
fn section_of_invalid_file(path: &Path) -> Kind {
    if path.to_string_lossy().ends_with(".mcp.json") {
        Kind::Mcp
    } else {
        Kind::Hooks
    }
}

fn section_card<'a>(
    state: &HarnessState,
    workspace: &Path,
    harness: &ProjectHarness,
    kind: Kind,
    width: f32,
) -> Element<'a, Msg> {
    let palette = theme::palette();
    let entries = harness.of(kind);
    let all = state.expanded.contains(&(workspace.to_path_buf(), kind));
    let per_turn = harness.always_on_tokens(Some(kind));
    let (action_icon, action_label) = section_action(kind);

    let mut header = row![
        icon(
            section_icon(kind),
            theme::HARNESS_SECTION_ICON,
            palette.text2
        ),
        Space::new().width(theme::HARNESS_SECTION_ICON_GAP),
        title_text(labels::harness_title(kind)),
        Space::new().width(theme::HARNESS_SECTION_COUNT_GAP),
        label(entries.len().to_string(), theme::SIZE_LABEL, palette.text3),
    ]
    .align_y(Alignment::Center);
    if per_turn > 0 {
        header = header
            .push(Space::new().width(theme::HARNESS_SECTION_PER_TURN_GAP))
            .push(label(
                labels::harness_per_turn(&format_tokens(per_turn)),
                theme::HARNESS_SECTION_PER_TURN_SIZE,
                palette.text4,
            ));
    }
    header = header.push(Space::new().width(Fill)).push(ghost_small(
        Some(action_icon),
        action_label,
        Msg::Create(workspace.to_path_buf(), kind),
    ));

    let mut content = column![
        header,
        Space::new().height(theme::HARNESS_SECTION_DESC_GAP),
        label(
            labels::harness_description(kind),
            theme::SIZE_LABEL,
            palette.text3
        ),
        Space::new().height(theme::HARNESS_SECTION_LIST_GAP),
    ];
    for file in harness
        .invalid_files
        .iter()
        .filter(|file| section_of_invalid_file(file) == kind)
    {
        content = content.push(invalid_row(file));
    }
    if entries.is_empty() {
        content = content.push(
            container(label(
                labels::HARNESS_EMPTY,
                theme::SIZE_LABEL,
                palette.text4,
            ))
            .padding([theme::HARNESS_SECTION_EMPTY_PADDING, 0.0]),
        );
    } else {
        let shown = if all {
            entries.len()
        } else {
            theme::HARNESS_VISIBLE_ENTRIES
        };
        // largura útil de uma linha: o cartão sem o padding dele e o da linha
        let row_width =
            width - theme::HARNESS_SECTION_PADDING.x() - 2.0 * theme::HARNESS_ROW_PADDING[1];
        for entry in entries.iter().take(shown) {
            content = content.push(entry_row(entry, row_width));
        }
        if entries.len() > theme::HARNESS_VISIBLE_ENTRIES {
            let toggle = if all {
                labels::HARNESS_SHOW_LESS.to_owned()
            } else {
                labels::harness_show_all(entries.len())
            };
            content = content.push(ghost_small(
                None,
                &toggle,
                Msg::ToggleAll(workspace.to_path_buf(), kind),
            ));
        }
    }
    container(content)
        .width(Length::Fixed(width))
        .padding(theme::HARNESS_SECTION_PADDING)
        .style(card_style)
        .into()
}

/// Aviso de JSON inválido; clicar abre o arquivo.
fn invalid_row<'a>(file: &Path) -> Element<'a, Msg> {
    let palette = theme::palette();
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    button(
        row![
            icon(
                labels::ICON_ERROR_OUTLINE,
                theme::HARNESS_INVALID_ICON,
                palette.error
            ),
            Space::new().width(theme::HARNESS_INVALID_GAP),
            label(
                labels::harness_invalid_json(&name),
                theme::SIZE_LABEL,
                palette.error
            )
            .width(Fill),
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding(theme::HARNESS_ROW_PADDING)
    .on_press(Msg::Open(file.to_path_buf()))
    .style(hover_style(theme::HARNESS_ROW_RADIUS))
    .into()
}

fn entry_row<'a>(entry: &Entry, row_width: f32) -> Element<'a, Msg> {
    let palette = theme::palette();
    let mono = matches!(entry.kind, Kind::Commands | Kind::Permissions | Kind::Hooks);
    let is_permission = entry.kind == Kind::Permissions;
    // Permissão: a regra é o nome; allow/ask/deny vira a cor do detalhe.
    let detail_color = if is_permission {
        match entry.detail.as_str() {
            "allow" => palette.online,
            "ask" => palette.warn,
            "deny" => palette.error,
            _ => palette.text3,
        }
    } else {
        palette.text3
    };
    let detail = if is_permission {
        labels::harness_rule(&entry.detail).to_owned()
    } else {
        entry.detail.clone()
    };
    let scope = labels::harness_scope(entry.scope);

    // O que não é texto, na ordem do Flutter: 8, [aviso + 6], chip, [6 + cadeado].
    let chip_width = 2.0 * theme::HARNESS_CHIP_PADDING[1]
        + scope.chars().count() as f32 * theme::HARNESS_CHIP_SIZE * BODY_CHAR_RATIO;
    let mut fixed = theme::HARNESS_ENTRY_GAP + chip_width;
    if !entry.issues.is_empty() {
        fixed += theme::HARNESS_ENTRY_ISSUE_ICON + theme::HARNESS_ENTRY_ISSUE_GAP;
    }
    if entry.path.is_none() {
        fixed += theme::HARNESS_ENTRY_LOCK_GAP + theme::HARNESS_ENTRY_LOCK_ICON;
    }
    let text_width = row_width - fixed;
    let name_ratio = if mono {
        MONO_CHAR_RATIO
    } else {
        BODY_CHAR_RATIO
    };
    let name_chars = chars_that_fit(text_width, theme::HARNESS_ENTRY_NAME_SIZE * name_ratio);
    let detail_chars = chars_that_fit(
        text_width,
        theme::HARNESS_ENTRY_DETAIL_SIZE * BODY_CHAR_RATIO,
    );

    let name_text = text(fit(&entry.name, name_chars))
        .font(if mono {
            theme::FONT_MONO
        } else {
            theme::FONT_BODY
        })
        .size(theme::HARNESS_ENTRY_NAME_SIZE)
        .line_height(if mono {
            theme::LINE_HEIGHT_MONO
        } else {
            theme::LINE_HEIGHT_BODY
        })
        .color(palette.text)
        .wrapping(Wrapping::None);
    let mut texts = column![name_text];
    if !detail.is_empty() {
        texts = texts.push(
            label(
                fit(&detail, detail_chars),
                theme::HARNESS_ENTRY_DETAIL_SIZE,
                detail_color,
            )
            .wrapping(Wrapping::None),
        );
    }

    let mut line = row![
        container(texts).width(Fill).clip(true),
        Space::new().width(theme::HARNESS_ENTRY_GAP),
    ]
    .align_y(Alignment::Start);
    if !entry.issues.is_empty() {
        let tip = entry
            .issues
            .iter()
            .map(|issue| labels::harness_issue(*issue))
            .collect::<Vec<_>>()
            .join("\n");
        line = line
            .push(tooltip(
                icon(
                    labels::ICON_WARNING_AMBER,
                    theme::HARNESS_ENTRY_ISSUE_ICON,
                    palette.warn,
                ),
                pane_chrome::tip_bubble(tip),
                tooltip::Position::Bottom,
            ))
            .push(Space::new().width(theme::HARNESS_ENTRY_ISSUE_GAP));
    }
    line = line.push(scope_chip(entry.scope));
    if entry.path.is_none() {
        line = line
            .push(Space::new().width(theme::HARNESS_ENTRY_LOCK_GAP))
            .push(tooltip(
                icon(
                    labels::ICON_LOCK_OUTLINE,
                    theme::HARNESS_ENTRY_LOCK_ICON,
                    palette.text4,
                ),
                pane_chrome::tip_bubble(labels::HARNESS_READ_ONLY),
                tooltip::Position::Bottom,
            ));
    }
    match &entry.path {
        Some(path) => button(line)
            .width(Fill)
            .padding(theme::HARNESS_ROW_PADDING)
            .on_press(Msg::Open(path.clone()))
            .style(hover_style(theme::HARNESS_ROW_RADIUS))
            .into(),
        None => container(line)
            .width(Fill)
            .padding(theme::HARNESS_ROW_PADDING)
            .into(),
    }
}

fn scope_chip<'a>(scope: Scope) -> Element<'a, Msg> {
    let background = match scope {
        Scope::Project => theme::palette().accent_soft,
        Scope::User | Scope::Local => theme::palette().panel3,
    };
    container(label(
        labels::harness_scope(scope),
        theme::HARNESS_CHIP_SIZE,
        theme::palette().text2,
    ))
    .padding(theme::HARNESS_CHIP_PADDING)
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(background)),
        border: Border {
            radius: theme::HARNESS_CHIP_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

/// `GhostButton(size: ButtonSize.small)`: ícone opcional à esquerda e rótulo.
fn ghost_small<'a>(leading: Option<&'static str>, label_text: &str, msg: Msg) -> Element<'a, Msg> {
    let palette = theme::palette();
    let mut content = row![]
        .spacing(theme::HARNESS_GHOST_GAP)
        .align_y(Alignment::Center);
    if let Some(glyph) = leading {
        content = content.push(icon(glyph, theme::HARNESS_GHOST_ICON, palette.text));
    }
    content = content.push(
        text(label_text.to_owned())
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::HARNESS_GHOST_TEXT)
            .color(palette.text),
    );
    button(content)
        .padding(theme::HARNESS_GHOST_PADDING)
        .on_press(msg)
        .style(|_: &Theme, status| button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed).then(
                || {
                    Background::Color(Color {
                        a: theme::HARNESS_GHOST_HOVER_ALPHA,
                        ..theme::palette().panel2
                    })
                },
            ),
            text_color: theme::palette().text,
            border: Border {
                radius: theme::HARNESS_GHOST_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

/// `HoverTap`: transparente, `panel3` ao passar o mouse (só habilitado).
fn hover_style(radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then(|| Background::Color(theme::palette().panel3)),
        text_color: theme::palette().text,
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Cartão: `panel2`, borda `border`, raio 8.
fn card_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme::palette().panel2)),
        border: Border {
            color: theme::palette().border,
            width: 1.0,
            radius: theme::HARNESS_CARD_RADIUS.into(),
        },
        ..Default::default()
    }
}

/// `typo.label` com a cor dada (corpo, altura de linha 1,3).
fn label<'a>(
    content: impl text::IntoFragment<'a>,
    size: f32,
    color: Color,
) -> text::Text<'a, Theme, iced::Renderer> {
    text(content)
        .font(theme::FONT_BODY)
        .size(size)
        .line_height(theme::LINE_HEIGHT_LABEL)
        .color(color)
}

/// `typo.title`: Montserrat semibold 13.
fn title_text<'a>(content: &'static str) -> Element<'a, Msg> {
    text(content)
        .font(theme::FONT_TITLE)
        .size(theme::SIZE_TITLE)
        .color(theme::palette().text)
        .into()
}

/// Ícone Material Symbols numa caixa quadrada de `size` (como o `Icon` do Flutter).
fn icon<'a, M: 'a>(glyph: &'static str, size: f32, color: Color) -> Element<'a, M> {
    text(glyph)
        .font(theme::FONT_ICON)
        .size(size)
        .line_height(1.0)
        .width(size)
        .height(size)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .color(color)
        .into()
}

fn chars_that_fit(width: f32, char_width: f32) -> usize {
    if char_width <= 0.0 || width <= 0.0 {
        return 0;
    }
    (width / char_width) as usize
}

/// Uma linha só, terminando em "…" quando não cabe em `max_chars` (`maxLines: 1` +
/// `TextOverflow.ellipsis` do Flutter; a largura é estimada, não medida).
fn fit(text: &str, max_chars: usize) -> String {
    let first = text.lines().next().unwrap_or_default();
    let multiline = first.len() < text.trim_end().len();
    if first.chars().count() <= max_chars.max(1) && !multiline {
        return first.to_owned();
    }
    let keep = max_chars.max(1).saturating_sub(1);
    let mut out: String = first.chars().take(keep).collect();
    out.push_str(labels::HARNESS_ELLIPSIS);
    out
}

/// `9.4k` / `820`: `toStringAsFixed` do Dart arredonda o empate exato para cima.
fn format_tokens(n: usize) -> String {
    if n < 1000 {
        return n.to_string();
    }
    let thousands = n as f64 / 1000.0;
    if n >= 10000 {
        let rounded = if n % 1000 == 500 {
            n / 1000 + 1
        } else {
            thousands.round() as usize
        };
        return format!("{rounded}k");
    }
    let value = if matches!(n % 1000, 250 | 750) {
        thousands + 0.05
    } else {
        thousands
    };
    format!("{value:.1}k")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_follow_the_dart_formatting() {
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(820), "820");
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1000), "1.0k");
        assert_eq!(format_tokens(9400), "9.4k");
        assert_eq!(format_tokens(1250), "1.3k"); // empate exato sobe
        assert_eq!(format_tokens(1150), "1.1k"); // 1.15 em binário é um pouco menos
        assert_eq!(format_tokens(9999), "10.0k");
        assert_eq!(format_tokens(10000), "10k");
        assert_eq!(format_tokens(10499), "10k");
        assert_eq!(format_tokens(10500), "11k");
        assert_eq!(format_tokens(123456), "123k");
    }

    #[test]
    fn fit_cuts_with_ellipsis_and_keeps_the_first_line() {
        assert_eq!(fit("abc", 3), "abc");
        assert_eq!(fit("abcd", 3), "ab…");
        assert_eq!(fit("a\nb", 10), "a…");
        assert_eq!(fit("abc", 0), "…");
        assert_eq!(fit("", 5), "");
    }

    #[test]
    fn invalid_json_warns_in_mcp_or_hooks() {
        assert_eq!(
            section_of_invalid_file(Path::new("/w/.mcp.json")),
            Kind::Mcp
        );
        assert_eq!(
            section_of_invalid_file(Path::new("/w/.claude/settings.local.json")),
            Kind::Hooks
        );
    }

    #[test]
    fn name_form_rejects_empty_and_taken_names_case_insensitively() {
        let form = |name: &str| NameForm {
            workspace: PathBuf::from("/w"),
            kind: Kind::Skills,
            name: name.to_owned(),
            taken: BTreeSet::from([String::from("deploy")]),
        };
        assert!(!form("  ").valid());
        assert!(!form(" Deploy ").valid());
        assert!(form("outro").valid());
    }

    #[test]
    fn create_asks_the_name_only_for_named_kinds() {
        let mut state = HarnessState::default();
        let ws = PathBuf::from("/w");
        for kind in [Kind::Skills, Kind::Agents, Kind::Commands, Kind::Hooks] {
            let _ = state.update(Msg::Create(ws.clone(), kind));
            assert!(state.name_form.is_some(), "{kind:?}");
            state.name_form = None;
        }
        for kind in [Kind::Memory, Kind::Permissions, Kind::Mcp] {
            let _ = state.update(Msg::Create(ws.clone(), kind));
            assert!(state.name_form.is_none(), "{kind:?}");
        }
    }

    #[test]
    fn taken_names_drop_the_slash_of_commands() {
        let mut state = HarnessState::default();
        let ws = PathBuf::from("/w");
        let entry = Entry {
            kind: Kind::Commands,
            name: String::from("/Git:PR"),
            detail: String::new(),
            scope: Scope::Project,
            path: None,
            issues: Vec::new(),
            always_on_chars: 0,
        };
        state.loaded.insert(
            ws.clone(),
            ProjectHarness {
                entries: vec![entry],
                invalid_files: Vec::new(),
            },
        );
        let _ = state.update(Msg::Create(ws, Kind::Commands));
        let form = state.name_form.as_ref().expect("form");
        assert!(form.taken.contains("git:pr"));
    }

    #[test]
    fn open_returns_the_file_and_toggle_flips_the_section() {
        let mut state = HarnessState::default();
        let (_, open) = state.update(Msg::Open(PathBuf::from("/w/a.md")));
        assert_eq!(open, Some(PathBuf::from("/w/a.md")));
        let ws = PathBuf::from("/w");
        let _ = state.update(Msg::ToggleAll(ws.clone(), Kind::Skills));
        assert!(state.expanded.contains(&(ws.clone(), Kind::Skills)));
        let _ = state.update(Msg::ToggleAll(ws.clone(), Kind::Skills));
        assert!(state.expanded.is_empty());
    }

    #[test]
    fn loaded_clears_loading_and_failure_keeps_nothing() {
        let mut state = HarnessState::default();
        let ws = PathBuf::from("/w");
        state.loading.insert(ws.clone());
        let _ = state.update(Msg::Loaded(ws.clone(), None));
        assert!(!state.is_loading(&ws));
        assert!(state.of(&ws).is_none());
        let _ = state.update(Msg::Loaded(ws.clone(), Some(ProjectHarness::default())));
        assert!(state.of(&ws).is_some());
    }

    #[test]
    fn view_builds_for_every_state() {
        let mut state = HarnessState::default();
        let ws = PathBuf::from("/w");
        let _ = page(&state, &ws, 800.0);
        state.loaded.insert(ws.clone(), ProjectHarness::default());
        let _ = page(&state, &ws, 800.0);
        let _ = page(&state, &ws, 1200.0);
    }
}
