//! Estado do banco no shell (`database_viewmodel.dart` + estado local de `db_panel.dart`):
//! conexões do workspace, caches de schema, árvore aberta, menus, diálogo de conexão e a
//! confirmação de exclusão.
//!
//! Todo acesso a disco e a rede roda em `spawn_blocking`; os resultados voltam por mensagem,
//! marcados com a geração em que foram pedidos (recarregar invalida o que estava em voo).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use gsh_db::schema::{qualified_ident, SchemaColumn, SchemaTable};
use gsh_db::{store, Connection, DbError, Endpoint, Engine, Origin, SecretStore, StoreError};
use iced::Task;

use crate::db_dialog::{ConnectionDialog, DialogMsg, Outcome};

/// Estado de uma carga assíncrona.
#[derive(Debug, Clone, PartialEq)]
pub enum Load<T> {
    Loading,
    Ready(T),
    Failed(DbError),
}

/// Menu aberto (só um por vez).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuState {
    /// Escolha do motor, do botão "+".
    Add,
    /// Menu de uma conexão, do botão "…".
    Conn(String),
}

/// Colunas de uma tabela: conexão, schema e nome.
pub type ColumnsKey = (String, Option<String>, String);

#[derive(Debug, Clone)]
pub enum DbMsg {
    Reload,
    Loaded(u64, Vec<Connection>),
    OpenAddMenu,
    OpenConnMenu(String),
    CloseMenu,
    AddEngine(Engine),
    Edit(String),
    CopyName(String),
    NewQuery(String, Option<SchemaTable>),
    AskDelete(String),
    ConfirmDelete,
    CancelDelete,
    Toggle(String),
    ToggleSchema(String, String),
    ToggleTable(String, SchemaTable),
    Tables(u64, String, Result<Vec<SchemaTable>, DbError>),
    Columns(u64, ColumnsKey, Result<Vec<SchemaColumn>, DbError>),
    Dialog(DialogMsg),
    Persisted(Result<(), StoreError>),
    Noop,
}

/// Pedido do painel ao shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbEffect {
    /// Abrir uma query nova (untitled) na conexão, opcionalmente com este SQL.
    NewQuery { conn: String, sql: Option<String> },
}

pub struct DbState {
    root: Option<PathBuf>,
    /// As conexões de `root` já foram lidas ao menos uma vez.
    loaded: bool,
    secrets: SecretStore,
    generation: u64,
    pub connections: Vec<Connection>,
    pub tables: BTreeMap<String, Load<Vec<SchemaTable>>>,
    pub columns: BTreeMap<ColumnsKey, Load<Vec<SchemaColumn>>>,
    pub expanded: BTreeSet<String>,
    pub open_tables: BTreeMap<String, BTreeSet<String>>,
    pub collapsed_schemas: BTreeMap<String, BTreeSet<String>>,
    pub menu: Option<MenuState>,
    pub dialog: Option<ConnectionDialog>,
    pub confirm_delete: Option<String>,
}

impl DbState {
    pub fn new(secrets: SecretStore) -> Self {
        Self {
            root: None,
            loaded: false,
            secrets,
            generation: 0,
            connections: Vec::new(),
            tables: BTreeMap::new(),
            columns: BTreeMap::new(),
            expanded: BTreeSet::new(),
            open_tables: BTreeMap::new(),
            collapsed_schemas: BTreeMap::new(),
            menu: None,
            dialog: None,
            confirm_delete: None,
        }
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn secrets(&self) -> &SecretStore {
        &self.secrets
    }

    pub fn connection(&self, name: &str) -> Option<&Connection> {
        self.connections.iter().find(|c| c.name == name)
    }

    /// Aponta o painel para a pasta do workspace e recarrega (`setWorkspace(force: true)`).
    pub fn set_workspace(&mut self, root: Option<PathBuf>) -> Task<DbMsg> {
        if self.root != root {
            self.expanded.clear();
            self.open_tables.clear();
            self.collapsed_schemas.clear();
            self.menu = None;
            self.dialog = None;
            self.confirm_delete = None;
        }
        self.root = root;
        self.reload()
    }

    /// Garante as conexões carregadas sem forçar (uma aba de query aberta sem o painel): só lê se
    /// a pasta mudou ou nunca foi lida.
    pub fn ensure_workspace(&mut self, root: Option<PathBuf>) -> Task<DbMsg> {
        if self.loaded && self.root == root {
            return Task::none();
        }
        self.set_workspace(root)
    }

    fn reload(&mut self) -> Task<DbMsg> {
        self.loaded = self.root.is_some();
        self.generation = self.generation.wrapping_add(1);
        self.tables.clear();
        self.columns.clear();
        let Some(root) = self.root.clone() else {
            self.connections.clear();
            return Task::none();
        };
        let generation = self.generation;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || store::load(&root))
                    .await
                    .unwrap_or_default()
            },
            move |connections| DbMsg::Loaded(generation, connections),
        )
    }

    /// Pede as tabelas das conexões abertas depois de uma recarga (as colunas das tabelas abertas
    /// vêm quando a lista chega, em [`Self::reopen_columns`]).
    fn refetch_open(&mut self) -> Task<DbMsg> {
        let open: Vec<String> = self.expanded.iter().cloned().collect();
        Task::batch(open.iter().map(|name| self.ensure_tables(name)))
    }

    /// Depois que as tabelas de `conn` chegam, pede as colunas das que estavam abertas.
    fn reopen_columns(&mut self, conn: &str) -> Task<DbMsg> {
        let Some(keys) = self.open_tables.get(conn).cloned() else {
            return Task::none();
        };
        let reopened: Vec<SchemaTable> = self
            .tables_of(conn)
            .into_iter()
            .filter(|table| keys.contains(&table_key(table)))
            .collect();
        Task::batch(
            reopened
                .iter()
                .map(|table| self.ensure_columns(conn, table)),
        )
    }

    fn tables_of(&self, conn: &str) -> Vec<SchemaTable> {
        match self.tables.get(conn) {
            Some(Load::Ready(tables)) => tables.clone(),
            _ => Vec::new(),
        }
    }

    fn engine_of(&self, name: &str) -> Option<Engine> {
        self.connection(name).map(|c| c.engine)
    }

    /// Carrega as tabelas da conexão se ainda não há cache.
    fn ensure_tables(&mut self, name: &str) -> Task<DbMsg> {
        if self.tables.contains_key(name) || self.root.is_none() {
            return Task::none();
        }
        self.tables.insert(name.to_string(), Load::Loading);
        let generation = self.generation;
        let key = name.to_string();
        self.job(name, |endpoint| gsh_db::driver::tables(&endpoint))
            .map(move |result| DbMsg::Tables(generation, key.clone(), result))
    }

    fn ensure_columns(&mut self, name: &str, table: &SchemaTable) -> Task<DbMsg> {
        let key: ColumnsKey = (name.to_string(), table.schema.clone(), table.name.clone());
        if self.columns.contains_key(&key) || self.root.is_none() {
            return Task::none();
        }
        self.columns.insert(key.clone(), Load::Loading);
        let generation = self.generation;
        let (table_name, schema) = (table.name.clone(), table.schema.clone());
        self.job(name, move |endpoint| {
            gsh_db::driver::columns(&endpoint, &table_name, schema.as_deref())
        })
        .map(move |result| DbMsg::Columns(generation, key.clone(), result))
    }

    /// Roda `work` sobre a conexão `name` (relida do disco, como o serviço do Flutter) fora da
    /// thread de UI.
    fn job<T: Send + 'static>(
        &self,
        name: &str,
        work: impl FnOnce(Endpoint) -> Result<T, DbError> + Send + 'static,
    ) -> Task<Result<T, DbError>> {
        let (root, secrets, name) = (
            self.root.clone().unwrap_or_default(),
            self.secrets.clone(),
            name.to_string(),
        );
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || run_on(&root, &secrets, &name, work)).await
            },
            |joined| joined.unwrap_or_else(|error| Err(DbError::QueryFailed(error.to_string()))),
        )
    }

    pub fn update(&mut self, message: DbMsg) -> (Task<DbMsg>, Option<DbEffect>) {
        match message {
            DbMsg::Reload => return (self.reload(), None),
            DbMsg::Loaded(generation, connections) => {
                if generation == self.generation {
                    self.connections = connections;
                    return (self.refetch_open(), None);
                }
            }
            DbMsg::OpenAddMenu => self.menu = Some(MenuState::Add),
            DbMsg::OpenConnMenu(name) => self.menu = Some(MenuState::Conn(name)),
            DbMsg::CloseMenu => self.menu = None,
            DbMsg::AddEngine(engine) => {
                self.menu = None;
                self.dialog = Some(ConnectionDialog::new(engine, None));
            }
            DbMsg::Edit(name) => {
                self.menu = None;
                if let Some(connection) = self.connection(&name).cloned() {
                    self.dialog = Some(ConnectionDialog::new(connection.engine, Some(connection)));
                }
            }
            DbMsg::CopyName(name) => {
                self.menu = None;
                return (iced::clipboard::write(name), None);
            }
            DbMsg::NewQuery(name, table) => {
                self.menu = None;
                let sql = table.and_then(|table| {
                    let engine = self.engine_of(&name)?;
                    Some(format!(
                        "SELECT * FROM {} LIMIT 100;",
                        qualified_ident(engine, table.schema.as_deref(), &table.name)
                    ))
                });
                return (Task::none(), Some(DbEffect::NewQuery { conn: name, sql }));
            }
            DbMsg::AskDelete(name) => {
                self.menu = None;
                // detectada/local não moram no databases.json: nada a excluir
                if self
                    .connection(&name)
                    .is_some_and(|c| c.origin == Origin::Registered)
                {
                    self.confirm_delete = Some(name);
                }
            }
            DbMsg::ConfirmDelete => {
                if let Some(name) = self.confirm_delete.take() {
                    return (self.remove(&name), None);
                }
            }
            DbMsg::CancelDelete => self.confirm_delete = None,
            DbMsg::Toggle(name) => {
                if self.expanded.remove(&name) {
                    // fechar descarta o estado da árvore, não o cache de tabelas
                    self.open_tables.remove(&name);
                    self.collapsed_schemas.remove(&name);
                } else {
                    self.expanded.insert(name.clone());
                    return (self.ensure_tables(&name), None);
                }
            }
            DbMsg::ToggleSchema(conn, schema) => {
                let set = self.collapsed_schemas.entry(conn).or_default();
                if !set.remove(&schema) {
                    set.insert(schema);
                }
            }
            DbMsg::ToggleTable(conn, table) => {
                let key = table_key(&table);
                let set = self.open_tables.entry(conn.clone()).or_default();
                if !set.remove(&key) {
                    set.insert(key);
                    return (self.ensure_columns(&conn, &table), None);
                }
            }
            DbMsg::Tables(generation, name, result) => {
                if generation == self.generation {
                    self.tables
                        .insert(name.clone(), result.map_or_else(Load::Failed, Load::Ready));
                    return (self.reopen_columns(&name), None);
                }
            }
            DbMsg::Columns(generation, key, result) => {
                if generation == self.generation {
                    self.columns
                        .insert(key, result.map_or_else(Load::Failed, Load::Ready));
                }
            }
            DbMsg::Dialog(message) => return self.update_dialog(message),
            DbMsg::Persisted(result) => {
                if let Err(error) = result {
                    eprintln!("[db] salvar conexões falhou: {error}");
                }
                return (self.reload(), None);
            }
            DbMsg::Noop => {}
        }
        (Task::none(), None)
    }

    fn update_dialog(&mut self, message: DialogMsg) -> (Task<DbMsg>, Option<DbEffect>) {
        let (Some(dialog), Some(root)) = (self.dialog.as_mut(), self.root.clone()) else {
            return (Task::none(), None);
        };
        let (task, outcome) = dialog.update(message, &root, &self.secrets);
        let task = task.map(DbMsg::Dialog);
        let Some(outcome) = outcome else {
            return (task, None);
        };
        self.dialog = None;
        let follow_up = match outcome {
            Outcome::Cancelled => Task::none(),
            Outcome::Saved {
                connection,
                password,
                previous_name,
            } => self.upsert(connection, password, previous_name),
            // só as registradas moram no arquivo
            Outcome::Deleted(connection) if connection.origin == Origin::Registered => {
                self.remove(&connection.name)
            }
            Outcome::Deleted(_) => Task::none(),
        };
        (Task::batch([task, follow_up]), None)
    }

    /// Cria ou atualiza uma conexão registrada. `previous_name` identifica a antiga num rename; a
    /// senha só vai ao cofre quando `save_password`, e sai de lá quando o flag desliga. Editar uma
    /// "detected" a promove a registrada.
    fn upsert(
        &self,
        connection: Connection,
        password: Option<String>,
        previous_name: Option<String>,
    ) -> Task<DbMsg> {
        let Some(root) = self.root.clone() else {
            return Task::none();
        };
        let secrets = self.secrets.clone();
        let mut registered: Vec<Connection> = self
            .connections
            .iter()
            .filter(|c| {
                c.origin == Origin::Registered
                    && c.name != *previous_name.as_deref().unwrap_or(&connection.name)
            })
            .cloned()
            .collect();
        registered.push(connection.clone());
        persist(move || {
            apply_upsert(
                &root,
                &secrets,
                &registered,
                &connection,
                password,
                previous_name,
            )
        })
    }

    fn remove(&self, name: &str) -> Task<DbMsg> {
        let Some(root) = self.root.clone() else {
            return Task::none();
        };
        let secrets = self.secrets.clone();
        let registered: Vec<Connection> = self
            .connections
            .iter()
            .filter(|c| c.origin == Origin::Registered && c.name != name)
            .cloned()
            .collect();
        let name = name.to_string();
        persist(move || apply_remove(&root, &secrets, &registered, &name))
    }
}

/// Grava as registradas (já com `connection`) e acerta o cofre: rename migra a senha, desligar
/// "Salvar senha" a apaga, campo vazio mantém a atual.
fn apply_upsert(
    root: &Path,
    secrets: &SecretStore,
    registered: &[Connection],
    connection: &Connection,
    password: Option<String>,
    previous_name: Option<String>,
) -> Result<(), StoreError> {
    store::save(root, registered)?;
    let key_root = root.to_string_lossy();
    let old = previous_name.as_deref().unwrap_or(&connection.name);
    if old != connection.name {
        // a senha guardada migra antes de a antiga sair; editar sem redigitar NUNCA a perde
        let existing = secrets.read(&key_root, old);
        let typed_nothing = password.as_deref().is_none_or(str::is_empty);
        if let (true, Some(existing), true) = (connection.save_password, existing, typed_nothing) {
            secrets.write(&key_root, &connection.name, &existing)?;
        }
        secrets.delete(&key_root, old)?;
    }
    match password.as_deref() {
        _ if !connection.save_password => secrets.delete(&key_root, &connection.name),
        Some(typed) if !typed.is_empty() => secrets.write(&key_root, &connection.name, typed),
        _ => Ok(()),
    }
}

fn apply_remove(
    root: &Path,
    secrets: &SecretStore,
    registered: &[Connection],
    name: &str,
) -> Result<(), StoreError> {
    store::save(root, registered)?;
    secrets.delete(&root.to_string_lossy(), name)
}

fn persist(work: impl FnOnce() -> Result<(), StoreError> + Send + 'static) -> Task<DbMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(work)
                .await
                .unwrap_or_else(|error| {
                    Err(StoreError::Io {
                        op: "background task",
                        detail: error.to_string(),
                    })
                })
        },
        DbMsg::Persisted,
    )
}

/// Resolve `name` entre as conexões do disco e roda `work` no endpoint dela. Conexão que sumiu é
/// `UnknownConnection` com a lista das que existem.
pub fn run_on<T>(
    root: &Path,
    secrets: &SecretStore,
    name: &str,
    work: impl FnOnce(Endpoint) -> Result<T, DbError>,
) -> Result<T, DbError> {
    let all = store::load(root);
    let Some(connection) = all.iter().find(|c| c.name == name) else {
        return Err(DbError::UnknownConnection {
            name: name.to_string(),
            available: all.iter().map(|c| c.name.clone()).collect(),
        });
    };
    let password = gsh_db::resolve_password(connection, root, secrets);
    work(Endpoint::resolve(connection, root, password)?)
}

/// Chave de uma tabela na árvore: schema e nome (homônimas em schemas distintos não colidem).
pub fn table_key(table: &SchemaTable) -> String {
    format!("{}.{}", table.schema.as_deref().unwrap_or(""), table.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_db::connection::Access;

    fn state() -> (tempfile::TempDir, DbState) {
        let dir = tempfile::tempdir().unwrap();
        let secrets = SecretStore::new(dir.path().join("secrets.json"));
        (dir, DbState::new(secrets))
    }

    fn sqlite(name: &str, path: &str) -> Connection {
        Connection::sqlite(name, path, Origin::Registered, Access::Read, true)
    }

    fn table(schema: Option<&str>, name: &str) -> SchemaTable {
        SchemaTable {
            name: name.into(),
            schema: schema.map(str::to_string),
            is_view: false,
        }
    }

    #[test]
    fn table_keys_separate_homonyms_across_schemas() {
        assert_ne!(
            table_key(&table(Some("a"), "t")),
            table_key(&table(Some("b"), "t"))
        );
        assert_eq!(table_key(&table(None, "t")), ".t");
    }

    #[test]
    fn new_query_from_a_table_qualifies_and_quotes_it() {
        let (_dir, mut db) = state();
        db.connections = vec![Connection::from_json(
            &serde_json::json!({"name": "pg", "url": "postgres://u@h/db"}),
            Origin::Registered,
        )
        .unwrap()];
        let (_t, effect) = db.update(DbMsg::NewQuery(
            "pg".into(),
            Some(table(Some("sales"), "Orders")),
        ));
        assert_eq!(
            effect,
            Some(DbEffect::NewQuery {
                conn: "pg".into(),
                sql: Some("SELECT * FROM \"sales\".\"Orders\" LIMIT 100;".into())
            })
        );
        let (_t, effect) = db.update(DbMsg::NewQuery("pg".into(), None));
        assert_eq!(
            effect,
            Some(DbEffect::NewQuery {
                conn: "pg".into(),
                sql: None
            })
        );
    }

    #[test]
    fn toggling_a_tile_keeps_the_table_cache_but_drops_the_tree_state() {
        let (_dir, mut db) = state();
        db.connections = vec![sqlite("a", "a.db")];
        db.tables
            .insert("a".into(), Load::Ready(vec![table(None, "t")]));
        let _ = db.update(DbMsg::Toggle("a".into()));
        assert!(db.expanded.contains("a"));
        let _ = db.update(DbMsg::ToggleSchema("a".into(), "s".into()));
        assert!(db.collapsed_schemas["a"].contains("s"));
        let _ = db.update(DbMsg::Toggle("a".into()));
        assert!(!db.expanded.contains("a"));
        assert!(!db.collapsed_schemas.contains_key("a"));
        assert!(db.tables.contains_key("a"));
    }

    #[test]
    fn only_registered_connections_can_be_deleted() {
        let (_dir, mut db) = state();
        let mut local = sqlite("l", "l.db");
        local.origin = Origin::Local;
        db.connections = vec![sqlite("r", "r.db"), local];
        let _ = db.update(DbMsg::AskDelete("l".into()));
        assert_eq!(db.confirm_delete, None);
        let _ = db.update(DbMsg::AskDelete("r".into()));
        assert_eq!(db.confirm_delete.as_deref(), Some("r"));
        let _ = db.update(DbMsg::CancelDelete);
        assert_eq!(db.confirm_delete, None);
    }

    #[test]
    fn stale_results_are_dropped_after_a_reload() {
        let (_dir, mut db) = state();
        db.root = Some(PathBuf::from("/none"));
        let old = db.generation;
        let _ = db.reload();
        let _ = db.update(DbMsg::Tables(old, "a".into(), Ok(vec![table(None, "t")])));
        assert!(db.tables.is_empty());
        let current = db.generation;
        let _ = db.update(DbMsg::Tables(
            current,
            "a".into(),
            Ok(vec![table(None, "t")]),
        ));
        assert!(db.tables.contains_key("a"));
    }

    #[test]
    fn run_on_reports_unknown_connections_with_the_available_names() {
        let dir = tempfile::tempdir().unwrap();
        store::save(dir.path(), &[sqlite("one", "x.db"), sqlite("two", "y.db")]).unwrap();
        let secrets = SecretStore::new(dir.path().join("s.json"));
        let error = run_on(dir.path(), &secrets, "nope", |_| Ok(())).unwrap_err();
        assert_eq!(
            error,
            DbError::UnknownConnection {
                name: "nope".into(),
                available: vec!["one".into(), "two".into()]
            }
        );
    }

    fn saved(name: &str, save_password: bool) -> Connection {
        let mut conn = sqlite(name, "a.db");
        conn.save_password = save_password;
        conn
    }

    #[test]
    fn rename_without_retyping_moves_the_password() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let secrets = SecretStore::new(root.join("s.json"));
        let key = root.to_string_lossy().into_owned();
        store::save(root, &[saved("old", true)]).unwrap();
        secrets.write(&key, "old", "pw").unwrap();

        let renamed = saved("new", true);
        apply_upsert(
            root,
            &secrets,
            std::slice::from_ref(&renamed),
            &renamed,
            None,
            Some("old".into()),
        )
        .unwrap();
        assert_eq!(secrets.read(&key, "new").as_deref(), Some("pw"));
        assert_eq!(secrets.read(&key, "old"), None);
        assert_eq!(store::load(root)[0].name, "new");
    }

    #[test]
    fn typing_overwrites_empty_keeps_and_unchecking_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let secrets = SecretStore::new(root.join("s.json"));
        let key = root.to_string_lossy().into_owned();
        let conn = saved("c", true);
        let all = std::slice::from_ref(&conn);

        apply_upsert(root, &secrets, all, &conn, Some("one".into()), None).unwrap();
        assert_eq!(secrets.read(&key, "c").as_deref(), Some("one"));
        apply_upsert(
            root,
            &secrets,
            all,
            &conn,
            Some(String::new()),
            Some("c".into()),
        )
        .unwrap();
        assert_eq!(secrets.read(&key, "c").as_deref(), Some("one"));
        apply_upsert(
            root,
            &secrets,
            all,
            &conn,
            Some("two".into()),
            Some("c".into()),
        )
        .unwrap();
        assert_eq!(secrets.read(&key, "c").as_deref(), Some("two"));

        let unchecked = saved("c", false);
        apply_upsert(
            root,
            &secrets,
            std::slice::from_ref(&unchecked),
            &unchecked,
            None,
            Some("c".into()),
        )
        .unwrap();
        assert_eq!(secrets.read(&key, "c"), None);
    }

    #[test]
    fn remove_drops_the_entry_and_its_password() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let secrets = SecretStore::new(root.join("s.json"));
        let key = root.to_string_lossy().into_owned();
        store::save(root, &[saved("a", true), saved("b", false)]).unwrap();
        secrets.write(&key, "a", "pw").unwrap();
        apply_remove(root, &secrets, &[saved("b", false)], "a").unwrap();
        assert_eq!(secrets.read(&key, "a"), None);
        let names: Vec<_> = store::load(root).into_iter().map(|c| c.name).collect();
        assert_eq!(names, ["b"]);
    }
}
