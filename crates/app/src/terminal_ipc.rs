use std::sync::mpsc::SyncSender;
use std::time::{Duration, Instant};

use base64::prelude::{Engine, BASE64_STANDARD};
use gsh_core::status::TurnStatus;
use iced::widget::pane_grid::Axis;
use serde_json::{json, Value};

use crate::{pane_id_for_term, term_id_from_pane_id, PaneTab, Shell, TermId};

const MAX_READ_LINES: u64 = 2000;
const IDLE_TIMEOUT: Duration = Duration::from_secs(44);

pub(crate) struct IdleWaiter {
    id: TermId,
    deadline: Instant,
    reply: SyncSender<Value>,
}

pub(crate) fn handles(cmd: &str) -> bool {
    matches!(
        cmd,
        "write"
            | "read-pane"
            | "rename-tab"
            | "interrupt"
            | "wait-idle"
            | "list-panes"
            | "new-tab"
            | "close-tab"
            | "db-list"
            | "db-schema"
            | "db-query"
            | "db-execute"
            | "db-run"
            | "redis-cmd"
            | "mongo-cmd"
    )
}

impl Shell {
    pub(crate) fn handle_terminal_ipc(&mut self, command: gsh_ipc::PendingCommand) {
        let request = &command.request;
        let cmd = request
            .get("cmd")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let tab_id = request.get("tabId").and_then(Value::as_str);
        let args = request.get("args").unwrap_or(&Value::Null);
        if cmd.starts_with("db-") || cmd == "redis-cmd" || cmd == "mongo-cmd" {
            self.handle_db_cli(command);
            return;
        }
        if cmd == "new-tab" {
            let result = self.new_terminal_tab(tab_id, args);
            let reply = match result {
                Ok(data) => json!({"ok": true, "data": data}),
                Err(error) => json!({"ok": false, "error": error}),
            };
            let _ = command.reply.send(reply);
            return;
        }
        if cmd == "close-tab" {
            match self.resolve_close_target(tab_id, args) {
                Ok(id) => {
                    let _ = command
                        .reply
                        .send(json!({"ok": true, "data": {"tabId": pane_id_for_term(id)}}));
                    self.close_terminal_tab(id);
                }
                Err(error) => {
                    let _ = command.reply.send(json!({"ok": false, "error": error}));
                }
            }
            return;
        }
        if cmd == "list-panes" {
            let data = self.list_terminal_tabs();
            let _ = command.reply.send(json!({"ok": true, "data": data}));
            return;
        }
        let result = self.terminal_ipc_target(tab_id, args).and_then(|id| {
            if cmd == "wait-idle" {
                return self.wait_for_terminal_idle(id, &command.reply);
            }
            let data = match cmd {
                "write" => self.write_terminal(id, args),
                "read-pane" => self.read_terminal(id, args),
                "rename-tab" => self.rename_terminal(id, args),
                "interrupt" => self.interrupt_terminal(id, args),
                _ => Err("unknown command".into()),
            }?;
            Ok(Some(data))
        });
        let reply = match result {
            Ok(Some(data)) => json!({"ok": true, "data": data}),
            Ok(None) => return,
            Err(error) => json!({"ok": false, "error": error}),
        };
        let _ = command.reply.send(reply);
    }

    fn handle_db_cli(&mut self, command: gsh_ipc::PendingCommand) {
        let args = command.request.get("args").cloned().unwrap_or(Value::Null);
        let workspace = args.get("workspace").and_then(Value::as_str);
        let caller = command
            .request
            .get("tabId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let selected = if let Some(workspace) = workspace {
            self.cli_workspace_index(Some(workspace), caller)
        } else if let Some(id) = term_id_from_pane_id(caller) {
            self.terminal_contexts
                .get(&id)
                .and_then(|context| context.workspace.as_ref())
                .and_then(|reference| {
                    self.workspaces
                        .items()
                        .iter()
                        .position(|item| match reference {
                            gsh_core::workspace::WorkspaceRef::Project(path) => {
                                item.path.as_ref() == Some(path)
                            }
                            gsh_core::workspace::WorkspaceRef::System(id) => item.id == *id,
                        })
                })
                .map(Ok)
                .unwrap_or_else(|| self.cli_workspace_index(None, caller))
        } else {
            self.cli_workspace_index(None, caller)
        };
        let root = selected
            .ok()
            .and_then(|index| self.workspaces.items().get(index))
            .and_then(|workspace| workspace.path.clone());
        let secrets = self.db.secrets().clone();
        let passphrases = self.db.ssh_passphrases.clone();
        let mongo_store = self.db.mongo_store().clone();
        let reply = command.reply;
        let verb = command
            .request
            .get("cmd")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        std::thread::spawn(move || {
            let response = match root {
                Some(root) => {
                    crate::db_cli::run(&root, &verb, &args, &secrets, &passphrases, &mongo_store)
                }
                None => serde_json::json!({
                    "ok": false,
                    "error": {"kind":"workspace_not_found","message":"missing target workspace (pass --workspace <id|path> or run inside a GSH workspace)"}
                }),
            };
            let _ = reply.send(response);
        });
    }

    fn new_terminal_tab(&mut self, caller: Option<&str>, args: &Value) -> Result<Value, String> {
        let cwd = args
            .get("cwd")
            .and_then(Value::as_str)
            .filter(|cwd| !cwd.is_empty())
            .ok_or("missing cwd")?;
        let directory = std::path::Path::new(cwd);
        if !directory.is_dir() {
            return Err(format!("directory not found: \"{cwd}\""));
        }
        let title = args
            .get("title")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|title| !title.is_empty() && !title.chars().any(char::is_control));
        let split = args.get("split").and_then(Value::as_str).unwrap_or("");
        let anchor = caller
            .and_then(term_id_from_pane_id)
            .filter(|id| self.terminals.contains_key(id))
            .and_then(|id| self.panes.pane_of(&PaneTab::Terminal(id)))
            .unwrap_or_else(|| self.panes.focused());
        let id = match split {
            "" => {
                let id = self.panes.alloc_term_id();
                let Some(model) = self.panes.model_mut(anchor) else {
                    return Err("target pane does not exist".into());
                };
                model.open_terminal(id);
                self.panes.focus(anchor);
                Some(id)
            }
            "right" => self.panes.split_with_terminal(anchor, Axis::Vertical),
            "down" => self.panes.split_with_terminal(anchor, Axis::Horizontal),
            _ => {
                return Err(format!(
                    "invalid split \"{split}\" (expected right or down)"
                ))
            }
        }
        .ok_or("could not create terminal pane")?;
        if let Some(title) = title {
            let pane = self
                .panes
                .pane_of(&PaneTab::Terminal(id))
                .ok_or("new tab is not in a pane")?;
            self.panes
                .model_mut(pane)
                .ok_or("new tab pane does not exist")?
                .set_terminal_label(id, title.to_owned());
            self.terminal_labels.insert(id, title.to_owned());
        }
        let terminal = self.spawn_terminal_for_dir(id, Some(directory));
        let context = caller
            .and_then(term_id_from_pane_id)
            .and_then(|caller_id| self.terminal_contexts.get(&caller_id).cloned())
            .unwrap_or_else(|| self.session_context_for_dir(Some(directory)));
        self.terminal_contexts.insert(id, context);
        self.terminals.insert(id, terminal);
        self.layout_save_due = Some(Instant::now() + Duration::from_millis(500));
        Ok(json!({"tabId": pane_id_for_term(id)}))
    }

    fn resolve_close_target(&self, caller: Option<&str>, args: &Value) -> Result<TermId, String> {
        let target = args
            .get("target")
            .and_then(Value::as_str)
            .filter(|target| !target.is_empty())
            .or(caller)
            .filter(|target| !target.is_empty())
            .ok_or("missing target (pass <label|tab-id> or run inside a GSH terminal)")?;
        if let Some(id) = term_id_from_pane_id(target)
            .filter(|id| self.terminals.contains_key(id) && pane_id_for_term(*id) == target)
        {
            return Ok(id);
        }
        let mut matches = self.terminal_labels.iter().filter_map(|(id, label)| {
            (self.terminals.contains_key(id) && label.eq_ignore_ascii_case(target)).then_some(*id)
        });
        match (matches.next(), matches.next()) {
            (Some(id), None) => Ok(id),
            (Some(_), Some(_)) => Err(format!("label \"{target}\" is ambiguous — use a tab-id")),
            _ => Err(format!("tab \"{target}\" does not exist")),
        }
    }

    fn close_terminal_tab(&mut self, id: TermId) {
        let tab = PaneTab::Terminal(id);
        let Some(pane) = self.panes.pane_of(&tab) else {
            return;
        };
        let Some(index) = self.panes.model(pane).and_then(|model| {
            model
                .tabs()
                .iter()
                .position(|tab| *tab == PaneTab::Terminal(id))
        }) else {
            return;
        };
        self.panes.focus(pane);
        if let Some(removed) = self
            .panes
            .model_mut(pane)
            .and_then(|model| model.close(index))
        {
            self.dispose_tab(removed);
        }
        self.panes.close_if_empty(pane);
        if self
            .panes
            .models()
            .iter()
            .all(|model| model.tabs().is_empty())
        {
            let replacement = self.panes.alloc_term_id();
            self.panes.focused_model_mut().open_terminal(replacement);
            let cwd = self
                .active_workspace_dir()
                .map(std::path::Path::to_path_buf);
            let terminal = self.spawn_terminal_for_dir(replacement, cwd.as_deref());
            self.terminals.insert(replacement, terminal);
        }
        self.layout_save_due = Some(Instant::now() + Duration::from_millis(500));
    }

    fn list_terminal_tabs(&self) -> Vec<Value> {
        let mut tabs = Vec::new();
        for pane in self.panes.leaves() {
            let Some(model) = self.panes.model(pane) else {
                continue;
            };
            for tab in model.tabs() {
                let (id, kind, title, label, working, harness, cwd) = match tab {
                    PaneTab::Terminal(id) => {
                        let tab_id = pane_id_for_term(*id);
                        let record = self.status_runtime.pane_record(&tab_id);
                        let harness = self
                            .active_harnesses
                            .get(id)
                            .and_then(|kind| match *kind {
                                "antigravity" => Some("antigravity"),
                                "claudecode" => Some("claudeCode"),
                                "codex" => Some("codex"),
                                "cursor" => Some("cursor"),
                                "githubcopilot" => Some("gitHubCopilot"),
                                "opencode" => Some("openCode"),
                                "pi" => Some("pi"),
                                _ => None,
                            })
                            .or_else(|| {
                                record.and_then(|record| record.harness()).map(|harness| {
                                    match harness {
                                        gsh_core::status::AgentHarness::Claude => "claudeCode",
                                        gsh_core::status::AgentHarness::Codex => "codex",
                                        gsh_core::status::AgentHarness::Pi => "pi",
                                    }
                                })
                            });
                        let working = record.is_some_and(|record| {
                            record.status() == gsh_core::status::TurnStatus::Working
                        }) || self
                            .output_activity
                            .get(id)
                            .is_some_and(|at| at.elapsed() < Duration::from_secs(3));
                        let terminal = self.terminals.get(id);
                        let title = terminal
                            .and_then(|terminal| terminal.title())
                            .unwrap_or_else(|| tab.title());
                        (
                            tab_id,
                            if harness.is_some() {
                                "agent"
                            } else {
                                "terminal"
                            },
                            title,
                            model.terminal_label(*id).map(str::to_owned),
                            working,
                            harness,
                            terminal
                                .and_then(|term| term.cwd())
                                .map(std::path::Path::to_path_buf),
                        )
                    }
                    PaneTab::File(path) => (
                        format!("f:{}", path.display()),
                        "file",
                        tab.title(),
                        None,
                        false,
                        None,
                        Some(path.clone()),
                    ),
                    _ => continue,
                };
                let workspace = match tab {
                    PaneTab::Terminal(id) => self
                        .terminal_contexts
                        .get(id)
                        .and_then(|context| context.workspace.as_ref())
                        .and_then(|reference| {
                            self.workspaces
                                .items()
                                .iter()
                                .find(|workspace| match reference {
                                    gsh_core::workspace::WorkspaceRef::Project(path) => {
                                        workspace.path.as_ref() == Some(path)
                                    }
                                    gsh_core::workspace::WorkspaceRef::System(id) => {
                                        workspace.id == *id
                                    }
                                })
                        }),
                    _ => None,
                }
                .or_else(|| {
                    cwd.as_deref().and_then(|cwd| {
                        self.workspaces
                            .items()
                            .iter()
                            .filter_map(|workspace| {
                                let root = workspace.path.as_deref()?;
                                cwd.starts_with(root).then_some((workspace, root))
                            })
                            .max_by_key(|(_, root)| root.components().count())
                            .map(|(workspace, _)| workspace)
                    })
                });
                tabs.push(json!({
                    "id": id,
                    "kind": kind,
                    "title": title,
                    "label": label,
                    "workspaceId": workspace.map(|workspace| workspace.id.as_str()),
                    "workspacePath": workspace.and_then(|workspace| workspace.path.as_ref()).map(|path| path.to_string_lossy().to_string()),
                    "working": working,
                    "harness": harness,
                }));
            }
        }
        tabs
    }

    fn terminal_ipc_target(&self, tab_id: Option<&str>, args: &Value) -> Result<TermId, String> {
        let named = args
            .get("target")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty());
        let target = named
            .or(tab_id)
            .filter(|s| !s.is_empty())
            .ok_or("missing target (pass <label|tab-id> or run inside a GSH terminal)")?;
        if named.is_none() && target == "@focused" {
            return match self.panes.focused_model().active_tab() {
                Some(PaneTab::Terminal(id)) if self.terminals.contains_key(id) => Ok(*id),
                _ => Err("the focused tab is not a terminal".into()),
            };
        }
        if let Some(id) = term_id_from_pane_id(target)
            .filter(|id| self.terminals.contains_key(id) && pane_id_for_term(*id) == target)
        {
            return Ok(id);
        }
        if named.is_some() {
            let mut matches = self.terminal_labels.iter().filter_map(|(id, label)| {
                (self.terminals.contains_key(id) && label.to_lowercase() == target.to_lowercase())
                    .then_some(*id)
            });
            if let Some(id) = matches.next() {
                return if matches.next().is_none() {
                    Ok(id)
                } else {
                    Err(format!("label \"{target}\" is ambiguous — use a tab-id"))
                };
            }
        }
        Err(format!("terminal tab \"{target}\" does not exist"))
    }

    fn write_terminal(&mut self, id: TermId, args: &Value) -> Result<Value, String> {
        let data = args
            .get("data")
            .and_then(Value::as_str)
            .ok_or("missing data")?;
        let text = BASE64_STANDARD
            .decode(data)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .ok_or("invalid data (base64-encoded UTF-8 expected)")?;
        self.send_terminal_input(id, &text)?;
        Ok(json!({"tabId": pane_id_for_term(id)}))
    }

    fn send_terminal_input(&mut self, id: TermId, text: &str) -> Result<(), String> {
        if !self
            .terminals
            .get_mut(&id)
            .is_some_and(|term| term.send_text(text))
        {
            return Err("target tab is not running".into());
        }
        self.terminal_input_at.insert(id, Instant::now());
        Ok(())
    }

    fn read_terminal(&self, id: TermId, args: &Value) -> Result<Value, String> {
        let requested = args
            .get("lines")
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
            .unwrap_or(100);
        let limit = requested.min(MAX_READ_LINES) as usize;
        let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0);
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        let from_start = args
            .get("fromStart")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let (text, lines, total) = self
            .terminals
            .get(&id)
            .and_then(|term| term.output_page(limit, offset, from_start))
            .ok_or("target tab has no readable output")?;
        Ok(json!({
            "text": BASE64_STANDARD.encode(text),
            "lines": lines,
            "total": total,
            "truncated": requested > MAX_READ_LINES,
        }))
    }

    fn rename_terminal(&mut self, id: TermId, args: &Value) -> Result<Value, String> {
        let label = args
            .get("title")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|title| !title.is_empty() && !title.chars().any(char::is_control))
            .ok_or("invalid title")?;
        let pane = self
            .panes
            .pane_of(&PaneTab::Terminal(id))
            .ok_or("target tab is closed")?;
        let model = self.panes.model_mut(pane).ok_or("target tab is closed")?;
        model.set_terminal_label(id, label.to_owned());
        self.terminal_labels.insert(id, label.to_owned());
        self.sync_island_from_status();
        Ok(json!({"tabId": pane_id_for_term(id)}))
    }

    fn interrupt_terminal(&mut self, id: TermId, args: &Value) -> Result<Value, String> {
        let pane_id = pane_id_for_term(id);
        let force = args.get("force").and_then(Value::as_bool).unwrap_or(false);
        let current_turn = self
            .status_runtime
            .pane_record(&pane_id)
            .is_some_and(|record| record.status() != TurnStatus::Idle);
        self.send_terminal_input(
            id,
            if !force && current_turn {
                "\x1b"
            } else {
                "\x03"
            },
        )?;
        self.cancel_pending_permissions(&pane_id);
        self.sync_island_from_status();
        Ok(json!({"tabId": pane_id}))
    }

    fn wait_for_terminal_idle(
        &mut self,
        id: TermId,
        reply: &SyncSender<Value>,
    ) -> Result<Option<Value>, String> {
        if self.terminal_turn_is_idle(id)? {
            return Ok(Some(json!({"tabId": pane_id_for_term(id)})));
        }
        self.idle_waiters.push(IdleWaiter {
            id,
            reply: reply.clone(),
            deadline: Instant::now() + IDLE_TIMEOUT,
        });
        Ok(None)
    }

    fn terminal_turn_is_idle(&self, id: TermId) -> Result<bool, String> {
        if !self.terminals.contains_key(&id) {
            return Err("target tab is closed".into());
        }
        self.status_runtime
            .pane_record(&pane_id_for_term(id))
            .filter(|record| record.observed())
            .map(|record| record.status() == TurnStatus::Idle && record.pending_ask().is_none())
            .ok_or_else(|| "target tab has no observed agent status".into())
    }

    pub(crate) fn poll_idle_waiters(&mut self) {
        for waiter in std::mem::take(&mut self.idle_waiters) {
            let reply = if Instant::now() >= waiter.deadline {
                json!({"ok": false, "error": "timed out waiting for tab to become idle"})
            } else {
                match self.terminal_turn_is_idle(waiter.id) {
                    Ok(true) => json!({"ok": true, "data": {"tabId": pane_id_for_term(waiter.id)}}),
                    Ok(false) => {
                        self.idle_waiters.push(waiter);
                        continue;
                    }
                    Err(error) => json!({"ok": false, "error": error}),
                }
            };
            let _ = waiter.reply.send(reply);
        }
    }

    pub(crate) fn cancel_idle_waiters(&mut self, id: TermId) {
        self.idle_waiters.retain(|waiter| {
            if waiter.id != id {
                return true;
            }
            let _ = waiter
                .reply
                .send(json!({"ok": false, "error": "target tab is closed"}));
            false
        });
    }
}

#[cfg(all(test, unix))]
mod tests;
