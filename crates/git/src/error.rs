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

    #[error("não há alterações staged para analisar")]
    NoStagedChanges,

    #[error("o diff staged excede o limite de {max_bytes} bytes")]
    StagedDiffTooLarge { max_bytes: usize },

    #[error("o git excedeu o tempo limite ao obter o diff staged")]
    StagedDiffTimeout,

    #[error("a árvore de trabalho tem alterações; faça commit ou stash antes de trocar de branch")]
    DirtyWorkingTree,

    #[error("branch local não encontrada: {name}")]
    BranchNotFound { name: String },

    #[error(transparent)]
    CommitMessageGenerationFailed(#[from] CommitMessageFailure),

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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CommitMessageFailure {
    #[error("a automação para sugestões de commit está desativada")]
    Disabled,
    #[error("{harness} não oferece um modo isolado para sugestões de commit")]
    UnsupportedHarness { harness: &'static str },
    #[error("executável do harness não encontrado ({binary})")]
    HarnessNotInstalled { binary: &'static str },
    #[error("o modelo do harness é inválido")]
    InvalidModel,
    #[error("o harness não respondeu em {seconds} segundos")]
    Timeout { seconds: u64 },
    #[error("o harness executou ou solicitou uma ferramenta")]
    ToolUse,
    #[error("o harness retornou uma mensagem de commit inválida")]
    InvalidResponse,
    #[error("a saída do harness excedeu o limite de {max_bytes} bytes")]
    OutputTooLarge { max_bytes: usize },
    #[error("o harness encerrou com falha ({exit_code:?}): {stderr}")]
    ProcessFailed {
        exit_code: Option<i32>,
        stderr: String,
    },
    #[error("falha de I/O ao executar o harness: {detail}")]
    Io { detail: String },
}
