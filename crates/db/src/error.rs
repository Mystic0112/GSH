//! Erros tipados do banco. A UI escolhe a frase; o texto de terceiros (driver) entra como campo.

use thiserror::Error;

use crate::connection::Engine;

/// Falha ao abrir, consultar ou resolver uma conexão.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DbError {
    /// Não abriu a conexão; o texto vem do driver.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    /// A consulta falhou; o texto vem do driver.
    #[error("query failed: {0}")]
    QueryFailed(String),
    /// Estourou o tempo (segundos).
    #[error("query exceeded {0}s")]
    Timeout(u64),
    /// O motor existe no formato do arquivo, mas ainda não tem driver aqui.
    #[error("{} support is not available yet", .0.label())]
    UnsupportedEngine(Engine),
    /// O `sslmode` pede TLS e este build ainda não fala TLS com o Postgres.
    #[error("TLS is not available yet (sslmode={0})")]
    TlsUnavailable(String),
    /// Nome de conexão que o workspace não tem.
    #[error("no connection named \"{name}\"")]
    UnknownConnection {
        name: String,
        available: Vec<String>,
    },
    /// Nada executável no buffer.
    #[error("nothing to run")]
    NothingToRun,
    /// Identificador (tabela/schema) fora de `[A-Za-z0-9_.$]+`.
    #[error("invalid table name: \"{0}\"")]
    InvalidIdentifier(String),
}

impl DbError {
    /// Categoria estável (a mesma que a CLI e o Flutter usam).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ConnectionFailed(_) | Self::TlsUnavailable(_) => "connection_failed",
            Self::QueryFailed(_) | Self::InvalidIdentifier(_) | Self::NothingToRun => {
                "query_failed"
            }
            Self::Timeout(_) => "timeout",
            Self::UnsupportedEngine(_) => "unsupported_engine",
            Self::UnknownConnection { .. } => "unknown_connection",
        }
    }
}

/// Falha ao gravar ou ler os arquivos de conexões e o cofre de senhas.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StoreError {
    /// Falha de disco; `op` diz o que se tentava e `detail` vem do sistema.
    #[error("{op}: {detail}")]
    Io { op: &'static str, detail: String },
    /// Não foi possível cifrar o cofre.
    #[error("could not encrypt the secret store")]
    Encrypt,
}

impl StoreError {
    pub(crate) fn io(op: &'static str, error: &std::io::Error) -> Self {
        Self::Io {
            op,
            detail: error.to_string(),
        }
    }
}
