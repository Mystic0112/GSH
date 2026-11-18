use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use fancy_regex::Regex as FancyRegex;
use tokio::sync::{broadcast, Mutex as AsyncMutex};
use tokio::task::JoinHandle;

use crate::config::{TelemetryProxyConfig, TelemetryWorkspaceConfig};
use crate::fingerprint::fingerprint_of;
use crate::model::{
    now_ms, TelemetryErrorNotice, TelemetryEvent, TelemetryRun, TelemetryRunSource,
    TelemetrySeverity, TelemetryStoreError, TelemetryStream, TelemetryTaskDef, TelemetryWorkspace,
    K_TELEMETRY_APP_WORKSPACE_ID,
};
use crate::otlp::{OtlpReceiver, OtlpSink};
use crate::parser::frames::{is_within_path, normalize_path_str};
use crate::parser::line_parser::{TelemetryLineParser, TelemetryParserConfig};
use crate::proxy::{HttpTelemetryProxy, ProxySink};
use crate::registry::TelemetryStoreRegistry;
use crate::store::TelemetryStore;
use crate::vm_service::{VmServiceAttachment, VmSink, VM_SERVICE_URI_PATTERN};

const BATCH_LINES: usize = 200;
const BATCH_DELAY: Duration = Duration::from_millis(100);
const DEDUP_WINDOW_MS: i64 = 1500;

pub struct TelemetryIngest {
    registry: Arc<TelemetryStoreRegistry>,
    workspaces: RwLock<HashMap<String, TelemetryWorkspace>>,
    configs: RwLock<HashMap<String, TelemetryWorkspaceConfig>>,
    proxies: AsyncMutex<HashMap<String, Arc<HttpTelemetryProxy>>>,
    live: RwLock<HashMap<String, Arc<TelemetryIngestSession>>>,
    notices_tx: broadcast::Sender<TelemetryErrorNotice>,
    otlp: RwLock<Option<Arc<OtlpReceiver>>>,
}

impl TelemetryIngest {
    pub fn new(registry: Arc<TelemetryStoreRegistry>) -> Arc<Self> {
        let (notices_tx, _) = broadcast::channel(64);
        Arc::new(Self {
            registry,
            workspaces: RwLock::new(HashMap::new()),
            configs: RwLock::new(HashMap::new()),
            proxies: AsyncMutex::new(HashMap::new()),
            live: RwLock::new(HashMap::new()),
            notices_tx,
            otlp: RwLock::new(None),
        })
    }

    pub async fn start_otlp(self: &Arc<Self>) -> String {
        let existing = self
            .otlp
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|r| r.endpoint()));
        if let Some(ep) = existing.filter(|s| !s.is_empty()) {
            return ep;
        }
        let sink: Arc<dyn OtlpSink> = Arc::clone(self) as Arc<dyn OtlpSink>;
        let rx = Arc::new(OtlpReceiver::new(sink));
        rx.start().await;
        let ep = rx.endpoint();
        if let Ok(mut g) = self.otlp.write() {
            *g = Some(rx);
        }
        ep
    }

    pub fn otlp_endpoint(&self) -> String {
        self.otlp
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|r| r.endpoint()))
            .unwrap_or_default()
    }

    pub fn subscribe_notices(&self) -> broadcast::Receiver<TelemetryErrorNotice> {
        self.notices_tx.subscribe()
    }

    pub fn register_workspace(self: &Arc<Self>, workspace: TelemetryWorkspace) {
        if workspace.id.is_empty() || workspace.path.is_empty() {
            return;
        }
        let cfg = TelemetryWorkspaceConfig::load(&workspace.path);
        let proxy_cfg = cfg.proxy.clone();
        if let Ok(mut w) = self.workspaces.write() {
            w.insert(workspace.id.clone(), workspace.clone());
        }
        if let Ok(mut c) = self.configs.write() {
            c.insert(workspace.id.clone(), cfg);
        }
        if let Some(pcfg) = proxy_cfg {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let this = Arc::clone(self);
                handle.spawn(async move {
                    this.ensure_proxy(workspace, pcfg).await;
                });
            }
        }
    }

    pub async fn ensure_proxy(self: &Arc<Self>, ws: TelemetryWorkspace, cfg: TelemetryProxyConfig) {
        {
            let guard = self.proxies.lock().await;
            if guard.contains_key(&ws.id) {
                return;
            }
        }
        let cmd = format!("proxy :{} → {}", cfg.listen, cfg.upstream);
        let Ok(Some(session)) = self
            .open_run_internal(
                &ws.path,
                &cmd,
                TelemetryRunSource::Wrapper,
                Some("proxy".to_string()),
                None,
                None,
                None,
            )
            .await
        else {
            return;
        };
        let sink_session = Arc::clone(&session);
        let sink: ProxySink = Arc::new(move |evs| {
            sink_session.add_events(evs);
        });
        let proxy = Arc::new(HttpTelemetryProxy::new(
            cfg,
            session.run_id().to_string(),
            sink,
        ));
        if proxy.start().await {
            self.proxies.lock().await.insert(ws.id, proxy);
        } else {
            let _ = session.close(Some(1)).await;
        }
    }

    pub async fn replay(
        &self,
        workspace_id: &str,
        since_ms: i64,
        until_ms: Option<i64>,
    ) -> Result<Option<usize>, TelemetryStoreError> {
        let proxy = {
            let guard = self.proxies.lock().await;
            guard.get(workspace_id).cloned()
        };
        let Some(proxy) = proxy else {
            return Ok(None);
        };
        let store = self.registry.for_workspace(workspace_id).await?;
        let events = store
            .events(crate::model::TelemetryQuery {
                run_id: Some(proxy.run_id().to_string()),
                since_ms: Some(since_ms),
                until_ms,
                limit: 2000,
                include_ignored: true,
                ..Default::default()
            })
            .await?;
        Ok(Some(proxy.replay(&events).await))
    }

    /// Workspace cuja root contém `cwd`; empate → root mais específica.
    pub fn resolve(&self, cwd: &str) -> Option<(TelemetryWorkspace, String)> {
        if cwd.is_empty() {
            return None;
        }
        let ncwd = normalize_path_str(cwd);
        let guard = self.workspaces.read().ok()?;
        let mut best: Option<(TelemetryWorkspace, String)> = None;
        for ws in guard.values() {
            let roots: Vec<&str> = if ws.roots.is_empty() {
                vec![ws.path.as_str()]
            } else {
                ws.roots.iter().map(String::as_str).collect()
            };
            for r in roots {
                let nr = normalize_path_str(r);
                if nr == ncwd || is_within_path(&nr, &ncwd) {
                    let take = best.as_ref().is_none_or(|(_, br)| nr.len() > br.len());
                    if take {
                        best = Some((ws.clone(), nr));
                    }
                }
            }
        }
        let (ws, best_root) = best?;
        let root_count = if ws.roots.is_empty() {
            1
        } else {
            ws.roots.len()
        };
        let project = if root_count <= 1 {
            ws.name.clone()
        } else {
            Path::new(&best_root)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| ws.name.clone())
        };
        Some((ws, project))
    }

    pub fn session(&self, run_id: &str) -> Option<Arc<TelemetryIngestSession>> {
        self.live.read().ok()?.get(run_id).cloned()
    }

    pub async fn open_wrapper_run(
        self: &Arc<Self>,
        cwd: &str,
        command: &str,
        name: Option<String>,
        pane_id: Option<String>,
        pid: Option<i64>,
    ) -> Result<Option<Arc<TelemetryIngestSession>>, TelemetryStoreError> {
        self.open_run_internal(
            cwd,
            command,
            TelemetryRunSource::Wrapper,
            name,
            None,
            pane_id,
            pid,
        )
        .await
    }

    pub async fn open_task_run(
        self: &Arc<Self>,
        def: &TelemetryTaskDef,
        command: &str,
        pid: Option<i64>,
        pane_id: Option<String>,
    ) -> Result<Option<Arc<TelemetryIngestSession>>, TelemetryStoreError> {
        if !def.telemetry_enabled || def.cwd.is_empty() {
            return Ok(None);
        }
        self.open_run_internal(
            &def.cwd,
            command,
            TelemetryRunSource::Task,
            Some(def.label.clone()),
            Some(def.id.clone()),
            pane_id,
            pid,
        )
        .await
    }

    pub async fn open_app_run(
        self: &Arc<Self>,
        version: &str,
    ) -> Result<Arc<TelemetryIngestSession>, TelemetryStoreError> {
        let store = self
            .registry
            .for_workspace(K_TELEMETRY_APP_WORKSPACE_ID)
            .await?;
        let pid = i64::from(std::process::id());
        let run = store
            .open_run(
                "app:gsh",
                "GSH",
                TelemetryRunSource::App,
                "",
                format!("GSH {version}"),
                Some("Clássico".to_string()),
                None,
                None,
                Some(pid),
                None,
            )
            .await?;
        let run_id = run.id.clone();
        let config = TelemetryParserConfig::default();
        let otlp = self.otlp_endpoint();
        let live_ref = Arc::downgrade(self);
        let session = TelemetryIngestSession::new(
            store,
            run,
            config,
            Vec::new(),
            otlp,
            "GSH".to_string(),
            Box::new(move |id| {
                if let Some(this) = live_ref.upgrade() {
                    if let Ok(mut g) = this.live.write() {
                        g.remove(id);
                    }
                }
            }),
            Box::new(|_| {}),
        );
        if let Ok(mut g) = self.live.write() {
            g.insert(run_id, Arc::clone(&session));
        }
        Ok(session)
    }

    #[allow(clippy::too_many_arguments)]
    async fn open_run_internal(
        self: &Arc<Self>,
        cwd: &str,
        command: &str,
        source: TelemetryRunSource,
        name: Option<String>,
        task_key: Option<String>,
        pane_id: Option<String>,
        pid: Option<i64>,
    ) -> Result<Option<Arc<TelemetryIngestSession>>, TelemetryStoreError> {
        if cwd.is_empty() {
            return Ok(None);
        }
        let Some((ws, project)) = self.resolve(cwd) else {
            return Ok(None);
        };
        let store = self.registry.for_workspace(&ws.id).await?;
        let key = TelemetryRun::key_for(cwd, command, name.as_deref());
        let run = store
            .open_run(
                key,
                project.clone(),
                source,
                cwd,
                command,
                name,
                task_key,
                pane_id.clone(),
                pid,
                None,
            )
            .await?;

        let roots = if ws.roots.is_empty() {
            vec![ws.path.clone()]
        } else {
            ws.roots.clone()
        };
        let ws_cfg = self
            .configs
            .read()
            .ok()
            .and_then(|g| g.get(&ws.id).cloned())
            .unwrap_or_default();
        let config = TelemetryParserConfig {
            unwrap: ws_cfg.unwrap,
            project_roots: roots,
            project_frames: ws_cfg.project_frames,
        };
        let run_id = run.id.clone();
        let ws_id = ws.id.clone();
        let proj_c = project.clone();
        let pane_c = pane_id.clone();
        let notices_tx = self.notices_tx.clone();
        let live_ref = Arc::downgrade(self);
        let session = TelemetryIngestSession::new(
            store,
            run,
            config,
            ws_cfg.ignore,
            self.otlp_endpoint(),
            project,
            Box::new(move |id| {
                if let Some(this) = live_ref.upgrade() {
                    if let Ok(mut g) = this.live.write() {
                        g.remove(id);
                    }
                }
            }),
            Box::new(move |fps| {
                let _ = notices_tx.send(TelemetryErrorNotice {
                    workspace_id: ws_id.clone(),
                    run_id: fps.0,
                    project: proj_c.clone(),
                    pane_id: pane_c.clone(),
                    fingerprints: fps.1,
                });
            }),
        );
        if let Ok(mut g) = self.live.write() {
            g.insert(run_id, Arc::clone(&session));
        }
        Ok(Some(session))
    }
}

impl OtlpSink for TelemetryIngest {
    fn run_for(&self, run_id: Option<&str>, service_name: Option<&str>) -> Option<String> {
        let guard = self.live.read().ok()?;
        if let Some(id) = run_id {
            if guard.contains_key(id) {
                return Some(id.to_string());
            }
        }
        let svc = service_name?;
        let name_key = format!("name:{svc}");
        for s in guard.values() {
            if s.run.name.as_deref() == Some(svc) || s.run.key == name_key {
                return Some(s.run_id().to_string());
            }
        }
        None
    }

    fn add_events(&self, run_id: &str, events: Vec<TelemetryEvent>) {
        if let Some(s) = self.session(run_id) {
            s.add_events(events);
        }
    }

    fn project_roots_of(&self, run_id: &str) -> Vec<String> {
        self.session(run_id)
            .map(|s| s.config.project_roots.clone())
            .unwrap_or_default()
    }
}

type OnClosedFn = Box<dyn Fn(&str) + Send + Sync>;
type OnErrorsFn = Box<dyn Fn((String, Vec<String>)) + Send + Sync>;

struct SessionState {
    partial: HashMap<TelemetryStream, String>,
    pending: Vec<TelemetryEvent>,
    parser: TelemetryLineParser,
    lines: i64,
    closed: bool,
    vm_tried: bool,
    vm: Option<VmServiceAttachment>,
    flush_handle: Option<JoinHandle<()>>,
    recent_fps: HashMap<String, i64>,
}

pub struct TelemetryIngestSession {
    store: TelemetryStore,
    run: TelemetryRun,
    config: TelemetryParserConfig,
    ignore: Vec<FancyRegex>,
    otlp_endpoint: String,
    project: String,
    on_closed: OnClosedFn,
    on_errors: OnErrorsFn,
    state: Mutex<SessionState>,
    write_lock: AsyncMutex<()>,
}

impl TelemetryIngestSession {
    #[allow(clippy::too_many_arguments)]
    fn new(
        store: TelemetryStore,
        run: TelemetryRun,
        config: TelemetryParserConfig,
        ignore: Vec<FancyRegex>,
        otlp_endpoint: String,
        project: String,
        on_closed: OnClosedFn,
        on_errors: OnErrorsFn,
    ) -> Arc<Self> {
        let parser = TelemetryLineParser::new(&run.id, config.clone());
        Arc::new(Self {
            store,
            run,
            config,
            ignore,
            otlp_endpoint,
            project,
            on_closed,
            on_errors,
            state: Mutex::new(SessionState {
                partial: HashMap::new(),
                pending: Vec::new(),
                parser,
                lines: 0,
                closed: false,
                vm_tried: false,
                vm: None,
                flush_handle: None,
                recent_fps: HashMap::new(),
            }),
            write_lock: AsyncMutex::new(()),
        })
    }

    pub fn run_id(&self) -> &str {
        &self.run.id
    }

    pub fn child_environment(&self) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        env.insert("GSH_RUN_ID".to_string(), self.run.id.clone());
        let svc = self
            .run
            .name
            .as_deref()
            .unwrap_or(&self.run.command)
            .to_string();
        env.insert("OTEL_SERVICE_NAME".to_string(), svc);
        env.insert(
            "OTEL_RESOURCE_ATTRIBUTES".to_string(),
            format!("gsh.run={},gsh.project={}", self.run.id, self.project),
        );
        if !self.otlp_endpoint.is_empty() {
            env.insert(
                "OTEL_EXPORTER_OTLP_ENDPOINT".to_string(),
                self.otlp_endpoint.clone(),
            );
        }
        env
    }

    pub fn add(self: &Arc<Self>, chunk: &str, stream: TelemetryStream) {
        if chunk.is_empty() {
            return;
        }
        let mut vm_uri: Option<String> = None;
        let should_flush_now;
        let should_schedule_timer;
        {
            let Ok(mut st) = self.state.lock() else {
                return;
            };
            if st.closed {
                return;
            }
            let buf = st.partial.entry(stream).or_default();
            buf.push_str(chunk);
            let text = std::mem::take(buf);
            let now = now_ms();
            let mut start = 0usize;
            let bytes = text.as_bytes();
            let mut lines_to_feed = Vec::new();
            for (i, &b) in bytes.iter().enumerate() {
                if b == b'\n' {
                    let mut end = i;
                    if end > start && bytes[end - 1] == b'\r' {
                        end -= 1;
                    }
                    let line = text[start..end].to_string();
                    lines_to_feed.push(line);
                    start = i + 1;
                }
            }
            if let Some(b) = st.partial.get_mut(&stream) {
                b.push_str(&text[start..]);
            }

            for line in lines_to_feed {
                let offset = st.lines;
                st.lines += 1;
                let evs = st.parser.feed(&line, stream, now, Some(offset));
                st.pending.extend(evs);
                if !st.vm_tried && vm_uri.is_none() {
                    if let Some(m) = VM_SERVICE_URI_PATTERN.captures(&line) {
                        if let Some(u) = m.get(1) {
                            st.vm_tried = true;
                            vm_uri = Some(u.as_str().trim().to_string());
                        }
                    }
                }
            }

            should_flush_now = st.pending.len() >= BATCH_LINES;
            should_schedule_timer =
                !should_flush_now && st.flush_handle.is_none() && !st.pending.is_empty();
        }

        if let Some(uri) = vm_uri {
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                let this = Arc::clone(self);
                rt.spawn(async move {
                    this.attach_vm(&uri).await;
                });
            }
        }

        if should_flush_now {
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                let this = Arc::clone(self);
                rt.spawn(async move {
                    let _ = this.flush_pending().await;
                });
            }
        } else if should_schedule_timer {
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                let this = Arc::clone(self);
                let handle = rt.spawn(async move {
                    tokio::time::sleep(BATCH_DELAY).await;
                    let _ = this.flush_pending().await;
                });
                if let Ok(mut st) = self.state.lock() {
                    st.flush_handle = Some(handle);
                }
            }
        }
    }

    pub fn add_events(self: &Arc<Self>, events: Vec<TelemetryEvent>) {
        if events.is_empty() {
            return;
        }
        let should_flush_now;
        let should_schedule_timer;
        {
            let Ok(mut st) = self.state.lock() else {
                return;
            };
            if st.closed {
                return;
            }
            st.pending.extend(events);
            should_flush_now = st.pending.len() >= BATCH_LINES;
            should_schedule_timer = !should_flush_now && st.flush_handle.is_none();
        }
        if should_flush_now {
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                let this = Arc::clone(self);
                rt.spawn(async move {
                    let _ = this.flush_pending().await;
                });
            }
        } else if should_schedule_timer {
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                let this = Arc::clone(self);
                let handle = rt.spawn(async move {
                    tokio::time::sleep(BATCH_DELAY).await;
                    let _ = this.flush_pending().await;
                });
                if let Ok(mut st) = self.state.lock() {
                    st.flush_handle = Some(handle);
                }
            }
        }
    }

    async fn attach_vm(self: &Arc<Self>, uri: &str) {
        let this_sink = Arc::clone(self);
        let sink: VmSink = Arc::new(move |evs| {
            this_sink.add_events(evs);
        });
        let cfg = self.config.clone();
        if let Some(mut att) =
            VmServiceAttachment::connect(uri, self.run.id.clone(), sink, cfg).await
        {
            let mut drop_now = false;
            if let Ok(mut st) = self.state.lock() {
                if st.closed {
                    drop_now = true;
                } else {
                    st.vm = Some(att);
                    return;
                }
            }
            if drop_now {
                att.close();
            }
        }
    }

    pub async fn flush_pending(&self) -> Result<(), TelemetryStoreError> {
        let batch = {
            let Ok(mut st) = self.state.lock() else {
                return Ok(());
            };
            if let Some(h) = st.flush_handle.take() {
                h.abort();
            }
            if st.pending.is_empty() {
                return Ok(());
            }
            let raw = std::mem::take(&mut st.pending);
            dedup_batch(&self.ignore, &mut st.recent_fps, raw)
        };
        if batch.is_empty() {
            return Ok(());
        }
        let _guard = self.write_lock.lock().await;
        let saved = self.store.append(batch).await?;
        let mut fps_set = HashSet::new();
        for ev in &saved {
            if ev.severity.index() >= TelemetrySeverity::Error.index() {
                if let Some(ref fp) = ev.fingerprint {
                    fps_set.insert(fp.clone());
                }
            }
        }
        if !fps_set.is_empty() {
            let mut fps: Vec<String> = fps_set.into_iter().collect();
            fps.sort();
            (self.on_errors)((self.run.id.clone(), fps));
        }
        Ok(())
    }

    pub async fn close(&self, exit_code: Option<i32>) -> Result<(), TelemetryStoreError> {
        {
            let Ok(mut st) = self.state.lock() else {
                return Ok(());
            };
            if st.closed {
                return Ok(());
            }
            st.closed = true;
            if let Some(h) = st.flush_handle.take() {
                h.abort();
            }
            let now = now_ms();
            let partials: Vec<(TelemetryStream, String)> = st
                .partial
                .drain()
                .filter(|(_, s)| !s.trim().is_empty())
                .collect();
            for (stream, rest) in partials {
                let offset = st.lines;
                st.lines += 1;
                let evs = st.parser.feed(&rest, stream, now, Some(offset));
                st.pending.extend(evs);
            }
            let flushed = st.parser.flush();
            st.pending.extend(flushed);
            if let Some(mut vm) = st.vm.take() {
                vm.close();
            }
        }
        let _ = self.flush_pending().await;
        let _guard = self.write_lock.lock().await;
        self.store.close_run(&self.run.id, exit_code, None).await?;
        (self.on_closed)(&self.run.id);
        Ok(())
    }
}

fn dedup_batch(
    ignore: &[FancyRegex],
    recent_fps: &mut HashMap<String, i64>,
    batch: Vec<TelemetryEvent>,
) -> Vec<TelemetryEvent> {
    let now = now_ms();
    recent_fps.retain(|_, &mut t| now.saturating_sub(t) <= DEDUP_WINDOW_MS * 4);
    let mut out = Vec::with_capacity(batch.len());
    for mut ev in batch {
        if !ignore.is_empty() {
            let text = match ev.error {
                Some(ref err) => format!("{}: {}", err.err_type, err.message),
                None => ev.body.clone(),
            };
            if ignore.iter().any(|re| re.is_match(&text).unwrap_or(false)) {
                continue;
            }
        }
        let fp = ev
            .fingerprint
            .clone()
            .or_else(|| ev.error.as_ref().map(fingerprint_of));
        if let Some(ref fp_str) = fp {
            if let Some(&seen) = recent_fps.get(fp_str) {
                if (ev.ts_ms - seen).abs() <= DEDUP_WINDOW_MS {
                    continue;
                }
            }
            recent_fps.insert(fp_str.clone(), ev.ts_ms);
            if ev.fingerprint.is_none() {
                ev.fingerprint = fp;
            }
        }
        out.push(ev);
    }
    out
}
