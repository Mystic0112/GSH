//! Linux stdio language server transport and document operations.
mod document;
mod launch;
mod server;
mod wire;

pub use document::Diagnostic;
pub use launch::{executable_on_path, format_external};

use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
    thread::JoinHandle,
};
use tokio::sync::mpsc;
use url::Url;

#[derive(Debug)]
pub enum Command {
    Open {
        uri: String,
        version: i64,
        text: Arc<str>,
    },
    Change {
        uri: String,
        version: i64,
        text: Arc<str>,
    },
    Save {
        uri: String,
    },
    Close {
        uri: String,
    },
    Format {
        uri: String,
        version: i64,
        tab_size: u8,
    },
}

#[derive(Debug)]
pub enum Event {
    Ready {
        formatting: bool,
    },
    Diagnostics {
        uri: String,
        version: i64,
        diagnostics: Vec<Diagnostic>,
    },
    Formatted {
        uri: String,
        version: i64,
        result: Result<String, String>,
    },
    Stopped(String),
}

pub fn file_uri(path: &Path) -> Option<String> {
    Url::from_file_path(path).ok().map(Into::into)
}

pub struct Client {
    commands: mpsc::Sender<Command>,
    events: mpsc::Receiver<Event>,
    worker: JoinHandle<()>,
}

impl Client {
    pub fn start(spec: String, root: PathBuf, language: String) -> io::Result<Self> {
        let (commands, receiver) = mpsc::channel(32);
        let (sender, events) = mpsc::channel(16);
        let worker = std::thread::Builder::new()
            .name(format!("lsp-{language}"))
            .spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .and_then(|runtime| {
                        runtime.block_on(server::run(&spec, &root, &language, receiver, &sender))
                    });
                if let Err(error) = result {
                    let _ = sender.blocking_send(Event::Stopped(error.to_string()));
                }
            })?;
        Ok(Self {
            commands,
            events,
            worker,
        })
    }

    pub fn send(&self, command: Command) -> bool {
        self.commands.try_send(command).is_ok()
    }

    pub fn next_event(&mut self) -> Option<Event> {
        self.events.try_recv().ok()
    }

    pub fn stop(self) -> JoinHandle<()> {
        self.worker
    }
}

#[cfg(test)]
mod tests;
