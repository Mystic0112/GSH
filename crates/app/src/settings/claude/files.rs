//! Leituras e gravações do `~/.claude`, do `.claude` do projeto e do `~/.gsh` que não passam
//! pela CLI (`ClaudeFiles`, `ProtectedHostsFile` e parsers de manifesto/memória do Flutter).

use super::cli::{InfoError, MarketplacePlugin, PluginInfo};
use super::file::write_atomic;
use gsh_harness::Json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    User,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryFileInfo {
    pub path: PathBuf,
    pub scope: MemoryScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoMemoryEntry {
    pub title: String,
    pub path: PathBuf,
    pub hook: Option<String>,
    pub kind: Option<String>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillSource {
    User,
    Plugin,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInfo {
    pub name: String,
    pub description: Option<String>,
    pub path: PathBuf,
    pub source: SkillSource,
    pub plugin: Option<String>,
    pub updated_at: Option<i64>,
}

/// Estilos de saída: os embutidos e os de `~/.claude/output-styles/*.md` (nome do
/// frontmatter ou do arquivo).
pub fn output_styles(home: &Path) -> Vec<String> {
    let mut out = vec![String::from("Explanatory"), String::from("Learning")];
    if home.as_os_str().is_empty() {
        return out;
    }
    let Ok(entries) = std::fs::read_dir(home.join(".claude").join("output-styles")) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let named = gsh_harness::parse_frontmatter(&text)
            .get("name")
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty());
        out.push(named.unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        }));
    }
    out
}

pub fn instructions_path(home: &Path) -> PathBuf {
    home.join(".claude").join("CLAUDE.md")
}

/// Conteúdo do `~/.claude/CLAUDE.md` ("" se não existe ou não dá para ler).
pub fn read_instructions(home: &Path) -> String {
    if home.as_os_str().is_empty() {
        return String::new();
    }
    std::fs::read_to_string(instructions_path(home)).unwrap_or_default()
}

/// Regrava o `~/.claude/CLAUDE.md` (cria se faltar).
pub fn write_instructions(home: &Path, text: &str) -> Result<(), InfoError> {
    if home.as_os_str().is_empty() {
        return Err(InfoError::Failed(None));
    }
    write_atomic(&instructions_path(home), text).map_err(|e| InfoError::Failed(Some(e.to_string())))
}

fn hosts_path(home: &Path) -> PathBuf {
    home.join(".gsh").join("protected-hosts.json")
}

/// Hosts protegidos (`~/.gsh/protected-hosts.json`, array de strings).
pub fn read_hosts(home: &Path) -> Vec<String> {
    if home.as_os_str().is_empty() {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(hosts_path(home)) else {
        return Vec::new();
    };
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str(&text) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Grava a lista em minúsculas, sem repetidos e sem vazios; lista vazia apaga o arquivo.
pub fn write_hosts(home: &Path, hosts: &[String]) -> std::io::Result<()> {
    let mut clean: Vec<String> = Vec::new();
    for host in hosts {
        let host = host.trim().to_lowercase();
        if !host.is_empty() && !clean.contains(&host) {
            clean.push(host);
        }
    }
    let path = hosts_path(home);
    if clean.is_empty() {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    let json: Vec<String> = clean
        .iter()
        .map(|h| serde_json::to_string(h).unwrap_or_default())
        .collect();
    write_atomic(&path, &format!("[{}]\n", json.join(",")))
}

/// `~/.claude/CLAUDE.md` e, do `workspace` até a raiz, `CLAUDE.md`, `.claude/CLAUDE.md`
/// e `CLAUDE.local.md`.
pub fn memory_files(home: &Path, workspace: Option<&Path>) -> Vec<MemoryFileInfo> {
    let mut out = Vec::new();
    let user = instructions_path(home);
    if !home.as_os_str().is_empty() && user.is_file() {
        out.push(MemoryFileInfo {
            path: user.clone(),
            scope: MemoryScope::User,
        });
    }
    let Some(ws) = workspace.filter(|w| !w.as_os_str().is_empty()) else {
        return out;
    };
    let mut dir = ws.to_path_buf();
    loop {
        for rel in ["CLAUDE.md", ".claude/CLAUDE.md", "CLAUDE.local.md"] {
            let candidate = dir.join(rel);
            if candidate != user && candidate.is_file() {
                out.push(MemoryFileInfo {
                    path: candidate,
                    scope: MemoryScope::Project,
                });
            }
        }
        let Some(parent) = dir.parent().filter(|p| *p != dir) else {
            break;
        };
        dir = parent.to_path_buf();
    }
    out
}

/// Pasta do projeto em `~/.claude/projects`: o cwd com tudo que não é alfanumérico trocado por `-`.
pub fn project_dir(home: &Path, workspace: &Path) -> PathBuf {
    let slug: String = workspace
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    home.join(".claude").join("projects").join(slug)
}

/// Memória automática do Claude Code para `workspace` (`MEMORY.md` e os arquivos citados).
pub fn auto_memory(home: &Path, workspace: Option<&Path>) -> Vec<AutoMemoryEntry> {
    if home.as_os_str().is_empty() {
        return Vec::new();
    }
    let Some(ws) = workspace.filter(|w| !w.as_os_str().is_empty()) else {
        return Vec::new();
    };
    let mem_dir = project_dir(home, ws).join("memory");
    let Ok(index_text) = std::fs::read_to_string(mem_dir.join("MEMORY.md")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (title, file_rel, hook) in parse_memory_index(&index_text) {
        let path = mem_dir.join(&file_rel);
        let exists = path.is_file();
        let kind = if exists {
            std::fs::read_to_string(&path).ok().and_then(|content| {
                gsh_harness::parse_frontmatter(&content)
                    .get("type")
                    .map(|t| t.trim().to_owned())
                    .filter(|t| !t.is_empty())
            })
        } else {
            None
        };
        let updated_at = if exists { mtime_secs(&path) } else { None };
        out.push(AutoMemoryEntry {
            title,
            path,
            hook,
            kind,
            updated_at,
        });
    }
    out
}

/// Índice `MEMORY.md`: `- [Título](arquivo.md) — gancho`.
pub fn parse_memory_index(text: &str) -> Vec<(String, String, Option<String>)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed
            .strip_prefix('-')
            .or_else(|| trimmed.strip_prefix('*'))
            .map(str::trim_start)
        else {
            continue;
        };
        let Some(rest) = rest.strip_prefix('[') else {
            continue;
        };
        let Some(close_bracket) = rest.find("](") else {
            continue;
        };
        let title = rest.get(..close_bracket).unwrap_or("").trim();
        let after_bracket = rest.get(close_bracket + 2..).unwrap_or("");
        let Some(close_paren) = after_bracket.find(')') else {
            continue;
        };
        let file = after_bracket.get(..close_paren).unwrap_or("").trim();
        if title.is_empty() || file.is_empty() {
            continue;
        }
        let tail = after_bracket.get(close_paren + 1..).unwrap_or("").trim();
        let hook = tail
            .strip_prefix('—')
            .or_else(|| tail.strip_prefix('–'))
            .or_else(|| tail.strip_prefix('-'))
            .map(str::trim)
            .filter(|h| !h.is_empty())
            .map(str::to_owned);
        out.push((title.to_owned(), file.to_owned(), hook));
    }
    out
}

/// Skills do usuário (`~/.claude/skills`), dos plugins habilitados e do projeto (`<workspace>/.claude/skills`).
pub fn skills(home: &Path, plugins: &[PluginInfo], workspace: Option<&Path>) -> Vec<SkillInfo> {
    let mut out = Vec::new();
    if !home.as_os_str().is_empty() {
        out.extend(skills_in(
            &home.join(".claude").join("skills"),
            SkillSource::User,
            None,
        ));
    }
    for pl in plugins {
        if pl.enabled {
            if let Some(install_path) = &pl.install_path {
                out.extend(skills_in(
                    &Path::new(install_path).join("skills"),
                    SkillSource::Plugin,
                    Some(pl.id.clone()),
                ));
            }
        }
    }
    if let Some(ws) = workspace.filter(|w| !w.as_os_str().is_empty()) {
        out.extend(skills_in(
            &ws.join(".claude").join("skills"),
            SkillSource::Project,
            None,
        ));
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

fn skills_in(dir: &Path, source: SkillSource, plugin: Option<String>) -> Vec<SkillInfo> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let skill_dir = entry.path();
        let file = skill_dir.join("SKILL.md");
        if !file.is_file() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&file) else {
            continue;
        };
        let fm = gsh_harness::parse_frontmatter(&content);
        let name = fm
            .get("name")
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                skill_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
        let description = fm
            .get("description")
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        out.push(SkillInfo {
            name,
            description,
            updated_at: mtime_secs(&file),
            path: file,
            source,
            plugin: plugin.clone(),
        });
    }
    out
}

/// `enabledPlugins` (`{"id": true}`) de `<workspace>/.claude/settings.json`.
pub fn project_plugins(workspace: Option<&Path>) -> Vec<String> {
    let Some(ws) = workspace.filter(|w| !w.as_os_str().is_empty()) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(ws.join(".claude").join("settings.json")) else {
        return Vec::new();
    };
    let Ok(root) = Json::parse(&text) else {
        return Vec::new();
    };
    let Some(enabled) = root.get("enabledPlugins").and_then(Json::as_obj) else {
        return Vec::new();
    };
    enabled
        .iter()
        .filter(|(_, v)| *v == Json::Bool(true))
        .map(|(k, _)| k.clone())
        .collect()
}

/// Enriquece plugins instalados com `.claude-plugin/plugin.json`.
pub fn with_manifests(plugins: Vec<PluginInfo>) -> Vec<PluginInfo> {
    plugins
        .into_iter()
        .map(|mut pl| {
            if let Some(dir) = &pl.install_path {
                let file = Path::new(dir).join(".claude-plugin").join("plugin.json");
                if let Ok(text) = std::fs::read_to_string(file) {
                    let (name, desc, author) = parse_plugin_manifest(&text);
                    if name.is_some() {
                        pl.display_name = name;
                    }
                    if desc.is_some() {
                        pl.description = desc;
                    }
                    if author.is_some() {
                        pl.author = author;
                    }
                }
            }
            pl
        })
        .collect()
}

pub fn parse_plugin_manifest(json: &str) -> (Option<String>, Option<String>, Option<String>) {
    let Ok(root) = Json::parse(json) else {
        return (None, None, None);
    };
    if root.as_obj().is_none() {
        return (None, None, None);
    }
    let name = root.get("name").and_then(Json::as_str).map(str::to_owned);
    let desc = root
        .get("description")
        .and_then(Json::as_str)
        .map(str::to_owned);
    let author = author_name(root.get("author"));
    (name, desc, author)
}

pub fn directory(
    locations: &BTreeMap<String, String>,
    installed: &BTreeSet<String>,
) -> Vec<MarketplacePlugin> {
    let mut out = Vec::new();
    for (name, dir) in locations {
        let file = Path::new(dir)
            .join(".claude-plugin")
            .join("marketplace.json");
        if let Ok(text) = std::fs::read_to_string(file) {
            out.extend(parse_marketplace_manifest(&text, name, installed));
        }
    }
    out
}

pub fn parse_marketplace_manifest(
    json: &str,
    marketplace: &str,
    installed: &BTreeSet<String>,
) -> Vec<MarketplacePlugin> {
    let Ok(root) = Json::parse(json) else {
        return Vec::new();
    };
    let Some(items) = root.get("plugins").and_then(Json::as_arr) else {
        return Vec::new();
    };
    let owner = author_name(root.get("owner"));
    let mut out = Vec::new();
    for item in items {
        let Some(name) = item.get("name").and_then(Json::as_str) else {
            continue;
        };
        let id = format!("{name}@{marketplace}");
        out.push(MarketplacePlugin {
            name: name.to_owned(),
            marketplace: marketplace.to_owned(),
            description: item
                .get("description")
                .and_then(Json::as_str)
                .map(str::to_owned),
            author: author_name(item.get("author")).or_else(|| owner.clone()),
            category: item
                .get("category")
                .and_then(Json::as_str)
                .map(str::to_owned),
            installed: installed.contains(&id),
        });
    }
    out
}

fn author_name(node: Option<&Json>) -> Option<String> {
    let node = node?;
    if let Some(s) = node.as_str() {
        return Some(s.to_owned());
    }
    node.get("name").and_then(Json::as_str).map(str::to_owned)
}

fn mtime_secs(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("gsh-claude-files-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn output_styles_lists_builtins_then_custom_files() {
        let h = home("styles");
        assert_eq!(output_styles(&h), ["Explanatory", "Learning"]);
        let dir = h.join(".claude/output-styles");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plain.md"), "sem frontmatter").unwrap();
        std::fs::write(dir.join("x.md"), "---\nname: Nomeado\n---\ncorpo").unwrap();
        std::fs::write(dir.join("ignorado.txt"), "x").unwrap();
        let mut styles = output_styles(&h);
        styles.sort();
        assert_eq!(styles, ["Explanatory", "Learning", "Nomeado", "plain"]);
        std::fs::remove_dir_all(h).unwrap();
    }

    #[test]
    fn instructions_round_trip_creating_the_folder() {
        let h = home("instr");
        assert_eq!(read_instructions(&h), "");
        write_instructions(&h, "# eu\nbreve").unwrap();
        assert_eq!(read_instructions(&h), "# eu\nbreve");
        assert!(write_instructions(Path::new(""), "x").is_err());
        std::fs::remove_dir_all(h).unwrap();
    }

    #[test]
    fn hosts_are_normalized_and_empty_list_deletes_the_file() {
        let h = home("hosts");
        assert!(read_hosts(&h).is_empty());
        write_hosts(
            &h,
            &[
                "Prod".into(),
                " ".into(),
                "prod".into(),
                "Homolog.X.com".into(),
            ],
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(h.join(".gsh/protected-hosts.json")).unwrap(),
            "[\"prod\",\"homolog.x.com\"]\n"
        );
        assert_eq!(read_hosts(&h), ["prod", "homolog.x.com"]);
        write_hosts(&h, &[]).unwrap();
        assert!(!h.join(".gsh/protected-hosts.json").exists());
        write_hosts(&h, &[]).unwrap();
        std::fs::remove_dir_all(h).unwrap();
    }

    #[test]
    fn memory_skills_plugins_and_manifests_are_parsed_from_temp_dir() {
        let h = home("mem_skills");
        let ws = h.join("ws");
        std::fs::create_dir_all(ws.join(".claude/skills/my-skill")).unwrap();
        std::fs::write(h.join(".claude/CLAUDE.md"), "user").ok();
        write_instructions(&h, "user").unwrap();
        std::fs::write(ws.join("CLAUDE.md"), "proj").unwrap();
        std::fs::write(
            ws.join(".claude/skills/my-skill/SKILL.md"),
            "---\nname: Skill X\ndescription: Faz X\n---\n",
        )
        .unwrap();
        std::fs::write(
            ws.join(".claude/settings.json"),
            r#"{"enabledPlugins":{"p1@mkt":true,"p2@mkt":false}}"#,
        )
        .unwrap();

        let mem = memory_files(&h, Some(&ws));
        assert_eq!(mem.len(), 2);
        assert_eq!(mem[0].scope, MemoryScope::User);
        assert_eq!(mem[1].scope, MemoryScope::Project);

        let pdir = project_dir(&h, &ws).join("memory");
        std::fs::create_dir_all(&pdir).unwrap();
        std::fs::write(pdir.join("MEMORY.md"), "- [Nota 1](n1.md) — gancho 1\n").unwrap();
        std::fs::write(pdir.join("n1.md"), "---\ntype: project\n---\ncorpo").unwrap();
        let auto = auto_memory(&h, Some(&ws));
        assert_eq!(auto.len(), 1);
        assert_eq!(auto[0].title, "Nota 1");
        assert_eq!(auto[0].hook.as_deref(), Some("gancho 1"));
        assert_eq!(auto[0].kind.as_deref(), Some("project"));

        let sk = skills(&h, &[], Some(&ws));
        assert_eq!(sk.len(), 1);
        assert_eq!(sk[0].name, "Skill X");
        assert_eq!(sk[0].description.as_deref(), Some("Faz X"));

        assert_eq!(project_plugins(Some(&ws)), vec!["p1@mkt".to_owned()]);

        let mkt = parse_marketplace_manifest(
            r#"{"owner":{"name":"Gorila"},"plugins":[{"name":"alpha","description":"Desc"}]}"#,
            "gorila",
            &BTreeSet::from(["alpha@gorila".to_owned()]),
        );
        assert_eq!(mkt.len(), 1);
        assert!(mkt[0].installed);
        assert_eq!(mkt[0].author.as_deref(), Some("Gorila"));

        std::fs::remove_dir_all(h).unwrap();
    }
}
