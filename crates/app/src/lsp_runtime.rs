use crate::{
    editor_pane::{EditorMsg, EditorPane},
    Msg, Shell,
};
use gsh_config::schema::Config;
use gsh_editor::lsp::{self, Client, Command, Diagnostic, Event};
use iced::Task;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    thread::JoinHandle,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Key {
    language: String,
    root: PathBuf,
}

#[derive(Debug)]
pub(crate) struct Document {
    key: Key,
    uri: String,
    pub version: i64,
    pub formatting: bool,
    pub pending: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub error: Option<String>,
    pub external: Option<String>,
}

impl Document {
    pub fn root(&self) -> &Path {
        &self.key.root
    }
}

struct Server {
    client: Client,
    files: HashMap<String, PathBuf>,
    opening: HashSet<String>,
    ready: bool,
    formatting: bool,
    overflow: bool,
}

#[derive(Default)]
pub(crate) struct Pool {
    servers: HashMap<Key, Server>,
    closing: Vec<JoinHandle<()>>,
    version: i64,
    commands: BTreeMap<String, String>,
    formatters: BTreeMap<String, String>,
}

impl Pool {
    pub fn active(&self) -> bool {
        !self.servers.is_empty() || !self.closing.is_empty()
    }

    pub fn configure(&mut self, config: &Config) -> bool {
        if self.commands == config.lsp_commands && self.formatters == config.lsp_formatters {
            return false;
        }
        self.commands.clone_from(&config.lsp_commands);
        self.formatters.clone_from(&config.lsp_formatters);
        true
    }

    fn next_version(&mut self) -> i64 {
        self.version += 1;
        self.version
    }

    pub fn open(&mut self, path: &Path, workspace: Option<&Path>) -> Option<Document> {
        let file = path.canonicalize().ok()?;
        let (language, markers, default) = language_for(&file)?;
        let uri = lsp::file_uri(&file)?;
        let key = Key {
            language: language.clone(),
            root: project_root(&file, workspace, markers)?,
        };
        let external = self.formatters.get(&language).cloned();
        let spec = self.commands.get(&language).cloned().or_else(|| {
            let command = default?;
            let program = command.split_whitespace().next()?;
            lsp::executable_on_path(program).map(|_| command.to_owned())
        });
        if external.is_none() && spec.is_none() {
            return None;
        }
        let mut document = Document {
            key: key.clone(),
            uri: uri.clone(),
            version: self.next_version(),
            formatting: false,
            pending: false,
            diagnostics: Vec::new(),
            error: None,
            external,
        };
        if let Some(spec) = spec {
            match self.servers.entry(key.clone()) {
                std::collections::hash_map::Entry::Occupied(_) => {}
                std::collections::hash_map::Entry::Vacant(entry) => {
                    match Client::start(spec, key.root.clone(), language) {
                        Ok(client) => {
                            entry.insert(Server {
                                client,
                                files: HashMap::new(),
                                opening: HashSet::new(),
                                ready: false,
                                formatting: false,
                                overflow: false,
                            });
                        }
                        Err(error) => document.error = Some(error.to_string()),
                    }
                }
            }
            if let Some(server) = self.servers.get_mut(&key) {
                server.files.insert(uri.clone(), path.to_owned());
                server.opening.insert(uri);
            }
        }
        Some(document)
    }

    pub fn close(&mut self, document: Document) {
        if let Some(server) = self.servers.get_mut(&document.key) {
            if server.files.remove(&document.uri).is_some() && !server.opening.remove(&document.uri)
            {
                server.overflow |= !server.client.send(Command::Close { uri: document.uri });
            }
            if server.files.is_empty() {
                if let Some(server) = self.servers.remove(&document.key) {
                    self.closing.push(server.client.stop());
                }
            }
        }
    }

    pub fn change(&mut self, document: &mut Document, text: Arc<str>) {
        document.version = self.next_version();
        document.diagnostics.clear();
        document.pending = false;
        self.send(
            document,
            Command::Change {
                uri: document.uri.clone(),
                version: document.version,
                text,
            },
        );
    }

    pub fn save(&mut self, document: &mut Document) {
        self.send(
            document,
            Command::Save {
                uri: document.uri.clone(),
            },
        );
    }

    pub fn format(&mut self, document: &mut Document, tab_size: u8) {
        document.pending = true;
        document.error = None;
        self.send(
            document,
            Command::Format {
                uri: document.uri.clone(),
                version: document.version,
                tab_size,
            },
        );
    }

    fn send(&mut self, document: &mut Document, command: Command) {
        let Some(server) = self
            .servers
            .get_mut(&document.key)
            .filter(|server| server.files.contains_key(&document.uri))
        else {
            if matches!(command, Command::Format { .. }) {
                document.pending = false;
            }
            return;
        };
        if server.opening.contains(&document.uri) {
            // didOpen will use the latest snapshot when initialization completes.
            return;
        }
        if server.overflow || !server.client.send(command) {
            // A missed notification would leave the worker and editor at different versions.
            server.overflow = true;
            document.error = Some("LSP connection busy or disconnected".into());
            document.pending = document.external.is_some() && document.pending;
        }
    }

    pub fn poll(&mut self, editors: &mut BTreeMap<PathBuf, EditorPane>) {
        self.closing.retain(|worker| !worker.is_finished());
        let mut changed = Vec::new();
        let mut stopped = Vec::new();
        'servers: for (key, server) in &mut self.servers {
            for _ in 0..16 {
                let Some(event) = server.client.next_event() else {
                    break;
                };
                match event {
                    Event::Ready { formatting } => {
                        server.ready = true;
                        server.formatting = formatting;
                    }
                    Event::Stopped(error) => {
                        mark_stopped(server, editors, &error);
                        stopped.push(key.clone());
                        continue 'servers;
                    }
                    Event::Diagnostics {
                        uri,
                        version,
                        diagnostics,
                    } => {
                        if let Some(document) = server
                            .files
                            .get(&uri)
                            .and_then(|path| editors.get_mut(path))
                            .and_then(|pane| pane.lsp.as_mut())
                            .filter(|doc| doc.key == *key && doc.version == version)
                        {
                            document.diagnostics = diagnostics;
                        }
                    }
                    Event::Formatted {
                        uri,
                        version,
                        result,
                    } => {
                        if let Some(path) = server.files.get(&uri) {
                            if let Some(editor) = editors.get_mut(path) {
                                if editor.finish_lsp_format(version, result) {
                                    changed.push(path.clone());
                                }
                            }
                        }
                    }
                }
            }
            if server.overflow {
                mark_stopped(server, editors, "LSP connection busy or disconnected");
                stopped.push(key.clone());
            } else if server.ready {
                server.opening.retain(|uri| {
                    let Some(editor) = server.files.get(uri).and_then(|path| editors.get_mut(path))
                    else {
                        return false;
                    };
                    let Some(document) = editor.lsp.as_mut() else {
                        return false;
                    };
                    if server.client.send(Command::Open {
                        uri: uri.clone(),
                        version: document.version,
                        text: editor.highlight_text.clone(),
                    }) {
                        document.formatting = server.formatting;
                        false
                    } else {
                        true
                    }
                });
            }
        }
        for key in stopped {
            if let Some(server) = self.servers.remove(&key) {
                self.closing.push(server.client.stop());
            }
        }
        for path in &changed {
            if let Some(editor) = editors.get_mut(path) {
                if let Some(document) = editor.lsp.as_mut() {
                    self.change(document, editor.highlight_text.clone());
                }
            }
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.closing
            .extend(self.servers.drain().map(|(_, server)| server.client.stop()));
        for worker in self.closing.drain(..) {
            let _ = worker.join();
        }
    }
}

fn mark_stopped(server: &Server, editors: &mut BTreeMap<PathBuf, EditorPane>, error: &str) {
    for path in server.files.values() {
        if let Some(document) = editors.get_mut(path).and_then(|pane| pane.lsp.as_mut()) {
            document.formatting = false;
            document.pending = document.external.is_some() && document.pending;
            document.diagnostics.clear();
            document.error = Some(error.to_owned());
        }
    }
}

impl Shell {
    pub(crate) fn attach_lsp(&mut self, path: &Path) {
        let workspace = self
            .workspaces
            .items()
            .iter()
            .filter_map(|ws| ws.path.as_ref())
            .filter(|root| path.starts_with(root))
            .max_by_key(|root| root.components().count());
        if let Some(editor) = self.editors.get_mut(path) {
            editor.lsp = self.lsp.open(path, workspace.map(PathBuf::as_path));
        }
    }

    pub(crate) fn configure_lsp(&mut self) {
        if !self.lsp.configure(&self.settings.current) {
            return;
        }
        for editor in self.editors.values_mut() {
            if let Some(document) = editor.lsp.take() {
                self.lsp.close(document);
            }
        }
        let paths: Vec<_> = self.editors.keys().cloned().collect();
        for path in paths {
            self.attach_lsp(&path);
        }
    }

    pub(crate) fn format_editor(&mut self, path: &Path) -> Task<Msg> {
        let Some(editor) = self
            .editors
            .get_mut(path)
            .filter(|editor| editor.state.notice.is_none())
        else {
            return Task::none();
        };
        let Some(document) = editor.lsp.as_mut().filter(|doc| !doc.pending) else {
            return Task::none();
        };
        if let Some(command) = document.external.clone() {
            document.pending = true;
            document.error = None;
            let version = document.version;
            let root = document.root().to_owned();
            let path = path.to_owned();
            let text = editor.highlight_text.clone();
            return Task::perform(
                async move {
                    let result = lsp::format_external(&command, &root, &path, &text)
                        .await
                        .map_err(|error| error.to_string());
                    (path, result)
                },
                move |(path, result)| Msg::Editor(path, EditorMsg::Formatted(version, result)),
            );
        }
        if document.formatting {
            self.lsp.format(document, editor.config.tab_size);
        }
        Task::none()
    }
}

fn project_root(file: &Path, workspace: Option<&Path>, markers: &[&str]) -> Option<PathBuf> {
    let parent = file.parent()?;
    let workspace = workspace
        .and_then(|root| root.canonicalize().ok())
        .filter(|root| file.starts_with(root));
    let root = parent
        .ancestors()
        .take(64)
        .take_while(|ancestor| {
            workspace
                .as_ref()
                .is_none_or(|root| ancestor.starts_with(root))
        })
        .find(|dir| markers.iter().any(|marker| dir.join(marker).is_file()));
    let git = || {
        parent
            .ancestors()
            .take(64)
            .find(|dir| dir.join(".git").exists())
    };
    root.or(workspace.as_deref())
        .or_else(git)
        .or(Some(parent))
        .map(Path::to_owned)
}

fn language_for(path: &Path) -> Option<(String, &'static [&'static str], Option<&'static str>)> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let (language, markers, command): (&str, &[&str], Option<&str>) = match ext.as_str() {
        "dart" => (
            "dart",
            &["pubspec.yaml"],
            Some("dart language-server --client-id gsh"),
        ),
        "ts" | "tsx" | "mts" | "cts" => (
            "typescript",
            &["tsconfig.json", "package.json"],
            Some("typescript-language-server --stdio"),
        ),
        "js" | "jsx" | "mjs" | "cjs" => (
            "javascript",
            &["jsconfig.json", "package.json"],
            Some("typescript-language-server --stdio"),
        ),
        "py" | "pyi" => (
            "python",
            &[
                "pyproject.toml",
                "setup.py",
                "setup.cfg",
                "requirements.txt",
                "Pipfile",
            ],
            Some("pyright-langserver --stdio"),
        ),
        "go" => ("go", &["go.mod", "go.work"], Some("gopls")),
        "rs" => ("rust", &["Cargo.toml"], Some("rust-analyzer")),
        "php" => ("php", &["composer.json"], Some("intelephense --stdio")),
        "cs" => ("csharp", &[], Some("csharp-ls")),
        "java" => (
            "java",
            &[
                "pom.xml",
                "build.gradle",
                "build.gradle.kts",
                "settings.gradle",
            ],
            Some("jdtls"),
        ),
        "c" => ("c", &["compile_commands.json", "compile_flags.txt"], None),
        "cc" | "cpp" | "cxx" | "h" | "hpp" => {
            ("cpp", &["compile_commands.json", "compile_flags.txt"], None)
        }
        "sh" | "bash" => ("shellscript", &[], None),
        "yml" => ("yaml", &[], None),
        "md" => ("markdown", &[], None),
        _ => (&ext, &[], None),
    };
    Some((language.to_owned(), markers, command))
}
