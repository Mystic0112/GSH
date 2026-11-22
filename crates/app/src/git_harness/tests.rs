use super::*;
use response::{parse_subject, subject};

#[test]
fn automation_disabled_and_unisolated_harnesses_fail_before_launch() {
    let repo = Path::new("nonexistent-git-harness-test-repo");
    assert_eq!(
        suggest_commit_message(repo, &Automation::default()),
        Err(Failure::Disabled.into())
    );
    for (kind, harness) in [
        (HarnessKind::Antigravity, "Antigravity"),
        (HarnessKind::ClaudeCode, "Claude Code"),
        (HarnessKind::Codex, "Codex"),
        (HarnessKind::Cursor, "Cursor"),
        (HarnessKind::GitHubCopilot, "GitHub Copilot"),
        (HarnessKind::OpenCode, "OpenCode"),
    ] {
        let automation = Automation {
            harness_id: Some(kind),
            model_id: None,
        };
        assert_eq!(
            suggest_commit_message(repo, &automation),
            Err(Failure::UnsupportedHarness { harness }.into())
        );
    }
}

#[test]
fn pi_command_selects_model_and_disables_startup_and_tool_discovery() {
    assert_eq!(executable(HarnessKind::Pi), Ok("pi"));
    let mut command = Command::new("pi");
    configure_command(&mut command, Some("provider/example")).unwrap();
    let args = command
        .as_std()
        .get_args()
        .map(|arg| arg.to_str().unwrap())
        .collect::<Vec<_>>();
    assert!(args
        .windows(2)
        .any(|pair| pair == ["--model", "provider/example"]));
    for flag in [
        "--no-tools",
        "--no-extensions",
        "--no-mcp",
        "--no-session",
        "--no-skills",
        "--no-context-files",
        "--no-prompt-templates",
        "--no-themes",
        "--no-approve",
    ] {
        assert!(args.contains(&flag));
    }

    let mut command = Command::new("pi");
    assert_eq!(
        configure_command(&mut command, Some("--tools")),
        Err(Failure::InvalidModel)
    );
    configure_command(&mut command, None).unwrap();
    assert!(!command.as_std().get_args().any(|arg| arg == "--model"));
}

#[test]
fn rejects_multiline_wrapped_and_non_conventional_responses() {
    assert_eq!(
        subject(" fix(git)!: handle empty diff \n"),
        Some("fix(git)!: handle empty diff")
    );
    for invalid in [
        "feat(git): add branch picker\nDetails",
        "```\nfix: handle empty diff\n```",
        "Please use this commit subject",
        "Here is your result: fix: handle empty diff",
        "fix(): empty scope",
        "fix: ",
    ] {
        assert_eq!(subject(invalid), None);
    }
}

#[test]
fn extracts_pi_subject_only_after_successful_tool_free_completion() {
    let pi = r#"{"type":"message_end","message":{"role":"user","content":"the diff"}}
{"type":"message_end","message":{"role":"assistant","stopReason":"stop","content":[{"type":"text","text":"fix: handle empty diff"}]}}
{"type":"agent_end"}"#;
    assert_eq!(parse_subject(pi), Ok("fix: handle empty diff".into()));
    assert_eq!(
        parse_subject(pi.lines().nth(1).unwrap()),
        Err(Failure::InvalidResponse)
    );
    assert_eq!(
        parse_subject(r#"{"type":"tool_execution_start"}"#),
        Err(Failure::ToolUse)
    );
}

#[cfg(unix)]
#[test]
fn subprocess_handles_stdin_and_enforces_a_deadline() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut echo = Command::new("cat");
        echo.arg("-");
        let prompt = "+@literal-file\n".repeat(5 * 1024);
        assert_eq!(
            capture(echo, "cat", Some(&prompt), Duration::from_secs(3)).await,
            Ok(prompt)
        );
        let mut delayed = Command::new("sleep");
        delayed.arg("3");
        assert_eq!(
            capture(delayed, "sleep", None, Duration::from_secs(1)).await,
            Err(Failure::Timeout { seconds: 1 })
        );
    });
}
