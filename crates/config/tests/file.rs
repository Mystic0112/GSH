//! Arquivo real em diretório temporário (nunca a home): padrões, arquivo
//! parcial, arquivo inválido mantendo a última válida, recarga por polling.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use gsh_config::schema::{AppFeature, AppThemeMode, Chat, Features, TranscriptWidth};
use gsh_config::{load, Config, ConfigError, ConfigEvent, ConfigStore, ConfigWatcher};

/// Diretório próprio do teste, apagado no `Drop`.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("gsh-config-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn file(&self) -> PathBuf {
        self.0.join("config.toml")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
}

/// Próximo evento em até `limit`.
fn next_event(rx: &Receiver<ConfigEvent>, limit: Duration) -> ConfigEvent {
    rx.recv_timeout(limit)
        .unwrap_or_else(|e| panic!("no event within {limit:?}: {e}"))
}

#[test]
fn missing_file_yields_defaults_and_is_not_created() {
    let tmp = TempDir::new("missing");
    let path = tmp.file();

    assert_eq!(load(&path).unwrap(), Config::default());

    let (store, result) = ConfigStore::open(&path);
    assert!(result.is_ok());
    assert_eq!(*store.current(), Config::default());
    assert!(!path.exists());
}

#[test]
fn partial_file_fills_only_missing_fields_with_defaults() {
    let tmp = TempDir::new("partial");
    let path = tmp.file();
    write(
        &path,
        "themeMode = \"dark\"\ninterfaceSize = 16\n\n[chat]\nfont = \"Inter\"\n\n[features]\nhidden = [\"farm\"]\n",
    );

    let config = load(&path).unwrap();

    let defaults = Config::default();
    let expected = Config {
        theme_mode: AppThemeMode::Dark,
        interface_size: 16.0,
        chat: Chat {
            font: Some("Inter".to_owned()),
            ..defaults.chat.clone()
        },
        features: Features {
            hidden: [AppFeature::Farm].into(),
        },
        ..defaults
    };
    assert_eq!(config, expected);
    // Campos da mesma tabela que não vieram ficam no padrão.
    assert_eq!(config.chat.transcript_width, TranscriptWidth::Medium);
    assert!(config.chat.classify_session_states);
}

#[test]
fn invalid_file_returns_typed_error_and_keeps_last_valid() {
    let tmp = TempDir::new("invalid");
    let path = tmp.file();
    write(&path, "codeSize = 15\n");
    let (mut store, result) = ConfigStore::open(&path);
    assert!(result.is_ok());
    assert_eq!(store.current().code_size, 15.0);

    // Sintaxe quebrada na linha 2.
    write(&path, "codeSize = 18\nthemeMode = \n");
    match store.reload() {
        Err(ConfigError::Parse {
            path: p,
            line,
            cause,
            ..
        }) => {
            assert_eq!(p, path);
            assert_eq!(line, Some(2));
            assert!(!cause.is_empty());
        }
        other => panic!("expected Parse error, got {other:?}"),
    }
    assert_eq!(store.current().code_size, 15.0);

    // Tipo errado também é inválido, com a posição do valor.
    write(&path, "codeSize = \"big\"\n");
    match store.reload() {
        Err(ConfigError::Parse { line, column, .. }) => {
            assert_eq!(line, Some(1));
            assert_eq!(column, Some(12));
        }
        other => panic!("expected Parse error, got {other:?}"),
    }
    assert_eq!(store.current().code_size, 15.0);

    // Número não finito.
    write(&path, "[sound]\nvolume = nan\n");
    assert!(matches!(
        store.reload(),
        Err(ConfigError::NonFinite {
            key: "sound.volume",
            ..
        })
    ));
    assert_eq!(store.current().code_size, 15.0);
}

#[test]
fn open_with_invalid_file_starts_from_defaults_and_reports() {
    let tmp = TempDir::new("open-invalid");
    let path = tmp.file();
    write(&path, "[[[\n");

    let (store, result) = ConfigStore::open(&path);

    assert!(matches!(result, Err(ConfigError::Parse { .. })));
    assert_eq!(*store.current(), Config::default());
}

#[test]
fn unreadable_path_is_a_read_error() {
    let tmp = TempDir::new("read-error");
    // Diretório no lugar do arquivo: existe, mas não dá para ler como texto.
    fs::create_dir_all(tmp.file()).unwrap();

    assert!(matches!(load(&tmp.file()), Err(ConfigError::Read { .. })));
}

#[test]
fn saving_the_file_delivers_new_config_within_two_seconds() {
    let tmp = TempDir::new("reload");
    let path = tmp.file();
    write(&path, "themeMode = \"light\"\n");
    let (store, result) = ConfigStore::open(&path);
    assert!(result.is_ok());
    let (_watcher, rx) = ConfigWatcher::spawn(store).unwrap();

    let started = Instant::now();
    write(&path, "themeMode = \"dark\"\ncodeSize = 14\n");
    match next_event(&rx, Duration::from_secs(2)) {
        ConfigEvent::Changed(config) => {
            assert_eq!(config.theme_mode, AppThemeMode::Dark);
            assert_eq!(config.code_size, 14.0);
        }
        ConfigEvent::Invalid(e) => panic!("unexpected error: {e}"),
    }
    assert!(started.elapsed() <= Duration::from_secs(2));

    // Inválido: avisa e mantém; a próxima gravação válida volta a valer.
    write(&path, "themeMode = = \"x\"\n");
    assert!(matches!(
        next_event(&rx, Duration::from_secs(2)),
        ConfigEvent::Invalid(ConfigError::Parse { .. })
    ));
    write(&path, "themeMode = \"system\"\n");
    match next_event(&rx, Duration::from_secs(2)) {
        ConfigEvent::Changed(config) => assert_eq!(config.theme_mode, AppThemeMode::System),
        ConfigEvent::Invalid(e) => panic!("unexpected error: {e}"),
    }

    // Apagar o arquivo volta aos padrões.
    write(&path, "codeSize = 20\n");
    assert!(matches!(
        next_event(&rx, Duration::from_secs(2)),
        ConfigEvent::Changed(_)
    ));
    fs::remove_file(&path).unwrap();
    match next_event(&rx, Duration::from_secs(2)) {
        ConfigEvent::Changed(config) => assert_eq!(*config, Config::default()),
        ConfigEvent::Invalid(e) => panic!("unexpected error: {e}"),
    }
}

#[test]
fn dropping_the_watcher_stops_the_thread_promptly() {
    let tmp = TempDir::new("drop");
    let (store, _) = ConfigStore::open(tmp.file());
    let (watcher, rx) = ConfigWatcher::spawn(store).unwrap();

    let started = Instant::now();
    drop(watcher);

    assert!(started.elapsed() < Duration::from_millis(400));
    // Thread encerrada: o canal fecha.
    assert!(rx.recv_timeout(Duration::from_secs(1)).is_err());
}
