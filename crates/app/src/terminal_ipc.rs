use std::sync::mpsc::SyncSender;
use std::time::{Duration, Instant};

use base64::prelude::{Engine, BASE64_STANDARD};
use gsh_core::status::TurnStatus;
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
        "write" | "read-pane" | "rename-tab" | "interrupt" | "wait-idle"
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
