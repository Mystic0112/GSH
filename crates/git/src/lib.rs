//! Motor Git do GSH (`gsh-git`): leitura de status, stage/unstage, commit via
//! `stdin` (`-F -`), histórico (`log`) e unified `diff` de arquivos, rodando o
//! binário `git` do sistema via [`std::process::Command`] com `-C <repo>`.
//!
//! Sem UI e sem shell.
//! // fase futura: push, pull, fetch, checkout de branch, descartar mudanças, reset, stash, merge, rebase

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod error;
pub mod parse;
pub mod runner;
pub mod types;

use std::path::Path;

pub use error::GitError;
pub use parse::{
    parse_log_output, parse_porcelain_v2_z, parse_unified_diff, LOG_FIELD_SEP, LOG_RECORD_SEP,
};
pub use runner::{resolve_git_binary, GitClient, GIT_BINARY_CANDIDATES};
pub use types::{
    normalize_rel_path, DiffHunk, DiffLine, DiffLineKind, FileDiff, FileDiffKind, GitCommitInfo,
    GitFileStatus, GitRepoStatus,
};

/// Atalho para ler o status do repositório usando o [`GitClient`] padrão.
pub fn status(repo: &Path) -> Result<GitRepoStatus, GitError> {
    GitClient::new().status(repo)
}

/// Atalho para adicionar caminhos ao index (`stage`) usando o [`GitClient`] padrão.
pub fn stage<P: AsRef<Path>>(repo: &Path, paths: &[P]) -> Result<(), GitError> {
    GitClient::new().stage(repo, paths)
}

/// Atalho para remover caminhos do index (`unstage`) usando o [`GitClient`] padrão.
pub fn unstage<P: AsRef<Path>>(repo: &Path, paths: &[P]) -> Result<(), GitError> {
    GitClient::new().unstage(repo, paths)
}

/// Atalho para criar um commit com mensagem via `stdin` (`-F -`) usando o [`GitClient`] padrão.
pub fn commit(repo: &Path, message: &str) -> Result<String, GitError> {
    GitClient::new().commit(repo, message)
}

/// Atalho para ler os últimos `limit` commits do repositório usando o [`GitClient`] padrão.
pub fn log(repo: &Path, limit: usize) -> Result<Vec<GitCommitInfo>, GitError> {
    GitClient::new().log(repo, limit)
}

/// Atalho para ler o diff unified de um arquivo usando o [`GitClient`] padrão.
pub fn diff_file(repo: &Path, rel_path: &Path, staged: bool) -> Result<FileDiff, GitError> {
    GitClient::new().diff_file(repo, rel_path, staged)
}

/// Lista branches locais do repositório.
pub fn branches(repo: &Path) -> Result<Vec<String>, GitError> {
    GitClient::new().branches(repo)
}

/// Troca para uma branch local se a árvore estiver limpa.
pub fn switch_branch(repo: &Path, name: &str) -> Result<(), GitError> {
    GitClient::new().switch_branch(repo, name)
}

/// Lê as alterações staged, encerrando a leitura quando `max_bytes` é excedido.
pub fn staged_diff<P: AsRef<Path>>(
    repo: &Path,
    paths: &[P],
    max_bytes: usize,
) -> Result<String, GitError> {
    GitClient::new().staged_diff(repo, paths, max_bytes)
}

/// Lê o diff staged de todo o index com limite de bytes e prazo de execução.
pub fn staged_index_diff(repo: &Path, max_bytes: usize) -> Result<String, GitError> {
    GitClient::new().staged_index_diff(repo, max_bytes)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEMP_SEQ: AtomicU64 = AtomicU64::new(1);

    struct TempGitRepo {
        path: PathBuf,
    }

    impl TempGitRepo {
        fn new() -> Self {
            let seq = TEMP_SEQ.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir()
                .join(format!("gsh-git-test-repo-{}-{seq}", std::process::id()));
            fs::create_dir_all(&path).expect("create temp git repo dir");
            Self { path }
        }
    }

    impl Drop for TempGitRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn test_client() -> Option<GitClient> {
        let client = GitClient::new()
            .with_config("user.name", "Bulma Briefs")
            .with_config("user.email", "bulma@capsule.corp");
        if !client.is_available() {
            eprintln!("SKIP: binário `git` não encontrado no sistema; pulando teste de integração");
            return None;
        }
        Some(client)
    }

    fn git(repo: &Path, args: &[&str]) {
        let output = Command::new(resolve_git_binary())
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .expect("run git fixture");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn switches_local_branches_only_when_worktree_is_clean() {
        let Some(client) = test_client() else {
            return;
        };
        let temp = TempGitRepo::new();
        git(&temp.path, &["init"]);
        git(&temp.path, &["config", "user.name", "GSH Test"]);
        git(&temp.path, &["config", "user.email", "gsh@example.test"]);
        fs::write(temp.path.join("file.txt"), "initial\n").unwrap();
        git(&temp.path, &["add", "file.txt"]);
        client.commit(&temp.path, "initial").unwrap();
        git(&temp.path, &["branch", "feature/viewers"]);
        assert!(client
            .branches(&temp.path)
            .unwrap()
            .contains(&"feature/viewers".into()));

        fs::write(temp.path.join("file.txt"), "unsaved\n").unwrap();
        assert_eq!(
            client.switch_branch(&temp.path, "feature/viewers"),
            Err(GitError::DirtyWorkingTree)
        );
        git(&temp.path, &["restore", "--worktree", "--", "file.txt"]);
        client.switch_branch(&temp.path, "feature/viewers").unwrap();
        assert_eq!(client.status(&temp.path).unwrap().branch, "feature/viewers");
        assert!(matches!(
            client.switch_branch(&temp.path, "missing"),
            Err(GitError::BranchNotFound { .. })
        ));
    }

    #[test]
    fn staged_diff_stops_at_the_requested_size_limit() {
        let Some(client) = test_client() else {
            return;
        };
        let temp = TempGitRepo::new();
        git(&temp.path, &["init"]);
        fs::write(temp.path.join("file.txt"), "initial\n").unwrap();
        git(&temp.path, &["add", "file.txt"]);
        client.commit(&temp.path, "initial").unwrap();
        fs::write(temp.path.join("file.txt"), "x".repeat(1024)).unwrap();
        client.stage(&temp.path, &["file.txt"]).unwrap();
        assert!(matches!(
            client.staged_diff(&temp.path, &["file.txt"], 64),
            Err(GitError::StagedDiffTooLarge { max_bytes: 64 })
        ));
        assert_eq!(
            client.staged_index_diff(&temp.path, 64),
            Err(GitError::StagedDiffTooLarge { max_bytes: 64 })
        );
        assert_eq!(
            client.staged_diff::<&str>(&temp.path, &[], 4096),
            Err(GitError::EmptyPaths)
        );
    }

    #[cfg(unix)]
    #[test]
    fn staged_index_diff_keeps_renames_and_ignores_external_converters() {
        let Some(client) = test_client() else {
            return;
        };
        let temp = TempGitRepo::new();
        git(&temp.path, &["init"]);
        fs::write(temp.path.join(".gitattributes"), "data.txt diff=marker\n").unwrap();
        fs::write(temp.path.join("old.txt"), "same contents\n").unwrap();
        fs::write(temp.path.join("data.txt"), "before\n").unwrap();
        git(&temp.path, &["add", "--all"]);
        client.commit(&temp.path, "initial").unwrap();
        assert_eq!(client.staged_index_diff(&temp.path, 4096).unwrap(), "");

        git(&temp.path, &["mv", "old.txt", "./:(exclude)renamed.txt"]);
        fs::write(temp.path.join("data.txt"), "staged data\n").unwrap();
        client.stage(&temp.path, &["data.txt"]).unwrap();
        fs::write(temp.path.join("data.txt"), "unstaged data\n").unwrap();
        let nested = temp.path.join("nested");
        fs::create_dir(&nested).unwrap();
        let marker = temp.path.join("converter-ran");
        let mark = "sh -c ': > converter-ran'";
        for key in ["diff.external", "diff.marker.textconv"] {
            let config = format!("{key}={mark}");
            git(
                &nested,
                &["-c", &config, "diff", "--cached", "--no-relative"],
            );
            fs::remove_file(&marker).unwrap();
        }
        let client = client
            .with_config("diff.relative", "true")
            .with_config("diff.renames", "true")
            .with_config("diff.external", mark)
            .with_config("diff.marker.textconv", mark);
        let diff = client.staged_index_diff(&nested, 4096).unwrap();
        assert!(!marker.exists());
        assert!(diff.contains("rename from old.txt"));
        assert!(diff.contains("rename to :(exclude)renamed.txt"));
        assert!(diff.contains("+staged data"));
        assert!(!diff.contains("unstaged data"));
    }

    #[test]
    fn parse_porcelain_v2_z_interpreta_branch_staged_unstaged_untracked_rename_conflito_e_pastas() {
        let fixture = concat!(
            "# branch.oid 3cfb4cf1234567890\0",
            "# branch.head feat/terminal-ui\0",
            "# branch.upstream origin/feat/terminal-ui\0",
            "# branch.ab +2 -1\0",
            "1 M. N... 100644 100644 100644 aaaaaaa bbbbbbb src/staged_only.rs\0",
            "1 .M N... 100644 100644 100644 aaaaaaa bbbbbbb src/ui/view.rs\0",
            "1 MM N... 100644 100644 100644 aaaaaaa bbbbbbb src/both.rs\0",
            "1 .D N... 100644 100644 000000 aaaaaaa 0000000 old/removed.rs\0",
            "2 R. N... 100644 100644 100644 aaaaaaa bbbbbbb R100 src/new_name.rs\0src/old_name.rs\0",
            "u UU N... 100644 100644 100644 100644 aaaa bbbb cccc src/conflict.rs\0",
            "? novo.txt\0",
            "? docs/rascunho/\0",
            "! target/\0",
        );

        let parsed = parse_porcelain_v2_z(fixture);
        assert_eq!(parsed.branch, "feat/terminal-ui");
        assert_eq!(parsed.ahead, 2);
        assert_eq!(parsed.behind, 1);

        // Staged
        assert_eq!(
            parsed.staged.get("src/staged_only.rs"),
            Some(&GitFileStatus::Staged)
        );
        assert_eq!(
            parsed.staged.get("src/both.rs"),
            Some(&GitFileStatus::Staged)
        );
        assert_eq!(
            parsed.staged.get("src/new_name.rs"),
            Some(&GitFileStatus::Staged)
        );
        assert_eq!(
            parsed.staged.get("src/conflict.rs"),
            Some(&GitFileStatus::Conflict)
        );
        assert!(!parsed.staged.contains_key("src/old_name.rs"));

        // Unstaged
        assert_eq!(
            parsed.unstaged.get("src/ui/view.rs"),
            Some(&GitFileStatus::Modified)
        );
        assert_eq!(
            parsed.unstaged.get("src/both.rs"),
            Some(&GitFileStatus::Modified)
        );
        assert_eq!(
            parsed.unstaged.get("old/removed.rs"),
            Some(&GitFileStatus::Deleted)
        );
        assert_eq!(
            parsed.unstaged.get("novo.txt"),
            Some(&GitFileStatus::Untracked)
        );
        assert_eq!(
            parsed.unstaged.get("src/conflict.rs"),
            Some(&GitFileStatus::Conflict)
        );

        // Precedência de pastas na árvore: `src` tem conflito -> Conflict;
        // `old` tem apenas Deleted -> sobe como Modified para a pasta.
        assert_eq!(
            parsed.status_for_rel_path("src"),
            Some(GitFileStatus::Conflict)
        );
        assert_eq!(
            parsed.status_for_rel_path("src/ui"),
            Some(GitFileStatus::Modified)
        );
        assert_eq!(
            parsed.status_for_rel_path("old"),
            Some(GitFileStatus::Modified)
        );
        assert_eq!(
            parsed.status_for_rel_path("docs/rascunho/nota.md"),
            Some(GitFileStatus::Untracked)
        );
        assert_eq!(
            parsed.status_for_rel_path("target/debug/app"),
            Some(GitFileStatus::Ignored)
        );
    }

    #[test]
    fn parse_log_e_unified_diff_puros_com_fixtures() {
        let log_fixture = format!(
            "1111111111111111111111111111111111111111{f}1111111{f}HEAD -> main, origin/main{f}Bulma{f}2026-11-10T12:00:00-03:00{f}2 hours ago{f}feat: adiciona radar do dragao{r}\n2222222222222222222222222222222222222222{f}2222222{f}{f}Vegeta{f}2026-11-09T10:00:00-03:00{f}yesterday{f}fix: calibra gravidade{r}\n",
            f = LOG_FIELD_SEP,
            r = LOG_RECORD_SEP
        );
        let commits = parse_log_output(&log_fixture);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].short_hash, "1111111");
        assert_eq!(commits[0].author, "Bulma");
        assert_eq!(commits[0].subject, "feat: adiciona radar do dragao");
        assert_eq!(commits[0].refs, vec!["HEAD -> main", "origin/main"]);
        assert_eq!(commits[1].short_hash, "2222222");
        assert!(commits[1].refs.is_empty());

        let diff_fixture = "\
diff --git a/src/lib.rs b/src/lib.rs
index 1234567..89abcdef 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,4 @@
 fn a() {}
-fn old() {}
+fn new_one() {}
+fn new_two() {}
 fn b() {}
";
        let diff = parse_unified_diff(diff_fixture, "src/lib.rs", false);
        assert_eq!(diff.kind, FileDiffKind::Modified);
        assert_eq!(diff.hunks.len(), 1);
        let lines = &diff.hunks[0].lines;
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0].kind, DiffLineKind::Context);
        assert_eq!(lines[1].kind, DiffLineKind::Removed);
        assert_eq!(lines[1].old_line, Some(2));
        assert_eq!(lines[2].kind, DiffLineKind::Added);
        assert_eq!(lines[2].new_line, Some(2));
        assert_eq!(lines[3].kind, DiffLineKind::Added);
        assert_eq!(lines[3].new_line, Some(3));
        assert_eq!(lines[4].kind, DiffLineKind::Context);
        assert_eq!(lines[4].old_line, Some(3));
        assert_eq!(lines[4].new_line, Some(4));
    }

    #[test]
    fn erros_tipados_para_git_ausente_e_pasta_que_nao_eh_repo() {
        let tmp = TempGitRepo::new();
        let bad_client = GitClient::new().with_binary("/caminho/inexistente/git-nao-existe-99999");
        let err = bad_client.status(&tmp.path).unwrap_err();
        assert!(matches!(err, GitError::GitNotInstalled { .. }));

        let Some(client) = test_client() else {
            return;
        };
        let not_repo_err = client.status(&tmp.path).unwrap_err();
        assert_eq!(
            not_repo_err,
            GitError::NotARepository {
                path: tmp.path.clone()
            }
        );
    }

    #[test]
    fn ciclo_completo_em_repo_temporario_init_stage_unstage_commit_diff_e_log() {
        let Some(client) = test_client() else {
            return;
        };
        let tmp = TempGitRepo::new();

        // git init
        let init_status = Command::new(resolve_git_binary())
            .arg("init")
            .arg(&tmp.path)
            .status()
            .expect("run git init");
        assert!(init_status.success());

        // Repo recém-iniciado: log vazio sem erro, status limpo
        assert!(client.log(&tmp.path, 10).unwrap().is_empty());
        assert!(client.status(&tmp.path).unwrap().is_clean());

        // Cria arquivo untracked
        let file_a = tmp.path.join("hello.txt");
        fs::write(&file_a, "linha 1\nlinha 2\n").expect("write hello.txt");

        let st1 = client.status(&tmp.path).unwrap();
        assert_eq!(
            st1.unstaged.get("hello.txt"),
            Some(&GitFileStatus::Untracked)
        );
        assert!(st1.staged.is_empty());

        // Diff de arquivo untracked mostra as linhas adicionadas
        let untracked_diff = client
            .diff_file(&tmp.path, Path::new("hello.txt"), false)
            .unwrap();
        assert_eq!(untracked_diff.kind, FileDiffKind::Added);
        assert_eq!(untracked_diff.hunks[0].lines.len(), 2);

        // Stage e unstage antes do primeiro commit (HEAD unborn)
        client.stage(&tmp.path, &[Path::new("hello.txt")]).unwrap();
        let st_staged = client.status(&tmp.path).unwrap();
        assert_eq!(
            st_staged.staged.get("hello.txt"),
            Some(&GitFileStatus::Staged)
        );
        assert!(st_staged.unstaged.is_empty());

        client
            .unstage(&tmp.path, &[Path::new("hello.txt")])
            .unwrap();
        let st_unstaged = client.status(&tmp.path).unwrap();
        assert!(st_unstaged.staged.is_empty());
        assert_eq!(
            st_unstaged.unstaged.get("hello.txt"),
            Some(&GitFileStatus::Untracked)
        );

        // Stage novamente e faz o primeiro commit via stdin (-F -)
        client.stage(&tmp.path, &[Path::new("hello.txt")]).unwrap();
        let short_sha = client
            .commit(&tmp.path, "feat: primeiro commit\n\ncorpo da mensagem")
            .unwrap();
        assert!(!short_sha.is_empty());

        let st_after_commit = client.status(&tmp.path).unwrap();
        assert!(st_after_commit.is_clean());

        // Modifica o arquivo, checa diff unstaged, faz stage, checa diff staged e unstage
        fs::write(&file_a, "linha 1\nlinha 2 alterada\nlinha 3\n").expect("modify hello.txt");
        let st_mod = client.status(&tmp.path).unwrap();
        assert_eq!(
            st_mod.unstaged.get("hello.txt"),
            Some(&GitFileStatus::Modified)
        );

        let diff_unstaged = client
            .diff_file(&tmp.path, Path::new("hello.txt"), false)
            .unwrap();
        assert_eq!(diff_unstaged.kind, FileDiffKind::Modified);
        assert!(diff_unstaged
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .any(|l| l.kind == DiffLineKind::Added && l.text == "linha 2 alterada"));

        client.stage(&tmp.path, &[Path::new("hello.txt")]).unwrap();
        let diff_staged = client
            .diff_file(&tmp.path, Path::new("hello.txt"), true)
            .unwrap();
        assert_eq!(diff_staged.kind, FileDiffKind::Modified);
        assert!(!diff_staged.hunks.is_empty());

        // Segundo commit e verificação do log
        client.commit(&tmp.path, "fix: atualiza linha 2").unwrap();
        let history = client.log(&tmp.path, 5).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].subject, "fix: atualiza linha 2");
        assert_eq!(history[0].author, "Bulma Briefs");
        assert_eq!(history[1].subject, "feat: primeiro commit");
    }
}
