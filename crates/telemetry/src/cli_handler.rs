use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use regex::Regex;
use serde_json::{json, Map, Value};

use crate::ingest::TelemetryIngest;
use crate::model::{
    format_iso_utc, now_ms, parse_iso_ms, TelemetryCase, TelemetryEvent, TelemetryQuery,
    TelemetryRun, TelemetrySeverity, TelemetryStream, TelemetryTriageActor, TelemetryTriageStatus,
    K_TELEMETRY_APP_WORKSPACE_ID,
};
use crate::registry::TelemetryStoreRegistry;
use crate::store::TelemetryStore;

/// Teto de bytes de uma resposta (o agente nunca recebe 2000 linhas).
pub const K_TELEMETRY_REPLY_CAP: usize = 16 * 1024;

static DURATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+)\s*(ms|s|m|h|d)?$").expect("duration regex"));
static HUNK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)").expect("hunk regex"));
static PROBE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"["']probe["']\s*:"#).expect("probe regex"));

pub type LastEditAtFn = Arc<dyn Fn(&str) -> Option<i64> + Send + Sync>;
pub type RootsOfFn = Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>;

pub struct TelemetryCliHandler {
    registry: Arc<TelemetryStoreRegistry>,
    ingest: Arc<TelemetryIngest>,
    last_edit_at: LastEditAtFn,
    roots_of: RootsOfFn,
}

impl TelemetryCliHandler {
    pub fn new(
        registry: Arc<TelemetryStoreRegistry>,
        ingest: Arc<TelemetryIngest>,
        last_edit_at: LastEditAtFn,
        roots_of: RootsOfFn,
    ) -> Self {
        Self {
            registry,
            ingest,
            last_edit_at,
            roots_of,
        }
    }

    pub fn handles(cmd: &str) -> bool {
        cmd.starts_with("telemetry-")
    }

    /// Trata um comando `telemetry-*` e devolve o envelope JSON `{"ok":true,"data":...}`
    /// ou `{"ok":false,"error":...}` igual ao protocolo de `gsh_ipc`.
    pub async fn handle(
        &self,
        cmd: &str,
        tab_id: Option<&str>,
        args: &Map<String, Value>,
        workspace_id: Option<&str>,
        workspace_root: &str,
    ) -> Value {
        if workspace_id.is_none()
            && matches!(
                cmd,
                "telemetry-open" | "telemetry-close" | "telemetry-probes"
            )
        {
            return fail("this verb needs a workspace (drop --app)");
        }

        match cmd {
            "telemetry-open" => {
                let cwd = opt_str(args.get("cwd")).unwrap_or_else(|| workspace_root.to_string());
                let command = opt_str(args.get("command")).unwrap_or_default();
                let name = opt_str(args.get("name"));
                let pid = opt_i64(args.get("pid"));
                match self
                    .ingest
                    .open_wrapper_run(&cwd, &command, name, tab_id.map(ToOwned::to_owned), pid)
                    .await
                {
                    Ok(Some(s)) => ok(json!({
                        "run": s.run_id(),
                        "env": s.child_environment(),
                    })),
                    Ok(None) => fail("no_workspace: cwd is not inside an open workspace"),
                    Err(e) => fail(&e.to_string()),
                }
            }
            "telemetry-ingest" => {
                let run_id = opt_str(args.get("run")).unwrap_or_default();
                let Some(s) = self.ingest.session(&run_id) else {
                    return fail("no_run: unknown or closed run");
                };
                let stream = if opt_str(args.get("stream")).as_deref() == Some("err") {
                    TelemetryStream::Err
                } else {
                    TelemetryStream::Out
                };
                if let Some(Value::Array(lines)) = args.get("lines") {
                    let mut joined: String =
                        lines.iter().map(val_to_str).collect::<Vec<_>>().join("\n");
                    if !lines.is_empty() {
                        joined.push('\n');
                    }
                    s.add(&joined, stream);
                } else if let Some(Value::String(b64)) = args.get("data") {
                    if let Ok(bytes) = B64.decode(b64) {
                        let text = String::from_utf8_lossy(&bytes);
                        s.add(&text, stream);
                    }
                }
                ok_empty()
            }
            "telemetry-close" => {
                let run_id = opt_str(args.get("run")).unwrap_or_default();
                let Some(s) = self.ingest.session(&run_id) else {
                    return fail("no_run: unknown or closed run");
                };
                let exit_code = opt_i64(args.get("exit_code")).and_then(|v| i32::try_from(v).ok());
                if let Err(e) = s.close(exit_code).await {
                    return fail(&e.to_string());
                }
                let ws_id = workspace_id.unwrap_or(K_TELEMETRY_APP_WORKSPACE_ID);
                let store = match self.registry.for_workspace(ws_id).await {
                    Ok(st) => st,
                    Err(e) => return fail(&e.to_string()),
                };
                let cases = match store
                    .cases(TelemetryQuery {
                        run_id: Some(s.run_id().to_string()),
                        limit: 500,
                        ..Default::default()
                    })
                    .await
                {
                    Ok(c) => c,
                    Err(e) => return fail(&e.to_string()),
                };
                let errors = cases
                    .iter()
                    .filter(|x| x.severity.index() >= TelemetrySeverity::Error.index())
                    .count();
                let warns = cases.len().saturating_sub(errors);
                ok(json!({
                    "run": s.run_id(),
                    "errors": errors,
                    "warnings": warns,
                }))
            }
            _ => {
                let ws_id = workspace_id.unwrap_or(K_TELEMETRY_APP_WORKSPACE_ID);
                let store = match self.registry.for_workspace(ws_id).await {
                    Ok(st) => st,
                    Err(e) => return fail(&e.to_string()),
                };
                self.handle_store_cmd(cmd, args, workspace_id, &store).await
            }
        }
    }

    async fn handle_store_cmd(
        &self,
        cmd: &str,
        args: &Map<String, Value>,
        workspace_id: Option<&str>,
        store: &TelemetryStore,
    ) -> Value {
        match cmd {
            "telemetry-perf" => self.perf(store, args).await,
            "telemetry-errors" => {
                let min_sev = opt_sev(args.get("level")).or(Some(TelemetrySeverity::Error));
                let q = self
                    .build_query(args, workspace_id, store, min_sev, 50)
                    .await;
                match store.cases(q).await {
                    Ok(cases) => {
                        let items: Vec<Value> = cases.iter().map(case_summary).collect();
                        capped("cases", items, "gsh telemetry show <id>")
                    }
                    Err(e) => fail(&e.to_string()),
                }
            }
            "telemetry-logs" | "telemetry-events" => {
                let min_sev = opt_sev(args.get("level"))
                    .or_else(|| (cmd == "telemetry-logs").then_some(TelemetrySeverity::Debug));
                let q = self
                    .build_query(args, workspace_id, store, min_sev, 100)
                    .await;
                match store.events(q).await {
                    Ok(evs) => {
                        let items: Vec<Value> = evs.iter().map(event_summary).collect();
                        capped("events", items, "narrow with --since/--run/--level")
                    }
                    Err(e) => fail(&e.to_string()),
                }
            }
            "telemetry-runs" => {
                let project = opt_str(args.get("project"));
                let key = opt_str(args.get("key"));
                let only_live = opt_bool(args.get("live"));
                let limit = opt_usize(args.get("limit")).unwrap_or(20);
                match store.runs(project, key, only_live, limit).await {
                    Ok(runs) => ok(json!({
                        "runs": runs.iter().map(run_summary).collect::<Vec<_>>(),
                    })),
                    Err(e) => fail(&e.to_string()),
                }
            }
            "telemetry-show" => {
                let id = opt_str(args.get("id")).unwrap_or_default();
                let ctx = opt_usize(args.get("context")).unwrap_or(12);
                self.show(store, &id, ctx).await
            }
            "telemetry-wait" => self.wait(store, args).await,
            "telemetry-mark" => {
                let Some(name) = opt_str(args.get("name")) else {
                    return fail("missing mark name");
                };
                if opt_str(args.get("action")).as_deref() == Some("end") {
                    if let Err(e) = store.mark_end(&name, None).await {
                        return fail(&e.to_string());
                    }
                } else if let Err(e) = store.mark_start(&name, None).await {
                    return fail(&e.to_string());
                }
                match store.mark(&name).await {
                    Ok(m) => ok(json!({
                        "mark": name,
                        "start": m.as_ref().map(|x| format_iso_utc(x.0)),
                        "end": m.and_then(|x| x.1).map(format_iso_utc),
                    })),
                    Err(e) => fail(&e.to_string()),
                }
            }
            "telemetry-triage" => {
                let id = opt_str(args.get("id")).unwrap_or_default();
                let cs = match store.case_by_id(&id).await {
                    Ok(Some(c)) => c,
                    Ok(None) => return fail("not_found: no case with that id"),
                    Err(e) => return fail(&e.to_string()),
                };
                let status = match opt_str(args.get("status")).as_deref() {
                    Some("resolve" | "resolved") => TelemetryTriageStatus::Resolved,
                    Some("ignore" | "ignored") => TelemetryTriageStatus::Ignored,
                    _ => TelemetryTriageStatus::Open,
                };
                let reason = opt_str(args.get("reason"));
                if let Err(e) = store
                    .triage(&cs.fingerprint, status, TelemetryTriageActor::Agent, reason)
                    .await
                {
                    return fail(&e.to_string());
                }
                ok(json!({
                    "id": cs.short_id(),
                    "status": status.as_str(),
                }))
            }
            "telemetry-clear" => {
                let fp = if let Some(case_id) = opt_str(args.get("case")) {
                    match store.case_by_id(&case_id).await {
                        Ok(Some(cs)) => Some(cs.fingerprint),
                        Ok(None) => return fail("not_found: no case with that id"),
                        Err(e) => return fail(&e.to_string()),
                    }
                } else {
                    None
                };
                if let Err(e) = store
                    .clear(fp, opt_str(args.get("run")), opt_str(args.get("project")))
                    .await
                {
                    return fail(&e.to_string());
                }
                ok(json!({"cleared": true}))
            }
            "telemetry-probes" => {
                let ws_id = workspace_id.unwrap_or("");
                self.probes(ws_id, opt_str(args.get("project")).as_deref())
                    .await
            }
            "telemetry-replay" => {
                let (since, until) = if let Some(mark) = opt_str(args.get("mark")) {
                    match store.mark(&mark).await {
                        Ok(Some((s, u))) => (Some(s), u),
                        Ok(None) => return fail("not_found: no such mark"),
                        Err(e) => return fail(&e.to_string()),
                    }
                } else if let Some(s_raw) = opt_str(args.get("since")) {
                    (parse_since(&s_raw), None)
                } else {
                    (None, None)
                };
                let Some(since_ms) = since else {
                    return fail("missing --mark <name> or --since <10m>");
                };
                let ws_id = workspace_id.unwrap_or(K_TELEMETRY_APP_WORKSPACE_ID);
                match self.ingest.replay(ws_id, since_ms, until).await {
                    Ok(Some(n)) => ok(json!({"replayed": n})),
                    Ok(None) => fail("no_proxy: add \"proxy\" to .gsh/telemetry.json first"),
                    Err(e) => fail(&e.to_string()),
                }
            }
            other => fail(&format!("unknown telemetry command {other}")),
        }
    }

    async fn build_query(
        &self,
        a: &Map<String, Value>,
        workspace_id: Option<&str>,
        store: &TelemetryStore,
        min_severity: Option<TelemetrySeverity>,
        default_limit: usize,
    ) -> TelemetryQuery {
        let mut since_ms: Option<i64> = opt_str(a.get("since")).and_then(|s| parse_since(&s));
        let mut until_ms: Option<i64> = None;
        let mut run_id = opt_str(a.get("run"));

        if a.get("since_edit") == Some(&Value::Bool(true)) {
            since_ms = Some(match workspace_id {
                Some(ws) => (self.last_edit_at)(ws).unwrap_or_else(now_ms),
                None => now_ms(),
            });
        }
        if a.get("since_run") == Some(&Value::Bool(true)) {
            if let Ok(runs) = store
                .runs(opt_str(a.get("project")), opt_str(a.get("key")), false, 1)
                .await
            {
                if let Some(first) = runs.first() {
                    since_ms = Some(first.started_at_ms);
                }
            }
        }
        if let Some(before) = opt_str(a.get("before")) {
            let ev = store.event(&before).await.ok().flatten();
            let anchor = match ev {
                Some(e) => Some(e),
                None => match store.case_by_id(&before).await.ok().flatten() {
                    Some(cs) => match cs.last_event_id {
                        Some(id) => store.event(&id).await.ok().flatten(),
                        None => None,
                    },
                    None => None,
                },
            };
            if let Some(anchor) = anchor {
                let win_ms = opt_str(a.get("window"))
                    .and_then(|w| parse_duration_ms(&w))
                    .unwrap_or(5_000);
                until_ms = Some(anchor.ts_ms);
                since_ms = Some(anchor.ts_ms - win_ms);
                if run_id.is_none() {
                    run_id = Some(anchor.run_id);
                }
            }
        }
        if let Some(mark) = opt_str(a.get("mark")) {
            if let Ok(Some((s, u))) = store.mark(&mark).await {
                since_ms = Some(s);
                until_ms = u;
            }
        }

        TelemetryQuery {
            project: opt_str(a.get("project")),
            run_id,
            run_key: opt_str(a.get("key")),
            task_key: opt_str(a.get("task")),
            pane_id: opt_str(a.get("tab")),
            min_severity,
            since_ms,
            until_ms,
            text: opt_str(a.get("text")),
            probe: opt_str(a.get("probe")),
            only_new: a.get("new") == Some(&Value::Bool(true)),
            include_ignored: a.get("include_ignored") == Some(&Value::Bool(true)),
            include_resolved: a.get("include_resolved") == Some(&Value::Bool(true)),
            only_errors: false,
            limit: opt_usize(a.get("limit")).unwrap_or(default_limit),
        }
    }

    async fn show(&self, store: &TelemetryStore, id: &str, context: usize) -> Value {
        if id.is_empty() {
            return fail("missing id");
        }
        if id.starts_with("r_") {
            let r = match store.run(id).await {
                Ok(Some(r)) => r,
                Ok(None) => return fail("not_found: no run with that id"),
                Err(e) => return fail(&e.to_string()),
            };
            let cases = store
                .cases(TelemetryQuery {
                    run_id: Some(id.to_string()),
                    min_severity: Some(TelemetrySeverity::Warn),
                    limit: 50,
                    ..Default::default()
                })
                .await
                .unwrap_or_default();
            return ok(json!({
                "run": run_summary(&r),
                "cases": cases.iter().map(case_summary).collect::<Vec<_>>(),
            }));
        }
        if id.starts_with("ev_") {
            let e = match store.event(id).await {
                Ok(Some(e)) => e,
                Ok(None) => return fail("not_found: no event with that id"),
                Err(e) => return fail(&e.to_string()),
            };
            let ctx = store.context(id, context, 4).await.unwrap_or_default();
            return ok(json!({
                "event": event_detail(&e),
                "context": ctx.iter().map(event_summary).collect::<Vec<_>>(),
            }));
        }
        let cs = match store.case_by_id(id).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                return fail("not_found: no case with that id (try `gsh telemetry errors`)")
            }
            Err(e) => return fail(&e.to_string()),
        };
        let last = match cs.last_event_id.as_deref() {
            Some(eid) => store.event(eid).await.ok().flatten(),
            None => None,
        };
        let ctx = match last.as_ref().and_then(|e| e.id.as_deref()) {
            Some(eid) => store.context(eid, context, 3).await.unwrap_or_default(),
            None => Vec::new(),
        };
        let correlated = last.as_ref().and_then(|l| {
            ctx.iter().rev().find(|x| {
                x.id != l.id && x.ts_ms <= l.ts_ms && !x.attrs.is_empty() && x.error.is_none()
            })
        });
        let triage_val = cs.triage.as_ref().map(|t| {
            json!({
                "status": t.status.as_str(),
                "by": t.by.as_str(),
                "at": format_iso_utc(t.at_ms),
                "reason": t.reason,
            })
        });
        let runs_val: Vec<Value> = cs
            .runs
            .iter()
            .map(|r| {
                json!({
                    "run": r.run_id,
                    "count": r.count,
                    "first": format_iso_utc(r.first_at_ms),
                    "last": format_iso_utc(r.last_at_ms),
                    "live": r.live,
                })
            })
            .collect();
        ok(json!({
            "case": case_summary(&cs),
            "triage": triage_val,
            "runs": runs_val,
            "last": last.as_ref().map(event_detail),
            "correlated": correlated.map(event_summary),
            "context": ctx.iter().map(event_summary).collect::<Vec<_>>(),
        }))
    }

    async fn perf(&self, store: &TelemetryStore, a: &Map<String, Value>) -> Value {
        let run_id = match opt_str(a.get("run")) {
            Some(r) => r,
            None => match store.runs(None, None, false, 1).await {
                Ok(runs) if !runs.is_empty() => runs[0].id.clone(),
                Ok(_) => return fail("no runs in this store (start the app with GSH_PERF=1)"),
                Err(e) => return fail(&e.to_string()),
            },
        };
        let wanted = opt_str(a.get("metric"));
        let events = match store
            .events(TelemetryQuery {
                run_id: Some(run_id.clone()),
                limit: 5000,
                include_ignored: true,
                ..Default::default()
            })
            .await
        {
            Ok(evs) => evs,
            Err(e) => return fail(&e.to_string()),
        };

        let mut samples: BTreeMap<String, BTreeMap<String, Vec<f64>>> = BTreeMap::new();
        for ev in events {
            let Some(metric) = ev.attrs.get("metric").map(val_to_str) else {
                continue;
            };
            if wanted.as_ref().is_some_and(|w| w != &metric) {
                continue;
            }
            let fields = samples.entry(metric).or_default();
            for (k, v) in ev.attrs {
                if k == "metric" {
                    continue;
                }
                if let Some(num) = v.as_f64() {
                    fields.entry(k).or_default().push(num);
                }
            }
        }

        let mut out = Map::new();
        for (metric, fields_map) in samples {
            let mut fobj = Map::new();
            for (field, vals) in fields_map {
                if vals.is_empty() {
                    continue;
                }
                let mut sorted = vals.clone();
                sorted.sort_by(|a, b| a.total_cmp(b));
                let pct = |q: f64| -> Value {
                    let idx = (((sorted.len() - 1) as f64) * q).round() as usize;
                    num_json(sorted[idx.min(sorted.len() - 1)])
                };
                fobj.insert(
                    field,
                    json!({
                        "count": sorted.len(),
                        "p50": pct(0.5),
                        "p95": pct(0.95),
                        "max": num_json(*sorted.last().unwrap_or(&0.0)),
                        "last": num_json(*vals.last().unwrap_or(&0.0)),
                    }),
                );
            }
            out.insert(metric, Value::Object(fobj));
        }
        let mut data = Map::new();
        data.insert("run".to_string(), Value::String(run_id));
        let empty = out.is_empty();
        data.insert("metrics".to_string(), Value::Object(out));
        if empty {
            data.insert(
                "hint".to_string(),
                Value::String("no metric events; run the app with GSH_PERF=1".to_string()),
            );
        }
        ok(Value::Object(data))
    }

    async fn wait(&self, store: &TelemetryStore, a: &Map<String, Value>) -> Value {
        let timeout_ms = opt_str(a.get("timeout"))
            .and_then(|s| parse_duration_ms(&s))
            .unwrap_or(60_000)
            .min(10 * 60_000);
        let absent_ms = opt_str(a.get("absent"))
            .and_then(|s| parse_duration_ms(&s))
            .unwrap_or(30_000);
        let fp_id = opt_str(a.get("fingerprint"));
        let route = opt_str(a.get("route"));
        let any_new = a.get("any_new") == Some(&Value::Bool(true));

        let start_ms = now_ms();
        let deadline_ms = start_ms + timeout_ms;
        let fp = match fp_id {
            Some(ref id) => match store.case_by_id(id).await {
                Ok(Some(cs)) => Some(cs.fingerprint),
                Ok(None) => return fail("not_found: no case with that id"),
                Err(e) => return fail(&e.to_string()),
            },
            None => None,
        };

        let baseline: HashSet<String> = if any_new {
            store
                .cases(TelemetryQuery {
                    only_new: true,
                    limit: 500,
                    ..Default::default()
                })
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|c| c.fingerprint)
                .collect()
        } else {
            HashSet::new()
        };

        let mut quiet_since: Option<i64> = None;
        while now_ms() < deadline_ms {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let recent = store
                .events(TelemetryQuery {
                    since_ms: Some(start_ms),
                    limit: 500,
                    include_ignored: true,
                    ..Default::default()
                })
                .await
                .unwrap_or_default();

            if let Some(ref target_fp) = fp {
                let hit_count = recent
                    .iter()
                    .filter(|e| e.fingerprint.as_deref() == Some(target_fp.as_str()))
                    .count();
                if hit_count > 0 {
                    return ok(json!({
                        "result": "hit",
                        "count": hit_count,
                        "elapsed_s": (now_ms() - start_ms) / 1000,
                    }));
                }
                if !recent.is_empty() {
                    let qs = *quiet_since.get_or_insert_with(now_ms);
                    if now_ms() - qs >= absent_ms {
                        return ok(json!({
                            "result": "ok",
                            "observed_events": recent.len(),
                            "elapsed_s": (now_ms() - start_ms) / 1000,
                        }));
                    }
                }
            }

            if let Some(ref rt) = route {
                let hit = recent.iter().rfind(|e| {
                    if e.stream != TelemetryStream::Proxy {
                        return false;
                    }
                    let m = e.attrs.get("method").map(val_to_str).unwrap_or_default();
                    let r = e.attrs.get("route").map(val_to_str).unwrap_or_default();
                    format!("{m} {r}").contains(rt)
                });
                if let Some(ev) = hit {
                    return ok(json!({
                        "result": "hit",
                        "status": ev.attrs.get("status").cloned().unwrap_or(Value::Null),
                        "duration_ms": ev.attrs.get("duration_ms").cloned().unwrap_or(Value::Null),
                        "request_id": ev.request_id,
                        "event": ev.id,
                    }));
                }
            }

            if any_new {
                let now_cases = store
                    .cases(TelemetryQuery {
                        only_new: true,
                        limit: 500,
                        ..Default::default()
                    })
                    .await
                    .unwrap_or_default();
                let fresh: Vec<Value> = now_cases
                    .iter()
                    .filter(|c| !baseline.contains(&c.fingerprint))
                    .map(case_summary)
                    .collect();
                if !fresh.is_empty() {
                    return ok(json!({
                        "result": "hit",
                        "cases": fresh,
                    }));
                }
            }
        }

        let res_str = if fp.is_some() && quiet_since.is_none() {
            "inconclusive"
        } else {
            "timeout"
        };
        ok(json!({
            "result": res_str,
            "elapsed_s": (now_ms() - start_ms) / 1000,
        }))
    }

    async fn probes(&self, workspace_id: &str, only_project: Option<&str>) -> Value {
        let roots = (self.roots_of)(workspace_id);
        let only = only_project.map(ToOwned::to_owned);
        let out = tokio::task::spawn_blocking(move || {
            let mut found = Vec::new();
            for root in roots {
                if let Some(ref only_p) = only {
                    if !root.ends_with(only_p.as_str()) {
                        continue;
                    }
                }
                let Ok(output) = std::process::Command::new("git")
                    .args(["diff", "HEAD", "--unified=0", "--no-color"])
                    .current_dir(&root)
                    .output()
                else {
                    continue;
                };
                if !output.status.success() {
                    continue;
                }
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut file: Option<String> = None;
                let mut line_num: i64 = 0;
                for l in stdout.lines() {
                    if let Some(rest) = l.strip_prefix("+++ b/") {
                        file = Some(rest.to_string());
                        continue;
                    }
                    if let Some(c) = HUNK_RE.captures(l) {
                        line_num = c.get(1).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
                        continue;
                    }
                    if l.starts_with('+') && !l.starts_with("+++") {
                        if PROBE_RE.is_match(l) {
                            found.push(json!({
                                "file": file,
                                "line": line_num,
                                "text": l[1..].trim(),
                                "root": root,
                            }));
                        }
                        line_num += 1;
                    } else if !l.starts_with('-') {
                        line_num += 1;
                    }
                }
            }
            found
        })
        .await
        .unwrap_or_default();

        let hint = if out.is_empty() {
            Value::Null
        } else {
            Value::String("remove before committing".to_string())
        };
        ok(json!({
            "probes": out,
            "hint": hint,
        }))
    }
}

fn ok_empty() -> Value {
    json!({"ok": true})
}

fn ok(data: Value) -> Value {
    json!({"ok": true, "data": data})
}

fn fail(msg: &str) -> Value {
    json!({"ok": false, "error": msg})
}

fn capped(key: &str, items: Vec<Value>, hint: &str) -> Value {
    let total = items.len();
    let mut list = items;
    let mut truncated = false;
    while !list.is_empty()
        && serde_json::to_string(&list).map(|s| s.len()).unwrap_or(0) > K_TELEMETRY_REPLY_CAP
    {
        let next_len = ((list.len() as f64) * 0.7).floor() as usize;
        let clamped = next_len.clamp(1, list.len().saturating_sub(1).max(1));
        if clamped >= list.len() {
            list.pop();
        } else {
            list.truncate(clamped);
        }
        truncated = true;
    }
    let mut data = Map::new();
    data.insert(key.to_string(), Value::Array(list));
    data.insert("total".to_string(), Value::from(total));
    if truncated {
        data.insert("truncated".to_string(), Value::Bool(true));
        data.insert("hint".to_string(), Value::String(hint.to_string()));
    }
    ok(Value::Object(data))
}

fn case_summary(c: &TelemetryCase) -> Value {
    let mut map = Map::new();
    map.insert("id".into(), Value::String(c.short_id()));
    map.insert(
        "level".into(),
        Value::String(c.severity.as_str().to_string()),
    );
    map.insert("type".into(), Value::String(c.err_type.clone()));
    map.insert("message".into(), Value::String(clip(&c.message, 300)));
    map.insert(
        "location".into(),
        c.location.clone().map_or(Value::Null, Value::String),
    );
    map.insert("count".into(), Value::from(c.count));
    map.insert("project".into(), Value::String(c.project.clone()));
    map.insert("key".into(), Value::String(c.run_key.clone()));
    map.insert(
        "status".into(),
        Value::String(c.status.as_str().to_string()),
    );
    if c.is_new {
        map.insert("new".into(), Value::Bool(true));
    }
    if c.is_regression {
        map.insert("regression".into(), Value::Bool(true));
    }
    map.insert("first".into(), Value::String(format_iso_utc(c.first_at_ms)));
    map.insert("last".into(), Value::String(format_iso_utc(c.last_at_ms)));
    map.insert("runs".into(), Value::from(c.runs.len()));
    map.insert(
        "last_event".into(),
        c.last_event_id.clone().map_or(Value::Null, Value::String),
    );
    Value::Object(map)
}

fn event_summary(e: &TelemetryEvent) -> Value {
    let mut map = Map::new();
    map.insert("id".into(), e.id.clone().map_or(Value::Null, Value::String));
    map.insert("ts".into(), Value::String(format_iso_utc(e.ts_ms)));
    map.insert("run".into(), Value::String(e.run_id.clone()));
    map.insert(
        "level".into(),
        Value::String(e.severity.as_str().to_string()),
    );
    map.insert("body".into(), Value::String(clip(&e.body, 400)));
    if !e.attrs.is_empty() {
        map.insert("attrs".into(), Value::Object(e.attrs.clone()));
    }
    if let Some(ref err) = e.error {
        map.insert(
            "error".into(),
            json!({
                "type": err.err_type,
                "location": err.project_frame().and_then(|f| f.location()),
            }),
        );
    }
    if e.repeat > 1 {
        map.insert("repeat".into(), Value::from(e.repeat));
    }
    Value::Object(map)
}

fn event_detail(e: &TelemetryEvent) -> Value {
    let Value::Object(mut map) = event_summary(e) else {
        return Value::Null;
    };
    map.insert(
        "stream".into(),
        Value::String(e.stream.as_str().to_string()),
    );
    map.insert(
        "line".into(),
        e.raw_line_offset.map_or(Value::Null, Value::from),
    );
    if let Some(ref tid) = e.trace_id {
        map.insert("trace_id".into(), Value::String(tid.clone()));
    }
    if let Some(ref rid) = e.request_id {
        map.insert("request_id".into(), Value::String(rid.clone()));
    }
    if let Some(ref err) = e.error {
        let frames: Vec<Value> = err
            .frames
            .iter()
            .map(|f| {
                let mut fm = Map::new();
                fm.insert("at".into(), Value::String(f.raw.trim().to_string()));
                if f.in_project {
                    fm.insert("project".into(), Value::Bool(true));
                }
                Value::Object(fm)
            })
            .collect();
        map.insert(
            "error".into(),
            json!({
                "type": err.err_type,
                "message": err.message,
                "location": err.project_frame().and_then(|f| f.location()),
                "frames": frames,
            }),
        );
    }
    Value::Object(map)
}

fn run_summary(r: &TelemetryRun) -> Value {
    json!({
        "id": r.id,
        "key": r.key,
        "project": r.project,
        "name": r.name,
        "command": r.command,
        "source": r.source.as_str(),
        "started": format_iso_utc(r.started_at_ms),
        "ended": r.ended_at_ms.map(format_iso_utc),
        "exit_code": r.exit_code,
        "live": r.is_live(),
    })
}

fn clip(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let prefix: String = s.chars().take(max_chars).collect();
        format!("{prefix}…")
    }
}

fn val_to_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn opt_str(v: Option<&Value>) -> Option<String> {
    let v = v?;
    if v.is_null() {
        return None;
    }
    let s = val_to_str(v);
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn opt_i64(v: Option<&Value>) -> Option<i64> {
    let v = v?;
    match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn opt_usize(v: Option<&Value>) -> Option<usize> {
    opt_i64(v).and_then(|n| usize::try_from(n).ok())
}

fn opt_bool(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Bool(true))) || matches!(v, Some(Value::String(s)) if s == "true")
}

fn opt_sev(v: Option<&Value>) -> Option<TelemetrySeverity> {
    v.filter(|x| !x.is_null())
        .map(|x| TelemetrySeverity::parse_value(Some(x)))
}

fn parse_duration_ms(raw: &str) -> Option<i64> {
    let caps = DURATION_RE.captures(raw.trim())?;
    let n: i64 = caps.get(1)?.as_str().parse().ok()?;
    Some(match caps.get(2).map(|m| m.as_str()) {
        Some("ms") => n,
        Some("m") => n * 60_000,
        Some("h") => n * 3_600_000,
        Some("d") => n * 86_400_000,
        _ => n * 1_000,
    })
}

fn parse_since(raw: &str) -> Option<i64> {
    if let Some(d) = parse_duration_ms(raw) {
        return Some(now_ms() - d);
    }
    parse_iso_ms(raw)
}

fn num_json(v: f64) -> Value {
    if v.fract() == 0.0 && v >= (i64::MIN as f64) && v <= (i64::MAX as f64) {
        Value::from(v as i64)
    } else {
        serde_json::Number::from_f64(v).map_or(Value::Null, Value::Number)
    }
}
