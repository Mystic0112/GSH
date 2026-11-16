//! Temas do Design Systems (`theme_registry.dart`, `theme_store.dart`), localização de
//! armazenamento (`storage_location.dart`), `KeepAwake` (`keep_awake.dart`) e autostart.

use super::file::write_atomic;
use gsh_harness::Json;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEntry {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub imported: bool,
}

/// Os 10 temas nativos do GSH (`builtInThemes` de `theme_registry.dart`), na ordem do seletor.
pub const BUILT_IN_THEMES: [(&str, &str, &str); 10] = [
    ("gsh", "GSH", "Gorila Software House"),
    ("classic", "Clássico", "Remote Pi"),
    ("classic.2", "Clássico 2", "Remote Pi"),
    ("violet", "Violet 1", "Remote Pi"),
    ("violet.2", "Violet 2", "Remote Pi"),
    ("midnight", "Midnight", "Remote Pi"),
    ("rose", "Rosê", "Remote Pi"),
    ("sun", "Sun", "Remote Pi"),
    ("flexoki", "Flexoki", "Steph Ango"),
    ("pantera", "Pantera", "Remote Pi"),
];

pub fn is_built_in_theme(id: &str) -> bool {
    BUILT_IN_THEMES.iter().any(|(bid, _, _)| *bid == id)
}

fn file_safe_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn themes_dir(home: &Path) -> PathBuf {
    data_dir(home).join("themes")
}

/// Lista todos os temas disponíveis (embutidos + importados de `<data_dir>/themes/*.json`).
pub fn load_themes(home: &Path) -> Vec<ThemeEntry> {
    let mut out: Vec<ThemeEntry> = BUILT_IN_THEMES
        .iter()
        .map(|(id, name, author)| ThemeEntry {
            id: (*id).to_owned(),
            name: (*name).to_owned(),
            author: Some((*author).to_owned()),
            imported: false,
        })
        .collect();
    if home.as_os_str().is_empty() {
        return out;
    }
    let mut custom = Vec::new();
    if let Ok(entries) = std::fs::read_dir(themes_dir(home)) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|e| e == "json") {
                if let Ok(spec) = parse_theme_file(&path) {
                    if !is_built_in_theme(&spec.id) {
                        custom.push(spec);
                    }
                }
            }
        }
    }
    custom.sort_by_key(|a| a.name.to_lowercase());
    out.extend(custom);
    out
}

pub fn parse_theme_file(path: &Path) -> Result<ThemeEntry, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("Não foi possível ler ou gravar o arquivo do tema: {e}"))?;
    let root = Json::parse(&raw).map_err(|e| format!("Este arquivo não é um JSON válido: {e}"))?;
    if root.as_obj().is_none() {
        return Err(String::from("Esperava um objeto em \"$\"."));
    }
    let id = root
        .get("id")
        .and_then(Json::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| String::from("Falta o campo obrigatório \"id\"."))?;
    let name = root
        .get("name")
        .and_then(Json::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| String::from("Falta o campo obrigatório \"name\"."))?;
    if let Some(ext) = root
        .get("extends")
        .and_then(Json::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if !is_built_in_theme(ext) {
            return Err(format!("Tema base \"{ext}\" desconhecido em \"extends\"."));
        }
    }
    let Some(variants) = root.get("variants").filter(|v| v.as_obj().is_some()) else {
        return Err(String::from(
            "O tema não declara nenhum variant. Adicione \"dark\", \"light\" ou os dois em \"variants\".",
        ));
    };
    let has_dark = variants.get("dark").is_some_and(|v| v.as_obj().is_some());
    let has_light = variants.get("light").is_some_and(|v| v.as_obj().is_some());
    if !has_dark && !has_light {
        return Err(String::from(
            "O tema não declara nenhum variant. Adicione \"dark\", \"light\" ou os dois em \"variants\".",
        ));
    }
    let author = root.get("author").and_then(Json::as_str).map(str::to_owned);
    Ok(ThemeEntry {
        id: id.to_owned(),
        name: name.to_owned(),
        author,
        imported: true,
    })
}

pub fn import_theme(home: &Path, source: &Path) -> Result<ThemeEntry, String> {
    let spec = parse_theme_file(source)?;
    if is_built_in_theme(&spec.id) {
        return Err(String::from(
            "Este tema usa o id de um tema nativo. Mude o \"id\" no arquivo e importe de novo.",
        ));
    }
    let raw = std::fs::read_to_string(source)
        .map_err(|e| format!("Não foi possível ler ou gravar o arquivo do tema: {e}"))?;
    let dest = themes_dir(home).join(format!("{}.json", file_safe_id(&spec.id)));
    write_atomic(&dest, &raw)
        .map_err(|e| format!("Não foi possível ler ou gravar o arquivo do tema: {e}"))?;
    Ok(spec)
}

pub fn delete_theme(home: &Path, id: &str) -> Result<(), String> {
    let path = themes_dir(home).join(format!("{}.json", file_safe_id(id)));
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!(
            "Não foi possível ler ou gravar o arquivo do tema: {e}"
        )),
    }
}

// ---------------------------------------------------------------------------
// StorageLocation (`storage_location.dart`)
// ---------------------------------------------------------------------------

fn pointer_path(home: &Path) -> PathBuf {
    home.join(".gsh").join("storage_root")
}

pub fn override_root(home: &Path) -> Option<PathBuf> {
    if home.as_os_str().is_empty() {
        return None;
    }
    let text = std::fs::read_to_string(pointer_path(home)).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

pub fn is_custom_storage(home: &Path) -> bool {
    override_root(home).is_some()
}

pub fn effective_root(home: &Path) -> PathBuf {
    if let Some(custom) = override_root(home) {
        return custom;
    }
    if home.as_os_str().is_empty() {
        return PathBuf::new();
    }
    let docs = home.join("Documents");
    if docs.is_dir() {
        docs
    } else {
        home.to_path_buf()
    }
}

pub fn data_dir(home: &Path) -> PathBuf {
    effective_root(home).join("gsh")
}

pub fn set_override_root(home: &Path, root: Option<&Path>) -> std::io::Result<()> {
    let ptr = pointer_path(home);
    match root.filter(|p| !p.as_os_str().is_empty()) {
        Some(path) => write_atomic(&ptr, &path.to_string_lossy()),
        None => match std::fs::remove_file(&ptr) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
    }
}

pub fn migrate_state_to(home: &Path, target_root: &Path) -> std::io::Result<bool> {
    let src = data_dir(home);
    let dst = target_root.join("gsh");
    if !src.is_dir() {
        return Ok(false);
    }
    if dst.is_dir()
        && std::fs::read_dir(&dst)
            .ok()
            .is_some_and(|mut it| it.next().is_some())
    {
        return Ok(false);
    }
    copy_tree(&src, &dst)?;
    Ok(true)
}

fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else if path.is_file() && path.extension().is_none_or(|e| e != "lock") {
            let _ = std::fs::copy(&path, &target);
        }
    }
    Ok(())
}

pub fn reset_all(home: &Path) -> std::io::Result<()> {
    if !home.as_os_str().is_empty() {
        let dir = data_dir(home);
        if dir.is_dir() {
            let _ = std::fs::remove_dir_all(dir);
        }
        set_override_root(home, None)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// KeepAwake (`keep_awake.dart`) & LaunchAtStartup (`launch_at_startup_service.dart`)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct KeepAwake {
    child: Option<Child>,
}

impl KeepAwake {
    pub fn is_active(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
                self.child = None;
            }
        }
        self.child.is_some()
    }

    pub fn set(&mut self, enable: bool) {
        if !enable {
            self.release();
            return;
        }
        if self.is_active() {
            return;
        }
        let pid = std::process::id();
        let spawned = if cfg!(target_os = "macos") {
            Command::new("caffeinate")
                .args(["-i", "-w", &pid.to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok()
        } else if cfg!(target_os = "linux") {
            Command::new("systemd-inhibit")
                .args([
                    "--what=sleep:idle",
                    "--who=GSH",
                    "--why=Remote access (keep awake)",
                    "--mode=block",
                    "sh",
                    "-c",
                    &format!("while kill -0 {pid} 2>/dev/null; do sleep 5; done"),
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok()
        } else {
            None
        };
        self.child = spawned;
    }

    pub fn release(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        self.release();
    }
}

pub fn apply_launch_at_startup(home: &Path, enabled: bool) {
    if home.as_os_str().is_empty() {
        return;
    }
    let desktop = home
        .join(".config")
        .join("autostart")
        .join("gsh-desktop.desktop");
    if !enabled {
        let _ = std::fs::remove_file(desktop);
        return;
    }
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("gsh-app"));
    let body = format!(
        "[Desktop Entry]\nType=Application\nName=GSH Desktop\nExec=\"{}\" --minimized\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        exe.display()
    );
    let _ = write_atomic(&desktop, &body);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_import_validates_and_deletes_and_storage_round_trips() {
        let dir = std::env::temp_dir().join(format!("gsh-claude-themes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        assert_eq!(load_themes(&dir).len(), 10);
        let src = dir.join("custom.json");
        std::fs::write(
            &src,
            r#"{"id":"acme.neon","name":"Neon","author":"Acme","variants":{"dark":{"ui":{}}}}"#,
        )
        .unwrap();
        let entry = import_theme(&dir, &src).unwrap();
        assert_eq!(entry.name, "Neon");
        assert_eq!(load_themes(&dir).len(), 11);

        let reserved = dir.join("reserved.json");
        std::fs::write(
            &reserved,
            r#"{"id":"gsh","name":"Dup","variants":{"dark":{}}}"#,
        )
        .unwrap();
        assert!(import_theme(&dir, &reserved).is_err());

        delete_theme(&dir, "acme.neon").unwrap();
        assert_eq!(load_themes(&dir).len(), 10);

        assert!(!is_custom_storage(&dir));
        let custom_root = dir.join("sync");
        set_override_root(&dir, Some(&custom_root)).unwrap();
        assert!(is_custom_storage(&dir));
        assert_eq!(effective_root(&dir), custom_root);
        reset_all(&dir).unwrap();
        assert!(!is_custom_storage(&dir));

        std::fs::remove_dir_all(dir).unwrap();
    }
}
