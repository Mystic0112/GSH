//! Erros tipados das operações de Git (`gsh-git`).

use std::path::PathBuf;
use thiserror::Error;

/// Erro tipado retornado pelas operações do `gsh-git`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GitError {
    /// O binário `git` não foi encontrado no sistema.
    #[error("git não instalado ou não encontrado ({binary})")]
    GitNotInstalled { binary: String },

    /// O caminho informado não é um repositório Git válido (ou não existe).
    #[error("não é um repositório git: {path}")]
    NotARepository { path: PathBuf },

    /// Tentativa de criar um commit com mensagem em branco.
    #[error("a mensagem de commit não pode ser vazia")]
    EmptyCommitMessage,

    /// Tentativa de executar stage/unstage sem nenhum caminho informado.
    #[error("nenhum caminho informado para a operação git")]
    EmptyPaths,

    /// O processo `git` encerrou com código de saída diferente de zero.
    #[error("falha no comando git ({command}): {stderr}")]
    CommandFailed {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },

    /// Falha de I/O ao iniciar processo ou comunicar via pipes (`stdin`/`stdout`).
    #[error("erro de I/O ao executar git: {detail}")]
    Io { detail: String },
}
