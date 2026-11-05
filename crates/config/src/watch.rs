//! Recarga ao salvar: polling do mtime numa thread (só std), entregando a
//! configuração nova num `std::sync::mpsc`.
//!
//! Polling e não inotify/FSEvents: um arquivo, 2 stats por segundo, zero
//! dependência, e funciona igual em editor que grava por rename (vim, Zed)
//! ou por truncate.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::error::ConfigError;
use crate::load::{fingerprint, ConfigStore, Fingerprint};
use crate::schema::Config;

/// Intervalo entre dois stats do arquivo.
pub const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// O que a thread entrega quando o arquivo muda no disco.
#[derive(Debug)]
pub enum ConfigEvent {
    /// Arquivo válido com conteúdo efetivo diferente do anterior (inclui o
    /// arquivo apagado, que volta aos padrões).
    Changed(Box<Config>),
    /// Arquivo inválido: a última configuração válida continua valendo.
    Invalid(ConfigError),
}

/// Dono da thread de polling. Soltar o watcher (ou o `Receiver`) encerra a
/// thread.
#[derive(Debug)]
pub struct ConfigWatcher {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ConfigWatcher {
    /// Começa a vigiar o arquivo do `store`, a partir do que ele já leu.
    pub fn spawn(store: ConfigStore) -> Result<(Self, Receiver<ConfigEvent>), ConfigError> {
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("gsh-config-watch".to_owned())
            .spawn(move || poll(store, &tx, &flag))
            .map_err(ConfigError::Spawn)?;
        Ok((
            Self {
                stop,
                thread: Some(thread),
            },
            rx,
        ))
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            // Acorda o `park_timeout` para não esperar o intervalo inteiro.
            thread.thread().unpark();
            let _ = thread.join();
        }
    }
}

/// A mudança só é lida quando o stat se repete em dois polls seguidos: um
/// editor que grava por truncate + write deixa o arquivo vazio por um instante,
/// e ler nesse instante entregaria os padrões. Latência: até 2 intervalos.
fn poll(mut store: ConfigStore, tx: &Sender<ConfigEvent>, stop: &AtomicBool) {
    let mut pending: Option<Fingerprint> = None;
    loop {
        thread::park_timeout(POLL_INTERVAL);
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let now = fingerprint(store.path());
        if now == store.seen() {
            pending = None;
            continue;
        }
        if pending != Some(now) {
            pending = Some(now);
            continue;
        }
        pending = None;
        let before = store.current().clone();
        let event = match store.reload() {
            // Salvou sem mudar nada efetivo (espaço, comentário, `touch`).
            Ok(config) if *config == before => continue,
            Ok(config) => ConfigEvent::Changed(Box::new(config.clone())),
            Err(e) => ConfigEvent::Invalid(e),
        };
        if tx.send(event).is_err() {
            return;
        }
    }
}
