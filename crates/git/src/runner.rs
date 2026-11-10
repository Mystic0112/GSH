//! Execução do binário `git` do sistema via [`std::process::Command`] com `-C <repo>`.
//!
//! Nunca invoca shell (`sh -c`).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use crate::error::GitError;
use crate::parse::{
    parse_log_output, parse_porcelain_v2_z, parse_unified_diff, LOG_FIELD_SEP, LOG_RECORD_SEP,
};
use crate::types::{
    normalize_rel_path, DiffHunk, DiffLine, DiffLineKind, FileDiff, FileDiffKind, GitCommitInfo,
    GitRepoStatus,
};

/// Candidatos conhecidos para o binário `git` (espelha `GitBinary` do Flutter).
pub const GIT_BINARY_CANDIDATES: [&str; 3] = [
    "/usr/bin/git",
    "/opt/homebrew/bin/git",
    "/usr/local/bin/git",
];

/// Resolve o caminho do binário `git` no sistema, caindo em `"git"` (PATH).
pub fn resolve_git_binary() -> PathBuf {
    for candidate in GIT_BINARY_CANDIDATES {
        let path = Path::new(candidate);
        if path.is_file() {
            return path.to_path_buf();
        }
    }
    PathBuf::from("git")
}

/// Cliente para rodar comandos `git -C <repo>` sem shell.
#[derive(Debug, Clone)]
pub struct GitClient {
    binary: PathBuf,
    config_overrides: Vec<(String, String)>,
}

impl Default for GitClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GitClient {
    /// Cria um cliente resolvendo o binário `git` do sistema.
    pub fn new() -> Self {
        Self {
            binary: resolve_git_binary(),
            config_overrides: Vec::new(),
        }
    }

    /// Sobrescreve o caminho do binário `git` (útil para testes de erro).
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Adiciona um parâmetro `-c <key>=<value>` em todas as chamadas ao `git`
    /// (ex.: `user.name` e `user.email` em testes independentes do ambiente).
    pub fn with_config(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config_overrides.push((key.into(), value.into()));
        self
    }

    /// Verifica se o binário `git` configurado está disponível e responde a `--version`.
    pub fn is_available(&self) -> bool {
        Command::new(&self.binary)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|st| st.success())
            .unwrap_or(false)
    }

    /// Lê o status completo do repositório (`git status --porcelain=v2 --branch -z -uall --ignored=matching`).
    pub fn status(&self, repo: &Path) -> Result<GitRepoStatus, GitError> {
        let out = self.run_args(
            repo,
            &[
                "status",
                "--porcelain=v2",
                "--branch",
                "-z",
                "-uall",
                "--ignored=matching",
            ],
        )?;
        let text = String::from_utf8_lossy(&out.stdout);
        Ok(parse_porcelain_v2_z(&text))
    }

    /// Adiciona caminhos relativos ao index (`git add -- <paths...>`).
    pub fn stage<P: AsRef<Path>>(&self, repo: &Path, paths: &[P]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Err(GitError::EmptyPaths);
        }
        let mut args = vec!["add".to_string(), "--".to_string()];
        for p in paths {
            args.push(normalize_rel_path(p.as_ref()));
        }
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let _ = self.run_args(repo, &arg_refs)?;
        Ok(())
    }

    /// Remove caminhos relativos do index (`git restore --staged -- <paths...>`,
    /// com fallback para `git rm -r --cached --` quando `HEAD` ainda não existe).
    pub fn unstage<P: AsRef<Path>>(&self, repo: &Path, paths: &[P]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Err(GitError::EmptyPaths);
        }
        let rels: Vec<String> = paths
            .iter()
            .map(|p| normalize_rel_path(p.as_ref()))
            .collect();
        let mut restore_args = vec!["restore", "--staged", "--"];
        for r in &rels {
            restore_args.push(r.as_str());
        }
        match self.run_args(repo, &restore_args) {
            Ok(_) => Ok(()),
            Err(GitError::CommandFailed { ref stderr, .. })
                if stderr_indicates_unborn_head(stderr) =>
            {
                let mut rm_args = vec!["rm", "-r", "--cached", "--"];
                for r in &rels {
                    rm_args.push(r.as_str());
                }
                let _ = self.run_args(repo, &rm_args)?;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// Cria um commit com a mensagem fornecida via `stdin` (`git commit -F -`).
    pub fn commit(&self, repo: &Path, message: &str) -> Result<String, GitError> {
        if message.trim().is_empty() {
            return Err(GitError::EmptyCommitMessage);
        }
        if !repo.exists() {
            return Err(GitError::NotARepository {
                path: repo.to_path_buf(),
            });
        }

        let mut cmd = self.base_command(repo);
        let args = ["commit", "-F", "-"];
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| self.map_spawn_error(repo, &e))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(message.as_bytes())
                .map_err(|e| GitError::Io {
                    detail: e.to_string(),
                })?;
        }
        let output = child.wait_with_output().map_err(|e| GitError::Io {
            detail: e.to_string(),
        })?;
        self.check_output(repo, &args, output)?;

        // Retorna o hash curto do novo HEAD quando disponível
        let short_head = self
            .run_args(repo, &["rev-parse", "--short", "HEAD"])
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        Ok(short_head)
    }

    /// Lê os últimos `limit` commits do repositório (`git log`).
    pub fn log(&self, repo: &Path, limit: usize) -> Result<Vec<GitCommitInfo>, GitError> {
        let limit_str = limit.max(1).to_string();
        let format_spec = format!(
            "--format=%H{f}%h{f}%D{f}%an{f}%aI{f}%ar{f}%s{r}",
            f = LOG_FIELD_SEP,
            r = LOG_RECORD_SEP
        );
        let args = [
            "log",
            "-n",
            limit_str.as_str(),
            "--decorate=short",
            "--date=iso-strict",
            format_spec.as_str(),
        ];
        match self.run_args(repo, &args) {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout);
                Ok(parse_log_output(&text))
            }
            Err(GitError::CommandFailed { ref stderr, .. })
                if stderr_indicates_no_commits(stderr) =>
            {
                Ok(Vec::new())
            }
            Err(err) => Err(err),
        }
    }

    /// Lê o unified diff de um arquivo relativo (`staged = true` usa `--cached`;
    /// `staged = false` compara working tree e trata arquivos untracked como adicionados).
    pub fn diff_file(
        &self,
        repo: &Path,
        rel_path: &Path,
        staged: bool,
    ) -> Result<FileDiff, GitError> {
        let rel = normalize_rel_path(rel_path);
        let out = if staged {
            self.run_args(repo, &["diff", "--cached", "--", &rel])?
        } else {
            self.run_args(repo, &["diff", "--", &rel])?
        };
        let patch = String::from_utf8_lossy(&out.stdout);
        if !patch.trim().is_empty() {
            return Ok(parse_unified_diff(&patch, &rel, staged));
        }

        // Se unstaged e o diff veio vazio, verifica se o arquivo é untracked
        if !staged && self.is_file_untracked(repo, &rel)? {
            return Ok(read_untracked_file_diff(repo, &rel));
        }

        Ok(FileDiff::unchanged(rel, staged))
    }

    fn is_file_untracked(&self, repo: &Path, rel: &str) -> Result<bool, GitError> {
        let out = self.run_args(repo, &["status", "--porcelain=v2", "-z", "--", rel])?;
        let text = String::from_utf8_lossy(&out.stdout);
        Ok(text.split('\0').any(|entry| entry.starts_with("? ")))
    }

    fn base_command(&self, repo: &Path) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.env("LC_ALL", "C");
        cmd.env("GIT_OPTIONAL_LOCKS", "0");
        cmd.arg("-C").arg(repo);
        for (k, v) in &self.config_overrides {
            cmd.arg("-c").arg(format!("{k}={v}"));
        }
        cmd
    }

    fn run_args(&self, repo: &Path, args: &[&str]) -> Result<Output, GitError> {
        if !repo.exists() {
            return Err(GitError::NotARepository {
                path: repo.to_path_buf(),
            });
        }
        let output = self
            .base_command(repo)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| self.map_spawn_error(repo, &e))?;
        self.check_output(repo, args, output)
    }

    fn map_spawn_error(&self, repo: &Path, err: &std::io::Error) -> GitError {
        if err.kind() == std::io::ErrorKind::NotFound {
            if !repo.exists() {
                GitError::NotARepository {
                    path: repo.to_path_buf(),
                }
            } else {
                GitError::GitNotInstalled {
                    binary: self.binary.display().to_string(),
                }
            }
        } else {
            GitError::Io {
                detail: err.to_string(),
            }
        }
    }

    fn check_output(&self, repo: &Path, args: &[&str], output: Output) -> Result<Output, GitError> {
        if output.status.success() {
            return Ok(output);
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        let lower = detail.to_ascii_lowercase();
        if lower.contains("not a git repository") {
            return Err(GitError::NotARepository {
                path: repo.to_path_buf(),
            });
        }
        Err(GitError::CommandFailed {
            command: format!("git {}", args.join(" ")),
            exit_code: output.status.code(),
            stderr: detail,
        })
    }
}

fn stderr_indicates_unborn_head(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("could not resolve head") || lower.contains("invalid reference: head")
}

fn stderr_indicates_no_commits(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("does not have any commits yet") || lower.contains("bad default revision 'head'")
}

fn read_untracked_file_diff(repo: &Path, rel: &str) -> FileDiff {
    let abs = repo.join(rel);
    let Ok(bytes) = fs::read(&abs) else {
        return FileDiff::unchanged(rel, false);
    };
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return FileDiff::binary(rel, false);
    }
    let content = String::from_utf8_lossy(&bytes);
    let lines: Vec<DiffLine> = content
        .lines()
        .enumerate()
        .map(|(idx, text)| DiffLine {
            kind: DiffLineKind::Added,
            old_line: None,
            new_line: Some(idx.saturating_add(1)),
            text: text.to_string(),
        })
        .collect();
    if lines.is_empty() {
        return FileDiff {
            path: rel.to_string(),
            staged: false,
            kind: FileDiffKind::Added,
            hunks: vec![DiffHunk {
                header: "@@ -0,0 +1,0 @@".to_string(),
                lines: Vec::new(),
            }],
        };
    }
    let count = lines.len();
    FileDiff {
        path: rel.to_string(),
        staged: false,
        kind: FileDiffKind::Added,
        hunks: vec![DiffHunk {
            header: format!("@@ -0,0 +1,{count} @@"),
            lines,
        }],
    }
}
