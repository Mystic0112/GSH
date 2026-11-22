use super::{
    document::{edit_text, end_position, validate_size, Positions, TextEdit},
    file_uri, launch,
    wire::{invalid, notification, read_frame, request, response_id, rpc_result, write_frame},
    Command, Diagnostic, Event,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, io, path::Path, sync::Arc, time::Duration};
use tokio::{
    io::BufReader,
    process::{ChildStdin, ChildStdout},
    sync::mpsc,
    time::{self, Instant},
};

const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

struct Document {
    version: i64,
    text: Arc<str>,
    synced: bool,
}

struct FormatRequest {
    uri: String,
    version: i64,
    deadline: Instant,
}

#[derive(Default)]
struct Capabilities {
    open_close: bool,
    change: u64,
    save: bool,
    save_text: bool,
    formatting: bool,
}

pub(super) async fn run(
    spec: &str,
    root: &Path,
    language: &str,
    commands: mpsc::Receiver<Command>,
    events: &mpsc::Sender<Event>,
) -> io::Result<()> {
    let mut child = launch::spawn(spec, root, None)?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| invalid("missing LSP stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| invalid("missing LSP stdout"))?;
    let result = run_stdio(root, language, stdin, stdout, commands, events).await;
    if result.is_err()
        || !matches!(
            time::timeout(Duration::from_secs(1), child.wait()).await,
            Ok(Ok(_))
        )
    {
        let _ = child.kill().await;
    }
    result
}

async fn run_stdio(
    root: &Path,
    language: &str,
    stdin: ChildStdin,
    stdout: ChildStdout,
    mut commands: mpsc::Receiver<Command>,
    events: &mpsc::Sender<Event>,
) -> io::Result<()> {
    let uri = file_uri(root).ok_or_else(|| invalid("invalid LSP workspace URI"))?;
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace");
    let workspace_folders = json!([{"uri": uri, "name": name}]);
    let (incoming, mut messages) = mpsc::channel(2);
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        loop {
            let frame = read_frame(&mut reader).await;
            let failed = frame.is_err();
            if incoming.send(frame).await.is_err() || failed {
                break;
            }
        }
    });
    let mut state = Server {
        stdin,
        workspace_folders,
        capabilities: Capabilities::default(),
        documents: HashMap::new(),
        pending: HashMap::new(),
        next_id: 1,
    };
    let params = json!({
        "processId": std::process::id(),
        "rootUri": uri,
        "workspaceFolders": state.workspace_folders,
        "clientInfo": {"name": "GSH"},
        "capabilities": {
            "general": {"positionEncodings": ["utf-16"]},
            "workspace": {"workspaceFolders": true, "configuration": true},
            "textDocument": {"publishDiagnostics": {"versionSupport": true}}
        }
    });
    let initialized = tokio::select! {
        response = state.exchange(&mut messages, "initialize", params, INITIALIZE_TIMEOUT) => response?,
        _ = events.closed() => return Ok(()),
    };
    let capabilities = initialized
        .get("capabilities")
        .filter(|value| value.is_object())
        .ok_or_else(|| invalid("invalid LSP server capabilities"))?;
    if capabilities
        .get("positionEncoding")
        .is_some_and(|encoding| encoding.as_str() != Some("utf-16"))
    {
        return Err(invalid(
            "LSP server selected an unsupported position encoding",
        ));
    }
    let sync = capabilities.get("textDocumentSync").unwrap_or(&Value::Null);
    state.capabilities = Capabilities {
        open_close: sync.as_u64().is_some_and(|sync| sync != 0)
            || sync.get("openClose").and_then(Value::as_bool) == Some(true),
        change: sync
            .as_u64()
            .or_else(|| sync.get("change").and_then(Value::as_u64))
            .unwrap_or_default(),
        save: sync
            .get("save")
            .is_some_and(|save| save.is_object() || save.as_bool() == Some(true)),
        save_text: sync.pointer("/save/includeText").and_then(Value::as_bool) == Some(true),
        formatting: capabilities
            .get("documentFormattingProvider")
            .is_some_and(|value| value.is_object() || value.as_bool() == Some(true)),
    };
    state.notify("initialized", json!({})).await?;
    send_event(
        events,
        Event::Ready {
            formatting: state.capabilities.formatting,
        },
    )
    .await?;
    let mut expire = time::interval(Duration::from_millis(250));
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => state.command(command, language, events).await?,
                None => break,
            },
            message = messages.recv() => state.message(recv(message)?, events).await?,
            _ = expire.tick(), if !state.pending.is_empty() => state.expire(events).await?,
            _ = events.closed() => break,
        }
    }
    let _ = time::timeout(Duration::from_secs(2), async {
        if state.capabilities.open_close {
            for (uri, _) in std::mem::take(&mut state.documents) {
                state
                    .notify(
                        "textDocument/didClose",
                        json!({"textDocument": {"uri": uri}}),
                    )
                    .await?;
            }
        }
        let _ = state
            .exchange(
                &mut messages,
                "shutdown",
                Value::Null,
                Duration::from_secs(1),
            )
            .await;
        state.notify("exit", Value::Null).await
    })
    .await;
    Ok(())
}

struct Server {
    stdin: ChildStdin,
    workspace_folders: Value,
    capabilities: Capabilities,
    documents: HashMap<String, Document>,
    pending: HashMap<i64, FormatRequest>,
    next_id: i64,
}

impl Server {
    async fn notify(&mut self, method: &str, params: Value) -> io::Result<()> {
        write_frame(&mut self.stdin, notification(method, params)).await
    }

    async fn request(&mut self, method: &str, params: Value) -> io::Result<i64> {
        let id = self.next_id;
        self.next_id += 1;
        write_frame(&mut self.stdin, request(id, method, params)).await?;
        Ok(id)
    }

    async fn exchange(
        &mut self,
        messages: &mut mpsc::Receiver<io::Result<Value>>,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> io::Result<Value> {
        let id = self.request(method, params).await?;
        time::timeout(timeout, async {
            loop {
                let message = recv(messages.recv().await)?;
                if response_id(&message) == Some(id) {
                    return rpc_result(message);
                }
                self.answer(&message).await?;
            }
        })
        .await?
    }

    async fn answer(&mut self, message: &Value) -> io::Result<()> {
        let (Some(method), Some(id)) = (
            message.get("method").and_then(Value::as_str),
            message.get("id"),
        ) else {
            return Ok(());
        };
        let result = match method {
            "workspace/configuration" => {
                let count = message
                    .pointer("/params/items")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len);
                json!(vec![Value::Null; count])
            }
            "workspace/workspaceFolders" => self.workspace_folders.clone(),
            "window/showMessageRequest" | "window/workDoneProgress/create" => Value::Null,
            _ => {
                return write_frame(
                    &mut self.stdin,
                    json!({"jsonrpc": "2.0", "id": id,
                    "error": {"code": -32601, "message": "Method not supported"}}),
                )
                .await;
            }
        };
        write_frame(
            &mut self.stdin,
            json!({"jsonrpc": "2.0", "id": id, "result": result}),
        )
        .await
    }

    async fn command(
        &mut self,
        command: Command,
        language: &str,
        events: &mpsc::Sender<Event>,
    ) -> io::Result<()> {
        match command {
            Command::Open { uri, version, text } => {
                validate_size(&text)?;
                if self.capabilities.open_close {
                    self.notify(
                        "textDocument/didOpen",
                        json!({"textDocument": {
                            "uri": uri, "languageId": language, "version": version, "text": &*text,
                        }}),
                    )
                    .await?;
                }
                self.documents.insert(
                    uri,
                    Document {
                        version,
                        text,
                        synced: self.capabilities.open_close,
                    },
                );
            }
            Command::Change { uri, version, text } => {
                validate_size(&text)?;
                let Some(previous) = self.documents.get(&uri) else {
                    return Ok(());
                };
                let change = match self.capabilities.change {
                    1 => Some(json!({"text": &*text})),
                    2 => {
                        let (line, character) = end_position(&previous.text);
                        Some(json!({"range": {"start": {"line": 0, "character": 0},
                            "end": {"line": line, "character": character}}, "text": &*text}))
                    }
                    _ => None,
                };
                let synced = change.is_some();
                if let Some(change) = change {
                    self.notify("textDocument/didChange", json!({
                        "textDocument": {"uri": uri, "version": version}, "contentChanges": [change],
                    })).await?;
                }
                self.documents.insert(
                    uri,
                    Document {
                        version,
                        text,
                        synced,
                    },
                );
            }
            Command::Save { uri } => {
                if let Some(document) = self.documents.get(&uri).filter(|_| self.capabilities.save)
                {
                    let mut params = json!({"textDocument": {"uri": uri}});
                    if self.capabilities.save_text {
                        if let Some(object) = params.as_object_mut() {
                            object.insert("text".into(), json!(&*document.text));
                        }
                    }
                    self.notify("textDocument/didSave", params).await?;
                }
            }
            Command::Close { uri } => {
                self.documents.remove(&uri);
                self.pending.retain(|_, request| request.uri != uri);
                if self.capabilities.open_close {
                    self.notify(
                        "textDocument/didClose",
                        json!({"textDocument": {"uri": uri}}),
                    )
                    .await?;
                }
            }
            Command::Format {
                uri,
                version,
                tab_size,
            } => {
                if !self.capabilities.formatting
                    || self.pending.len() >= 64
                    || !self
                        .documents
                        .get(&uri)
                        .is_some_and(|doc| doc.synced && doc.version == version)
                {
                    return send_event(
                        events,
                        Event::Formatted {
                            uri,
                            version,
                            result: Err("LSP formatting unavailable".into()),
                        },
                    )
                    .await;
                }
                let id = self.request("textDocument/formatting", json!({
                    "textDocument": {"uri": uri}, "options": {"tabSize": tab_size, "insertSpaces": true},
                })).await?;
                self.pending.insert(
                    id,
                    FormatRequest {
                        uri,
                        version,
                        deadline: Instant::now() + REQUEST_TIMEOUT,
                    },
                );
            }
        }
        Ok(())
    }

    async fn message(
        &mut self,
        mut message: Value,
        events: &mpsc::Sender<Event>,
    ) -> io::Result<()> {
        if let Some(id) = response_id(&message) {
            if let Some(request) = self.pending.remove(&id) {
                if let Some(document) = self
                    .documents
                    .get(&request.uri)
                    .filter(|doc| doc.version == request.version)
                {
                    let result = rpc_result(message)
                        .and_then(|result| {
                            let edits: Option<Vec<TextEdit>> = serde_json::from_value(result)?;
                            edit_text(&document.text, edits.unwrap_or_default())
                        })
                        .map_err(|error| error.to_string());
                    send_event(
                        events,
                        Event::Formatted {
                            uri: request.uri,
                            version: request.version,
                            result,
                        },
                    )
                    .await?;
                }
            }
        } else if message.get("method").and_then(Value::as_str)
            == Some("textDocument/publishDiagnostics")
            && message.get("id").is_none()
        {
            if let Some(params) = message.get_mut("params").map(Value::take) {
                if let Ok(published) = serde_json::from_value::<Published>(params) {
                    if let Some(document) = self.documents.get(&published.uri) {
                        // shortcut: unversioned diagnostics are advisory, use a versioned server for exact freshness.
                        let version = published.version.unwrap_or(document.version);
                        if version == document.version {
                            let positions = (!published.diagnostics.is_empty())
                                .then(|| Positions::new(&document.text));
                            let diagnostics = published
                                .diagnostics
                                .into_iter()
                                .take(100)
                                .filter(|diagnostic| {
                                    positions
                                        .as_ref()
                                        .and_then(|positions| positions.range(diagnostic.range))
                                        .is_some()
                                })
                                .map(|mut diagnostic| {
                                    diagnostic.message =
                                        diagnostic.message.chars().take(400).collect();
                                    diagnostic
                                })
                                .collect();
                            send_event(
                                events,
                                Event::Diagnostics {
                                    uri: published.uri,
                                    version,
                                    diagnostics,
                                },
                            )
                            .await?;
                        }
                    }
                }
            }
        } else {
            self.answer(&message).await?;
        }
        Ok(())
    }

    async fn expire(&mut self, events: &mpsc::Sender<Event>) -> io::Result<()> {
        let now = Instant::now();
        let expired: Vec<_> = self
            .pending
            .iter()
            .filter_map(|(id, request)| (request.deadline <= now).then_some(*id))
            .collect();
        for id in expired {
            if let Some(request) = self.pending.remove(&id) {
                self.notify("$/cancelRequest", json!({"id": id})).await?;
                send_event(
                    events,
                    Event::Formatted {
                        uri: request.uri,
                        version: request.version,
                        result: Err("LSP formatting timed out".into()),
                    },
                )
                .await?;
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct Published {
    uri: String,
    version: Option<i64>,
    diagnostics: Vec<Diagnostic>,
}

fn recv(message: Option<io::Result<Value>>) -> io::Result<Value> {
    message.ok_or_else(|| invalid("LSP server disconnected"))?
}

async fn send_event(sender: &mpsc::Sender<Event>, event: Event) -> io::Result<()> {
    let _ = sender.send(event).await;
    Ok(())
}
