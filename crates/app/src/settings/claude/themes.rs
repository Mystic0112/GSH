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

pub fn selected_theme_json(home: &Path, id: &str) -> Option<String> {
    if home.as_os_str().is_empty() || is_built_in_theme(id) || id.is_empty() {
        return None;
    }
    let path = themes_dir(home).join(format!("{}.json", file_safe_id(id)));
    let raw = std::fs::read_to_string(&path).ok()?;
    let spec = parse_theme_file(&path).ok()?;
    (spec.id == id).then_some(raw)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerSource {
    Ac,
    Battery,
    #[default]
    Unknown,
}

pub fn probe_linux_power_source(root: &Path) -> PowerSource {
    let Ok(entries) = std::fs::read_dir(root) else {
        return PowerSource::Unknown;
    };
    let mut saw_battery = false;
    for entry in entries.flatten() {
        let dir = entry.path();
        let kind = std::fs::read_to_string(dir.join("type"))
            .ok()
            .map(|s| s.trim().to_owned());
        match kind.as_deref() {
            Some("Mains") => {
                let online = std::fs::read_to_string(dir.join("online")).ok();
                if online.as_deref().map(str::trim) == Some("1") {
                    return PowerSource::Ac;
                }
            }
            Some("Battery") => {
                saw_battery = true;
            }
            _ => {}
        }
    }
    if saw_battery {
        PowerSource::Battery
    } else {
        PowerSource::Ac
    }
}

pub fn parse_pmset_batt(out: &str) -> PowerSource {
    let head = out.lines().next().unwrap_or_default();
    if head.contains("'AC Power'") {
        PowerSource::Ac
    } else if head.contains("'Battery Power'") {
        PowerSource::Battery
    } else {
        PowerSource::Unknown
    }
}

pub fn probe_power_source() -> PowerSource {
    if cfg!(target_os = "linux") {
        probe_linux_power_source(Path::new("/sys/class/power_supply"))
    } else if cfg!(target_os = "macos") {
        Command::new("pmset")
            .args(["-g", "batt"])
            .output()
            .ok()
            .filter(|out| out.status.success())
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map_or(PowerSource::Unknown, |out| parse_pmset_batt(&out))
    } else {
        PowerSource::Unknown
    }
}

pub const LID_MIN_BATTERY_PCT: u8 = 20;
pub const LID_MAX_TEMP_C: i32 = 85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LidSessionDuration {
    Minutes30,
    #[default]
    Hour1,
    Hours2,
    Hours4,
}

impl LidSessionDuration {
    pub const ALL: [Self; 4] = [Self::Minutes30, Self::Hour1, Self::Hours2, Self::Hours4];

    pub const fn seconds(self) -> u64 {
        match self {
            Self::Minutes30 => 30 * 60,
            Self::Hour1 => 60 * 60,
            Self::Hours2 => 2 * 60 * 60,
            Self::Hours4 => 4 * 60 * 60,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Minutes30 => "30 min",
            Self::Hour1 => "1 h",
            Self::Hours2 => "2 h",
            Self::Hours4 => "4 h",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LidSafetyStop {
    LowBattery(u8),
    HighTemp(i32),
}

pub fn probe_linux_battery_capacity(root: &Path) -> Option<u8> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut min_capacity: Option<u8> = None;
    for entry in entries.flatten() {
        let dir = entry.path();
        let kind = std::fs::read_to_string(dir.join("type")).ok();
        if kind.as_deref().map(str::trim) != Some("Battery") {
            continue;
        }
        if let Some(pct) = std::fs::read_to_string(dir.join("capacity"))
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok())
        {
            min_capacity = Some(min_capacity.map_or(pct, |cur| cur.min(pct)));
        }
    }
    min_capacity
}

pub fn probe_linux_max_temp_c(root: &Path) -> Option<i32> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut max_temp: Option<i32> = None;
    for entry in entries.flatten() {
        let dir = entry.path();
        let Some(raw) = std::fs::read_to_string(dir.join("temp"))
            .ok()
            .and_then(|s| s.trim().parse::<i64>().ok())
        else {
            continue;
        };
        let temp_c = if raw >= 1000 {
            (raw / 1000) as i32
        } else {
            raw as i32
        };
        max_temp = Some(max_temp.map_or(temp_c, |cur| cur.max(temp_c)));
    }
    max_temp
}

pub fn check_lid_safety_at(power_root: &Path, thermal_root: &Path) -> Result<(), LidSafetyStop> {
    if let Some(temp_c) = probe_linux_max_temp_c(thermal_root) {
        if temp_c >= LID_MAX_TEMP_C {
            return Err(LidSafetyStop::HighTemp(temp_c));
        }
    }
    if probe_linux_power_source(power_root) == PowerSource::Battery {
        if let Some(pct) = probe_linux_battery_capacity(power_root) {
            if pct <= LID_MIN_BATTERY_PCT {
                return Err(LidSafetyStop::LowBattery(pct));
            }
        }
    }
    Ok(())
}

#[derive(Default)]
pub struct KeepAwake {
    child: Option<Child>,
    lid_child: Option<Child>,
    lid_deadline: Option<std::time::Instant>,
    lid_duration: LidSessionDuration,
    lid_stop_reason: Option<LidSafetyStop>,
    power_source: PowerSource,
}

impl KeepAwake {
    pub fn command(pid: u32) -> Option<(&'static str, Vec<String>)> {
        if cfg!(target_os = "macos") {
            Some((
                "caffeinate",
                vec!["-i".into(), "-w".into(), pid.to_string()],
            ))
        } else if cfg!(target_os = "linux") {
            Some((
                "systemd-inhibit",
                vec![
                    "--what=sleep:idle".into(),
                    "--who=GSH".into(),
                    "--why=Remote access (keep awake)".into(),
                    "--mode=block".into(),
                    "sh".into(),
                    "-c".into(),
                    format!("while kill -0 {pid} 2>/dev/null; do sleep 5; done"),
                ],
            ))
        } else {
            None
        }
    }

    pub fn lid_command(pid: u32, duration_secs: u64) -> Option<(&'static str, Vec<String>)> {
        if cfg!(target_os = "linux") {
            Some((
                "systemd-inhibit",
                vec![
                    "--what=handle-lid-switch:sleep:idle".into(),
                    "--who=GSH".into(),
                    "--why=Lid-closed session (keep awake)".into(),
                    "--mode=block".into(),
                    "sh".into(),
                    "-c".into(),
                    format!(
                        "end=$(( $(date +%s) + {duration_secs} )); while [ $(date +%s) -lt $end ] && kill -0 {pid} 2>/dev/null; do sleep 5; done"
                    ),
                ],
            ))
        } else {
            None
        }
    }

    pub fn lid_duration(&self) -> LidSessionDuration {
        self.lid_duration
    }

    pub fn set_lid_duration(&mut self, duration: LidSessionDuration) {
        self.lid_duration = duration;
    }

    pub fn lid_stop_reason(&self) -> Option<LidSafetyStop> {
        self.lid_stop_reason
    }

    pub fn lid_remaining_minutes(&self) -> Option<u64> {
        self.lid_deadline
            .and_then(|deadline| deadline.checked_duration_since(std::time::Instant::now()))
            .map(|rem| rem.as_secs().div_ceil(60).max(1))
    }

    pub fn is_lid_active(&mut self) -> bool {
        if let Some(deadline) = self.lid_deadline {
            if std::time::Instant::now() >= deadline {
                self.stop_lid_session();
                return false;
            }
        }
        if let Some(child) = &mut self.lid_child {
            if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
                self.lid_child = None;
                self.lid_deadline = None;
            }
        }
        if self.lid_child.is_some() && cfg!(target_os = "linux") {
            self.power_source = probe_power_source();
            if let Err(reason) = check_lid_safety_at(
                Path::new("/sys/class/power_supply"),
                Path::new("/sys/class/thermal"),
            ) {
                self.stop_lid_session();
                self.lid_stop_reason = Some(reason);
                return false;
            }
        }
        self.lid_child.is_some()
    }

    pub fn start_lid_session(&mut self, duration: LidSessionDuration) -> Result<(), LidSafetyStop> {
        self.lid_duration = duration;
        if cfg!(target_os = "linux") {
            if let Err(reason) = check_lid_safety_at(
                Path::new("/sys/class/power_supply"),
                Path::new("/sys/class/thermal"),
            ) {
                self.lid_stop_reason = Some(reason);
                return Err(reason);
            }
        }
        self.stop_lid_session();
        let pid = std::process::id();
        let spawned = Self::lid_command(pid, duration.seconds()).and_then(|(program, args)| {
            Command::new(program)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok()
        });
        self.lid_child = spawned;
        if self.lid_child.is_some() {
            self.lid_deadline = Some(
                std::time::Instant::now() + std::time::Duration::from_secs(duration.seconds()),
            );
            self.lid_stop_reason = None;
            self.power_source = probe_power_source();
        }
        Ok(())
    }

    pub fn stop_lid_session(&mut self) {
        if let Some(mut child) = self.lid_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.lid_deadline = None;
    }

    pub fn is_active(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
                self.child = None;
            }
        }
        let lid_active = self.is_lid_active();
        self.child.is_some() || lid_active
    }

    pub fn on_battery(&self) -> bool {
        (self.child.is_some() || self.lid_child.is_some())
            && self.power_source == PowerSource::Battery
    }

    pub fn set(&mut self, enable: bool) {
        if !enable {
            self.release();
            return;
        }
        if self.is_active() {
            self.power_source = probe_power_source();
            return;
        }
        let pid = std::process::id();
        let spawned = Self::command(pid).and_then(|(program, args)| {
            Command::new(program)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok()
        });
        self.child = spawned;
        if self.child.is_some() {
            self.power_source = probe_power_source();
        }
    }

    pub fn release(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.stop_lid_session();
        self.power_source = PowerSource::Unknown;
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

    #[test]
    fn power_source_and_keep_awake_command_match_flutter_contract() {
        assert_eq!(
            parse_pmset_batt("Now drawing from 'AC Power'\n -InternalBattery-0 100%"),
            PowerSource::Ac
        );
        assert_eq!(
            parse_pmset_batt("Now drawing from 'Battery Power'\n"),
            PowerSource::Battery
        );
        assert_eq!(parse_pmset_batt(""), PowerSource::Unknown);

        let dir = tempfile::tempdir().unwrap();
        assert_eq!(probe_linux_power_source(dir.path()), PowerSource::Ac);
        let ac = dir.path().join("AC");
        let bat = dir.path().join("BAT0");
        std::fs::create_dir(&ac).unwrap();
        std::fs::create_dir(&bat).unwrap();
        std::fs::write(ac.join("type"), "Mains\n").unwrap();
        std::fs::write(ac.join("online"), "1\n").unwrap();
        std::fs::write(bat.join("type"), "Battery\n").unwrap();
        assert_eq!(probe_linux_power_source(dir.path()), PowerSource::Ac);
        std::fs::write(ac.join("online"), "0\n").unwrap();
        assert_eq!(probe_linux_power_source(dir.path()), PowerSource::Battery);

        #[cfg(target_os = "linux")]
        {
            let (prog, args) = KeepAwake::command(4242).unwrap();
            assert_eq!(prog, "systemd-inhibit");
            assert!(args.iter().any(|a| a == "--what=sleep:idle"));
            assert!(args.last().is_some_and(|a| a.contains("kill -0 4242")));

            let (lid_prog, lid_args) = KeepAwake::lid_command(4242, 3600).unwrap();
            assert_eq!(lid_prog, "systemd-inhibit");
            assert!(lid_args
                .iter()
                .any(|a| a == "--what=handle-lid-switch:sleep:idle"));
            assert!(lid_args.last().is_some_and(|a| a.contains("3600")));
        }

        let thermal_dir = tempfile::tempdir().unwrap();
        let tz0 = thermal_dir.path().join("thermal_zone0");
        std::fs::create_dir(&tz0).unwrap();
        std::fs::write(tz0.join("temp"), "62000\n").unwrap();
        std::fs::write(bat.join("capacity"), "55\n").unwrap();
        assert_eq!(check_lid_safety_at(dir.path(), thermal_dir.path()), Ok(()));

        std::fs::write(bat.join("capacity"), "18\n").unwrap();
        assert_eq!(
            check_lid_safety_at(dir.path(), thermal_dir.path()),
            Err(LidSafetyStop::LowBattery(18))
        );

        std::fs::write(bat.join("capacity"), "80\n").unwrap();
        std::fs::write(tz0.join("temp"), "86000\n").unwrap();
        assert_eq!(
            check_lid_safety_at(dir.path(), thermal_dir.path()),
            Err(LidSafetyStop::HighTemp(86))
        );
    }
}
