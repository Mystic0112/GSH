//! Leitura do harness direto dos arquivos (`ClaudeHarnessSource.read` do Flutter).
//!
//! - instruções: `CLAUDE.md` do usuário e do projeto para cima (a subida do Claude
//!   Code), `CLAUDE.local.md` e `AGENTS.md` (Codex e outros);
//! - `skills/*/SKILL.md`, `agents/*.md`, `commands/**/*.md` do usuário e do projeto;
//! - hooks e permissões de `settings.json` (usuário e projeto) e `settings.local.json`;
//! - MCP: `.mcp.json` (projeto), `~/.claude.json` (usuário e local por pasta).
//!
//! Nada aqui falha: arquivo ilegível some da lista, JSON inválido vai para
//! `invalid_files`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::frontmatter::parse_frontmatter;
use crate::hook::hook_script_path;
use crate::json::Json;
use crate::paths::{normalize, relative};
use crate::{listing_chars, Entry, Issue, Kind, ProjectHarness, Scope, MAX_INSTRUCTION_CHARS};

/// Profundidade máxima de `commands/` (guarda contra link simbólico em laço).
const MAX_COMMAND_DEPTH: usize = 16;

/// Lê o harness de `workspace`; `home` vazio = sem nada do usuário.
pub fn read(workspace: &Path, home: &Path) -> ProjectHarness {
    let has_home = !home.as_os_str().is_empty();
    let user_claude = home.join(".claude");
    let project_claude = workspace.join(".claude");
    let mut invalid = Vec::new();

    let mut entries = memory(workspace, home);
    entries.extend(skills(workspace, home));
    entries.extend(markdowns(
        Kind::Agents,
        &project_claude.join("agents"),
        Scope::Project,
    ));
    if has_home {
        entries.extend(markdowns(
            Kind::Agents,
            &user_claude.join("agents"),
            Scope::User,
        ));
    }
    entries.extend(markdowns(
        Kind::Commands,
        &project_claude.join("commands"),
        Scope::Project,
    ));
    if has_home {
        entries.extend(markdowns(
            Kind::Commands,
            &user_claude.join("commands"),
            Scope::User,
        ));
    }
    let mut settings = vec![
        (project_claude.join("settings.json"), Scope::Project),
        (project_claude.join("settings.local.json"), Scope::Local),
    ];
    if has_home {
        settings.push((user_claude.join("settings.json"), Scope::User));
    }
    for (path, scope) in settings {
        let json = read_json(&path, Some(&mut invalid));
        entries.extend(settings_entries(
            json.as_ref(),
            &path,
            scope,
            workspace,
            home,
        ));
    }
    let mcp_path = workspace.join(".mcp.json");
    let mcp = read_json(&mcp_path, Some(&mut invalid));
    entries.extend(mcp_entries(mcp.as_ref(), &mcp_path, Scope::Project, true));
    if has_home {
        entries.extend(claude_json_mcp(workspace, home));
    }
    ProjectHarness {
        entries,
        invalid_files: invalid,
    }
}

/// JSON de `path`; inválido entra em `invalid` (o Claude Code ignora o arquivo
/// inteiro sem avisar, o painel avisa).
pub(crate) fn read_json(path: &Path, invalid: Option<&mut Vec<PathBuf>>) -> Option<Json> {
    if !path.is_file() {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let parsed = String::from_utf8(bytes)
        .ok()
        .and_then(|text| Json::parse(&text).ok());
    if parsed.is_none() {
        if let Some(invalid) = invalid {
            invalid.push(path.to_path_buf());
        }
    }
    parsed
}

fn read_text(path: &Path) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Nome minúsculo para ordenar (`toLowerCase` do Dart).
fn sort_key(name: &str) -> String {
    name.to_lowercase()
}

/// Entradas de `dir` por nome (a ordem do disco não é estável).
fn sorted_dir(dir: &Path) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = read_dir.filter_map(Result::ok).map(|e| e.path()).collect();
    paths.sort();
    paths
}

fn too_large(size: u64) -> Vec<Issue> {
    if size > MAX_INSTRUCTION_CHARS {
        vec![Issue::TooLarge]
    } else {
        Vec::new()
    }
}

/// `~/.claude/CLAUDE.md` e, do workspace até a raiz, `CLAUDE.md`,
/// `.claude/CLAUDE.md` e `CLAUDE.local.md` (a subida que o Claude Code faz), mais o
/// `AGENTS.md` do workspace.
fn memory(workspace: &Path, home: &Path) -> Vec<Entry> {
    let user_claude = home.join(".claude");
    let user_md = user_claude.join("CLAUDE.md");
    let mut files: Vec<(PathBuf, bool)> = Vec::new();
    if !home.as_os_str().is_empty() && user_md.is_file() {
        files.push((user_md.clone(), true));
    }
    if !workspace.as_os_str().is_empty() {
        let mut dir = normalize(workspace);
        loop {
            for name in ["CLAUDE.md", ".claude/CLAUDE.md", "CLAUDE.local.md"] {
                let file = dir.join(name);
                if file != user_md && file.is_file() {
                    files.push((file, false));
                }
            }
            match dir.parent() {
                Some(parent) if parent != dir => dir = parent.to_path_buf(),
                _ => break,
            }
        }
    }
    let mut out: Vec<Entry> = files
        .into_iter()
        .map(|(path, is_user)| {
            let size = file_len(&path);
            let scope = if is_user {
                Scope::User
            } else if file_name(&path) == "CLAUDE.local.md" {
                Scope::Local
            } else {
                Scope::Project
            };
            Entry {
                kind: Kind::Memory,
                name: memory_name(&path, workspace, &user_claude),
                detail: path.to_string_lossy().into_owned(),
                scope,
                issues: too_large(size),
                // A instrução inteira entra em todo turno.
                always_on_chars: usize::try_from(size).unwrap_or(usize::MAX),
                path: Some(path),
            }
        })
        .collect();
    let agents = workspace.join("AGENTS.md");
    if agents.is_file() {
        out.push(Entry {
            kind: Kind::Memory,
            name: String::from("AGENTS.md"),
            detail: agents.to_string_lossy().into_owned(),
            scope: Scope::Project,
            issues: too_large(file_len(&agents)),
            // O Claude Code não lê o AGENTS.md (é do Codex e outros): não pesa no turno dele.
            always_on_chars: 0,
            path: Some(agents),
        });
    }
    out
}

/// `CLAUDE.md` do próprio workspace aparece só pelo nome; os de pastas acima,
/// relativos a ele (`../CLAUDE.md`).
fn memory_name(path: &Path, workspace: &Path, user_claude: &Path) -> String {
    if path != user_claude && path.starts_with(user_claude) {
        return format!("~/.claude/{}", file_name(path));
    }
    relative(path, workspace).to_string_lossy().into_owned()
}

struct Skill {
    name: String,
    description: Option<String>,
    path: PathBuf,
    user: bool,
}

/// `~/.claude/skills/*/SKILL.md` e `<workspace>/.claude/skills/*/SKILL.md` (os de
/// plugin ficam de fora), por nome.
fn skills(workspace: &Path, home: &Path) -> Vec<Entry> {
    let mut found = Vec::new();
    if !home.as_os_str().is_empty() {
        found.extend(skills_in(&home.join(".claude").join("skills"), true));
    }
    if !workspace.as_os_str().is_empty() {
        found.extend(skills_in(&workspace.join(".claude").join("skills"), false));
    }
    found.sort_by_key(|skill| sort_key(&skill.name));
    found
        .into_iter()
        .map(|skill| {
            let description = skill.description.unwrap_or_default();
            Entry {
                kind: Kind::Skills,
                always_on_chars: listing_chars(&skill.name, &description),
                issues: if description.trim().is_empty() {
                    vec![Issue::NoDescription]
                } else {
                    Vec::new()
                },
                name: skill.name,
                detail: description,
                scope: if skill.user {
                    Scope::User
                } else {
                    Scope::Project
                },
                path: Some(skill.path),
            }
        })
        .collect()
}

fn skills_in(dir: &Path, user: bool) -> Vec<Skill> {
    let mut out = Vec::new();
    for entry in sorted_dir(dir) {
        let file = entry.join("SKILL.md");
        if !file.is_file() {
            continue;
        }
        let Some(text) = read_text(&file) else {
            continue;
        };
        let fm = parse_frontmatter(&text);
        out.push(Skill {
            name: clean(fm.get("name")).unwrap_or_else(|| file_name(&entry)),
            description: clean(fm.get("description")),
            path: file,
            user,
        });
    }
    out
}

/// Valor aparado; vazio = ausente.
fn clean(value: Option<&String>) -> Option<String> {
    let trimmed = value?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// `*.md` de `dir` (comandos: também em subpastas, que viram `pasta:nome`). Nome e
/// descrição do frontmatter; sem ele, nome do arquivo.
fn markdowns(kind: Kind, dir: &Path, scope: Scope) -> Vec<Entry> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut files = Vec::new();
    collect_markdowns(dir, &PathBuf::new(), kind == Kind::Commands, 0, &mut files);
    let mut out: Vec<Entry> = files
        .into_iter()
        .filter_map(|(path, rel)| {
            let text = read_text(&path)?;
            let fm = parse_frontmatter(&text);
            let base = markdown_base(&rel);
            let name = if kind == Kind::Commands {
                format!("/{base}")
            } else {
                fm.get("name")
                    .map_or_else(|| base.clone(), |name| name.trim().to_owned())
            };
            let description = fm
                .get("description")
                .map(|d| d.trim().to_owned())
                .unwrap_or_default();
            Some(Entry {
                kind,
                issues: if kind == Kind::Agents && description.is_empty() {
                    vec![Issue::NoDescription]
                } else {
                    Vec::new()
                },
                always_on_chars: listing_chars(&name, &description),
                name,
                detail: description,
                scope,
                path: Some(path),
            })
        })
        .collect();
    out.sort_by_key(|entry| sort_key(&entry.name));
    out
}

/// Arquivos `.md` (caminho completo e relativo a `root`).
fn collect_markdowns(
    dir: &Path,
    rel: &Path,
    recursive: bool,
    depth: usize,
    out: &mut Vec<(PathBuf, PathBuf)>,
) {
    for path in sorted_dir(dir) {
        let Some(name) = path.file_name() else {
            continue;
        };
        let rel = rel.join(name);
        if path.is_dir() {
            if recursive && depth < MAX_COMMAND_DEPTH {
                collect_markdowns(&path, &rel, recursive, depth + 1, out);
            }
        } else if path.is_file() && name.to_string_lossy().ends_with(".md") {
            out.push((path, rel));
        }
    }
}

/// `git/pr.md` -> `git:pr` (um arquivo chamado só `.md` fica como está, sem extensão
/// no `withoutExtension` do Dart).
fn markdown_base(rel: &Path) -> String {
    let joined = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(":");
    let hidden_only = rel.file_name().is_some_and(|name| name == ".md");
    if hidden_only {
        return joined;
    }
    joined.strip_suffix(".md").unwrap_or(&joined).to_owned()
}

/// `mcpServers` do `~/.claude.json`: do usuário (topo) e os locais desta pasta
/// (`projects[<workspace>].mcpServers`). Só leitura, e JSON quebrado ali não é avisado.
fn claude_json_mcp(workspace: &Path, home: &Path) -> Vec<Entry> {
    let path = home.join(".claude.json");
    let Some(root) = read_json(&path, None) else {
        return Vec::new();
    };
    if root.as_obj().is_none() {
        return Vec::new();
    }
    let local = root
        .get("projects")
        .and_then(|projects| projects.get(&workspace.to_string_lossy()));
    let mut out = mcp_entries(Some(&root), &path, Scope::User, false);
    out.extend(mcp_entries(local, &path, Scope::Local, false));
    out
}

/// Hooks (`hooks.<Evento>[].hooks[].command`) e permissões
/// (`permissions.allow|ask|deny`) de um `settings.json`.
fn settings_entries(
    json: Option<&Json>,
    path: &Path,
    scope: Scope,
    workspace: &Path,
    home: &Path,
) -> Vec<Entry> {
    let mut out = Vec::new();
    let Some(json) = json.filter(|j| j.as_obj().is_some()) else {
        return out;
    };
    if let Some(events) = json.get("hooks").and_then(Json::as_obj) {
        for (event, groups) in events {
            let Some(groups) = groups.as_arr() else {
                continue;
            };
            for group in groups.iter().filter(|g| g.as_obj().is_some()) {
                let matcher = group
                    .get("matcher")
                    .and_then(Json::as_str)
                    .map_or("", str::trim);
                let hooks = group.get("hooks").and_then(Json::as_arr).unwrap_or(&[]);
                for hook in hooks.iter().filter(|h| h.as_obj().is_some()) {
                    let command = hook.get("command");
                    let script = command
                        .and_then(Json::as_str)
                        .and_then(|command| hook_script_path(command, workspace, home));
                    let detail = ["command", "url", "type"]
                        .iter()
                        .find_map(|key| hook.non_null(key))
                        .map(Json::dart_string)
                        .unwrap_or_default();
                    out.push(Entry {
                        kind: Kind::Hooks,
                        name: if matcher.is_empty() || matcher == "*" {
                            event.clone()
                        } else {
                            format!("{event} · {matcher}")
                        },
                        detail,
                        scope,
                        path: Some(path.to_path_buf()),
                        issues: match script {
                            Some(script) if !script.is_file() => vec![Issue::MissingScript],
                            _ => Vec::new(),
                        },
                        always_on_chars: 0,
                    });
                }
            }
        }
    }
    if let Some(perms) = json.get("permissions").filter(|p| p.as_obj().is_some()) {
        for rule_kind in ["allow", "ask", "deny"] {
            let rules = perms.get(rule_kind).and_then(Json::as_arr).unwrap_or(&[]);
            for rule in rules.iter().filter_map(Json::as_str) {
                out.push(Entry {
                    kind: Kind::Permissions,
                    name: rule.to_owned(),
                    detail: rule_kind.to_owned(),
                    scope,
                    path: Some(path.to_path_buf()),
                    issues: Vec::new(),
                    always_on_chars: 0,
                });
            }
        }
    }
    out
}

/// Servidores de `mcpServers` (stdio: comando; http/sse: url). `~/.claude.json` é o
/// estado do CLI (tokens, histórico): só leitura aqui (`editable` falso, sem caminho);
/// mexe-se nele com `claude mcp add/remove`.
fn mcp_entries(json: Option<&Json>, path: &Path, scope: Scope, editable: bool) -> Vec<Entry> {
    let Some(servers) = json
        .and_then(|j| j.get("mcpServers"))
        .and_then(Json::as_obj)
    else {
        return Vec::new();
    };
    servers
        .iter()
        .filter(|(_, server)| server.as_obj().is_some())
        .map(|(name, server)| {
            let detail = match server.get("url").and_then(Json::as_str) {
                Some(url) => {
                    let kind = server
                        .non_null("type")
                        .map_or_else(|| String::from("http"), Json::dart_string);
                    format!("{kind} · {url}")
                }
                None => server
                    .non_null("command")
                    .into_iter()
                    .chain(
                        server
                            .get("args")
                            .and_then(Json::as_arr)
                            .unwrap_or(&[])
                            .iter()
                            .filter(|arg| !arg.is_null()),
                    )
                    .map(Json::dart_string)
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            Entry {
                kind: Kind::Mcp,
                name: name.clone(),
                detail,
                scope,
                path: editable.then(|| path.to_path_buf()),
                issues: Vec::new(),
                always_on_chars: 0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        home: PathBuf,
        ws: PathBuf,
    }

    /// A mesma árvore do `setUp` de `claude_harness_source_test.dart`.
    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let ws = dir.path().join("repo");
        write(&home.join(".claude/CLAUDE.md"), "# eu");
        write(
            &home.join(".claude/agents/levi.md"),
            "---\nname: levi\ndescription: QA\n---\n",
        );
        let claude_json = format!(
            "{{\"mcpServers\": {{\"iris\": {{\"command\": \"npx\", \"args\": [\"iris\"]}}}}, \"projects\": {{{}: {{\"mcpServers\": {{\"db\": {{\"type\": \"http\", \"url\": \"http://x\"}}}}}}}}}}",
            serde_json::to_string(&ws.to_string_lossy()).unwrap()
        );
        write(&home.join(".claude.json"), &claude_json);
        write(&ws.join("CLAUDE.md"), "# repo");
        write(&ws.join("AGENTS.md"), "# agents");
        write(
            &ws.join(".claude/skills/deploy/SKILL.md"),
            "---\nname: deploy\ndescription: sobe\n---\n",
        );
        write(
            &ws.join(".claude/commands/git/pr.md"),
            "---\ndescription: abre PR\n---\n",
        );
        write(
            &ws.join(".claude/settings.json"),
            r#"{"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "gsh hook"}]}]},
               "permissions": {"allow": ["Bash(npm test *)"], "deny": ["Read(.env)"]}}"#,
        );
        write(&ws.join(".claude/settings.local.json"), "{inválido");
        write(
            &ws.join(".mcp.json"),
            r#"{"mcpServers": {"pg": {"command": "pg-mcp"}}}"#,
        );
        Fixture {
            _dir: dir,
            home,
            ws,
        }
    }

    fn of(h: &ProjectHarness, kind: Kind) -> Vec<(String, Scope)> {
        h.of(kind)
            .into_iter()
            .map(|e| (e.name.clone(), e.scope))
            .collect()
    }

    #[test]
    fn reads_each_section_with_scope_and_file() {
        let f = fixture();
        let h = read(&f.ws, &f.home);
        let memory = of(&h, Kind::Memory);
        for expected in [
            ("~/.claude/CLAUDE.md", Scope::User),
            ("CLAUDE.md", Scope::Project),
            ("AGENTS.md", Scope::Project),
        ] {
            assert!(
                memory.contains(&(expected.0.to_owned(), expected.1)),
                "{memory:?}"
            );
        }
        assert_eq!(
            of(&h, Kind::Skills),
            [("deploy".to_owned(), Scope::Project)]
        );
        assert_eq!(of(&h, Kind::Agents), [("levi".to_owned(), Scope::User)]);
        assert_eq!(
            of(&h, Kind::Commands),
            [("/git:pr".to_owned(), Scope::Project)]
        );
        let hooks = h.of(Kind::Hooks);
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].name, "PreToolUse · Bash");
        assert_eq!(hooks[0].detail, "gsh hook");
        assert_eq!(
            of(&h, Kind::Permissions),
            [
                ("Bash(npm test *)".to_owned(), Scope::Project),
                ("Read(.env)".to_owned(), Scope::Project)
            ]
        );
        let mcp = of(&h, Kind::Mcp);
        for expected in [
            ("pg", Scope::Project),
            ("iris", Scope::User),
            ("db", Scope::Local),
        ] {
            assert!(
                mcp.contains(&(expected.0.to_owned(), expected.1)),
                "{mcp:?}"
            );
        }
        // Lint: settings.local.json quebrado aparece; skill com descrição não tem aviso.
        assert_eq!(h.invalid_files.len(), 1);
        assert!(h.invalid_files[0].ends_with("settings.local.json"));
        assert!(h.of(Kind::Skills)[0].issues.is_empty());
        assert!(h.always_on_tokens(Some(Kind::Memory)) > 0);
        assert_eq!(h.issue_count(), 1);
        // ~/.claude.json é só leitura.
        let iris = h
            .of(Kind::Mcp)
            .into_iter()
            .find(|e| e.name == "iris")
            .unwrap();
        assert_eq!(iris.path, None);
        assert_eq!(iris.detail, "npx iris");
        let db = h
            .of(Kind::Mcp)
            .into_iter()
            .find(|e| e.name == "db")
            .unwrap();
        assert_eq!(db.detail, "http · http://x");
        let pg = h
            .of(Kind::Mcp)
            .into_iter()
            .find(|e| e.name == "pg")
            .unwrap();
        assert_eq!(pg.path, Some(f.ws.join(".mcp.json")));
    }

    #[test]
    fn agents_md_costs_nothing_and_listings_cost_name_description_plus_frame() {
        let f = fixture();
        let h = read(&f.ws, &f.home);
        let agents = h
            .of(Kind::Memory)
            .into_iter()
            .find(|e| e.name == "AGENTS.md")
            .unwrap();
        assert_eq!(agents.always_on_chars, 0);
        let repo = h
            .of(Kind::Memory)
            .into_iter()
            .find(|e| e.name == "CLAUDE.md")
            .unwrap();
        assert_eq!(repo.always_on_chars, "# repo".len());
        let skill = &h.of(Kind::Skills)[0];
        assert_eq!(skill.always_on_chars, "deploy".len() + "sobe".len() + 12);
        let command = &h.of(Kind::Commands)[0];
        assert_eq!(
            command.always_on_chars,
            "/git:pr".len() + "abre PR".len() + 12
        );
    }

    #[test]
    fn climbs_parents_and_names_them_relative_to_the_workspace() {
        let f = fixture();
        let sub = f.ws.join("pkg/app");
        write(&sub.join("CLAUDE.local.md"), "x");
        write(&f.ws.join(".claude/CLAUDE.md"), "y");
        let h = read(&sub, &f.home);
        let names = of(&h, Kind::Memory);
        assert!(
            names.contains(&("CLAUDE.local.md".to_owned(), Scope::Local)),
            "{names:?}"
        );
        assert!(
            names.contains(&("../../CLAUDE.md".to_owned(), Scope::Project)),
            "{names:?}"
        );
        assert!(
            names.contains(&("../../.claude/CLAUDE.md".to_owned(), Scope::Project)),
            "{names:?}"
        );
    }

    #[test]
    fn big_instruction_and_agent_without_description() {
        let f = fixture();
        write(
            &f.ws.join("CLAUDE.md"),
            &"x".repeat(MAX_INSTRUCTION_CHARS as usize + 1),
        );
        write(&f.ws.join(".claude/agents/mudo.md"), "sem frontmatter");
        let h = read(&f.ws, &f.home);
        let md = h
            .of(Kind::Memory)
            .into_iter()
            .find(|e| e.name == "CLAUDE.md")
            .unwrap();
        assert_eq!(md.issues, [Issue::TooLarge]);
        let mudo = h
            .of(Kind::Agents)
            .into_iter()
            .find(|e| e.name == "mudo")
            .unwrap();
        assert_eq!(mudo.issues, [Issue::NoDescription]);
        assert!(h.always_on_tokens(None) > MAX_INSTRUCTION_CHARS as usize / 4);
        // no limite exato não avisa
        write(
            &f.ws.join("CLAUDE.md"),
            &"x".repeat(MAX_INSTRUCTION_CHARS as usize),
        );
        let h = read(&f.ws, &f.home);
        let md = h
            .of(Kind::Memory)
            .into_iter()
            .find(|e| e.name == "CLAUDE.md")
            .unwrap();
        assert!(md.issues.is_empty());
    }

    #[test]
    fn hook_with_missing_script_is_flagged() {
        let json = Json::parse(
            r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "./nao-existe.sh"}]}],
                          "PreToolUse": [{"matcher": "*", "hooks": [{"type": "http", "url": "http://h"}]}]}}"#,
        )
        .unwrap();
        let entries = settings_entries(
            Some(&json),
            Path::new("/w/.claude/settings.json"),
            Scope::Project,
            Path::new("/w"),
            Path::new("/h"),
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "Stop");
        assert_eq!(entries[0].issues, [Issue::MissingScript]);
        // matcher `*` some do nome; sem command, o detalhe é a url
        assert_eq!(entries[1].name, "PreToolUse");
        assert_eq!(entries[1].detail, "http://h");
        assert!(entries[1].issues.is_empty());
    }

    #[test]
    fn items_follow_file_order_not_alphabetical() {
        let f = fixture();
        write(
            &f.ws.join(".mcp.json"),
            r#"{"mcpServers": {"zz": {"command": "a"}, "aa": {"command": "b"}}}"#,
        );
        let h = read(&f.ws, &f.home);
        let names: Vec<String> = h
            .of(Kind::Mcp)
            .into_iter()
            .filter(|e| e.scope == Scope::Project)
            .map(|e| e.name.clone())
            .collect();
        assert_eq!(names, ["zz", "aa"]);
    }

    #[test]
    fn commands_are_recursive_and_sorted_agents_use_frontmatter_name() {
        let f = fixture();
        write(&f.ws.join(".claude/commands/b.md"), "x");
        write(&f.ws.join(".claude/commands/A.md"), "x");
        write(&f.ws.join(".claude/commands/git/deep/x.md"), "x");
        write(&f.ws.join(".claude/commands/readme.txt"), "x");
        write(
            &f.ws.join(".claude/agents/file.md"),
            "---\nname: ' Nome '\n---\n",
        );
        let h = read(&f.ws, &f.home);
        let commands: Vec<String> = h
            .of(Kind::Commands)
            .into_iter()
            .map(|e| e.name.clone())
            .collect();
        assert_eq!(commands, ["/A", "/b", "/git:deep:x", "/git:pr"]);
        let agents: Vec<String> = h
            .of(Kind::Agents)
            .into_iter()
            .map(|e| e.name.clone())
            .collect();
        assert_eq!(agents, ["Nome", "levi"]);
    }

    #[test]
    fn empty_home_reads_nothing_of_the_user() {
        let f = fixture();
        let h = read(&f.ws, Path::new(""));
        assert!(h.entries.iter().all(|e| e.scope != Scope::User));
    }

    #[test]
    fn missing_workspace_files_give_an_empty_harness() {
        let dir = tempfile::tempdir().unwrap();
        let h = read(&dir.path().join("nada"), &dir.path().join("sem-home"));
        assert!(h.entries.is_empty());
        assert!(h.invalid_files.is_empty());
        assert_eq!(h.issue_count(), 0);
    }
}
