//! Transformações puras dos dados Git para apresentação.
use crate::{labels, theme};
use gsh_git::{DiffLine, DiffLineKind, GitCommitInfo, GitFileStatus};
use iced::Color;
use std::collections::BTreeMap;

pub fn files(statuses: &BTreeMap<String, GitFileStatus>) -> Vec<(&str, GitFileStatus)> {
    statuses
        .iter()
        .map(|(path, status)| (path.as_str(), *status))
        .collect()
}
pub fn commit(commit: &GitCommitInfo) -> (&str, String) {
    let title = if commit.subject.is_empty() {
        labels::GIT_HISTORY_UNTITLED
    } else {
        &commit.subject
    };
    (
        title,
        labels::format_commit_meta(
            &commit.short_hash,
            &commit.author,
            if commit.date_relative.is_empty() {
                &commit.date_iso
            } else {
                &commit.date_relative
            },
        ),
    )
}
pub fn diff_line(line: &DiffLine) -> String {
    let prefix = match line.kind {
        DiffLineKind::Added => labels::GIT_DIFF_PREFIX_ADD,
        DiffLineKind::Removed => labels::GIT_DIFF_PREFIX_DEL,
        DiffLineKind::Context => labels::GIT_DIFF_PREFIX_CTX,
    };
    format!("{prefix}{}", line.text)
}

pub fn shorten(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_owned();
    }
    format!(
        "{}{}",
        value
            .chars()
            .take(limit.saturating_sub(1))
            .collect::<String>(),
        labels::GIT_ELLIPSIS
    )
}
pub fn status_color(status: GitFileStatus) -> Color {
    match status {
        GitFileStatus::Staged => theme::palette().git_staged,
        GitFileStatus::Untracked => theme::palette().git_untracked,
        GitFileStatus::Deleted => theme::palette().git_deleted,
        GitFileStatus::Modified => theme::palette().warn,
        GitFileStatus::Conflict => theme::GIT_CONFLICT,
        GitFileStatus::Ignored => theme::palette().text4,
    }
}
pub fn diff_style(kind: DiffLineKind) -> (Color, Option<Color>) {
    match kind {
        DiffLineKind::Added => (theme::palette().git_staged, Some(theme::GIT_ADDED_BG)),
        DiffLineKind::Removed => (theme::palette().git_deleted, Some(theme::GIT_REMOVED_BG)),
        DiffLineKind::Context => (theme::palette().text, None),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sorted_files_and_all_diff_kinds_and_relative_date() {
        assert!(files(&BTreeMap::new()).is_empty());
        let map = BTreeMap::from([
            ("z".into(), GitFileStatus::Modified),
            ("a".into(), GitFileStatus::Untracked),
        ]);
        assert_eq!(
            files(&map)
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>(),
            vec!["a", "z"]
        );
        for (kind, expected) in [
            (DiffLineKind::Added, "+line"),
            (DiffLineKind::Context, " line"),
            (DiffLineKind::Removed, "-line"),
        ] {
            let line = DiffLine {
                kind,
                old_line: None,
                new_line: None,
                text: "line".into(),
            };
            assert_eq!(diff_line(&line), expected);
        }
        let info = GitCommitInfo {
            hash: String::new(),
            short_hash: "abc".into(),
            author: "author".into(),
            date_iso: "iso".into(),
            date_relative: "relative".into(),
            subject: "subject".into(),
            refs: vec![],
        };
        let (_, meta) = commit(&info);
        assert_eq!(
            meta,
            labels::format_commit_meta("abc", "author", "relative")
        );
        assert!(!meta.contains("iso"));
    }
    #[test]
    fn maps_status_history_and_diff() {
        let statuses = BTreeMap::from([("a".into(), GitFileStatus::Untracked)]);
        assert_eq!(files(&statuses), vec![("a", GitFileStatus::Untracked)]);
        let line = DiffLine {
            kind: DiffLineKind::Removed,
            old_line: Some(1),
            new_line: None,
            text: "old".into(),
        };
        assert_eq!(diff_line(&line), "-old");
        let info = GitCommitInfo {
            hash: "123".into(),
            short_hash: "123".into(),
            author: "Bulma".into(),
            date_iso: "2026".into(),
            date_relative: String::new(),
            subject: String::new(),
            refs: vec![],
        };
        let (title, meta) = commit(&info);
        assert_eq!(title, labels::GIT_HISTORY_UNTITLED);
        assert!(meta.contains("123") && meta.contains("Bulma") && meta.contains("2026"));
    }
}
