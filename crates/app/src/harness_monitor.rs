//! Detecção do harness interativo ativo na árvore de processos de cada PTY.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::process::Command;

pub type ProcessInfo = (u32, u32, bool, bool, String, Vec<String>);

pub fn detect(root: u32, processes: &[ProcessInfo]) -> Option<&'static str> {
    let by_pid = processes
        .iter()
        .map(|p| (p.0, p))
        .collect::<BTreeMap<_, _>>();
    if !by_pid.contains_key(&root) {
        return None;
    }
    let mut children = BTreeMap::<u32, Vec<u32>>::new();
    for p in processes {
        children.entry(p.1).or_default().push(p.0);
    }
    let mut queue = VecDeque::from([(root, 0usize)]);
    let mut seen = BTreeSet::new();
    let mut found = None;
    let mut deepest = 0;
    while let Some((pid, depth)) = queue.pop_front() {
        if !seen.insert(pid) {
            continue;
        }
        if let Some(p) = by_pid.get(&pid) {
            if p.2 && !p.3 {
                if let Some(kind) = classify(&p.4, &p.5) {
                    if depth >= deepest {
                        deepest = depth;
                        found = Some(kind);
                    }
                }
            }
            queue.extend(
                children
                    .get(&pid)
                    .into_iter()
                    .flatten()
                    .map(|child| (*child, depth + 1)),
            );
        }
    }
    found
}

fn classify(executable: &str, argv: &[String]) -> Option<&'static str> {
    let exe = std::path::Path::new(executable)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(executable)
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    let args = argv.get(1..).unwrap_or_default();
    let runtime = matches!(
        exe.as_str(),
        "node" | "bun" | "deno" | "python" | "python3" | "npx" | "uvx"
    );
    let (entry, harness_args) = if runtime {
        let index = args.iter().position(|arg| !arg.starts_with('-'))?;
        let script = args.get(index)?;
        let path = script.to_ascii_lowercase();
        let entry = if path.contains("claude-code") {
            "claude".to_owned()
        } else if path.contains("pi-coding-agent") {
            "pi".to_owned()
        } else if path.contains("cursor-agent") {
            "cursor-agent".to_owned()
        } else {
            std::path::Path::new(script)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(script)
                .to_owned()
        };
        (entry, args.get(index + 1..).unwrap_or_default())
    } else {
        (exe, args)
    };
    let first = harness_args
        .first()
        .map(|arg| arg.to_ascii_lowercase())
        .unwrap_or_default();
    if harness_args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "-v" | "--version"))
    {
        return None;
    }
    let print_flags: &[&str] = match entry.as_str() {
        "agy" | "antigravity" | "claude" | "claude-code" | "cursor" | "cursor-agent" | "agent"
        | "pi" => &["-p", "--print"],
        "copilot" | "github-copilot" => &["-p", "--prompt", "-i", "--input"],
        _ => &[],
    };
    if harness_args
        .iter()
        .any(|arg| print_flags.contains(&arg.as_str()))
    {
        return None;
    }
    if entry == "pi"
        && harness_args.iter().enumerate().any(|(index, arg)| {
            let mode = arg.strip_prefix("--mode=").or_else(|| {
                (arg == "--mode")
                    .then(|| harness_args.get(index + 1).map(String::as_str))
                    .flatten()
            });
            mode.is_some_and(|mode| {
                matches!(mode.to_ascii_lowercase().as_str(), "text" | "json" | "rpc")
            })
        })
    {
        return None;
    }
    let noninteractive = |args: &[&str]| args.contains(&first.as_str());
    match entry.as_str() {
        "agy" | "antigravity"
            if !noninteractive(&[
                "help", "version", "config", "auth", "models", "update", "-p", "--print",
            ]) =>
        {
            Some("antigravity")
        }
        "claude" | "claude-code"
            if !noninteractive(&[
                "mcp", "plugin", "config", "auth", "doctor", "update", "version", "help", "-p",
                "--print",
            ]) =>
        {
            Some("claudecode")
        }
        "codex" if !noninteractive(&["exec", "auth", "config", "version", "help", "models"]) => {
            Some("codex")
        }
        "cursor-agent" | "cursor" | "agent"
            if !noninteractive(&[
                "auth", "login", "logout", "list", "version", "help", "-p", "--print",
            ]) =>
        {
            Some("cursor")
        }
        "copilot" | "github-copilot"
            if !noninteractive(&[
                "auth", "config", "alias", "version", "help", "-p", "--prompt", "-i", "--input",
            ]) =>
        {
            Some("githubcopilot")
        }
        "opencode" if !noninteractive(&["run", "auth", "config", "models", "version", "help"]) => {
            Some("opencode")
        }
        "pi" if !noninteractive(&[
            "export", "auth", "config", "models", "version", "help", "doctor", "-p", "--print",
        ]) =>
        {
            Some("pi")
        }
        _ => None,
    }
}

pub fn scan(root_pids: &[u32]) -> Vec<ProcessInfo> {
    if root_pids.is_empty() {
        return Vec::new();
    }
    #[cfg(target_os = "linux")]
    {
        return scan_proc(root_pids);
    }
    #[cfg(target_os = "macos")]
    {
        let Ok(output) = Command::new("ps")
            .args(["-eo", "pid,ppid,state,tpgid,pgid,command"])
            .output()
        else {
            return Vec::new();
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let all = text
            .lines()
            .skip(1)
            .filter_map(|l| {
                let mut p = l.split_whitespace();
                let pid = p.next()?.parse().ok()?;
                let parent = p.next()?.parse().ok()?;
                let state = p.next()?.to_owned();
                let tpgid: i32 = p.next()?.parse().ok()?;
                let pgid: i32 = p.next()?.parse().ok()?;
                let argv = p.map(str::to_owned).collect::<Vec<_>>();
                let exe = argv.first()?.clone();
                Some((
                    pid,
                    parent,
                    tpgid <= 0 || tpgid == pgid,
                    state.contains('T'),
                    exe,
                    argv,
                ))
            })
            .collect::<Vec<_>>();
        return rooted(all, root_pids);
    }
    #[cfg(target_os = "windows")]
    {
        let Ok(output) = Command::new("powershell.exe").args(["-NoProfile", "-Command", "Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,ExecutablePath,CommandLine | ConvertTo-Json -Compress"]).output() else { return Vec::new() };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
            return Vec::new();
        };
        let list = value.as_array().cloned().unwrap_or_else(|| vec![value]);
        let all = list
            .iter()
            .filter_map(|v| {
                let pid = v.get("ProcessId")?.as_u64()? as u32;
                let parent = v.get("ParentProcessId")?.as_u64()? as u32;
                let exe = v.get("ExecutablePath")?.as_str()?.to_owned();
                let cmd = v
                    .get("CommandLine")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&exe);
                let argv = cmd.split_whitespace().map(str::to_owned).collect();
                Some((pid, parent, true, false, exe, argv))
            })
            .collect::<Vec<_>>();
        return rooted(all, root_pids);
    }
    #[allow(unreachable_code)]
    Vec::new()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn rooted(all: Vec<ProcessInfo>, roots: &[u32]) -> Vec<ProcessInfo> {
    let mut children = BTreeMap::<u32, Vec<u32>>::new();
    let by_pid = all.iter().map(|p| (p.0, p)).collect::<BTreeMap<_, _>>();
    for p in &all {
        children.entry(p.1).or_default().push(p.0);
    }
    let mut pending = VecDeque::from(roots.to_vec());
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    while let Some(pid) = pending.pop_front() {
        if !seen.insert(pid) {
            continue;
        }
        if let Some(p) = by_pid.get(&pid) {
            result.push((*p).clone());
            pending.extend(children.get(&pid).into_iter().flatten().copied());
        }
    }
    result
}

#[cfg(target_os = "linux")]
fn scan_proc(roots: &[u32]) -> Vec<ProcessInfo> {
    let mut pending = VecDeque::from(roots.to_vec());
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    while let Some(pid) = pending.pop_front() {
        if !seen.insert(pid) {
            continue;
        }
        let base = format!("/proc/{pid}");
        let Ok(stat) = std::fs::read_to_string(format!("{base}/stat")) else {
            continue;
        };
        let Some(end) = stat.rfind(')') else { continue };
        let Some(comm) = stat.get(stat.find('(').unwrap_or(0) + 1..end) else {
            continue;
        };
        let fields = stat
            .get(end + 2..)
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>();
        let (Some(state), Some(parent), Some(pgrp), Some(tpgid)) =
            (fields.first(), fields.get(1), fields.get(2), fields.get(5))
        else {
            continue;
        };
        let parent = parent.parse::<u32>().unwrap_or(0);
        let pgrp = pgrp.parse::<i32>().unwrap_or(0);
        let tpgid = tpgid.parse::<i32>().unwrap_or(-1);
        let argv = std::fs::read(format!("{base}/cmdline"))
            .ok()
            .map(|b| {
                b.split(|c| *c == 0)
                    .filter(|p| !p.is_empty())
                    .map(|p| String::from_utf8_lossy(p).into_owned())
                    .collect::<Vec<_>>()
            })
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| vec![comm.to_owned()]);
        let Some(executable) = argv.first().cloned() else {
            continue;
        };
        for child in std::fs::read_to_string(format!("{base}/task/{pid}/children"))
            .unwrap_or_default()
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
        {
            pending.push_back(child);
        }
        out.push((
            pid,
            parent,
            tpgid <= 0 || pgrp == tpgid,
            state.contains('T'),
            executable,
            argv,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::detect;

    #[test]
    fn detects_only_foreground_interactive_descendants_of_the_pty() {
        let processes = vec![
            (10, 1, true, false, "/bin/zsh".into(), vec!["zsh".into()]),
            (
                11,
                10,
                true,
                false,
                "/usr/bin/agy".into(),
                vec!["agy".into()],
            ),
            (
                12,
                10,
                false,
                false,
                "/usr/bin/claude".into(),
                vec!["claude".into()],
            ),
            (
                20,
                1,
                true,
                false,
                "/usr/bin/codex".into(),
                vec!["codex".into()],
            ),
        ];
        assert_eq!(detect(10, &processes), Some("antigravity"));
        assert_eq!(detect(20, &processes), Some("codex"));
        assert_eq!(detect(99, &processes), None);
    }

    #[test]
    fn ignores_non_interactive_invocations() {
        let processes = vec![
            (10, 1, true, false, "zsh".into(), vec!["zsh".into()]),
            (
                11,
                10,
                true,
                false,
                "claude".into(),
                vec!["claude".into(), "--print".into()],
            ),
        ];
        assert_eq!(detect(10, &processes), None);
    }

    #[test]
    fn classifies_wrapped_runtime_commands_and_prefers_deepest_harness() {
        let processes = vec![
            (10, 1, true, false, "zsh".into(), vec!["zsh".into()]),
            (11, 10, true, false, "agy".into(), vec!["agy".into()]),
            (
                12,
                10,
                true,
                false,
                "node".into(),
                vec![
                    "node".into(),
                    "/usr/lib/node_modules/@anthropic-ai/claude-code/cli.js".into(),
                ],
            ),
        ];
        assert_eq!(detect(10, &processes), Some("claudecode"));
    }

    #[test]
    fn process_tree_and_classifier_match_flutter_dart_golden() {
        let golden: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../golden/harness_monitor_golden.json"
        ))
        .expect("Dart golden JSON is valid");
        let scenarios = [
            (
                "agy",
                vec![
                    (10, 1, true, false, "zsh".into(), vec!["zsh".into()]),
                    (11, 10, true, false, "agy".into(), vec!["agy".into()]),
                ],
            ),
            (
                "claude_print",
                vec![
                    (10, 1, true, false, "zsh".into(), vec!["zsh".into()]),
                    (
                        11,
                        10,
                        true,
                        false,
                        "claude".into(),
                        vec!["claude".into(), "--print".into()],
                    ),
                ],
            ),
            (
                "wrapped_claude",
                vec![
                    (10, 1, true, false, "zsh".into(), vec!["zsh".into()]),
                    (
                        11,
                        10,
                        true,
                        false,
                        "node".into(),
                        vec![
                            "node".into(),
                            "/usr/lib/node_modules/claude-code/cli.js".into(),
                        ],
                    ),
                ],
            ),
            (
                "foreground_only",
                vec![
                    (10, 1, true, false, "zsh".into(), vec!["zsh".into()]),
                    (11, 10, false, false, "agy".into(), vec!["agy".into()]),
                    (12, 10, true, false, "codex".into(), vec!["codex".into()]),
                ],
            ),
        ];
        for (key, processes) in scenarios {
            assert_eq!(
                detect(10, &processes),
                golden.get(key).and_then(serde_json::Value::as_str),
                "{key}"
            );
        }
    }
}
