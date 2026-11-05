//! Erros do carregamento. Só dados (caminho, posição, causa tipada ou de
//! terceiro); o texto para o usuário é da camada de UI.

use std::fmt;
use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Causa tipada de erro ao decodificar o `settings.json` do Flutter, sem expor
/// a mensagem crua do `serde_json` (que pode conter valores do arquivo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonParseCause {
    /// Sintaxe JSON inválida.
    Syntax,
    /// Fim inesperado do arquivo JSON.
    UnexpectedEof,
    /// Valor incompatível com o tipo esperado pelo schema.
    InvalidData,
    /// Erro de I/O reportado pelo decodificador JSON.
    Io,
    /// A raiz do JSON não é um objeto.
    RootNotObject,
    /// `version` do `JsonStateStore` diferente de `1`.
    UnsupportedVersion,
    /// Falta a chave `"data"` no envelope do `JsonStateStore`.
    MissingData,
    /// `"data"` não é um objeto JSON.
    DataNotObject,
    /// `"data.app"` não é um objeto JSON.
    AppNotObject,
    /// Chaves com ponto conflitam entre valor escalar e sub-tabela.
    ConflictingKey,
}

impl JsonParseCause {
    pub(crate) fn from_category(cat: serde_json::error::Category) -> Self {
        match cat {
            serde_json::error::Category::Io => Self::Io,
            serde_json::error::Category::Syntax => Self::Syntax,
            serde_json::error::Category::Data => Self::InvalidData,
            serde_json::error::Category::Eof => Self::UnexpectedEof,
        }
    }
}

impl fmt::Display for JsonParseCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Syntax => "invalid json syntax",
            Self::UnexpectedEof => "unexpected end of json",
            Self::InvalidData => "invalid json data type",
            Self::Io => "json io error",
            Self::RootNotObject => "root must be a json object",
            Self::UnsupportedVersion => "unsupported store version",
            Self::MissingData => "missing 'data' object in store",
            Self::DataNotObject => "'data' must be a json object",
            Self::AppNotObject => "'data.app' must be a json object",
            Self::ConflictingKey => "conflicting dotted key path",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    /// `dirs::home_dir()` não achou a home: sem caminho padrão.
    #[error("home: none")]
    NoHome,
    /// O arquivo existe mas não deu para ler (permissão, diretório, não UTF-8).
    #[error("{}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// Não deu para gravar o `config.toml` migrado.
    #[error("{}: {source}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// `config.toml` apareceu entre a checagem inicial e a gravação exclusiva
    /// (`create_new`).
    #[error("{}: already exists", path.display())]
    AlreadyExists { path: PathBuf },
    /// O caminho de migração (`settings.json` ou `storage_root`) não é arquivo
    /// regular (diretório, FIFO, socket, device).
    #[error("{}: not a regular file", path.display())]
    NotRegularFile { path: PathBuf },
    /// O arquivo de migração (`settings.json` ou `storage_root`) excede o
    /// tamanho máximo permitido.
    #[error("{}: file too large ({size} bytes, max {max})", path.display())]
    FileTooLarge { path: PathBuf, size: u64, max: u64 },
    /// `storage_root` contém um caminho relativo (só caminho absoluto é aceito).
    #[error("{}: storage_root must be an absolute path", path.display())]
    RelativeStorageRoot { path: PathBuf },
    /// Chave pontilhada em `data.app` excedeu o limite de segmentos.
    #[error("{}: dotted key exceeds max depth of {max}", path.display())]
    KeyDepthExceeded { path: PathBuf, max: usize },
    /// `settings.json` inválido (sintaxe, envelope ou tipo de campo), sem
    /// incluir a mensagem crua do `serde_json` nem valores do arquivo.
    #[error("{}:{}:{}: {cause}", path.display(), opt(*line), opt(*column))]
    JsonParse {
        path: PathBuf,
        line: Option<usize>,
        column: Option<usize>,
        cause: JsonParseCause,
    },
    /// TOML inválido ou valor de tipo errado. `line`/`column` são 1-based e
    /// ausentes quando o parser não indica posição; `cause` é a mensagem do
    /// parser, sem tradução.
    #[error("{}:{}:{}: {cause}", path.display(), opt(*line), opt(*column))]
    Parse {
        path: PathBuf,
        line: Option<usize>,
        column: Option<usize>,
        cause: String,
    },
    /// Número `nan`/`inf` (o TOML aceita, o JSON do Flutter não tinha como).
    #[error("{}: {key} = {value}", path.display())]
    NonFinite {
        path: PathBuf,
        key: &'static str,
        value: f64,
    },
    /// Não deu para criar a thread de polling.
    #[error("spawn: {0}")]
    Spawn(#[source] io::Error),
}

fn opt(n: Option<usize>) -> String {
    n.map_or_else(|| "?".to_owned(), |n| n.to_string())
}
