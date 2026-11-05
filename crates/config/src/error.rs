//! Erros do carregamento. Só dados (caminho, posição, causa de terceiro); o
//! texto para o usuário é da camada de UI.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

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
