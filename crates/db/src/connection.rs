//! Conexão de banco do workspace (`db_connection.dart`): a URL é a forma canônica.
//!
//! Nunca carrega a senha: ela mora no cofre ([`crate::secrets`]); aqui só o flag
//! `save_password`.

use std::cmp::Ordering;
use std::fmt;

use serde::Serialize;
use serde_json::Value;
use zeroize::Zeroize;

use crate::url;

/// Motor de banco; a ordem é a do menu do "+" no painel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    Sqlite,
    Postgres,
    Mysql,
    Mssql,
    Redis,
    Mongo,
}

impl Engine {
    pub const ALL: [Self; 6] = [
        Self::Sqlite,
        Self::Postgres,
        Self::Mysql,
        Self::Mssql,
        Self::Redis,
        Self::Mongo,
    ];

    /// Motores que este build sabe executar.
    pub const SUPPORTED: [Self; 6] = Self::ALL;

    /// Nome estável (asset do ícone e `Enum.name` do Flutter).
    pub const fn id(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgres => "postgres",
            Self::Mysql => "mysql",
            Self::Mssql => "mssql",
            Self::Redis => "redis",
            Self::Mongo => "mongo",
        }
    }

    /// Rótulo de exibição (inglês, como no Flutter).
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sqlite => "SQLite",
            Self::Postgres => "Postgres",
            Self::Mysql => "MySQL",
            Self::Mssql => "SQL Server",
            Self::Redis => "Redis",
            Self::Mongo => "MongoDB",
        }
    }

    /// Porta padrão (0 = não se aplica).
    pub const fn default_port(self) -> u16 {
        match self {
            Self::Sqlite => 0,
            Self::Postgres => 5432,
            Self::Mysql => 3306,
            Self::Mssql => 1433,
            Self::Redis => 6379,
            Self::Mongo => 27017,
        }
    }

    /// Motores SQL: consulta tabular, `.dbq` e árvore de schema.
    pub const fn is_sql(self) -> bool {
        matches!(
            self,
            Self::Sqlite | Self::Postgres | Self::Mysql | Self::Mssql
        )
    }

    /// Este build tem driver para o motor.
    pub const fn is_supported(self) -> bool {
        true
    }

    /// Motores que aceitam túnel SSH (todos exceto SQLite).
    pub const fn supports_ssh_tunnel(self) -> bool {
        !matches!(self, Self::Sqlite)
    }

    /// Esquema da URL (`mongodb` no Mongo).
    pub const fn scheme(self) -> &'static str {
        match self {
            Self::Mongo => "mongodb",
            other => other.id(),
        }
    }

    /// Motor de um esquema de URL (`mongodb+srv` e `rediss` incluídos).
    pub fn from_scheme(scheme: &str) -> Option<Self> {
        match scheme {
            "mongodb+srv" => Some(Self::Mongo),
            "rediss" => Some(Self::Redis),
            other => Self::ALL
                .into_iter()
                .find(|engine| engine.scheme() == other),
        }
    }
}

/// Modo de acesso do caminho dos agentes (CLI); a GUI não é limitada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Access {
    #[default]
    Read,
    ReadWrite,
}

impl Access {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::ReadWrite => "readwrite",
        }
    }

    /// Ausente ou desconhecido vira `Read` (seguro por padrão).
    pub fn from_name(name: Option<&str>) -> Self {
        if name == Some("readwrite") {
            Self::ReadWrite
        } else {
            Self::Read
        }
    }
}

/// De onde a conexão veio; decide o que o `save` grava e os selos do painel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// `.gsh/databases.json` (versionado).
    Registered,
    /// `.gsh/databases.local.json` (fora do git, sobrepõe por nome).
    Local,
    /// Arquivo SQLite achado no workspace; nunca é gravado.
    Detected,
}

/// Túnel SSH de uma conexão (`SshTunnelConfig`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SshTunnel {
    pub host: String,
    pub port: i64,
    pub user: String,
    #[serde(rename = "keyPath")]
    pub key_path: String,
    #[serde(rename = "savePassphrase")]
    pub save_passphrase: bool,
}

pub const SSH_DEFAULT_PORT: i64 = 22;

impl SshTunnel {
    /// Identidade do endpoint SSH (`user@host:port`): chave do cache de túneis e de host keys.
    pub fn endpoint(&self) -> String {
        format!("{}@{}:{}", self.user, self.host, self.port)
    }

    /// Rótulo curto para exibição.
    pub fn label(&self) -> String {
        if self.port == SSH_DEFAULT_PORT {
            format!("{}@{}", self.user, self.host)
        } else {
            self.endpoint()
        }
    }

    /// Bloco `ssh` do JSON; malformado (sem host, usuário ou chave) vira `None`.
    pub fn from_json(raw: Option<&Value>) -> Option<Self> {
        let map = raw?.as_object()?;
        let text = |key: &str| {
            map.get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        let (host, user, key_path) = (text("host"), text("user"), text("keyPath"));
        if host.is_empty() || user.is_empty() || key_path.is_empty() {
            return None;
        }
        let port = map
            .get("port")
            .and_then(Value::as_i64)
            .filter(|port| *port > 0)
            .unwrap_or(SSH_DEFAULT_PORT);
        let save_passphrase = map
            .get("savePassphrase")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Some(Self {
            host,
            port,
            user,
            key_path,
            save_passphrase,
        })
    }
}

/// URL de um motor desconhecido ou ilegível.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unsupported database URL: {0}")]
pub struct UnsupportedUrl(pub String);

/// Uma conexão do workspace.
#[derive(Clone, PartialEq, Eq)]
pub struct Connection {
    pub name: String,
    pub engine: Engine,
    pub url: String,
    pub save_password: bool,
    pub origin: Origin,
    pub access: Access,
    /// `false`: invisível na CLI dos agentes; a GUI vê normalmente.
    pub agents: bool,
    pub ssh: Option<SshTunnel>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.url.zeroize();
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("name", &self.name)
            .field("engine", &self.engine)
            .field("url", &redact_url(&self.url))
            .field("save_password", &self.save_password)
            .field("origin", &self.origin)
            .field("access", &self.access)
            .field("agents", &self.agents)
            .field("ssh", &self.ssh)
            .finish()
    }
}

/// Mascara a senha embutida em uma URL (`scheme://user:secret@host` -> `scheme://user:***@host`).
pub fn redact_url(raw: &str) -> String {
    let Ok(parsed) = url::parse(raw) else {
        return "[REDACTED]".to_string();
    };
    let Some((user, _pass)) = parsed.user_info.split_once(':') else {
        return raw.to_string();
    };
    raw.replacen(&parsed.user_info, &format!("{user}:***"), 1)
}

impl Connection {
    /// Conexão SQLite a partir de um caminho.
    pub fn sqlite(name: &str, path: &str, origin: Origin, access: Access, agents: bool) -> Self {
        Self {
            name: name.to_string(),
            engine: Engine::Sqlite,
            url: format!("sqlite:{path}"),
            save_password: false,
            origin,
            access,
            agents,
            ssh: None,
        }
    }

    /// Lê uma entrada do `databases.json`. Motor desconhecido é erro (a entrada é pulada).
    pub fn from_json(json: &Value, origin: Origin) -> Result<Self, UnsupportedUrl> {
        let raw_url = json.get("url").and_then(Value::as_str).unwrap_or_default();
        let url = url::normalize(raw_url).map_err(|_| UnsupportedUrl(raw_url.to_string()))?;
        let engine = engine_from_url(&url).ok_or_else(|| UnsupportedUrl(url.clone()))?;
        Ok(Self {
            name: json
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            engine,
            url,
            save_password: json
                .get("savePassword")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            origin,
            access: Access::from_name(json.get("access").and_then(Value::as_str)),
            agents: json.get("agents").and_then(Value::as_bool).unwrap_or(true),
            ssh: SshTunnel::from_json(json.get("ssh")),
        })
    }

    /// Ordem canônica: alfabética por nome sem distinguir caixa; empate pelo nome cru.
    pub fn compare_by_name(a: &Self, b: &Self) -> Ordering {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    }

    /// Caminho do arquivo SQLite (só faz sentido no motor SQLite).
    pub fn sqlite_path(&self) -> &str {
        self.url.strip_prefix("sqlite:").unwrap_or(&self.url)
    }

    fn parsed(&self) -> url::ParsedUrl {
        url::parse(&self.url).unwrap_or_default()
    }

    pub fn host(&self) -> String {
        self.parsed().host
    }

    pub fn port(&self) -> u16 {
        self.parsed()
            .port
            .unwrap_or_else(|| self.engine.default_port())
    }

    /// Primeiro segmento do caminho, decodificado.
    pub fn database(&self) -> String {
        let parsed = self.parsed();
        let path = parsed.path.strip_prefix('/').unwrap_or(&parsed.path);
        let first = path.split('/').next().unwrap_or_default();
        url::decode_component(first)
    }

    /// Só o usuário do `userinfo`, nunca o trecho `:senha`.
    pub fn user(&self) -> String {
        let info = self.parsed().user_info;
        let user = info.split_once(':').map_or(info.as_str(), |(user, _)| user);
        url::decode_component(user)
    }

    /// Senha embutida na URL (`user:senha@`), se houver.
    pub fn url_password(&self) -> Option<String> {
        let info = self.parsed().user_info;
        info.split_once(':')
            .map(|(_, password)| url::decode_component(password))
    }

    /// URL Mongo em formato SRV (sem porta).
    pub fn is_srv(&self) -> bool {
        self.url.starts_with("mongodb+srv://")
    }

    /// Valor de um parâmetro da query (`sslmode`, ...).
    pub fn query_param(&self, key: &str) -> Option<String> {
        url::query_params(&self.parsed().query)
            .into_iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// Alvo curto para a lista: caminho no SQLite, `host:porta` nos demais.
    pub fn display_target(&self) -> String {
        if self.engine == Engine::Sqlite {
            self.sqlite_path().to_string()
        } else if self.is_srv() {
            self.host()
        } else {
            format!("{}:{}", self.host(), self.port())
        }
    }

    /// TLS configurado na conexão (mesma regra do `DbConnection.useTls` no Flutter).
    pub fn use_tls(&self) -> bool {
        match self.engine {
            Engine::Sqlite => false,
            Engine::Redis => self.url.starts_with("rediss://"),
            Engine::Mongo => {
                self.is_srv()
                    || self.query_param("tls").as_deref() == Some("true")
                    || self.query_param("ssl").as_deref() == Some("true")
            }
            Engine::Postgres => matches!(
                self.query_param("sslmode").as_deref(),
                Some("require" | "verify-ca" | "verify-full")
            ),
            Engine::Mysql => matches!(
                self.query_param("ssl-mode")
                    .map(|v| v.to_ascii_uppercase())
                    .as_deref(),
                Some("REQUIRED" | "VERIFY_CA" | "VERIFY_IDENTITY")
            ),
            Engine::Mssql => self.query_param("encrypt").as_deref() == Some("true"),
        }
    }

    /// Cópia apontando para outro endpoint TCP (`withEndpoint` do Flutter), sem túnel SSH.
    pub fn with_endpoint(&self, host: &str, port: u16) -> Self {
        let mut rebuilt = self.url.clone();
        if let Some(sep) = self.url.find("://") {
            let start = sep + 3;
            let tail = self.url.get(start..).unwrap_or_default();
            let rest_at = tail.find(['/', '?', '#']).unwrap_or(tail.len());
            let authority = tail.get(..rest_at).unwrap_or_default();
            let user_prefix = authority
                .rfind('@')
                .and_then(|at| authority.get(..=at))
                .unwrap_or_default();
            let rest = tail.get(rest_at..).unwrap_or_default();
            let host_fmt = if host.contains(':') && !host.starts_with('[') {
                format!("[{host}]")
            } else {
                host.to_string()
            };
            rebuilt = format!(
                "{}{user_prefix}{host_fmt}:{port}{rest}",
                self.url.get(..start).unwrap_or_default()
            );
        }
        Self {
            name: self.name.clone(),
            engine: self.engine,
            url: rebuilt,
            save_password: self.save_password,
            origin: self.origin,
            access: self.access,
            agents: self.agents,
            ssh: None,
        }
    }

    /// Cópia que roteia o Mongo pelo proxy SOCKS5 local (`withSocksProxy` do Flutter).
    pub fn with_socks_proxy(&self, proxy_host: &str, proxy_port: u16) -> Self {
        let (base, fragment) = match self.url.split_once('#') {
            Some((b, f)) => (b, Some(f)),
            None => (self.url.as_str(), None),
        };
        let (before_q, query) = base.split_once('?').unwrap_or((base, ""));
        let mut params: Vec<(String, String)> = url::query_params(query)
            .into_iter()
            .filter(|(k, _)| k != "proxyHost" && k != "proxyPort")
            .collect();
        params.push(("proxyHost".to_string(), proxy_host.to_string()));
        params.push(("proxyPort".to_string(), proxy_port.to_string()));
        let encoded: Vec<String> = params
            .into_iter()
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    url::encode_component(&k),
                    url::encode_component(&v)
                )
            })
            .collect();
        let mut rebuilt = format!("{before_q}?{}", encoded.join("&"));
        if let Some(frag) = fragment {
            rebuilt.push('#');
            rebuilt.push_str(frag);
        }
        Self {
            name: self.name.clone(),
            engine: self.engine,
            url: rebuilt,
            save_password: self.save_password,
            origin: self.origin,
            access: self.access,
            agents: self.agents,
            ssh: None,
        }
    }
}

fn engine_from_url(url: &str) -> Option<Engine> {
    if url.starts_with("sqlite:") {
        return Some(Engine::Sqlite);
    }
    Engine::from_scheme(&url::parse(url).ok()?.scheme)
}

/// Forma gravada no `databases.json`: a ordem dos campos é a do Flutter.
#[derive(Serialize)]
pub(crate) struct Entry<'a> {
    name: &'a str,
    url: &'a str,
    #[serde(rename = "savePassword")]
    save_password: bool,
    access: &'static str,
    agents: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    ssh: Option<&'a SshTunnel>,
}

impl<'a> From<&'a Connection> for Entry<'a> {
    fn from(c: &'a Connection) -> Self {
        Self {
            name: &c.name,
            url: &c.url,
            save_password: c.save_password,
            access: c.access.name(),
            agents: c.agents,
            ssh: c.ssh.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn postgres(url: &str) -> Connection {
        Connection::from_json(&json!({"name": "pg", "url": url}), Origin::Registered).unwrap()
    }

    #[test]
    fn network_parts_come_from_the_url() {
        let c = postgres("postgres://ana:s%3Acr%40t@db.local:5433/meu%20banco?sslmode=require");
        assert_eq!(c.engine, Engine::Postgres);
        assert_eq!(c.host(), "db.local");
        assert_eq!(c.port(), 5433);
        assert_eq!(c.database(), "meu banco");
        assert_eq!(c.user(), "ana");
        assert_eq!(c.url_password().as_deref(), Some("s:cr@t"));
        assert_eq!(c.query_param("sslmode").as_deref(), Some("require"));
        assert_eq!(c.display_target(), "db.local:5433");
    }

    #[test]
    fn default_port_and_empty_database() {
        let c = postgres("postgres://db.local/");
        assert_eq!(c.port(), 5432);
        assert_eq!(c.database(), "");
        assert_eq!(c.user(), "");
        assert_eq!(c.url_password(), None);
    }

    #[test]
    fn unknown_engine_is_an_error() {
        let bad = json!({"name": "x", "url": "oracle://h/db"});
        assert!(Connection::from_json(&bad, Origin::Registered).is_err());
        let missing = json!({"name": "x"});
        assert!(Connection::from_json(&missing, Origin::Registered).is_err());
    }

    #[test]
    fn sqlite_and_defaults() {
        let c = Connection::from_json(
            &json!({"name": "app", "url": "sqlite:./data/app.db"}),
            Origin::Local,
        )
        .unwrap();
        assert_eq!(c.engine, Engine::Sqlite);
        assert_eq!(c.sqlite_path(), "./data/app.db");
        assert_eq!(c.display_target(), "./data/app.db");
        assert_eq!(c.access, Access::Read);
        assert!(c.agents);
        assert!(!c.save_password);
        assert_eq!(c.origin, Origin::Local);
    }

    #[test]
    fn ordering_ignores_case_then_falls_back_to_raw_name() {
        let named =
            |name: &str| Connection::sqlite(name, "x.db", Origin::Registered, Access::Read, true);
        let mut list = [named("beta"), named("Alpha"), named("alpha")];
        list.sort_by(Connection::compare_by_name);
        let names: Vec<_> = list.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Alpha", "alpha", "beta"]);
    }

    #[test]
    fn ssh_block_is_kept_only_when_complete() {
        let full = json!({"host":" bastion ","user":"u","keyPath":"~/.ssh/id","port":2200,"savePassphrase":true});
        let tunnel = SshTunnel::from_json(Some(&full)).unwrap();
        assert_eq!(tunnel.host, "bastion");
        assert_eq!(tunnel.port, 2200);
        assert!(tunnel.save_passphrase);
        assert!(SshTunnel::from_json(Some(&json!({"host":"h"}))).is_none());
        assert!(SshTunnel::from_json(None).is_none());
        let defaults = json!({"host":"h","user":"u","keyPath":"k","port":-1});
        assert_eq!(SshTunnel::from_json(Some(&defaults)).unwrap().port, 22);
    }

    #[test]
    fn debug_redacts_password_embedded_in_url() {
        let c = postgres("postgres://ana:super-secret@db.local:5432/app");
        let dbg = format!("{c:?}");
        assert!(!dbg.contains("super-secret"), "{dbg}");
        assert!(dbg.contains("ana:***"), "{dbg}");
    }

    #[test]
    fn with_endpoint_and_with_socks_proxy_preserve_parts_and_drop_ssh() {
        let mut c = Connection::from_json(
            &json!({
                "name": "m",
                "url": "mongodb://u:p@db.internal:27017/app?authSource=admin",
                "ssh": {"host": "bastion", "user": "ops", "keyPath": "~/.ssh/id_ed25519"}
            }),
            Origin::Registered,
        )
        .unwrap();
        let tunneled = c.with_endpoint("127.0.0.1", 55123);
        assert_eq!(
            tunneled.url,
            "mongodb://u:p@127.0.0.1:55123/app?authSource=admin"
        );
        assert!(tunneled.ssh.is_none());

        let proxied = c.with_socks_proxy("127.0.0.1", 55444);
        assert_eq!(
            proxied.query_param("proxyHost").as_deref(),
            Some("127.0.0.1")
        );
        assert_eq!(proxied.query_param("proxyPort").as_deref(), Some("55444"));
        assert_eq!(proxied.query_param("authSource").as_deref(), Some("admin"));
        assert!(proxied.ssh.is_none());

        c.ssh = None;
        assert!(!c.use_tls());
    }
}
