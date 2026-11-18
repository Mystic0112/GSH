//! Estado da aba Telemetria (`telemetry_viewmodel.dart`) e das abas de caso
//! (`telemetry_case_session.dart`), desacoplado do widget raiz (`Shell`).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use gsh_telemetry::{
    now_ms, AppTelemetryBridge, TelemetryCase, TelemetryCliHandler, TelemetryEvent,
    TelemetryIngest, TelemetryQuery, TelemetryRun, TelemetrySeverity, TelemetryStore,
    TelemetryStoreRegistry, TelemetryTriageActor, TelemetryTriageStatus, TelemetryWorkspace,
    K_TELEMETRY_APP_WORKSPACE_ID,
};
use iced::Task;

use crate::{clock, labels};

/// Filtro de triagem da aba Telemetria (`TelemetryFilter` do Flutter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TelemetryFilter {
    #[default]
    Open,
    New,
    Resolved,
    Ignored,
}

/// Último commit que tocou a linha de um frame (`_gitBlame`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryBlame {
    pub sha: Option<String>,
    pub at_ms: Option<i64>,
    pub author: Option<String>,
}

/// Dados carregados para uma aba `PaneTab::TelemetryCase`.
#[derive(Debug, Clone)]
pub struct TelemetryCaseDetail {
    pub case: TelemetryCase,
    pub last_event: Option<TelemetryEvent>,
    pub correlated_log: Option<TelemetryEvent>,
    pub context: Vec<TelemetryEvent>,
    pub runs_by_id: HashMap<String, TelemetryRun>,
    pub rep_run: Option<TelemetryRun>,
    pub blame: Option<TelemetryBlame>,
}

/// Efeitos colaterais que só o `Shell` consegue cumprir (abrir aba/arquivo, copiar, focar terminal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryEffect {
    OpenCaseTab { fingerprint: String, title: String },
    OpenFile { path: PathBuf, line: Option<u32> },
    CopyText(String),
    FocusTerminalPane(String),
    RetitleCaseTab { fingerprint: String, title: String },
}

#[derive(Debug, Clone)]
pub enum TelemetryMsg {
    SearchChanged(String),
    SetFilter(TelemetryFilter),
    ToggleWarnings,
    ToggleAppSource,
    ToggleProject(String),
    ToggleRun(String),
    HoverCase(Option<String>),
    OpenCase(String),
    Refresh,
    PollTick,
    Refreshed {
        generation: u64,
        workspace_id: String,
        runs: Vec<TelemetryRun>,
        all_cases: Vec<TelemetryCase>,
        db_size_bytes: u64,
    },
    CaseDetailLoaded {
        fingerprint: String,
        detail: Option<Box<TelemetryCaseDetail>>,
    },
    ResolveCase(String),
    IgnoreCase(String),
    ReopenCase(String),
    ClearCase(String),
    ClearRun(String),
    ClearProject(String),
    TriageDone(String),
    ClearDone,
    OpenLocation {
        path: String,
        line: Option<u32>,
    },
    CopyCliCommand {
        fingerprint: String,
        short_id: String,
    },
    ClearCopied(String),
    ShowInTerminal(String),
    OtlpStarted(String),
    Noop,
}

/// Contadores exibidos nos chips (`TelemetryViewModel` do Flutter).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TelemetryCounts {
    pub open: usize,
    pub fresh: usize,
    pub resolved: usize,
    pub ignored: usize,
    pub warnings: usize,
}

/// Grupo de um run e os casos cuja chave pertence a ele.
#[derive(Debug, Clone)]
pub struct RunGroup<'a> {
    pub run: Option<&'a TelemetryRun>,
    pub run_key: &'a str,
    pub cases: Vec<&'a TelemetryCase>,
}

/// Agrupamento projeto -> runs -> casos espelhando `_tree` em `telemetry_panel.dart`.
#[derive(Debug, Clone)]
pub struct ProjectGroup<'a> {
    pub project: &'a str,
    pub runs: Vec<RunGroup<'a>>,
    pub total_cases: usize,
}

/// Estado completo da Telemetria no `Shell`.
pub struct TelemetryState {
    pub registry: Arc<TelemetryStoreRegistry>,
    pub ingest: Arc<TelemetryIngest>,
    pub app_bridge: Arc<AppTelemetryBridge>,
    pub cli_handler: Arc<TelemetryCliHandler>,
    pub last_edit_at: Arc<Mutex<HashMap<String, i64>>>,
    workspace_roots_map: Arc<Mutex<HashMap<String, Vec<String>>>>,
    pub workspace_id: Option<String>,
    pub workspace_name: Option<String>,
    pub workspace_root: Option<PathBuf>,
    pub workspace_roots: Vec<PathBuf>,
    pub developer_mode: bool,
    pub show_app_telemetry: bool,
    pub filter: TelemetryFilter,
    pub show_warnings: bool,
    pub search: String,
    pub runs: Vec<TelemetryRun>,
    pub cases: Vec<TelemetryCase>,
    pub counts: TelemetryCounts,
    pub db_size_bytes: u64,
    pub collapsed_projects: HashSet<String>,
    pub collapsed_runs: HashSet<String>,
    pub hovered_case: Option<String>,
    pub case_details: HashMap<String, TelemetryCaseDetail>,
    pub copied_fingerprint: Option<String>,
    pub loading: bool,
    refresh_generation: u64,
    otlp_started: bool,
    static_test_mode: bool,
}

impl TelemetryState {
    /// Cria o estado com o registry em disco (produção). O receiver OTLP é
    /// iniciado via `boot_task()` depois que o runtime do iced sobe.
    pub fn new() -> Self {
        Self::with_registry(Arc::new(TelemetryStoreRegistry::new()), false)
    }

    /// Variante em memória para testes sem tocar `~/.local/share/.../telemetry`.
    pub fn in_memory() -> Self {
        Self::with_registry(Arc::new(TelemetryStoreRegistry::memory()), true)
    }

    fn with_registry(registry: Arc<TelemetryStoreRegistry>, static_test_mode: bool) -> Self {
        let ingest = TelemetryIngest::new(Arc::clone(&registry));
        let last_edit_at = Arc::new(Mutex::new(HashMap::new()));
        let edit_ref = Arc::clone(&last_edit_at);
        let workspace_roots_map: Arc<Mutex<HashMap<String, Vec<String>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let roots_ref = Arc::clone(&workspace_roots_map);
        let cli_handler = Arc::new(TelemetryCliHandler::new(
            Arc::clone(&registry),
            Arc::clone(&ingest),
            Arc::new(move |ws_id| edit_ref.lock().ok().and_then(|map| map.get(ws_id).copied())),
            Arc::new(move |ws_id| {
                roots_ref
                    .lock()
                    .ok()
                    .and_then(|map| map.get(ws_id).cloned())
                    .unwrap_or_default()
            }),
        ));
        Self {
            registry,
            ingest,
            app_bridge: Arc::new(AppTelemetryBridge::new()),
            cli_handler,
            last_edit_at,
            workspace_roots_map,
            workspace_id: None,
            workspace_name: None,
            workspace_root: None,
            workspace_roots: Vec::new(),
            developer_mode: false,
            show_app_telemetry: false,
            filter: TelemetryFilter::Open,
            show_warnings: true,
            search: String::new(),
            runs: Vec::new(),
            cases: Vec::new(),
            counts: TelemetryCounts::default(),
            db_size_bytes: 0,
            collapsed_projects: HashSet::new(),
            collapsed_runs: HashSet::new(),
            hovered_case: None,
            case_details: HashMap::new(),
            copied_fingerprint: None,
            loading: false,
            refresh_generation: 0,
            otlp_started: false,
            static_test_mode,
        }
    }

    /// `true` quando o workspace tem até uma root (a árvore omite o cabeçalho do projeto).
    pub fn single_root(&self) -> bool {
        self.workspace_roots.len() <= 1
    }

    /// Inicia o listener OTLP e o bridge interno (se `developer_mode` estiver ligado).
    pub fn boot_task(&mut self) -> Task<TelemetryMsg> {
        if self.static_test_mode || self.otlp_started {
            return Task::none();
        }
        self.otlp_started = true;
        let ingest = Arc::clone(&self.ingest);
        let bridge = Arc::clone(&self.app_bridge);
        let dev_mode = self.developer_mode;
        Task::perform(
            async move {
                let ep = ingest.start_otlp().await;
                if dev_mode {
                    bridge.start(&ingest, env!("CARGO_PKG_VERSION")).await;
                }
                ep
            },
            TelemetryMsg::OtlpStarted,
        )
    }

    /// Sincroniza a flag `developer_mode` das Configurações com o `AppTelemetryBridge`.
    pub fn set_developer_mode(&mut self, enabled: bool) -> Task<TelemetryMsg> {
        if self.developer_mode == enabled {
            return Task::none();
        }
        self.developer_mode = enabled;
        if !enabled {
            self.show_app_telemetry = false;
        }
        if self.static_test_mode {
            return Task::none();
        }
        let ingest = Arc::clone(&self.ingest);
        let bridge = Arc::clone(&self.app_bridge);
        Task::perform(
            async move {
                if enabled {
                    bridge.start(&ingest, env!("CARGO_PKG_VERSION")).await;
                } else {
                    bridge.close().await;
                }
                if enabled {
                    TelemetryMsg::Refresh
                } else {
                    TelemetryMsg::Noop
                }
            },
            |m| m,
        )
    }

    /// Registra a troca de workspace ativo (ou `None` ao limpar) e dispara carga inicial.
    pub fn set_workspace(
        &mut self,
        id: Option<String>,
        name: Option<String>,
        root: Option<PathBuf>,
        roots: Vec<PathBuf>,
    ) -> Task<TelemetryMsg> {
        if self.workspace_id == id && self.workspace_root == root {
            return Task::none();
        }
        self.workspace_id = id.clone();
        self.workspace_name = name.clone();
        self.workspace_root = root.clone();
        self.workspace_roots = if roots.is_empty() {
            root.iter().cloned().collect()
        } else {
            roots
        };
        self.runs.clear();
        self.cases.clear();
        self.counts = TelemetryCounts::default();
        self.db_size_bytes = 0;
        self.hovered_case = None;

        if let (Some(ws_id), Some(ws_root)) = (id, root) {
            let root_str = ws_root.to_string_lossy().to_string();
            let root_strs: Vec<String> = self
                .workspace_roots
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            if let Ok(mut map) = self.workspace_roots_map.lock() {
                map.insert(ws_id.clone(), root_strs.clone());
            }
            self.ingest.register_workspace(TelemetryWorkspace {
                id: ws_id,
                name: name.unwrap_or_else(|| {
                    ws_root
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("workspace")
                        .to_string()
                }),
                path: root_str,
                roots: root_strs,
            });
            return self.refresh_task();
        }
        Task::none()
    }

    /// Registra o instante de salvamento de um arquivo no editor (para `--since-edit`).
    pub fn record_file_edit(&self, path: &Path) {
        let Some(ws_id) = self.workspace_id.as_deref() else {
            return;
        };
        let in_ws = self
            .workspace_roots
            .iter()
            .any(|root| path.starts_with(root));
        if !in_ws {
            return;
        }
        if let Ok(mut map) = self.last_edit_at.lock() {
            map.insert(ws_id.to_string(), now_ms());
        }
    }

    /// Id efetivo da base exibida (`__gsh__` quando o chip `GSH Desktop` está ativo).
    pub fn effective_workspace_id(&self) -> Option<&str> {
        if self.developer_mode && self.show_app_telemetry {
            Some(K_TELEMETRY_APP_WORKSPACE_ID)
        } else {
            self.workspace_id.as_deref()
        }
    }

    pub fn live_run_count(&self) -> usize {
        self.runs.iter().filter(|r| r.is_live()).count()
    }

    /// Calcula os contadores e aplica `filter` + `show_warnings` sobre a lista completa (`refresh`).
    fn compute_filtered(
        all_cases: Vec<TelemetryCase>,
        filter: TelemetryFilter,
        show_warnings: bool,
    ) -> (TelemetryCounts, Vec<TelemetryCase>) {
        let mut counts = TelemetryCounts::default();
        for c in &all_cases {
            match c.status {
                TelemetryTriageStatus::Open => {
                    counts.open += 1;
                    if c.is_new || c.is_regression {
                        counts.fresh += 1;
                    }
                    if c.severity == TelemetrySeverity::Warn {
                        counts.warnings += 1;
                    }
                }
                TelemetryTriageStatus::Resolved => counts.resolved += 1,
                TelemetryTriageStatus::Ignored => counts.ignored += 1,
            }
        }
        let min_sev = if show_warnings {
            TelemetrySeverity::Warn
        } else {
            TelemetrySeverity::Error
        };
        let filtered = all_cases
            .into_iter()
            .filter(|c| {
                if c.severity.index() < min_sev.index() {
                    return false;
                }
                match filter {
                    TelemetryFilter::Open => c.status == TelemetryTriageStatus::Open,
                    TelemetryFilter::New => {
                        c.status == TelemetryTriageStatus::Open && (c.is_new || c.is_regression)
                    }
                    TelemetryFilter::Resolved => c.status == TelemetryTriageStatus::Resolved,
                    TelemetryFilter::Ignored => c.status == TelemetryTriageStatus::Ignored,
                }
            })
            .collect();
        (counts, filtered)
    }

    pub fn refresh_task(&mut self) -> Task<TelemetryMsg> {
        let Some(ws_id) = self.effective_workspace_id().map(ToOwned::to_owned) else {
            self.runs.clear();
            self.cases.clear();
            self.counts = TelemetryCounts::default();
            self.db_size_bytes = 0;
            return Task::none();
        };
        if self.static_test_mode {
            return Task::none();
        }
        self.refresh_generation = self.refresh_generation.saturating_add(1);
        let generation = self.refresh_generation;
        let text_q = self.search.trim();
        let query = TelemetryQuery {
            text: (!text_q.is_empty()).then(|| text_q.to_string()),
            min_severity: Some(TelemetrySeverity::Warn),
            include_ignored: true,
            include_resolved: true,
            limit: 500,
            ..TelemetryQuery::default()
        };
        let registry = Arc::clone(&self.registry);
        self.loading = true;
        Task::perform(
            async move {
                let Ok(store) = registry.for_workspace(&ws_id).await else {
                    return TelemetryMsg::Refreshed {
                        generation,
                        workspace_id: ws_id,
                        runs: Vec::new(),
                        all_cases: Vec::new(),
                        db_size_bytes: 0,
                    };
                };
                let runs = store.runs(None, None, false, 100).await.unwrap_or_default();
                let all_cases = store.cases(query).await.unwrap_or_default();
                let db_size_bytes = store.size_in_bytes().await.unwrap_or(0);
                TelemetryMsg::Refreshed {
                    generation,
                    workspace_id: ws_id,
                    runs,
                    all_cases,
                    db_size_bytes,
                }
            },
            |m| m,
        )
    }

    pub fn load_case_detail_task(&self, fingerprint: String) -> Task<TelemetryMsg> {
        let Some(ws_id) = self.effective_workspace_id().map(ToOwned::to_owned) else {
            return Task::none();
        };
        if self.static_test_mode {
            return Task::none();
        }
        let registry = Arc::clone(&self.registry);
        let roots = self.workspace_roots.clone();
        Task::perform(
            async move {
                let detail = fetch_case_detail(&registry, &ws_id, &fingerprint, &roots)
                    .await
                    .map(Box::new);
                TelemetryMsg::CaseDetailLoaded {
                    fingerprint,
                    detail,
                }
            },
            |m| m,
        )
    }

    pub fn update(&mut self, msg: TelemetryMsg) -> (Task<TelemetryMsg>, Option<TelemetryEffect>) {
        match msg {
            TelemetryMsg::Noop | TelemetryMsg::OtlpStarted(_) => (Task::none(), None),
            TelemetryMsg::SearchChanged(value) => {
                self.search = value;
                (self.refresh_task(), None)
            }
            TelemetryMsg::SetFilter(filter) => {
                if self.filter != filter {
                    self.filter = filter;
                    return (self.refresh_task(), None);
                }
                (Task::none(), None)
            }
            TelemetryMsg::ToggleWarnings => {
                self.show_warnings = !self.show_warnings;
                (self.refresh_task(), None)
            }
            TelemetryMsg::ToggleAppSource => {
                if self.developer_mode {
                    self.show_app_telemetry = !self.show_app_telemetry;
                    self.runs.clear();
                    self.cases.clear();
                    self.counts = TelemetryCounts::default();
                    return (self.refresh_task(), None);
                }
                (Task::none(), None)
            }
            TelemetryMsg::ToggleProject(project) => {
                if !self.collapsed_projects.remove(&project) {
                    self.collapsed_projects.insert(project);
                }
                (Task::none(), None)
            }
            TelemetryMsg::ToggleRun(run_id) => {
                if !self.collapsed_runs.remove(&run_id) {
                    self.collapsed_runs.insert(run_id);
                }
                (Task::none(), None)
            }
            TelemetryMsg::HoverCase(fp) => {
                self.hovered_case = fp;
                (Task::none(), None)
            }
            TelemetryMsg::OpenCase(fingerprint) => {
                let title = self
                    .cases
                    .iter()
                    .find(|c| c.fingerprint == fingerprint)
                    .or_else(|| self.case_details.get(&fingerprint).map(|d| &d.case))
                    .map(case_tab_title)
                    .unwrap_or_else(|| fingerprint.chars().take(12).collect());
                let task = self.load_case_detail_task(fingerprint.clone());
                (
                    task,
                    Some(TelemetryEffect::OpenCaseTab { fingerprint, title }),
                )
            }
            TelemetryMsg::Refresh | TelemetryMsg::PollTick => {
                let mut tasks = vec![self.refresh_task()];
                for fp in self.case_details.keys() {
                    tasks.push(self.load_case_detail_task(fp.clone()));
                }
                (Task::batch(tasks), None)
            }
            TelemetryMsg::Refreshed {
                generation,
                workspace_id,
                runs,
                all_cases,
                db_size_bytes,
            } => {
                if generation == self.refresh_generation
                    && self.effective_workspace_id() == Some(workspace_id.as_str())
                {
                    self.loading = false;
                    self.runs = runs;
                    let (counts, cases) =
                        Self::compute_filtered(all_cases, self.filter, self.show_warnings);
                    self.counts = counts;
                    self.cases = cases;
                    self.db_size_bytes = db_size_bytes;
                }
                (Task::none(), None)
            }
            TelemetryMsg::CaseDetailLoaded {
                fingerprint,
                detail,
            } => {
                let mut effect = None;
                if let Some(detail) = detail {
                    let title = case_tab_title(&detail.case);
                    effect = Some(TelemetryEffect::RetitleCaseTab {
                        fingerprint: fingerprint.clone(),
                        title,
                    });
                    self.case_details.insert(fingerprint, *detail);
                }
                (Task::none(), effect)
            }
            TelemetryMsg::ResolveCase(fp) => {
                (self.triage_task(fp, TelemetryTriageStatus::Resolved), None)
            }
            TelemetryMsg::IgnoreCase(fp) => {
                (self.triage_task(fp, TelemetryTriageStatus::Ignored), None)
            }
            TelemetryMsg::ReopenCase(fp) => {
                (self.triage_task(fp, TelemetryTriageStatus::Open), None)
            }
            TelemetryMsg::TriageDone(fp) => {
                let mut tasks = vec![self.refresh_task()];
                if self.case_details.contains_key(&fp) {
                    tasks.push(self.load_case_detail_task(fp));
                }
                (Task::batch(tasks), None)
            }
            TelemetryMsg::ClearCase(fp) => (self.clear_task(Some(fp), None, None), None),
            TelemetryMsg::ClearRun(run_id) => (self.clear_task(None, Some(run_id), None), None),
            TelemetryMsg::ClearProject(project) => {
                (self.clear_task(None, None, Some(project)), None)
            }
            TelemetryMsg::ClearDone => (self.refresh_task(), None),
            TelemetryMsg::OpenLocation { path, line } => {
                let resolved = resolve_project_file(&self.workspace_roots, &path)
                    .unwrap_or_else(|| PathBuf::from(&path));
                (
                    Task::none(),
                    Some(TelemetryEffect::OpenFile {
                        path: resolved,
                        line,
                    }),
                )
            }
            TelemetryMsg::CopyCliCommand {
                fingerprint,
                short_id,
            } => {
                self.copied_fingerprint = Some(fingerprint.clone());
                let cmd = format!("gsh telemetry show {short_id}");
                let clear_fp = fingerprint;
                let timer = if self.static_test_mode {
                    Task::none()
                } else {
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(|| {
                                std::thread::sleep(std::time::Duration::from_millis(1600));
                            })
                            .await
                            .ok();
                            clear_fp
                        },
                        TelemetryMsg::ClearCopied,
                    )
                };
                (timer, Some(TelemetryEffect::CopyText(cmd)))
            }
            TelemetryMsg::ClearCopied(fp) => {
                if self.copied_fingerprint.as_deref() == Some(fp.as_str()) {
                    self.copied_fingerprint = None;
                }
                (Task::none(), None)
            }
            TelemetryMsg::ShowInTerminal(pane_id) => (
                Task::none(),
                Some(TelemetryEffect::FocusTerminalPane(pane_id)),
            ),
        }
    }

    fn triage_task(&mut self, fp: String, status: TelemetryTriageStatus) -> Task<TelemetryMsg> {
        if let Some(c) = self.cases.iter_mut().find(|c| c.fingerprint == fp) {
            c.status = status;
        }
        if let Some(detail) = self.case_details.get_mut(&fp) {
            detail.case.status = status;
        }
        if self.static_test_mode {
            return Task::none();
        }
        let Some(ws_id) = self.effective_workspace_id().map(ToOwned::to_owned) else {
            return Task::none();
        };
        let registry = Arc::clone(&self.registry);
        Task::perform(
            async move {
                if let Ok(store) = registry.for_workspace(&ws_id).await {
                    let _ = store
                        .triage(&fp, status, TelemetryTriageActor::Human, None)
                        .await;
                }
                TelemetryMsg::TriageDone(fp)
            },
            |m| m,
        )
    }

    fn clear_task(
        &mut self,
        fingerprint: Option<String>,
        run_id: Option<String>,
        project: Option<String>,
    ) -> Task<TelemetryMsg> {
        if let Some(ref fp) = fingerprint {
            self.cases.retain(|c| &c.fingerprint != fp);
        }
        if self.static_test_mode {
            return Task::none();
        }
        let Some(ws_id) = self.effective_workspace_id().map(ToOwned::to_owned) else {
            return Task::none();
        };
        let registry = Arc::clone(&self.registry);
        Task::perform(
            async move {
                if let Ok(store) = registry.for_workspace(&ws_id).await {
                    let _ = store.clear(fingerprint, run_id, project).await;
                }
                TelemetryMsg::ClearDone
            },
            |m| m,
        )
    }

    /// Agrupa os casos visíveis em árvore projeto -> run -> casos (`telemetry_panel.dart`).
    pub fn grouped_tree(&self) -> Vec<ProjectGroup<'_>> {
        let mut by_id: HashMap<&str, &TelemetryRun> = HashMap::new();
        let mut by_key: HashMap<&str, &TelemetryRun> = HashMap::new();
        for run in &self.runs {
            by_id.insert(run.id.as_str(), run);
        }
        for case in &self.cases {
            if let Some(first_cr) = case.runs.first() {
                if let Some(run) = by_id.get(first_cr.run_id.as_str()).copied() {
                    let entry = by_key.entry(case.run_key.as_str()).or_insert(run);
                    if run.started_at_ms > entry.started_at_ms {
                        *entry = run;
                    }
                }
            }
        }

        let mut tree: BTreeMap<&str, HashMap<&str, Vec<&TelemetryCase>>> = BTreeMap::new();
        for case in &self.cases {
            tree.entry(case.project.as_str())
                .or_default()
                .entry(case.run_key.as_str())
                .or_default()
                .push(case);
        }

        tree.into_iter()
            .map(|(project, runs_map)| {
                let mut total_cases = 0usize;
                let mut run_groups: Vec<RunGroup<'_>> = runs_map
                    .into_iter()
                    .map(|(run_key, mut cases)| {
                        cases.sort_by_key(|a| std::cmp::Reverse(a.last_at_ms));
                        total_cases += cases.len();
                        RunGroup {
                            run: by_key.get(run_key).copied(),
                            run_key,
                            cases,
                        }
                    })
                    .collect();
                run_groups.sort_by(|a, b| {
                    let a_ts = a.run.map(|r| r.started_at_ms);
                    let b_ts = b.run.map(|r| r.started_at_ms);
                    match (b_ts, a_ts) {
                        (Some(bt), Some(at)) => bt.cmp(&at),
                        _ => a.run_key.cmp(b.run_key),
                    }
                });
                ProjectGroup {
                    project,
                    runs: run_groups,
                    total_cases,
                }
            })
            .collect()
    }
}

impl Default for TelemetryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Remove prefixo `"name:"` ou `"cmd:"` da `run_key` (`telemetry_panel.dart:443`).
pub fn strip_run_key_prefix(run_key: &str) -> &str {
    run_key
        .strip_prefix("name:")
        .or_else(|| run_key.strip_prefix("cmd:"))
        .unwrap_or(run_key)
}

/// Rótulo exibido para um run (`run?.name ?? run?.command ?? runKey sem prefixo`).
pub fn run_display_label<'a>(run: Option<&'a TelemetryRun>, run_key: &'a str) -> &'a str {
    run.and_then(|r| {
        r.name
            .as_deref()
            .filter(|n| !n.trim().is_empty())
            .or(Some(r.command.as_str()))
    })
    .unwrap_or_else(|| strip_run_key_prefix(run_key))
}

/// Replica `TextPainter.ellipsis` do Flutter (`text_painter.dart:932-936`):
/// - `max_lines == None` (`maxLines: null, overflow: TextOverflow.ellipsis`):
///   mantém cada linha que cabe em `max_chars`; ao encontrar a PRIMEIRA linha
///   maior que `max_chars`, corta nela com `"…"` e descarta todo o resto.
/// - `max_lines == Some(n)` (`maxLines: n, overflow: TextOverflow.ellipsis`):
///   mantém até `n` linhas; se a última estourar `max_chars` ou houver linhas
///   sobrando além de `n`, aplica `"…"` na última linha visível e descarta o resto.
pub fn flutter_ellipsis(input: &str, max_chars: usize, max_lines: Option<usize>) -> String {
    let normalized = input.replace("\r\n", "\n");
    let clean = normalized.trim_end_matches('\n');
    let raw_lines: Vec<&str> = clean.split('\n').collect();

    if let Some(limit) = max_lines {
        if limit == 0 {
            return String::new();
        }
        let mut wrapped: Vec<(String, bool)> = Vec::new();
        for line in &raw_lines {
            let chars: Vec<char> = line.chars().collect();
            if chars.is_empty() {
                wrapped.push((String::new(), false));
                continue;
            }
            let width = max_chars.max(1);
            for chunk in chars.chunks(width) {
                let s: String = chunk.iter().collect();
                wrapped.push((s, chars.len() > width));
                if wrapped.len() > limit {
                    break;
                }
            }
            if wrapped.len() > limit {
                break;
            }
        }
        let overflowed_lines = wrapped.len() > limit;
        wrapped.truncate(limit);
        if overflowed_lines {
            if let Some((last, _)) = wrapped.last_mut() {
                *last = truncate_line_ellipsis(last, max_chars);
            }
        } else if let Some((last, line_too_wide)) = wrapped.last_mut() {
            if *line_too_wide || last.chars().count() > max_chars {
                *last = truncate_line_ellipsis(last, max_chars);
            }
        }
        return wrapped
            .into_iter()
            .map(|(line, _)| line)
            .collect::<Vec<_>>()
            .join("\n");
    }

    let mut out = Vec::new();
    for line in raw_lines {
        if line.chars().count() > max_chars {
            out.push(truncate_line_ellipsis(line, max_chars));
            break;
        }
        out.push(line.to_string());
    }
    out.join("\n")
}

fn truncate_line_ellipsis(line: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return "…".to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let prefix: String = line.chars().take(keep).collect();
    let trimmed = prefix.trim_end();
    format!("{trimmed}…")
}

pub fn case_tab_title(case: &TelemetryCase) -> String {
    let file_part = case
        .location
        .as_deref()
        .and_then(|loc| loc.rsplit('/').next())
        .unwrap_or(case.project.as_str());
    labels::telemetry_case_tab_title(&case.err_type, file_part)
}

async fn fetch_case_detail(
    registry: &TelemetryStoreRegistry,
    ws_id: &str,
    fingerprint: &str,
    roots: &[PathBuf],
) -> Option<TelemetryCaseDetail> {
    let store: TelemetryStore = registry.for_workspace(ws_id).await.ok()?;
    let case = store.case_by_id(fingerprint).await.ok().flatten()?;
    let last_event = match case.last_event_id.as_deref() {
        Some(id) => store.event(id).await.ok().flatten(),
        None => None,
    };
    let (context, correlated_log) = match last_event.as_ref().and_then(|e| e.id.as_deref()) {
        Some(eid) => {
            let ctx = store.context(eid, 6, 4).await.unwrap_or_default();
            let log = fetch_correlated_log(&store, last_event.as_ref()).await;
            (ctx, log)
        }
        None => (Vec::new(), None),
    };

    let all_runs = store.runs(None, None, false, 100).await.unwrap_or_default();
    let mut runs_by_id = HashMap::new();
    for r in all_runs {
        runs_by_id.insert(r.id.clone(), r);
    }
    let rep_run = case
        .runs
        .first()
        .and_then(|cr| runs_by_id.get(&cr.run_id).cloned())
        .or_else(|| {
            last_event
                .as_ref()
                .and_then(|e| runs_by_id.get(&e.run_id).cloned())
        });

    let blame_target = last_event
        .as_ref()
        .and_then(|e| e.error.as_ref())
        .and_then(|err| err.project_frame())
        .and_then(|frame| {
            let file = frame.file.as_deref()?;
            let line = frame.line?;
            resolve_project_file(roots, file).map(|p| (p, line))
        })
        .or_else(|| {
            let loc = case.location.as_deref()?;
            let (file, line, _) = split_location(loc);
            let line = line?;
            resolve_project_file(roots, &file).map(|p| (p, line))
        });

    let blame = match blame_target {
        Some((path, line)) => tokio::task::spawn_blocking(move || git_blame(&path, line))
            .await
            .ok()
            .flatten(),
        None => None,
    };

    Some(TelemetryCaseDetail {
        case,
        last_event,
        correlated_log,
        context,
        runs_by_id,
        rep_run,
        blame,
    })
}

async fn fetch_correlated_log(
    store: &TelemetryStore,
    last_event: Option<&TelemetryEvent>,
) -> Option<TelemetryEvent> {
    let ev = last_event?;
    let events = store
        .events(TelemetryQuery {
            run_id: Some(ev.run_id.clone()),
            until_ms: Some(ev.ts_ms),
            limit: 40,
            ..TelemetryQuery::default()
        })
        .await
        .ok()?;
    events
        .into_iter()
        .filter(|e| !e.is_error() && !e.attrs.is_empty() && e.id != ev.id)
        .max_by_key(|e| e.ts_ms)
}

/// Separa `"path:line:col"` em `(path, Some(line), Some(col))`, preservando driver Windows (`C:\...`).
pub fn split_location(raw: &str) -> (String, Option<u32>, Option<u32>) {
    let trimmed = raw.trim();
    let mut parts = trimmed.rsplitn(3, ':');
    let last = parts.next();
    let mid = parts.next();
    let head = parts.next();
    if let (Some(col_s), Some(line_s), Some(path_s)) = (last, mid, head) {
        if let (Ok(col), Ok(line)) = (col_s.parse::<u32>(), line_s.parse::<u32>()) {
            if !path_s.is_empty() {
                return (path_s.to_string(), Some(line), Some(col));
            }
        }
    }
    if let Some((path_s, num_s)) = trimmed.rsplit_once(':') {
        if let Ok(line) = num_s.parse::<u32>() {
            if !path_s.is_empty() {
                return (path_s.to_string(), Some(line), None);
            }
        }
    }
    (trimmed.to_string(), None, None)
}

pub fn resolve_project_file(roots: &[PathBuf], raw: &str) -> Option<PathBuf> {
    let candidate = Path::new(raw);
    if candidate.is_absolute() && candidate.exists() {
        return Some(candidate.to_path_buf());
    }
    let clean = raw.trim_start_matches("./");
    for root in roots {
        let joined = root.join(clean);
        if joined.exists() {
            return Some(joined);
        }
    }
    None
}

pub fn git_blame(path: &Path, line: u32) -> Option<TelemetryBlame> {
    if line == 0 {
        return None;
    }
    let parent = path.parent()?;
    let range = format!("{line},{line}");
    let output = std::process::Command::new("git")
        .args(["blame", "-L", &range, "--porcelain", "--"])
        .arg(path)
        .current_dir(parent)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_git_blame_porcelain(&stdout)
}

pub fn parse_git_blame_porcelain(stdout: &str) -> Option<TelemetryBlame> {
    let mut lines = stdout.lines();
    let first = lines.next()?;
    let sha_full = first.split_whitespace().next()?;
    if sha_full.len() < 7 || !sha_full.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    if sha_full.chars().all(|c| c == '0') {
        return Some(TelemetryBlame {
            sha: None,
            at_ms: None,
            author: None,
        });
    }
    let mut author = None;
    let mut author_time_secs = None;
    for l in lines {
        if let Some(rest) = l.strip_prefix("author ") {
            author = Some(rest.trim().to_string());
        } else if let Some(rest) = l.strip_prefix("author-time ") {
            author_time_secs = rest.trim().parse::<i64>().ok();
        }
    }
    Some(TelemetryBlame {
        sha: Some(sha_full.chars().take(7).collect()),
        at_ms: author_time_secs.map(|s| s.saturating_mul(1000)),
        author,
    })
}

pub fn format_relative_ms(ts_ms: i64, now: i64) -> String {
    let diff_secs = ((now - ts_ms) / 1000).max(0);
    if diff_secs < 60 {
        return labels::TELEMETRY_JUST_NOW.to_string();
    }
    let diff_min = diff_secs / 60;
    if diff_min < 60 {
        return labels::telemetry_minutes_ago(diff_min);
    }
    let diff_hours = diff_min / 60;
    if diff_hours < 24 {
        return labels::telemetry_hours_ago(diff_hours);
    }
    labels::telemetry_days_ago(diff_hours / 24)
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 * 1024 {
        format!("{} KB", (bytes as f64 / 1024.0).round() as u64)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn format_date_time(ts_ms: i64) -> String {
    if ts_ms <= 0 {
        return labels::TELEMETRY_DASH.to_string();
    }
    let ms_u64 = u64::try_from(ts_ms).unwrap_or(0);
    let lt = clock::local_time(ms_u64);
    let hms = clock::hhmmss(ms_u64);
    format!("{:02}/{:02} {hms}", lt.day, lt.month)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_location_handles_line_col_and_windows_drive() {
        assert_eq!(
            split_location("lib/main.dart:42:7"),
            ("lib/main.dart".to_string(), Some(42), Some(7))
        );
        assert_eq!(
            split_location("src/lib.rs:19"),
            ("src/lib.rs".to_string(), Some(19), None)
        );
        assert_eq!(
            split_location(r"C:\app\main.dart:12:3"),
            (r"C:\app\main.dart".to_string(), Some(12), Some(3))
        );
        assert_eq!(
            split_location("src/only_file.rs"),
            ("src/only_file.rs".to_string(), None, None)
        );
    }

    #[test]
    fn parse_git_blame_distinguishes_uncommitted_and_commit() {
        let dirty = "0000000000000000000000000000000000000000 1 1 1\nauthor Not Committed Yet\nauthor-time 1700000000\n";
        assert_eq!(
            parse_git_blame_porcelain(dirty),
            Some(TelemetryBlame {
                sha: None,
                at_ms: None,
                author: None,
            })
        );

        let committed = "abcdef1234567890abcdef1234567890abcdef12 5 5 1\nauthor Alice\nauthor-time 1700000000\n";
        assert_eq!(
            parse_git_blame_porcelain(committed),
            Some(TelemetryBlame {
                sha: Some("abcdef1".to_string()),
                at_ms: Some(1_700_000_000_000),
                author: Some("Alice".to_string()),
            })
        );
    }

    #[test]
    fn format_relative_and_bytes_match_flutter() {
        let base = 1_700_000_000_000_i64;
        assert_eq!(format_relative_ms(base - 20_000, base), "agora");
        assert_eq!(format_relative_ms(base - 5 * 60_000, base), "há 5 min");
        assert_eq!(format_relative_ms(base - 3 * 3_600_000, base), "há 3 h");
        assert_eq!(format_relative_ms(base - 2 * 86_400_000, base), "há 2 d");

        assert_eq!(format_bytes(0), "0 KB");
        assert_eq!(format_bytes(4096), "4 KB");
        assert_eq!(format_bytes(3 * 1024 * 1024 / 2), "1.5 MB");
    }

    #[test]
    fn flutter_ellipsis_matches_text_painter_semantics() {
        let py = "python3 -c '\nimport sys\nprint(\"WARN [cache] slow miss\", file=sys.stderr)\ndef load_user():\n    raise RuntimeError(\"failed\")\nload_user()\n'";
        // maxLines == None: mantém linhas até a primeira que exceder max_chars, aplica '…' nela e descarta o resto
        assert_eq!(
            flutter_ellipsis(py, 18, None),
            "python3 -c '\nimport sys\nprint(\"WARN [cach…"
        );
        // Todas as linhas cabem
        assert_eq!(
            flutter_ellipsis("linha 1\nlinha 2", 20, None),
            "linha 1\nlinha 2"
        );
        // Linha 0 já excede
        assert_eq!(flutter_ellipsis("abcdefghij\nsegunda", 6, None), "abcde…");
        // maxLines == 1 (linha de mensagem de caso)
        assert_eq!(flutter_ellipsis("hello\nworld", 20, Some(1)), "hello…");
        assert_eq!(flutter_ellipsis("abcdefghij", 6, Some(1)), "abcde…");
        assert_eq!(flutter_ellipsis("curta", 20, Some(1)), "curta");
        // maxLines == 3 (linha de contexto cru)
        assert_eq!(flutter_ellipsis("a\nb\nc\nd", 20, Some(3)), "a\nb\nc…");
    }
}
