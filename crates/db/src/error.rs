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
    /// Chave privada SSH ausente, ilegível ou inválida.
    #[error("{0}")]
    SshKeyMissing(String),
    /// Chave SSH encriptada sem passphrase disponível (ou passphrase errada).
    #[error("{0}")]
    SshCredentialRequired(String),
    /// Autenticação SSH rejeitada pelo servidor.
    #[error("{0}")]
    SshAuthFailed(String),
    /// Host key do bastion ainda não confiada (TOFU).
    #[error("{message}")]
    SshHostKeyUnknown {
        endpoint: String,
        fingerprint: String,
        message: String,
    },
    /// Host key do bastion mudou desde o último uso (recusa dura).
    #[error(
        "The host key for {endpoint} changed. Expected {expected}, got {actual}. \
         If the server was rebuilt, remove the saved key; otherwise this may be a \
         machine-in-the-middle attack."
    )]
    SshHostKeyChanged {
        endpoint: String,
        expected: String,
        actual: String,
    },
    /// Falha ao alcançar o servidor SSH ou abrir o encaminhamento local.
    #[error("{0}")]
    SshConnectFailed(String),
}

impl DbError {
    /// Categoria estável (a mesma que a CLI e o Flutter usam).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ConnectionFailed(_) => "connection_failed",
            Self::QueryFailed(_) | Self::InvalidIdentifier(_) | Self::NothingToRun => {
                "query_failed"
            }
            Self::Timeout(_) => "timeout",
            Self::UnsupportedEngine(_) => "unsupported_engine",
            Self::UnknownConnection { .. } => "unknown_connection",
            Self::SshKeyMissing(_) => "ssh_key_missing",
            Self::SshCredentialRequired(_) => "ssh_credential_required",
            Self::SshAuthFailed(_) => "ssh_auth_failed",
            Self::SshHostKeyUnknown { .. } => "ssh_host_key_unknown",
            Self::SshHostKeyChanged { .. } => "ssh_host_key_changed",
            Self::SshConnectFailed(_) => "ssh_connect_failed",
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
