//! Parsers puros para saídas do binário `git` (`status --porcelain=v2 -z`,
//! `log` com separadores ASCII `\x1f`/`\x1e` e unified `diff`).

use std::collections::{BTreeMap, BTreeSet};

use crate::types::{
    DiffHunk, DiffLine, DiffLineKind, FileDiff, FileDiffKind, GitCommitInfo, GitFileStatus,
    GitRepoStatus,
};

/// Separador de registros no formato estruturado do `git log` (`RS`, `0x1e`).
pub const LOG_RECORD_SEP: char = '\x1e';
/// Separador de campos no formato estruturado do `git log` (`US`, `0x1f`).
pub const LOG_FIELD_SEP: char = '\x1f';

/// Parseia a saída de `git status --porcelain=v2 --branch -z` (também aceita
/// entradas `XY <path>` do `--porcelain=v1 -z` por compatibilidade).
pub fn parse_porcelain_v2_z(raw: &str) -> GitRepoStatus {
    let mut branch_head: Option<String> = None;
    let mut branch_oid: Option<String> = None;
    let mut ahead: usize = 0;
    let mut behind: usize = 0;
    let mut staged = BTreeMap::new();
    let mut unstaged = BTreeMap::new();
    let mut files = BTreeMap::new();
    let mut untracked_dirs = BTreeSet::new();
    let mut ignored_roots = BTreeSet::new();

    let mut tokens = raw.split('\0');
    while let Some(entry) = tokens.next() {
        let entry = entry.trim_start_matches(['\n', '\r']);
        if entry.is_empty() {
            continue;
        }
        if let Some(header) = entry.strip_prefix("# ") {
            parse_branch_header(
                header,
                &mut branch_head,
                &mut branch_oid,
                &mut ahead,
                &mut behind,
            );
            continue;
        }
        if let Some(rest) = entry.strip_prefix("1 ") {
            // 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
            if let Some((xy, path)) = extract_v2_xy_and_path(rest, 7) {
                record_xy_entry(xy, path, &mut files, &mut staged, &mut unstaged);
            }
            continue;
        }
        if let Some(rest) = entry.strip_prefix("2 ") {
            // 2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <X><score> <path>\0<origPath>
            if let Some((xy, path)) = extract_v2_xy_and_path(rest, 8) {
                let _orig_path = tokens.next();
                record_xy_entry(xy, path, &mut files, &mut staged, &mut unstaged);
            }
            continue;
        }
        if let Some(rest) = entry.strip_prefix("u ") {
            // u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>
            if let Some((_xy, path)) = extract_v2_xy_and_path(rest, 9) {
                let clean = path.trim_end_matches('/').to_string();
                if !clean.is_empty() {
                    files.insert(clean.clone(), GitFileStatus::Conflict);
                    staged.insert(clean.clone(), GitFileStatus::Conflict);
                    unstaged.insert(clean, GitFileStatus::Conflict);
                }
            }
            continue;
        }
        if let Some(path) = entry.strip_prefix("? ") {
            record_untracked(path, &mut files, &mut unstaged, &mut untracked_dirs);
            continue;
        }
        if let Some(path) = entry.strip_prefix("! ") {
            let clean = path.trim_end_matches('/');
            if !clean.is_empty() {
                ignored_roots.insert(clean.to_string());
            }
            continue;
        }
        // Fallback para entradas porcelain v1 (`XY <path>`)
        parse_v1_entry(
            entry,
            &mut tokens,
            &mut files,
            &mut staged,
            &mut unstaged,
            &mut untracked_dirs,
            &mut ignored_roots,
        );
    }

    let branch = resolve_branch_name(branch_head, branch_oid);
    let folder_statuses = build_folder_statuses(&files);

    GitRepoStatus {
        branch,
        ahead,
        behind,
        staged,
        unstaged,
        files,
        folder_statuses,
        untracked_dirs,
        ignored_roots,
    }
}

fn parse_branch_header(
    header: &str,
    branch_head: &mut Option<String>,
    branch_oid: &mut Option<String>,
    ahead: &mut usize,
    behind: &mut usize,
) {
    if let Some(oid) = header.strip_prefix("branch.oid ") {
        *branch_oid = Some(oid.trim().to_string());
    } else if let Some(head) = header.strip_prefix("branch.head ") {
        *branch_head = Some(head.trim().to_string());
    } else if let Some(ab) = header.strip_prefix("branch.ab ") {
        for part in ab.split_whitespace() {
            if let Some(a) = part.strip_prefix('+') {
                *ahead = a.parse::<usize>().unwrap_or(0);
            } else if let Some(b) = part.strip_prefix('-') {
                *behind = b.parse::<usize>().unwrap_or(0);
            }
        }
    }
}

fn resolve_branch_name(branch_head: Option<String>, branch_oid: Option<String>) -> String {
    match branch_head.as_deref() {
        Some("(detached)") => branch_oid
            .as_deref()
            .filter(|oid| *oid != "(initial)" && !oid.is_empty())
            .map(|oid| oid.chars().take(7).collect())
            .unwrap_or_else(|| "HEAD".to_string()),
        Some(name) if !name.is_empty() => name.to_string(),
        _ => "HEAD".to_string(),
    }
}

/// Extrai `(XY, path)` de uma linha porcelain v2 após o prefixo `"1 "` / `"2 "` / `"u "`.
/// `fields_before_path` indica quantos campos separados por `' '` existem antes do `<path>`
/// (incluindo o campo `XY`).
fn extract_v2_xy_and_path(rest: &str, fields_before_path: usize) -> Option<((char, char), &str)> {
    let mut chars = rest.chars();
    let x = chars.next()?;
    let y = chars.next()?;
    let mut cursor = rest;
    for _ in 0..fields_before_path {
        let (_, after_space) = cursor.split_once(' ')?;
        cursor = after_space;
    }
    Some(((x, y), cursor))
}

fn record_untracked(
    raw_path: &str,
    files: &mut BTreeMap<String, GitFileStatus>,
    unstaged: &mut BTreeMap<String, GitFileStatus>,
    untracked_dirs: &mut BTreeSet<String>,
) {
    let is_dir = raw_path.ends_with('/');
    let clean = raw_path.trim_end_matches('/');
    if clean.is_empty() {
        return;
    }
    if is_dir {
        untracked_dirs.insert(clean.to_string());
    } else {
        files.insert(clean.to_string(), GitFileStatus::Untracked);
        unstaged.insert(clean.to_string(), GitFileStatus::Untracked);
    }
}

fn record_xy_entry(
    (x, y): (char, char),
    raw_path: &str,
    files: &mut BTreeMap<String, GitFileStatus>,
    staged: &mut BTreeMap<String, GitFileStatus>,
    unstaged: &mut BTreeMap<String, GitFileStatus>,
) {
    let clean = raw_path.trim_end_matches('/');
    if clean.is_empty() {
        return;
    }
    if let Some(tree_st) = classify_combined(x, y) {
        files.insert(clean.to_string(), tree_st);
    }
    if let Some(idx_st) = classify_index(x, y) {
        staged.insert(clean.to_string(), idx_st);
    }
    if let Some(wt_st) = classify_worktree(x, y) {
        unstaged.insert(clean.to_string(), wt_st);
    }
}

fn parse_v1_entry<'a, I>(
    entry: &str,
    tokens: &mut I,
    files: &mut BTreeMap<String, GitFileStatus>,
    staged: &mut BTreeMap<String, GitFileStatus>,
    unstaged: &mut BTreeMap<String, GitFileStatus>,
    untracked_dirs: &mut BTreeSet<String>,
    ignored_roots: &mut BTreeSet<String>,
) where
    I: Iterator<Item = &'a str>,
{
    let mut chars = entry.chars();
    let (Some(x), Some(y), Some(' ')) = (chars.next(), chars.next(), chars.next()) else {
        return;
    };
    let path_part = chars.as_str();
    if x == 'R' || x == 'C' {
        let _orig = tokens.next();
    }
    if x == '!' && y == '!' {
        let clean = path_part.trim_end_matches('/');
        if !clean.is_empty() {
            ignored_roots.insert(clean.to_string());
        }
        return;
    }
    if x == '?' && y == '?' {
        record_untracked(path_part, files, unstaged, untracked_dirs);
        return;
    }
    record_xy_entry((x, y), path_part, files, staged, unstaged);
}

fn is_conflict_pair(x: char, y: char) -> bool {
    x == 'U' || y == 'U' || (x == 'D' && y == 'D') || (x == 'A' && y == 'A')
}

fn classify_index(x: char, y: char) -> Option<GitFileStatus> {
    if matches!(x, '.' | ' ' | '?' | '!') {
        return None;
    }
    if is_conflict_pair(x, y) {
        return Some(GitFileStatus::Conflict);
    }
    if x == 'D' {
        return Some(GitFileStatus::Deleted);
    }
    Some(GitFileStatus::Staged)
}

fn classify_worktree(x: char, y: char) -> Option<GitFileStatus> {
    if x == '?' && y == '?' {
        return Some(GitFileStatus::Untracked);
    }
    if matches!(y, '.' | ' ' | '!') {
        return None;
    }
    if is_conflict_pair(x, y) {
        return Some(GitFileStatus::Conflict);
    }
    if y == 'D' {
        return Some(GitFileStatus::Deleted);
    }
    Some(GitFileStatus::Modified)
}

fn classify_combined(x: char, y: char) -> Option<GitFileStatus> {
    if x == '?' && y == '?' {
        return Some(GitFileStatus::Untracked);
    }
    if x == '!' && y == '!' {
        return None;
    }
    if is_conflict_pair(x, y) {
        return Some(GitFileStatus::Conflict);
    }
    if x == 'D' || y == 'D' {
        return Some(GitFileStatus::Deleted);
    }
    if matches!(y, 'M' | 'T' | 'R' | 'C') {
        return Some(GitFileStatus::Modified);
    }
    if matches!(x, 'M' | 'T' | 'R' | 'C' | 'A') {
        return Some(GitFileStatus::Staged);
    }
    None
}

fn build_folder_statuses(
    files: &BTreeMap<String, GitFileStatus>,
) -> BTreeMap<String, GitFileStatus> {
    let mut folders = BTreeMap::new();
    for (path, &status) in files {
        let mut current = path.as_str();
        while let Some((parent, _)) = current.rsplit_once('/') {
            if parent.is_empty() {
                break;
            }
            let updated = GitFileStatus::strongest_for_folder(folders.get(parent).copied(), status);
            if let Some(st) = updated {
                folders.insert(parent.to_string(), st);
            }
            current = parent;
        }
    }
    folders
}

/// Parseia a saída estruturada do `git log` com formato
/// `%H%x1f%h%x1f%D%x1f%an%x1f%aI%x1f%ar%x1f%s%x1e`.
pub fn parse_log_output(raw: &str) -> Vec<GitCommitInfo> {
    let mut commits = Vec::new();
    for record in raw.split(LOG_RECORD_SEP) {
        let trimmed = record.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(7, LOG_FIELD_SEP);
        let (
            Some(hash),
            Some(short_hash),
            Some(decorations),
            Some(author),
            Some(date_iso),
            Some(date_relative),
            Some(subject),
        ) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        )
        else {
            continue;
        };
        if hash.is_empty() {
            continue;
        }
        let refs = decorations
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        commits.push(GitCommitInfo {
            hash: hash.to_string(),
            short_hash: short_hash.to_string(),
            author: author.to_string(),
            date_iso: date_iso.to_string(),
            date_relative: date_relative.to_string(),
            subject: subject.to_string(),
            refs,
        });
    }
    commits
}

/// Parseia a saída de um unified diff (`git diff`) em hunks e linhas numeradas.
pub fn parse_unified_diff(raw: &str, path: &str, staged: bool) -> FileDiff {
    if raw.trim().is_empty() {
        return FileDiff::unchanged(path, staged);
    }
    if looks_like_binary_diff(raw) {
        return FileDiff::binary(path, staged);
    }

    let mut kind = FileDiffKind::Modified;
    let mut hunks: Vec<DiffHunk> = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_lines: Vec<DiffLine> = Vec::new();
    let mut old_no: usize = 0;
    let mut new_no: usize = 0;

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("--- ") {
            if rest.starts_with("/dev/null") {
                kind = FileDiffKind::Added;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            if rest.starts_with("/dev/null") {
                kind = FileDiffKind::Deleted;
            }
            continue;
        }
        if line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("new file")
            || line.starts_with("deleted file")
            || line.starts_with("similarity ")
            || line.starts_with("rename ")
        {
            continue;
        }
        if let Some((start_old, start_new)) = parse_hunk_header(line) {
            if let Some(header) = current_header.take() {
                hunks.push(DiffHunk {
                    header,
                    lines: std::mem::take(&mut current_lines),
                });
            }
            old_no = start_old;
            new_no = start_new;
            current_header = Some(line.to_string());
            continue;
        }
        if current_header.is_none() || line.starts_with('\\') {
            continue;
        }
        if let Some(added) = line.strip_prefix('+') {
            current_lines.push(DiffLine {
                kind: DiffLineKind::Added,
                old_line: None,
                new_line: Some(new_no),
                text: added.to_string(),
            });
            new_no = new_no.saturating_add(1);
        } else if let Some(removed) = line.strip_prefix('-') {
            current_lines.push(DiffLine {
                kind: DiffLineKind::Removed,
                old_line: Some(old_no),
                new_line: None,
                text: removed.to_string(),
            });
            old_no = old_no.saturating_add(1);
        } else if let Some(ctx) = line.strip_prefix(' ') {
            current_lines.push(DiffLine {
                kind: DiffLineKind::Context,
                old_line: Some(old_no),
                new_line: Some(new_no),
                text: ctx.to_string(),
            });
            old_no = old_no.saturating_add(1);
            new_no = new_no.saturating_add(1);
        }
    }

    if let Some(header) = current_header.take() {
        hunks.push(DiffHunk {
            header,
            lines: current_lines,
        });
    }
    if hunks.is_empty() {
        return FileDiff::unchanged(path, staged);
    }
    FileDiff {
        path: path.to_string(),
        staged,
        kind,
        hunks,
    }
}

fn looks_like_binary_diff(raw: &str) -> bool {
    raw.lines().any(|line| {
        (line.starts_with("Binary files ") && line.ends_with(" differ"))
            || line == "GIT binary patch"
    })
}

/// Extrai `(old_start, new_start)` de um cabeçalho `@@ -old[,count] +new[,count] @@`.
fn parse_hunk_header(line: &str) -> Option<(usize, usize)> {
    let body = line.strip_prefix("@@ -")?;
    let (ranges, _) = body.split_once(" @@")?;
    let (old_part, new_part) = ranges.split_once(" +")?;
    let old_start = old_part
        .split_once(',')
        .map_or(old_part, |(s, _)| s)
        .parse::<usize>()
        .ok()?;
    let new_start = new_part
        .split_once(',')
        .map_or(new_part, |(s, _)| s)
        .parse::<usize>()
        .ok()?;
    Some((old_start, new_start))
}
