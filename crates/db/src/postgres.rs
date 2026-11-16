//! Driver PostgreSQL (`tokio-postgres` + `tokio-postgres-rustls`). Modelo efêmero: cada chamada
//! abre a conexão, executa e fecha, num runtime próprio — o chamador usa `spawn_blocking`.
//!
//! Os valores vêm do protocolo de texto (`simple_query`), que serve para qualquer tipo; os
//! tipos de coluna vêm de um `prepare` e só bool, inteiros, floats e bytea viram tipados.

use std::error::Error as _;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::WebPkiServerVerifier;
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, CryptoProvider};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    CertificateError, ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
};
use tokio_postgres::config::SslMode;
use tokio_postgres::types::Type;
use tokio_postgres::{Client, Config, NoTls, SimpleQueryMessage};
use tokio_postgres_rustls::MakeRustlsConnect;
use zeroize::Zeroize;

use crate::error::DbError;
use crate::result::{Cell, QueryResult};

/// Destino de uma conexão Postgres, já com a senha resolvida.
#[derive(Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub database: String,
    /// `?sslmode=` da URL.
    pub sslmode: Option<String>,
}

impl Drop for Target {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

impl fmt::Debug for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Target")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("user", &self.user)
            .field("password", &self.password.as_ref().map(|_| "[REDACTED]"))
            .field("database", &self.database)
            .field("sslmode", &self.sslmode)
            .finish()
    }
}

fn describe(error: &tokio_postgres::Error) -> String {
    if let Some(db) = error.as_db_error() {
        return match db.detail() {
            Some(detail) => format!("{}\n{detail}", db.message()),
            None => db.message().to_string(),
        };
    }
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

fn query_error(error: &tokio_postgres::Error) -> DbError {
    DbError::QueryFailed(describe(error))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedSslMode {
    Disable,
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

impl ParsedSslMode {
    fn from_opt(raw: Option<&str>) -> Result<Self, DbError> {
        match raw.map(str::to_ascii_lowercase).as_deref() {
            None | Some("") | Some("prefer") | Some("allow") => Ok(Self::Prefer),
            Some("disable") => Ok(Self::Disable),
            Some("require") => Ok(Self::Require),
            Some("verify-ca") => Ok(Self::VerifyCa),
            Some("verify-full") => Ok(Self::VerifyFull),
            Some(other) => Err(DbError::ConnectionFailed(format!(
                "unsupported Postgres sslmode: {other}"
            ))),
        }
    }
}

fn root_cert_store() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let native = rustls_native_certs::load_native_certs();
    for cert in native.certs {
        let _ = roots.add(cert);
    }
    roots
}

fn ring_provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Verificador para `sslmode=require` e `prefer`: exige transporte cifrado sem validar CA/hostname,
/// espelhando `libpq` / `sqlx`.
#[derive(Debug)]
struct AcceptAnyCertVerifier {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for AcceptAnyCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Verificador para `sslmode=verify-ca`: valida a cadeia até a CA confiável e ignora apenas o
/// hostname (`NotValidForName` / `NotValidForNameContext`).
#[derive(Debug)]
struct VerifyCaVerifier {
    inner: Arc<WebPkiServerVerifier>,
}

impl ServerCertVerifier for VerifyCaVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        match self.inner.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        ) {
            Ok(verified) => Ok(verified),
            Err(rustls::Error::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. },
            )) => Ok(ServerCertVerified::assertion()),
            Err(other) => Err(other),
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

fn tls_config(mode: ParsedSslMode) -> Result<ClientConfig, DbError> {
    let provider = ring_provider();
    let builder = ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_safe_default_protocol_versions()
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
    match mode {
        ParsedSslMode::Disable => Ok(builder
            .with_root_certificates(RootCertStore::empty())
            .with_no_client_auth()),
        ParsedSslMode::Prefer | ParsedSslMode::Require => Ok(builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyCertVerifier { provider }))
            .with_no_client_auth()),
        ParsedSslMode::VerifyCa => {
            let roots = Arc::new(root_cert_store());
            let inner = WebPkiServerVerifier::builder_with_provider(roots, provider)
                .build()
                .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            Ok(builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(VerifyCaVerifier { inner }))
                .with_no_client_auth())
        }
        ParsedSslMode::VerifyFull => Ok(builder
            .with_root_certificates(root_cert_store())
            .with_no_client_auth()),
    }
}

fn config(target: &Target, timeout: Duration) -> Result<(Config, ParsedSslMode), DbError> {
    let mode = ParsedSslMode::from_opt(target.sslmode.as_deref())?;
    let mut config = Config::new();
    config
        .host(&target.host)
        .port(target.port)
        .user(&target.user)
        .connect_timeout(timeout);
    if !target.database.is_empty() {
        config.dbname(&target.database);
    }
    if let Some(password) = &target.password {
        config.password(password);
    }
    config.ssl_mode(match mode {
        ParsedSslMode::Disable => SslMode::Disable,
        ParsedSslMode::Prefer => SslMode::Prefer,
        ParsedSslMode::Require | ParsedSslMode::VerifyCa | ParsedSslMode::VerifyFull => {
            SslMode::Require
        }
    });
    Ok((config, mode))
}

async fn connect(target: &Target, timeout: Duration) -> Result<Client, DbError> {
    let (config, mode) = config(target, timeout)?;
    if mode == ParsedSslMode::Disable {
        let (client, connection) = config
            .connect(NoTls)
            .await
            .map_err(|e| DbError::ConnectionFailed(describe(&e)))?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        return Ok(client);
    }
    let tls = MakeRustlsConnect::new(tls_config(mode)?);
    let (client, connection) = config
        .connect(tls)
        .await
        .map_err(|e| DbError::ConnectionFailed(describe(&e)))?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    Ok(client)
}

fn block_on<T>(
    timeout: Duration,
    work: impl std::future::Future<Output = Result<T, DbError>>,
) -> Result<T, DbError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
    runtime.block_on(async {
        tokio::time::timeout(timeout, work)
            .await
            .map_err(|_| DbError::Timeout(timeout.as_secs()))?
    })
}

/// Valor de texto do protocolo na célula tipada do resultado.
fn cell(ty: &Type, text: Option<&str>) -> Cell {
    let Some(text) = text else {
        return Cell::Null;
    };
    let typed = if *ty == Type::BOOL {
        match text {
            "t" => Some(Cell::Bool(true)),
            "f" => Some(Cell::Bool(false)),
            _ => None,
        }
    } else if [Type::INT2, Type::INT4, Type::INT8, Type::OID].contains(ty) {
        text.parse().ok().map(Cell::Int)
    } else if [Type::FLOAT4, Type::FLOAT8].contains(ty) {
        text.parse().ok().map(Cell::Real)
    } else if *ty == Type::BYTEA {
        text.strip_prefix("\\x").and_then(hex).map(Cell::Blob)
    } else {
        None
    };
    typed.unwrap_or_else(|| Cell::Text(text.to_string()))
}

fn hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len() / 2)
        .map(|index| {
            let pair = text.get(index * 2..index * 2 + 2)?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect()
}

async fn run_query(
    target: &Target,
    sql: &str,
    limit: usize,
    timeout: Duration,
) -> Result<QueryResult, DbError> {
    let started = Instant::now();
    let client = connect(target, timeout).await?;
    let statement = client.prepare(sql).await.map_err(|e| query_error(&e))?;
    let types: Vec<Type> = statement
        .columns()
        .iter()
        .map(|column| column.type_().clone())
        .collect();
    let messages = client
        .simple_query(sql)
        .await
        .map_err(|e| query_error(&e))?;
    if types.is_empty() {
        return Ok(QueryResult::empty(started.elapsed()));
    }
    let mut names: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut truncated = false;
    for message in messages {
        let SimpleQueryMessage::Row(row) = message else {
            continue;
        };
        if rows.len() >= limit {
            truncated = true;
            break;
        }
        if names.is_empty() {
            names = row.columns().iter().map(|c| c.name().to_string()).collect();
        }
        rows.push(
            types
                .iter()
                .enumerate()
                .map(|(index, ty)| cell(ty, row.get(index)))
                .collect(),
        );
    }
    Ok(QueryResult::from_rows(
        names,
        rows,
        truncated,
        started.elapsed(),
    ))
}

async fn run_execute(
    target: &Target,
    sql: &str,
    timeout: Duration,
) -> Result<QueryResult, DbError> {
    let started = Instant::now();
    let client = connect(target, timeout).await?;
    let messages = client
        .simple_query(sql)
        .await
        .map_err(|e| query_error(&e))?;
    let affected = messages
        .iter()
        .filter_map(|message| match message {
            SimpleQueryMessage::CommandComplete(count) => Some(*count),
            _ => None,
        })
        .last()
        .unwrap_or(0);
    Ok(QueryResult::affected(affected, started.elapsed()))
}

/// Executa `sql` e devolve até `limit` linhas; uma a mais marca `truncated`.
pub fn query(
    target: &Target,
    sql: &str,
    limit: usize,
    timeout: Duration,
) -> Result<QueryResult, DbError> {
    block_on(timeout, run_query(target, sql, limit, timeout))
}

/// Executa DML/DDL e devolve as linhas afetadas.
pub fn execute(target: &Target, sql: &str, timeout: Duration) -> Result<QueryResult, DbError> {
    block_on(timeout, run_execute(target, sql, timeout))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(sslmode: Option<&str>) -> Target {
        Target {
            host: "127.0.0.1".into(),
            port: 1,
            user: "x".into(),
            password: Some("secret-pg-pass".into()),
            database: "x".into(),
            sslmode: sslmode.map(str::to_string),
        }
    }

    #[test]
    fn text_values_become_typed_cells() {
        assert_eq!(cell(&Type::BOOL, Some("t")), Cell::Bool(true));
        assert_eq!(cell(&Type::INT4, Some("-7")), Cell::Int(-7));
        assert_eq!(
            cell(&Type::INT8, Some("9000000000")),
            Cell::Int(9_000_000_000)
        );
        assert_eq!(cell(&Type::FLOAT8, Some("2.5")), Cell::Real(2.5));
        assert_eq!(
            cell(&Type::BYTEA, Some("\\x0aff")),
            Cell::Blob(vec![10, 255])
        );
        assert_eq!(
            cell(&Type::NUMERIC, Some("1.10")),
            Cell::Text("1.10".into())
        );
        assert_eq!(cell(&Type::TEXT, None), Cell::Null);
        // valor que não casa com o tipo declarado não se perde
        assert_eq!(cell(&Type::INT4, Some("oops")), Cell::Text("oops".into()));
        assert_eq!(
            cell(&Type::BYTEA, Some("\\xzz")),
            Cell::Text("\\xzz".into())
        );
    }

    #[test]
    fn every_sslmode_builds_valid_tls_config() {
        for (raw, expected) in [
            (None, ParsedSslMode::Prefer),
            (Some("disable"), ParsedSslMode::Disable),
            (Some("prefer"), ParsedSslMode::Prefer),
            (Some("require"), ParsedSslMode::Require),
            (Some("verify-ca"), ParsedSslMode::VerifyCa),
            (Some("verify-full"), ParsedSslMode::VerifyFull),
        ] {
            let mode = ParsedSslMode::from_opt(raw).unwrap();
            assert_eq!(mode, expected);
            assert!(tls_config(mode).is_ok());
        }
        assert!(ParsedSslMode::from_opt(Some("bogus")).is_err());
    }

    #[test]
    fn debug_redacts_password() {
        let t = target(Some("require"));
        let dbg = format!("{t:?}");
        assert!(!dbg.contains("secret-pg-pass"), "{dbg}");
        assert!(dbg.contains("[REDACTED]"), "{dbg}");
    }

    #[test]
    fn unreachable_server_is_a_connection_failure() {
        let error = query(&target(None), "select 1", 1, Duration::from_secs(3)).unwrap_err();
        assert!(matches!(error, DbError::ConnectionFailed(_)), "{error:?}");
        assert_eq!(error.kind(), "connection_failed");
    }
}
