//! Estado do banco no shell (`database_viewmodel.dart` + estado local de `db_panel.dart`):
//! conexões do workspace, caches de schema, árvore aberta, menus, diálogo de conexão e a
//! confirmação de exclusão.
//!
//! Todo acesso a disco e a rede roda em `spawn_blocking`; os resultados voltam por mensagem,
//! marcados com a geração em que foram pedidos (recarregar invalida o que estava em voo).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use gsh_db::mongo_browse;
use gsh_db::mongo_store::MongoDatabaseStore;
use gsh_db::schema::{qualified_ident, SchemaColumn, SchemaTable};
use gsh_db::ssh::SshTunnelManager;
use gsh_db::ssh_keys::SshHostKeyStore;
use gsh_db::{store, Connection, DbError, Endpoint, Engine, Origin, SecretStore, StoreError};
use iced::Task;
use zeroize::Zeroize;

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

/// Coleções de um database Mongo: conexão e database (`""` quando fixo na URL).
pub type MongoCollectionsKey = (String, String);

#[derive(Debug, Clone)]
pub enum DbMsg {
    Reload,
    Loaded(u64, Vec<Connection>, BTreeMap<String, String>),
    OpenAddMenu,
    OpenConnMenu(String),
    CloseMenu,
    AddEngine(Engine),
    Edit(String),
    CopyName(String),
    NewQuery(String, Option<SchemaTable>),
    BrowseRedis(String),
    AskDelete(String),
    ConfirmDelete,
    CancelDelete,
    Toggle(String),
    ToggleSchema(String, String),
    ToggleTable(String, SchemaTable),
    ToggleMongoDatabase(String, String),
    OpenMongoCollection(String, String, String),
    Tables(u64, String, Result<Vec<SchemaTable>, DbError>),
    Columns(u64, ColumnsKey, Result<Vec<SchemaColumn>, DbError>),
    MongoDatabases(u64, String, Result<Vec<String>, DbError>),
    MongoCollections(u64, MongoCollectionsKey, Result<Vec<String>, DbError>),
    MongoDbSelected(String, String, Result<(), StoreError>),
    Dialog(DialogMsg),
    Persisted(Result<(), StoreError>),
    Noop,
}

/// Pedido do painel ao shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbEffect {
    /// Abrir uma query nova (untitled) na conexão, opcionalmente com este SQL.
    NewQuery { conn: String, sql: Option<String> },
    /// Abrir o navegador de chaves Redis.
    BrowseRedis { conn: String },
    /// Abrir o navegador de coleções MongoDB.
    BrowseMongo { conn: String, collection: String },
    /// Operação barrou em host key desconhecida ou chave SSH com passphrase.
    SshPromptNeeded {
        conn: String,
        error: DbError,
        from_dialog: bool,
    },
}

pub struct DbState {
    root: Option<PathBuf>,
    /// As conexões de `root` já foram lidas ao menos uma vez.
    loaded: bool,
    secrets: SecretStore,
    mongo_store: MongoDatabaseStore,
    host_keys: SshHostKeyStore,
    generation: u64,
    pub connections: Vec<Connection>,
    pub tables: BTreeMap<String, Load<Vec<SchemaTable>>>,
    pub columns: BTreeMap<ColumnsKey, Load<Vec<SchemaColumn>>>,
    pub mongo_databases: BTreeMap<String, Load<Vec<String>>>,
    pub mongo_open_dbs: BTreeMap<String, BTreeSet<String>>,
    pub mongo_collections: BTreeMap<MongoCollectionsKey, Load<Vec<String>>>,
    pub mongo_selected_db: BTreeMap<String, String>,
    pub expanded: BTreeSet<String>,
    pub open_tables: BTreeMap<String, BTreeSet<String>>,
    pub collapsed_schemas: BTreeMap<String, BTreeSet<String>>,
    pub menu: Option<MenuState>,
    pub dialog: Option<ConnectionDialog>,
    pub confirm_delete: Option<String>,
    /// Passphrases em memória por sessão (`(ssh_endpoint, key_path) -> passphrase`).
    pub ssh_passphrases: BTreeMap<(String, String), String>,
}

impl Drop for DbState {
    fn drop(&mut self) {
        for pass in self.ssh_passphrases.values_mut() {
            pass.zeroize();
        }
    }
}

impl DbState {
    pub fn new(secrets: SecretStore) -> Self {
        Self::with_stores(
            secrets,
            MongoDatabaseStore::system(),
            SshHostKeyStore::system(),
        )
    }

    pub fn with_stores(
        secrets: SecretStore,
        mongo_store: MongoDatabaseStore,
        host_keys: SshHostKeyStore,
    ) -> Self {
        Self {
            root: None,
            loaded: false,
            secrets,
            mongo_store,
            host_keys,
            generation: 0,
            connections: Vec::new(),
            tables: BTreeMap::new(),
            columns: BTreeMap::new(),
            mongo_databases: BTreeMap::new(),
            mongo_open_dbs: BTreeMap::new(),
            mongo_collections: BTreeMap::new(),
            mongo_selected_db: BTreeMap::new(),
            expanded: BTreeSet::new(),
            open_tables: BTreeMap::new(),
            collapsed_schemas: BTreeMap::new(),
            menu: None,
            dialog: None,
            confirm_delete: None,
            ssh_passphrases: BTreeMap::new(),
        }
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn secrets(&self) -> &SecretStore {
        &self.secrets
    }

    pub fn mongo_store(&self) -> &MongoDatabaseStore {
        &self.mongo_store
    }

    pub fn host_keys(&self) -> &SshHostKeyStore {
        &self.host_keys
    }

    pub fn cache_ssh_passphrase(&mut self, endpoint: String, key_path: String, passphrase: String) {
        self.ssh_passphrases
            .insert((endpoint, key_path), passphrase);
    }

    pub fn cached_ssh_passphrase(&self, conn: &Connection) -> Option<String> {
        let ssh = conn.ssh.as_ref()?;
        self.ssh_passphrases
            .get(&(ssh.endpoint(), ssh.key_path.clone()))
            .cloned()
    }

    pub fn mongo_needs_database(conn: &Connection) -> bool {
        conn.engine == Engine::Mongo && conn.database().is_empty()
    }

    pub fn mongo_database(&self, conn: &Connection) -> Option<String> {
        if conn.engine != Engine::Mongo {
            return None;
        }
        let from_url = conn.database();
        if !from_url.is_empty() {
            return Some(from_url);
        }
        self.mongo_selected_db.get(&conn.name).cloned()
    }

    /// Limpa erros de carregamento na árvore após confiar host key ou destravar passphrase SSH e
    /// rebusca nós abertos.
    pub fn retry_open_trees(&mut self) -> Task<DbMsg> {
        self.tables
            .retain(|_, load| !matches!(load, Load::Failed(_)));
        self.columns
            .retain(|_, load| !matches!(load, Load::Failed(_)));
        self.mongo_databases
            .retain(|_, load| !matches!(load, Load::Failed(_)));
        self.mongo_collections
            .retain(|_, load| !matches!(load, Load::Failed(_)));
        self.refetch_open()
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
            self.mongo_open_dbs.clear();
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
        self.mongo_databases.clear();
        self.mongo_collections.clear();
        self.mongo_selected_db.clear();
        let Some(root) = self.root.clone() else {
            self.connections.clear();
            return Task::none();
        };
        let generation = self.generation;
        let mongo_store = self.mongo_store.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let loaded = store::load(&root);
                    let key_root = root.to_string_lossy();
                    let mut selected = BTreeMap::new();
                    for conn in &loaded {
                        if conn.engine == Engine::Mongo {
                            if let Some(db) = mongo_store.selected(&key_root, &conn.name) {
                                selected.insert(conn.name.clone(), db);
                            }
                        }
                    }
                    (loaded, selected)
                })
                .await
                .unwrap_or_default()
            },
            move |(connections, selected)| DbMsg::Loaded(generation, connections, selected),
        )
    }

    /// Pede as tabelas/databases das conexões abertas depois de uma recarga.
    fn refetch_open(&mut self) -> Task<DbMsg> {
        let open: Vec<String> = self.expanded.iter().cloned().collect();
        let mut tasks: Vec<Task<DbMsg>> = Vec::new();
        for name in open {
            match self.engine_of(&name) {
                Some(e) if e.is_sql() => tasks.push(self.ensure_tables(&name)),
                Some(Engine::Mongo) => tasks.push(self.ensure_mongo(&name)),
                _ => {}
            }
        }
        Task::batch(tasks)
    }

    fn ensure_mongo(&mut self, name: &str) -> Task<DbMsg> {
        let Some(conn) = self.connection(name).cloned() else {
            return Task::none();
        };
        if Self::mongo_needs_database(&conn) {
            let mut tasks = vec![self.ensure_mongo_databases(name)];
            if let Some(current) = self.mongo_selected_db.get(name).cloned() {
                self.mongo_open_dbs
                    .entry(name.to_string())
                    .or_default()
                    .insert(current.clone());
                tasks.push(self.ensure_mongo_collections(name, &current));
            }
            Task::batch(tasks)
        } else {
            self.mongo_open_dbs
                .entry(name.to_string())
                .or_default()
                .insert(String::new());
            self.ensure_mongo_collections(name, "")
        }
    }

    fn ensure_mongo_databases(&mut self, name: &str) -> Task<DbMsg> {
        if self.mongo_databases.contains_key(name) || self.root.is_none() {
            return Task::none();
        }
        self.mongo_databases.insert(name.to_string(), Load::Loading);
        let generation = self.generation;
        let key = name.to_string();
        self.job(name, |endpoint| mongo_browse::list_databases(&endpoint))
            .map(move |result| DbMsg::MongoDatabases(generation, key.clone(), result))
    }

    fn ensure_mongo_collections(&mut self, name: &str, database: &str) -> Task<DbMsg> {
        let key: MongoCollectionsKey = (name.to_string(), database.to_string());
        if self.mongo_collections.contains_key(&key) || self.root.is_none() {
            return Task::none();
        }
        self.mongo_collections.insert(key.clone(), Load::Loading);
        let generation = self.generation;
        let db_arg = (!database.is_empty()).then(|| database.to_string());
        self.job(name, move |endpoint| {
            mongo_browse::list_collections(&endpoint, db_arg.as_deref())
        })
        .map(move |result| DbMsg::MongoCollections(generation, key.clone(), result))
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
        let (root, secrets, passphrases, name) = (
            self.root.clone().unwrap_or_default(),
            self.secrets.clone(),
            self.ssh_passphrases.clone(),
            name.to_string(),
        );
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    run_on_with_ssh(&root, &secrets, &name, &passphrases, work)
                })
                .await
            },
            |joined| joined.unwrap_or_else(|error| Err(DbError::QueryFailed(error.to_string()))),
        )
    }

    pub fn update(&mut self, message: DbMsg) -> (Task<DbMsg>, Option<DbEffect>) {
        match message {
            DbMsg::Reload => return (self.reload(), None),
            DbMsg::Loaded(generation, connections, selected) => {
                if generation == self.generation {
                    self.connections = connections;
                    self.mongo_selected_db = selected;
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
            DbMsg::BrowseRedis(name) => {
                self.menu = None;
                return (Task::none(), Some(DbEffect::BrowseRedis { conn: name }));
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
                    self.mongo_open_dbs.remove(&name);
                } else {
                    self.expanded.insert(name.clone());
                    let task = match self.engine_of(&name) {
                        Some(Engine::Mongo) => self.ensure_mongo(&name),
                        Some(e) if e.is_sql() => self.ensure_tables(&name),
                        _ => Task::none(),
                    };
                    return (task, None);
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
            DbMsg::ToggleMongoDatabase(conn, database) => {
                let set = self.mongo_open_dbs.entry(conn.clone()).or_default();
                if !set.remove(&database) {
                    set.insert(database.clone());
                    return (self.ensure_mongo_collections(&conn, &database), None);
                }
            }
            DbMsg::OpenMongoCollection(conn, database, collection) => {
                let mut persist_task = Task::none();
                if !database.is_empty()
                    && self.mongo_selected_db.get(&conn).map(String::as_str)
                        != Some(database.as_str())
                {
                    self.mongo_selected_db
                        .insert(conn.clone(), database.clone());
                    if let Some(root) = self.root.clone() {
                        let key_root = root.to_string_lossy().into_owned();
                        let store = self.mongo_store.clone();
                        let c = conn.clone();
                        let c2 = conn.clone();
                        let d = database.clone();
                        let d2 = database.clone();
                        persist_task = Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || store.select(&key_root, &c, &d))
                                    .await
                                    .unwrap_or_else(|e| {
                                        Err(StoreError::Io {
                                            op: "mongo store",
                                            detail: e.to_string(),
                                        })
                                    })
                            },
                            move |res| DbMsg::MongoDbSelected(c2.clone(), d2.clone(), res),
                        );
                    }
                }
                return (
                    persist_task,
                    Some(DbEffect::BrowseMongo { conn, collection }),
                );
            }
            DbMsg::MongoDbSelected(_, _, res) => {
                if let Err(err) = res {
                    eprintln!("[db] salvar database mongo falhou: {err}");
                }
            }
            DbMsg::Tables(generation, name, result) => {
                if generation == self.generation {
                    let prompt_err = result
                        .as_ref()
                        .err()
                        .filter(|e| needs_ssh_prompt(e))
                        .cloned();
                    self.tables
                        .insert(name.clone(), result.map_or_else(Load::Failed, Load::Ready));
                    if let Some(error) = prompt_err {
                        return (
                            Task::none(),
                            Some(DbEffect::SshPromptNeeded {
                                conn: name,
                                error,
                                from_dialog: false,
                            }),
                        );
                    }
                    return (self.reopen_columns(&name), None);
                }
            }
            DbMsg::Columns(generation, key, result) => {
                if generation == self.generation {
                    self.columns
                        .insert(key, result.map_or_else(Load::Failed, Load::Ready));
                }
            }
            DbMsg::MongoDatabases(generation, name, result) => {
                if generation == self.generation {
                    let prompt_err = result
                        .as_ref()
                        .err()
                        .filter(|e| needs_ssh_prompt(e))
                        .cloned();
                    self.mongo_databases
                        .insert(name.clone(), result.map_or_else(Load::Failed, Load::Ready));
                    if let Some(error) = prompt_err {
                        return (
                            Task::none(),
                            Some(DbEffect::SshPromptNeeded {
                                conn: name,
                                error,
                                from_dialog: false,
                            }),
                        );
                    }
                }
            }
            DbMsg::MongoCollections(generation, key, result) => {
                if generation == self.generation {
                    let prompt_err = result
                        .as_ref()
                        .err()
                        .filter(|e| needs_ssh_prompt(e))
                        .cloned();
                    let conn_name = key.0.clone();
                    self.mongo_collections
                        .insert(key, result.map_or_else(Load::Failed, Load::Ready));
                    if let Some(error) = prompt_err {
                        return (
                            Task::none(),
                            Some(DbEffect::SshPromptNeeded {
                                conn: conn_name,
                                error,
                                from_dialog: false,
                            }),
                        );
                    }
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
        if let DialogMsg::Tested(Err(ref err)) = message {
            if needs_ssh_prompt(err) {
                let conn_name = self
                    .dialog
                    .as_ref()
                    .and_then(|d| d.built_ssh_info())
                    .map(|(n, _, _)| n)
                    .unwrap_or_default();
                let effect = DbEffect::SshPromptNeeded {
                    conn: conn_name,
                    error: err.clone(),
                    from_dialog: true,
                };
                if let (Some(dialog), Some(root)) = (self.dialog.as_mut(), self.root.clone()) {
                    let _ = dialog.update(message, &root, &self.secrets);
                }
                return (Task::none(), Some(effect));
            }
        }
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
                ssh_passphrase,
                previous_name,
            } => {
                if let (Some(ssh), Some(pass)) = (
                    &connection.ssh,
                    ssh_passphrase.as_deref().filter(|p| !p.is_empty()),
                ) {
                    if ssh.save_passphrase {
                        // Also keep in memory for immediate reuse during this session.
                        self.ssh_passphrases
                            .insert((ssh.endpoint(), ssh.key_path.clone()), pass.to_string());
                    } else {
                        self.ssh_passphrases
                            .insert((ssh.endpoint(), ssh.key_path.clone()), pass.to_string());
                    }
                }
                self.upsert(connection, password, ssh_passphrase, previous_name)
            }
            // só as registradas moram no arquivo
            Outcome::Deleted(connection) if connection.origin == Origin::Registered => {
                if let Some(ssh) = &connection.ssh {
                    self.ssh_passphrases
                        .remove(&(ssh.endpoint(), ssh.key_path.clone()));
                }
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
        ssh_passphrase: Option<String>,
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
            apply_upsert_with_ssh(
                &root,
                &secrets,
                &registered,
                &connection,
                password,
                ssh_passphrase,
                previous_name,
            )
        })
    }

    fn remove(&self, name: &str) -> Task<DbMsg> {
        let Some(root) = self.root.clone() else {
            return Task::none();
        };
        let secrets = self.secrets.clone();
        let host_keys = self.host_keys.clone();
        let ssh_endpoint = self
            .connection(name)
            .and_then(|c| c.ssh.as_ref())
            .map(|s| s.endpoint());
        let registered: Vec<Connection> = self
            .connections
            .iter()
            .filter(|c| c.origin == Origin::Registered && c.name != name)
            .cloned()
            .collect();
        let name = name.to_string();
        persist(move || {
            apply_remove_with_ssh(
                &root,
                &secrets,
                &host_keys,
                ssh_endpoint.as_deref(),
                &registered,
                &name,
            )
        })
    }
}

pub fn needs_ssh_prompt(error: &DbError) -> bool {
    matches!(
        error,
        DbError::SshHostKeyUnknown { .. } | DbError::SshCredentialRequired(_)
    )
}

/// Grava as registradas (já com `connection`) e acerta o cofre: rename migra a senha, desligar
/// "Salvar senha" a apaga, campo vazio mantém a atual.
#[cfg(test)]
fn apply_upsert(
    root: &Path,
    secrets: &SecretStore,
    registered: &[Connection],
    connection: &Connection,
    password: Option<String>,
    previous_name: Option<String>,
) -> Result<(), StoreError> {
    apply_upsert_with_ssh(
        root,
        secrets,
        registered,
        connection,
        password,
        None,
        previous_name,
    )
}

fn apply_upsert_with_ssh(
    root: &Path,
    secrets: &SecretStore,
    registered: &[Connection],
    connection: &Connection,
    password: Option<String>,
    ssh_passphrase: Option<String>,
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

        let old_ssh = SecretStore::ssh_key_for(&key_root, old);
        let new_ssh = SecretStore::ssh_key_for(&key_root, &connection.name);
        let existing_ssh = secrets.read_raw(&old_ssh);
        let typed_no_ssh = ssh_passphrase.as_deref().is_none_or(str::is_empty);
        let save_ssh = connection.ssh.as_ref().is_some_and(|s| s.save_passphrase);
        if let (true, Some(existing_ssh), true) = (save_ssh, existing_ssh, typed_no_ssh) {
            secrets.write_raw(&new_ssh, &existing_ssh)?;
        }
        secrets.delete_raw(&old_ssh)?;
    }
    match password.as_deref() {
        _ if !connection.save_password => secrets.delete(&key_root, &connection.name)?,
        Some(typed) if !typed.is_empty() => secrets.write(&key_root, &connection.name, typed)?,
        _ => {}
    }

    let ssh_key = SecretStore::ssh_key_for(&key_root, &connection.name);
    let save_ssh = connection.ssh.as_ref().is_some_and(|s| s.save_passphrase);
    if !save_ssh {
        secrets.delete_raw(&ssh_key)?;
    } else if let Some(pass) = ssh_passphrase.as_deref().filter(|p| !p.is_empty()) {
        secrets.write_raw(&ssh_key, pass)?;
    }
    Ok(())
}

#[cfg(test)]
fn apply_remove(
    root: &Path,
    secrets: &SecretStore,
    registered: &[Connection],
    name: &str,
) -> Result<(), StoreError> {
    apply_remove_with_ssh(
        root,
        secrets,
        &SshHostKeyStore::system(),
        None,
        registered,
        name,
    )
}

fn apply_remove_with_ssh(
    root: &Path,
    secrets: &SecretStore,
    host_keys: &SshHostKeyStore,
    ssh_endpoint: Option<&str>,
    registered: &[Connection],
    name: &str,
) -> Result<(), StoreError> {
    store::save(root, registered)?;
    let key_root = root.to_string_lossy();
    secrets.delete(&key_root, name)?;
    secrets.delete_raw(&SecretStore::ssh_key_for(&key_root, name))?;
    if let Some(endpoint) = ssh_endpoint {
        let _ = host_keys.forget(endpoint);
    }
    Ok(())
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

/// Resolve `name` entre as conexões do disco e roda `work` no endpoint dela.
pub fn run_on<T>(
    root: &Path,
    secrets: &SecretStore,
    name: &str,
    work: impl FnOnce(Endpoint) -> Result<T, DbError>,
) -> Result<T, DbError> {
    run_on_with_ssh(root, secrets, name, &BTreeMap::new(), work)
}

/// Resolve `name` entre as conexões do disco, abrindo túnel SSH se houver (com passphrase da
/// sessão ou cofre) e rodando `work` no endpoint resolvido.
pub fn run_on_with_ssh<T>(
    root: &Path,
    secrets: &SecretStore,
    name: &str,
    ssh_passphrases: &BTreeMap<(String, String), String>,
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
    let passphrase = connection.ssh.as_ref().and_then(|ssh| {
        ssh_passphrases
            .get(&(ssh.endpoint(), ssh.key_path.clone()))
            .cloned()
            .or_else(|| {
                if ssh.save_passphrase {
                    secrets.read_raw(&SecretStore::ssh_key_for(
                        &root.to_string_lossy(),
                        &connection.name,
                    ))
                } else {
                    None
                }
            })
    });
    let endpoint = Endpoint::resolve_with_tunnel(
        connection,
        root,
        password,
        passphrase.as_deref(),
        SshTunnelManager::global(),
    )?;
    work(endpoint)
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
        let names: Vec<_> = store::load(root)
            .into_iter()
            .map(|c| c.name.clone())
            .collect();
        assert_eq!(names, ["b"]);
    }

    #[test]
    fn redis_browse_and_mongo_collection_emit_expected_effects() {
        let (_dir, mut db) = state();
        let (_t, effect) = db.update(DbMsg::BrowseRedis("cache".into()));
        assert_eq!(
            effect,
            Some(DbEffect::BrowseRedis {
                conn: "cache".into()
            })
        );

        let (_t, effect) = db.update(DbMsg::OpenMongoCollection(
            "atlas".into(),
            "prod".into(),
            "users".into(),
        ));
        assert_eq!(
            db.mongo_selected_db.get("atlas").map(String::as_str),
            Some("prod")
        );
        assert_eq!(
            effect,
            Some(DbEffect::BrowseMongo {
                conn: "atlas".into(),
                collection: "users".into(),
            })
        );
    }

    #[test]
    fn ssh_passphrase_migrates_and_host_key_forgets_on_remove() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let secrets = SecretStore::new(root.join("s.json"));
        let host_keys = SshHostKeyStore::new(root.join("hosts.json"));
        host_keys.trust("u@b:22", "SHA256:xyz").unwrap();
        let key_root = root.to_string_lossy().into_owned();
        let conn = Connection::from_json(
            &serde_json::json!({
                "name": "pg1",
                "url": "postgres://u@h/db",
                "ssh": {"host": "b", "port": 22, "user": "u", "keyPath": "k", "savePassphrase": true}
            }),
            Origin::Registered,
        )
        .unwrap();

        apply_upsert_with_ssh(
            root,
            &secrets,
            std::slice::from_ref(&conn),
            &conn,
            None,
            Some("ssh-pass".into()),
            None,
        )
        .unwrap();
        assert_eq!(
            secrets
                .read_raw(&SecretStore::ssh_key_for(&key_root, "pg1"))
                .as_deref(),
            Some("ssh-pass")
        );

        let mut renamed = conn.clone();
        renamed.name = "pg2".into();
        apply_upsert_with_ssh(
            root,
            &secrets,
            std::slice::from_ref(&renamed),
            &renamed,
            None,
            None,
            Some("pg1".into()),
        )
        .unwrap();
        assert_eq!(
            secrets.read_raw(&SecretStore::ssh_key_for(&key_root, "pg1")),
            None
        );
        assert_eq!(
            secrets
                .read_raw(&SecretStore::ssh_key_for(&key_root, "pg2"))
                .as_deref(),
            Some("ssh-pass")
        );

        apply_remove_with_ssh(root, &secrets, &host_keys, Some("u@b:22"), &[], "pg2").unwrap();
        assert_eq!(
            secrets.read_raw(&SecretStore::ssh_key_for(&key_root, "pg2")),
            None
        );
        assert_eq!(host_keys.trusted("u@b:22"), None);
    }
}
