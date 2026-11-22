//! Agendamento local e histórico das execuções de agentes em terminais.

use crate::clock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const ALLOWED_ROUTINE_HARNESSES: [&str; 4] = ["claude", "codex", "agy", "pi"];
pub const WEEKDAYS: [&str; 7] = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];

pub fn every_day() -> [bool; 7] {
    [true; 7]
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledRoutine {
    pub id: String,
    pub name: String,
    pub time_hhmm: String,
    pub workspace: String,
    pub harness: String,
    pub prompt: String,
    #[serde(default = "every_day")]
    pub weekdays: [bool; 7],
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_date: Option<String>,
}

pub fn normalize_hhmm(raw: &str) -> Option<String> {
    let (h_str, m_str) = raw.trim().split_once(':')?;
    let hour: u32 = h_str.trim().parse().ok()?;
    let minute: u32 = m_str.trim().parse().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }
    Some(format!("{hour:02}:{minute:02}"))
}

pub fn normalize_routine_harness(raw: &str) -> Option<String> {
    let lower = raw.trim().to_ascii_lowercase();
    let mapped = match lower.as_str() {
        "claude" | "claude code" | "claudecode" => "claude",
        "codex" => "codex",
        "agy" | "antigravity" => "agy",
        "pi" => "pi",
        _ => return None,
    };
    Some(mapped.to_owned())
}

fn local_schedule_time(epoch_ms: u64) -> (String, String, usize) {
    let lt = clock::local_time(epoch_ms);
    // Month offsets for the Gregorian calendar (Sunday = 0).
    let month_offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let month_offset = month_offsets
        .get(lt.month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or_default();
    let year = lt.year - i64::from(lt.month < 3);
    let weekday = (year + year.div_euclid(4) - year.div_euclid(100)
        + year.div_euclid(400)
        + month_offset
        + i64::from(lt.day)
        + 6)
    .rem_euclid(7) as usize;
    (
        format!("{:04}-{:02}-{:02}", lt.year, lt.month, lt.day),
        format!("{:02}:{:02}", lt.hour, lt.minute),
        weekday,
    )
}

#[derive(Debug, Clone)]
pub struct ScheduledStore {
    path: Option<PathBuf>,
    routines: Vec<ScheduledRoutine>,
    next_seq: u64,
    runs: Vec<ScheduledRun>,
}

impl Default for ScheduledStore {
    fn default() -> Self {
        Self::load_default()
    }
}

impl ScheduledStore {
    pub fn default_path() -> Option<PathBuf> {
        if let Ok(override_path) = std::env::var("GSH_SCHEDULED_ROUTINES_PATH") {
            let trimmed = override_path.trim();
            if !trimmed.is_empty() {
                return Some(PathBuf::from(trimmed));
            }
        }
        dirs::config_dir().map(|dir| dir.join("gsh").join("scheduled_routines.json"))
    }

    pub fn load_default() -> Self {
        Self::load_from(Self::default_path())
    }

    pub fn load_from(path: Option<PathBuf>) -> Self {
        let routines: Vec<ScheduledRoutine> = path
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        let next_seq = routines
            .iter()
            .filter_map(|routine| routine.id.strip_prefix("routine-")?.parse::<u64>().ok())
            .max()
            .unwrap_or_default()
            .saturating_add(1);
        let mut runs = path
            .as_deref()
            .and_then(|path| std::fs::read_dir(path.with_extension("runs")).ok())
            .into_iter()
            .flatten()
            .filter_map(|entry| ScheduledRun::load(entry.ok()?.path()))
            .collect::<Vec<_>>();
        runs.sort_by_key(|run| run.metadata.started_ms);
        Self {
            path,
            routines,
            next_seq,
            runs,
        }
    }

    pub fn routines(&self) -> &[ScheduledRoutine] {
        &self.routines
    }

    pub fn manual_run(&self, id: &str) -> Option<ScheduledRoutine> {
        self.routines
            .iter()
            .find(|routine| routine.id == id && !self.is_running(id))
            .cloned()
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.runs
            .iter()
            .any(|run| run.metadata.routine_id == id && run.status == RunStatus::Running)
    }

    pub fn add(
        &mut self,
        name: &str,
        time_hhmm: &str,
        weekdays: [bool; 7],
        workspace: &str,
        harness: &str,
        prompt: &str,
    ) -> Result<ScheduledRoutine, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err(String::from("Informe um nome para a rotina."));
        }
        let Some(time_hhmm) = normalize_hhmm(time_hhmm) else {
            return Err(String::from(
                "Horário inválido: use o formato HH:MM (ex.: 09:00).",
            ));
        };
        if !weekdays.contains(&true) {
            return Err(String::from("Selecione pelo menos um dia da semana."));
        }
        let workspace = workspace.trim();
        let workspace = std::fs::canonicalize(workspace)
            .ok()
            .filter(|path| path.is_dir())
            .ok_or_else(|| {
                String::from("Informe uma pasta de repositório existente para a rotina.")
            })?;
        let Some(harness) = normalize_routine_harness(harness) else {
            return Err(String::from(
                "Harness inválido: escolha claude, codex, agy ou pi.",
            ));
        };
        let prompt = prompt.trim();
        if prompt.is_empty() || prompt.contains('\0') {
            return Err(String::from("Informe o prompt que será enviado ao agente."));
        }
        let id = format!("routine-{}", self.next_seq);
        self.next_seq = self.next_seq.saturating_add(1);
        let routine = ScheduledRoutine {
            id,
            name: name.to_owned(),
            time_hhmm,
            weekdays,
            workspace: workspace.to_string_lossy().into_owned(),
            harness,
            prompt: prompt.to_owned(),
            enabled: true,
            last_run_date: None,
        };
        self.routines.push(routine.clone());
        self.save();
        Ok(routine)
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        let Some(routine) = self.routines.iter_mut().find(|r| r.id == id) else {
            return false;
        };
        routine.enabled = enabled;
        self.save();
        true
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.routines.len();
        self.routines.retain(|r| r.id != id);
        if self.routines.len() != before {
            self.save();
            true
        } else {
            false
        }
    }

    /// Dispara no máximo uma vez por dia selecionado, no horário local ou depois dele.
    pub fn take_due(&mut self, epoch_ms: u64) -> Vec<ScheduledRoutine> {
        let (today, now_hhmm, weekday) = local_schedule_time(epoch_ms);
        let mut due = Vec::new();
        for routine in &mut self.routines {
            if !routine.enabled
                || !routine.weekdays.get(weekday).copied().unwrap_or_default()
                || self.runs.iter().any(|run| {
                    run.metadata.routine_id == routine.id && run.status == RunStatus::Running
                })
            {
                continue;
            }
            if routine.last_run_date.as_deref() == Some(today.as_str()) {
                continue;
            }
            if now_hhmm.as_str() >= routine.time_hhmm.as_str() {
                routine.last_run_date = Some(today.clone());
                due.push(routine.clone());
            }
        }
        if !due.is_empty() {
            self.save();
        }
        due
    }

    pub fn runs(&self) -> &[ScheduledRun] {
        &self.runs
    }

    #[cfg(target_os = "linux")]
    pub fn begin_run(
        &mut self,
        routine: &ScheduledRoutine,
        manual: bool,
        epoch_ms: u64,
    ) -> std::io::Result<&mut ScheduledRun> {
        let root = self
            .path
            .as_deref()
            .ok_or_else(|| std::io::Error::other("pasta de configuração indisponível"))?
            .with_extension("runs");
        std::fs::create_dir_all(&root)?;
        let dir = tempfile::tempdir_in(root.canonicalize()?)?;
        let metadata = RunMetadata {
            routine_id: routine.id.clone(),
            name: routine.name.clone(),
            started_ms: epoch_ms,
            manual,
        };
        std::fs::write(dir.path().join("run.json"), serde_json::to_vec(&metadata)?)?;
        std::fs::File::create(dir.path().join("output.log"))?;
        self.runs.push(ScheduledRun {
            metadata,
            dir: dir.keep(),
            status: RunStatus::Running,
            terminal: None,
        });
        self.runs
            .last_mut()
            .ok_or_else(|| std::io::Error::other("não foi possível registrar a execução"))
    }

    pub fn poll_runs(
        &mut self,
        terminal_is_open: impl Fn(u64, u32) -> bool,
    ) -> impl Iterator<Item = &ScheduledRun> {
        self.runs.iter_mut().filter_map(move |run| {
            if run.status != RunStatus::Running {
                return None;
            }
            run.status = run.read_status().unwrap_or_else(|| {
                if run
                    .terminal
                    .is_some_and(|(id, pid)| terminal_is_open(id, pid))
                {
                    RunStatus::Running
                } else {
                    run.read_status().unwrap_or(RunStatus::Interrupted)
                }
            });
            (run.status != RunStatus::Running).then_some(&*run)
        })
    }

    fn save(&self) {
        let Some(path) = self.path.as_deref() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Ok(bytes) = serde_json::to_vec_pretty(&self.routines) else {
            return;
        };
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, bytes).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunMetadata {
    routine_id: String,
    name: String,
    started_ms: u64,
    manual: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Running,
    Succeeded,
    Failed(i32),
    Interrupted,
}

#[derive(Debug, Clone)]
pub struct ScheduledRun {
    metadata: RunMetadata,
    dir: PathBuf,
    pub status: RunStatus,
    pub terminal: Option<(u64, u32)>,
}

impl ScheduledRun {
    fn load(dir: PathBuf) -> Option<Self> {
        let metadata = serde_json::from_slice(&std::fs::read(dir.join("run.json")).ok()?).ok()?;
        let mut run = Self {
            metadata,
            dir,
            status: RunStatus::Interrupted,
            terminal: None,
        };
        run.status = run.read_status().unwrap_or(RunStatus::Interrupted);
        Some(run)
    }

    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    pub fn summary(&self) -> String {
        let (date, time, _) = local_schedule_time(self.metadata.started_ms);
        let mode = if self.metadata.manual {
            "Teste"
        } else {
            "Agendada"
        };
        let status = match self.status {
            RunStatus::Running => String::from("Em andamento"),
            RunStatus::Succeeded => String::from("Concluída"),
            RunStatus::Failed(code) => format!("Falhou (código {code})"),
            RunStatus::Interrupted => String::from("Interrompida"),
        };
        format!("{date} {time} · {} · {mode} · {status}", self.name())
    }

    pub fn log(&self) -> PathBuf {
        self.dir.join("output.log")
    }

    fn read_status(&self) -> Option<RunStatus> {
        let result = std::fs::read_to_string(self.dir.join("result")).ok()?;
        match result.trim().parse().ok()? {
            0 => Some(RunStatus::Succeeded),
            code => Some(RunStatus::Failed(code)),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn fail_to_start(&self, error: &str) -> std::io::Result<()> {
        std::fs::write(self.log(), format!("Falha ao iniciar a rotina: {error}\n"))?;
        std::fs::write(self.dir.join("result"), "125\n")
    }

    #[cfg(target_os = "linux")]
    pub fn command(&self, routine: &ScheduledRoutine) -> Result<gsh_term::ShellCommand, String> {
        let args = match routine.harness.as_str() {
            "claude" => &[
                "claude",
                "--print",
                "--permission-mode",
                "dontAsk",
                "--permission-prompts",
                "none",
                "--",
            ][..],
            "codex" => &[
                "codex",
                "--ask-for-approval",
                "never",
                "exec",
                "--sandbox",
                "workspace-write",
                "--",
            ],
            "agy" => &[
                "agy",
                "--sandbox",
                "--mode",
                "accept-edits",
                "--print",
                "--",
            ],
            "pi" => &["pi", "--print", "--no-tools", "--no-extensions", "--"],
            _ => return Err(String::from("Harness inválido.")),
        };
        let workspace = std::path::Path::new(&routine.workspace);
        if !workspace.is_dir() {
            return Err(String::from(
                "A pasta configurada para esta rotina não existe.",
            ));
        }
        Ok(gsh_term::ShellCommand::new("/bin/bash")
            .args(["-c", CAPTURE_RUN, "gsh-routine"])
            .arg(&self.dir)
            .args(args)
            .arg(&routine.prompt)
            .env("BASH_ENV", "")
            .cwd(workspace))
    }
}

// Arguments are passed through as an argv array. The prompt is never shell source.
#[cfg(target_os = "linux")]
const CAPTURE_RUN: &str = r#"
dir=$1; shift
"$@" </dev/null 2>&1 | /usr/bin/tee "$dir/output.log"
codes=("${PIPESTATUS[@]}")
code=${codes[0]}
if [ "${codes[1]}" -ne 0 ]; then code=${codes[1]}; fi
printf '%s\n' "$code" >"$dir/result.tmp" && /usr/bin/mv -f -- "$dir/result.tmp" "$dir/result"
exit "$code"
"#;

#[cfg(test)]
mod tests;
