//! Testes de integração da migração do `settings.json` do Flutter para o
//! `config.toml` (sempre em diretórios temporários, nunca na home real).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use gsh_config::schema::{
    AppFeature, AppThemeMode, Automation, Chat, Code, Editor, Farm, FarmScenery, FarmTeam,
    Features, FileEditorEngine, HarnessKind, Links, Sound, SoundEvent, SourceControl,
    SourceControlViewMode, Terminal, TerminalEngine, TerminalFontWeight, TranscriptView,
    TranscriptWidth, UpdateCheckFrequency,
};
use gsh_config::{
    load, load_with_migration, migrate_if_needed, parse_flutter_json, to_toml, Config, ConfigError,
    ConfigEvent, ConfigStore, JsonParseCause, MigrationOutcome,
};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "gsh-config-migrate-test-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn config_path(&self) -> PathBuf {
        self.0.join(".gsh").join("config.toml")
    }

    fn flutter_path(&self) -> PathBuf {
        self.0
            .join("Documents")
            .join("gsh")
            .join("state")
            .join("settings.json")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

const FULL_FLUTTER_SETTINGS_JSON: &str = r#"{
  "version": 1,
  "data": {
    "dismissed_update_version": "2.1.0",
    "ssh_known_host_keys": {
      "10.0.0.1:22": "SHA256:abc"
    },
    "app": {
      "themeMode": "dark",
      "themeId": "dracula",
      "interfaceFont": "Inter",
      "interfaceSize": 15.5,
      "codeFont": "Fira Code",
      "codeSize": 14,
      "terminalFont": "JetBrains Mono",
      "terminalSize": 12.5,
      "terminalFontWeight": "semiBold",
      "pinUserMessage": false,
      "lastOpenAppId": "vscode",
      "lspCommands": {
        "rust": "rust-analyzer",
        "dart": "dart language-server",
        "empty": "   "
      },
      "lspFormatters": {
        "typescript": "prettier --write %FILE%"
      },
      "formatOnSave": true,
      "closeTabOnShellExit": true,
      "editor.engine": "neovim",
      "notificationsEnabled": false,
      "sound.events": {
        "turnDone": false,
        "actionRequired": true,
        "unknownEvent": false
      },
      "sound.overrides": {
        "turnDone": "/tmp/custom.wav"
      },
      "sound.onActiveTab": {
        "turnDone": true,
        "actionRequired": false
      },
      "sound.volume": 85,
      "searchPanelHeight": 320,
      "tasksPanelHeight": 240,
      "swapSidePanels": true,
      "railVisible": true,
      "treeVisible": true,
      "showGsh": false,
      "developerMode": true,
      "launchAtStartup": true,
      "terminal.default_profile_id": "zsh-login",
      "terminal.engine": "xterm",
      "locale": "pt-BR",
      "sourceControl.viewMode": "tree",
      "automation.harnessId": "claudeCode",
      "automation.modelId": "claude-sonnet-4-5",
      "updateCheckFrequency": "weekly",
      "lastUpdateCheckTime": "2026-10-01T12:00:00.000Z",
      "chat.classifySessionStates": false,
      "chat.remoteControl": true,
      "chat.permissionMode": "acceptEdits",
      "chat.effort": "low",
      "code.themeLight": "github-light",
      "code.themeDark": "one-dark",
      "chat.transcriptView": "verbose",
      "chat.font": "IBM Plex Sans",
      "chat.transcriptWidth": "wide",
      "features.hidden": ["farm", "browser", "removedFeature"],
      "links.openInApp": true,
      "farm.scenery": "office",
      "farm.team.enabled": true,
      "farm.team.name": "gorila",
      "farm.team.me": "helio",
      "farm.team.showTitles": true,
      "farm.team.network": "192.168.10.0/24",
      "farm.team.secret": "s3cr3t",
      "farm.team.doorAsk": true,
      "farm.team.doorAllow": "ana, bruno",
      "farm.plots": {
        "/home/helio/proj": 2
      }
    }
  }
}"#;

fn expected_full_config() -> Config {
    Config {
        theme_mode: AppThemeMode::Dark,
        theme_id: "dracula".to_owned(),
        interface_font: Some("Inter".to_owned()),
        interface_size: 15.5,
        code_font: Some("Fira Code".to_owned()),
        code_size: 14.0,
        terminal_font: Some("JetBrains Mono".to_owned()),
        terminal_size: Some(12.5),
        terminal_font_weight: TerminalFontWeight::SemiBold,
        pin_user_message: false,
        format_on_save: true,
        close_tab_on_shell_exit: true,
        notifications_enabled: false,
        swap_side_panels: true,
        show_gsh: false,
        developer_mode: true,
        launch_at_startup: true,
        locale: Some("pt-BR".to_owned()),
        update_check_frequency: UpdateCheckFrequency::Weekly,
        lsp_commands: BTreeMap::from([
            ("dart".to_owned(), "dart language-server".to_owned()),
            ("rust".to_owned(), "rust-analyzer".to_owned()),
        ]),
        lsp_formatters: BTreeMap::from([(
            "typescript".to_owned(),
            "prettier --write %FILE%".to_owned(),
        )]),
        editor: Editor {
            engine: FileEditorEngine::Neovim,
        },
        terminal: Terminal {
            default_profile_id: Some("zsh-login".to_owned()),
            engine: TerminalEngine::Xterm,
        },
        sound: Sound {
            volume: 85.0,
            events: BTreeMap::from([
                (SoundEvent::TurnDone, false),
                (SoundEvent::ActionRequired, true),
            ]),
            on_active_tab: BTreeMap::from([
                (SoundEvent::TurnDone, true),
                (SoundEvent::ActionRequired, false),
            ]),
        },
        source_control: SourceControl {
            view_mode: SourceControlViewMode::Tree,
        },
        automation: Automation {
            harness_id: Some(HarnessKind::ClaudeCode),
            model_id: Some("claude-sonnet-4-5".to_owned()),
        },
        chat: Chat {
            classify_session_states: false,
            remote_control: true,
            transcript_view: TranscriptView::Verbose,
            font: Some("IBM Plex Sans".to_owned()),
            transcript_width: TranscriptWidth::Wide,
        },
        code: Code {
            theme_light: Some("github-light".to_owned()),
            theme_dark: Some("one-dark".to_owned()),
        },
        features: Features {
            hidden: BTreeSet::from([AppFeature::Farm, AppFeature::Browser]),
        },
        links: Links { open_in_app: true },
        farm: Farm {
            scenery: FarmScenery::Office,
            team: FarmTeam {
                enabled: true,
                name: "gorila".to_owned(),
                me: "helio".to_owned(),
                show_titles: true,
                network: "192.168.10.0/24".to_owned(),
                secret: "s3cr3t".to_owned(),
                door_ask: true,
                door_allow: "ana, bruno".to_owned(),
            },
        },
    }
}

#[test]
fn full_flutter_json_migrates_to_toml_with_all_expected_fields() {
    let tmp = TempDir::new("full");
    let config_path = tmp.config_path();
    let flutter_path = tmp.flutter_path();
    write_file(&flutter_path, FULL_FLUTTER_SETTINGS_JSON);

    let outcome = migrate_if_needed(&config_path, [&flutter_path]).unwrap();

    assert_eq!(outcome, MigrationOutcome::Imported);
    assert!(config_path.exists());

    let loaded = load(&config_path).unwrap();
    assert_eq!(loaded, expected_full_config());
}

#[test]
fn partial_flutter_json_fills_missing_fields_with_defaults() {
    let tmp = TempDir::new("partial");
    let config_path = tmp.config_path();
    let flutter_path = tmp.flutter_path();
    write_file(
        &flutter_path,
        r#"{
          "version": 1,
          "data": {
            "app": {
              "themeMode": "light",
              "interfaceFont": null,
              "terminalSize": null,
              "chat.font": "  Inter  ",
              "automation.harnessId": "copilot",
              "sound.volume": 140
            }
          }
        }"#,
    );

    let outcome = migrate_if_needed(&config_path, [&flutter_path]).unwrap();
    assert_eq!(outcome, MigrationOutcome::Imported);

    let loaded = load(&config_path).unwrap();
    let defaults = Config::default();
    let expected = Config {
        theme_mode: AppThemeMode::Light,
        chat: Chat {
            font: Some("Inter".to_owned()),
            ..defaults.chat.clone()
        },
        automation: Automation {
            harness_id: Some(HarnessKind::GitHubCopilot),
            model_id: None,
        },
        sound: Sound {
            volume: 100.0,
            ..defaults.sound.clone()
        },
        ..defaults
    };
    assert_eq!(loaded, expected);
}

#[test]
fn invalid_flutter_json_returns_typed_error_and_does_not_create_config_toml() {
    let tmp = TempDir::new("invalid");
    let config_path = tmp.config_path();
    let flutter_path = tmp.flutter_path();

    // 1. Sintaxe JSON quebrada.
    write_file(&flutter_path, "{\n  \"version\": 1,\n  \"data\": \n");
    match migrate_if_needed(&config_path, [&flutter_path]) {
        Err(ConfigError::JsonParse {
            path, line, cause, ..
        }) => {
            assert_eq!(path, flutter_path);
            assert!(line.is_some());
            assert_eq!(cause, JsonParseCause::UnexpectedEof);
        }
        other => panic!("expected JsonParse error, got {other:?}"),
    }
    assert!(!config_path.exists());

    // 2. Envelope inválido (sem objeto "data").
    write_file(&flutter_path, r#"{"version": 1}"#);
    match migrate_if_needed(&config_path, [&flutter_path]) {
        Err(ConfigError::JsonParse { path, cause, .. }) => {
            assert_eq!(path, flutter_path);
            assert_eq!(cause, JsonParseCause::MissingData);
        }
        other => panic!("expected JsonParse error, got {other:?}"),
    }
    assert!(!config_path.exists());

    // 3. Tipo inválido dentro de "data.app".
    write_file(
        &flutter_path,
        r#"{"version": 1, "data": {"app": {"interfaceSize": "huge"}}}"#,
    );
    match migrate_if_needed(&config_path, [&flutter_path]) {
        Err(ConfigError::JsonParse { path, cause, .. }) => {
            assert_eq!(path, flutter_path);
            assert_eq!(cause, JsonParseCause::InvalidData);
        }
        other => panic!("expected JsonParse error, got {other:?}"),
    }
    assert!(!config_path.exists());
}

#[test]
fn existing_config_toml_is_never_overwritten() {
    let tmp = TempDir::new("already-migrated");
    let config_path = tmp.config_path();
    let flutter_path = tmp.flutter_path();

    let existing_toml = "themeMode = \"light\"\ncodeSize = 19\n";
    write_file(&config_path, existing_toml);
    write_file(&flutter_path, FULL_FLUTTER_SETTINGS_JSON);

    let outcome = migrate_if_needed(&config_path, [&flutter_path]).unwrap();

    assert_eq!(outcome, MigrationOutcome::AlreadyMigrated);
    assert_eq!(fs::read_to_string(&config_path).unwrap(), existing_toml);
}

#[test]
fn original_settings_json_bytes_remain_identical_after_migration() {
    let tmp = TempDir::new("preserve-bytes");
    let config_path = tmp.config_path();
    let flutter_path = tmp.flutter_path();
    write_file(&flutter_path, FULL_FLUTTER_SETTINGS_JSON);
    let before = fs::read(&flutter_path).unwrap();

    let outcome = migrate_if_needed(&config_path, [&flutter_path]).unwrap();

    assert_eq!(outcome, MigrationOutcome::Imported);
    let after = fs::read(&flutter_path).unwrap();
    assert_eq!(before, after);
}

#[test]
fn migration_is_idempotent() {
    let tmp = TempDir::new("idempotent");
    let config_path = tmp.config_path();
    let flutter_path = tmp.flutter_path();
    write_file(&flutter_path, FULL_FLUTTER_SETTINGS_JSON);
    let flutter_before = fs::read(&flutter_path).unwrap();

    let first = migrate_if_needed(&config_path, [&flutter_path]).unwrap();
    assert_eq!(first, MigrationOutcome::Imported);
    let toml_after_first = fs::read(&config_path).unwrap();

    let second = migrate_if_needed(&config_path, [&flutter_path]).unwrap();
    assert_eq!(second, MigrationOutcome::AlreadyMigrated);
    let toml_after_second = fs::read(&config_path).unwrap();

    assert_eq!(toml_after_first, toml_after_second);
    assert_eq!(flutter_before, fs::read(&flutter_path).unwrap());
}

#[test]
fn nothing_to_import_when_no_candidate_exists() {
    let tmp = TempDir::new("nothing");
    let config_path = tmp.config_path();
    let missing_a = tmp.0.join("a").join("settings.json");
    let missing_b = tmp.0.join("b").join("settings.json");

    let outcome = migrate_if_needed(&config_path, [&missing_a, &missing_b]).unwrap();

    assert_eq!(outcome, MigrationOutcome::NothingToImport);
    assert!(!config_path.exists());
}

#[test]
fn load_and_store_entry_points_support_optional_migration() {
    let tmp = TempDir::new("entry-points");
    let config_path = tmp.config_path();
    let flutter_path = tmp.flutter_path();
    write_file(
        &flutter_path,
        r#"{"version": 1, "data": {"app": {"themeMode": "dark", "codeSize": 17}}}"#,
    );

    // Com `None`, não migra e não cria config.toml.
    let without_migration = load_with_migration(&config_path, None).unwrap();
    assert_eq!(without_migration, Config::default());
    assert!(!config_path.exists());

    // Com `Some`, migra e já carrega a configuração importada.
    let candidates = vec![flutter_path];
    let (store, result) = ConfigStore::open_with_migration(&config_path, Some(&candidates));
    assert_eq!(result.unwrap(), Some(MigrationOutcome::Imported));
    assert_eq!(store.current().theme_mode, AppThemeMode::Dark);
    assert_eq!(store.current().code_size, 17.0);
    assert!(config_path.exists());
}

#[test]
fn flutter_from_json_normalization_rules_are_preserved() {
    let json = r#"{
      "version": 1,
      "data": {
        "app": {
          "themeId": "   ",
          "interfaceFont": "  ",
          "codeFont": " JetBrains Mono ",
          "terminal.default_profile_id": "",
          "locale": " ",
          "chat.font": "   ",
          "code.themeLight": "  ",
          "code.themeDark": " nord ",
          "automation.harnessId": "claude",
          "automation.modelId": "   ",
          "sound.volume": -10,
          "soundEnabled": false,
          "editor.neovim.enabled": true,
          "syntaxTheme": "dracula"
        }
      }
    }"#;
    let c = parse_flutter_json(Path::new("settings.json"), json).unwrap();
    assert_eq!(c.theme_id, "gsh");
    assert_eq!(c.interface_font, None);
    assert_eq!(c.code_font.as_deref(), Some("JetBrains Mono"));
    assert_eq!(c.terminal.default_profile_id, None);
    assert_eq!(c.locale, None);
    assert_eq!(c.chat.font, None);
    assert_eq!(c.code.theme_light, None);
    assert_eq!(c.code.theme_dark.as_deref(), Some("nord"));
    assert_eq!(c.automation.harness_id, Some(HarnessKind::ClaudeCode));
    assert_eq!(c.automation.model_id.as_deref(), Some(""));
    assert_eq!(c.automation.model(), None);
    assert_eq!(c.sound.volume, 0.0);
    // Chaves de migrações antigas do Flutter são ignoradas.
    assert!(c.sound.events.is_empty());
    assert_eq!(c.editor.engine, FileEditorEngine::Gsh);
}

#[cfg(unix)]
#[test]
fn a01_unix_creates_gsh_dir_0700_and_config_toml_0600_without_altering_existing_dir() {
    use std::os::unix::fs::PermissionsExt;

    // 1. Diretório .gsh novo: criado com 0700, config.toml com 0600.
    let tmp_new = TempDir::new("a01-new-dir");
    let config_new = tmp_new.config_path();
    let flutter_new = tmp_new.flutter_path();
    write_file(&flutter_new, FULL_FLUTTER_SETTINGS_JSON);

    let outcome = migrate_if_needed(&config_new, [&flutter_new]).unwrap();
    assert_eq!(outcome, MigrationOutcome::Imported);

    let gsh_dir_mode = fs::metadata(config_new.parent().unwrap())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    let config_mode = fs::metadata(&config_new).unwrap().permissions().mode() & 0o777;
    assert_eq!(gsh_dir_mode, 0o700, "expected 0700, got {gsh_dir_mode:o}");
    assert_eq!(config_mode, 0o600, "expected 0600, got {config_mode:o}");

    // 2. Diretório .gsh já existente (ex.: 0755): não altera permissões do diretório.
    let tmp_existing = TempDir::new("a01-existing-dir");
    let config_existing = tmp_existing.config_path();
    let gsh_existing = config_existing.parent().unwrap();
    fs::create_dir_all(gsh_existing).unwrap();
    fs::set_permissions(gsh_existing, fs::Permissions::from_mode(0o755)).unwrap();

    let flutter_existing = tmp_existing.flutter_path();
    write_file(&flutter_existing, FULL_FLUTTER_SETTINGS_JSON);

    let outcome = migrate_if_needed(&config_existing, [&flutter_existing]).unwrap();
    assert_eq!(outcome, MigrationOutcome::Imported);

    let preserved_dir_mode = fs::metadata(gsh_existing).unwrap().permissions().mode() & 0o777;
    let file_mode = fs::metadata(&config_existing).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        preserved_dir_mode, 0o755,
        "existing dir mode should stay 0755, got {preserved_dir_mode:o}"
    );
    assert_eq!(file_mode, 0o600, "expected 0600, got {file_mode:o}");
}

#[test]
fn a02_debug_redacts_team_secret_across_all_containers() {
    let secret = "TOP_SECRET_LAN_KEY_999";
    let tmp = TempDir::new("a02-debug");
    let config_path = tmp.config_path();
    write_file(
        &config_path,
        &format!("[farm.team]\nenabled = true\nname = \"gorila\"\nsecret = \"{secret}\"\n"),
    );

    let (store, result) = ConfigStore::open(&config_path);
    assert!(result.is_ok());

    let config = store.current().clone();
    let farm = &config.farm;
    let team = &farm.team;
    assert_eq!(team.secret, secret);
    let event = ConfigEvent::Changed(Box::new(config.clone()));

    for formatted in [
        format!("{team:?}"),
        format!("{team:#?}"),
        format!("{farm:?}"),
        format!("{config:?}"),
        format!("{config:#?}"),
        format!("{store:?}"),
        format!("{event:?}"),
    ] {
        assert!(
            !formatted.contains(secret),
            "Debug leaked secret in: {formatted}"
        );
        assert!(
            formatted.contains("<redacted>"),
            "Debug missing <redacted> in: {formatted}"
        );
    }
}

#[test]
fn a03_relative_storage_root_returns_typed_error_and_migration_does_not_act() {
    let tmp = TempDir::new("a03-relative-root");
    let config_path = tmp.config_path();
    let storage_root_path = config_path.parent().unwrap().join("storage_root");
    let flutter_path = tmp.flutter_path();

    write_file(&storage_root_path, "relative/custom/root\n");
    write_file(&flutter_path, FULL_FLUTTER_SETTINGS_JSON);

    let err = migrate_if_needed(&config_path, [&flutter_path]).unwrap_err();
    match err {
        ConfigError::RelativeStorageRoot { path } => {
            assert_eq!(path, storage_root_path);
        }
        other => panic!("expected RelativeStorageRoot, got {other:?}"),
    }
    assert!(!config_path.exists());
}

#[test]
fn m01_exclusive_create_new_fails_if_config_appears_before_write() {
    let tmp = TempDir::new("m01-create-new");
    let config_path = tmp.config_path();
    let flutter_path = tmp.flutter_path();
    write_file(&flutter_path, FULL_FLUTTER_SETTINGS_JSON);

    let existing_content = "themeMode = \"light\"\ncodeSize = 21\n";
    let race_candidates = std::iter::once_with(|| {
        write_file(&config_path, existing_content);
        flutter_path.clone()
    });

    let err = migrate_if_needed(&config_path, race_candidates).unwrap_err();
    match err {
        ConfigError::AlreadyExists { path } => {
            assert_eq!(path, config_path);
        }
        other => panic!("expected AlreadyExists, got {other:?}"),
    }
    assert_eq!(fs::read_to_string(&config_path).unwrap(), existing_content);
}

#[test]
fn m02_expand_dotted_keys_rejects_excessive_depth_with_typed_error() {
    let tmp = TempDir::new("m02-depth");
    let config_path = tmp.config_path();
    let flutter_path = tmp.flutter_path();

    // 32 segmentos (no limite) passa e é ignorado como chave desconhecida.
    let ok_key = vec!["a"; 32].join(".");
    let ok_json = format!(r#"{{"version": 1, "data": {{"app": {{"{ok_key}": 1}}}}}}"#);
    assert_eq!(
        parse_flutter_json(&flutter_path, &ok_json).unwrap(),
        Config::default()
    );

    // 33 segmentos (acima do limite) e 2000 segmentos (muitos pontos) falham com erro tipado.
    for segments in [33usize, 2000] {
        let deep_key = vec!["a"; segments].join(".");
        let deep_json = format!(r#"{{"version": 1, "data": {{"app": {{"{deep_key}": 1}}}}}}"#);
        write_file(&flutter_path, &deep_json);

        let err = migrate_if_needed(&config_path, [&flutter_path]).unwrap_err();
        match err {
            ConfigError::KeyDepthExceeded { path, max } => {
                assert_eq!(path, flutter_path);
                assert_eq!(max, 32);
            }
            other => panic!("expected KeyDepthExceeded for {segments} segments, got {other:?}"),
        }
        assert!(!config_path.exists());
    }
}

#[test]
fn m03_rejects_non_regular_files_and_oversized_files_before_reading() {
    let max_bytes: usize = 1024 * 1024;

    // 1. Diretório no lugar de settings.json.
    let tmp_dir_settings = TempDir::new("m03-dir-settings");
    let config_path = tmp_dir_settings.config_path();
    let flutter_path = tmp_dir_settings.flutter_path();
    fs::create_dir_all(&flutter_path).unwrap();

    match migrate_if_needed(&config_path, [&flutter_path]) {
        Err(ConfigError::NotRegularFile { path }) => assert_eq!(path, flutter_path),
        other => panic!("expected NotRegularFile for settings.json dir, got {other:?}"),
    }
    assert!(!config_path.exists());

    // 2. settings.json maior que 1 MiB.
    let tmp_big_settings = TempDir::new("m03-big-settings");
    let config_path = tmp_big_settings.config_path();
    let flutter_path = tmp_big_settings.flutter_path();
    write_file(&flutter_path, &" ".repeat(max_bytes + 1));

    match migrate_if_needed(&config_path, [&flutter_path]) {
        Err(ConfigError::FileTooLarge { path, size, max }) => {
            assert_eq!(path, flutter_path);
            assert_eq!(size, (max_bytes + 1) as u64);
            assert_eq!(max, max_bytes as u64);
        }
        other => panic!("expected FileTooLarge for settings.json, got {other:?}"),
    }
    assert!(!config_path.exists());

    // 3. Diretório no lugar de storage_root.
    let tmp_dir_root = TempDir::new("m03-dir-root");
    let config_path = tmp_dir_root.config_path();
    let storage_root = config_path.parent().unwrap().join("storage_root");
    let flutter_path = tmp_dir_root.flutter_path();
    fs::create_dir_all(&storage_root).unwrap();
    write_file(&flutter_path, FULL_FLUTTER_SETTINGS_JSON);

    match migrate_if_needed(&config_path, [&flutter_path]) {
        Err(ConfigError::NotRegularFile { path }) => assert_eq!(path, storage_root),
        other => panic!("expected NotRegularFile for storage_root dir, got {other:?}"),
    }
    assert!(!config_path.exists());

    // 4. storage_root maior que 1 MiB.
    let tmp_big_root = TempDir::new("m03-big-root");
    let config_path = tmp_big_root.config_path();
    let storage_root = config_path.parent().unwrap().join("storage_root");
    let flutter_path = tmp_big_root.flutter_path();
    write_file(&storage_root, &"/".repeat(max_bytes + 1));
    write_file(&flutter_path, FULL_FLUTTER_SETTINGS_JSON);

    match migrate_if_needed(&config_path, [&flutter_path]) {
        Err(ConfigError::FileTooLarge { path, size, max }) => {
            assert_eq!(path, storage_root);
            assert_eq!(size, (max_bytes + 1) as u64);
            assert_eq!(max, max_bytes as u64);
        }
        other => panic!("expected FileTooLarge for storage_root, got {other:?}"),
    }
    assert!(!config_path.exists());
}

#[test]
fn b01_json_parse_error_carries_position_and_typed_cause_without_leaking_values() {
    let secret = "LEAKED_SECRET_VALUE_42";
    let path = Path::new("settings.json");

    let cases = [
        (
            format!(r#"{{"version": 1, "data": {{"app": {{"interfaceSize": "{secret}"}}}}}}"#),
            JsonParseCause::InvalidData,
            false,
        ),
        (
            format!(r#"{{"version": 1, "data": "{secret}"}}"#),
            JsonParseCause::DataNotObject,
            false,
        ),
        (
            format!("{{\n  \"version\": 1,\n  \"data\": {secret}\n}}"),
            JsonParseCause::Syntax,
            true,
        ),
    ];

    for (raw_json, expected_cause, has_position) in cases {
        let err = parse_flutter_json(path, &raw_json).unwrap_err();
        let display = err.to_string();
        let debug = format!("{err:?}");
        assert!(
            !display.contains(secret),
            "Display leaked value in: {display}"
        );
        assert!(!debug.contains(secret), "Debug leaked value in: {debug}");

        match err {
            ConfigError::JsonParse {
                path: p,
                line,
                column,
                cause,
            } => {
                assert_eq!(p, path);
                assert_eq!(cause, expected_cause);
                if has_position {
                    assert_eq!(line, Some(3));
                    assert!(column.is_some());
                }
            }
            other => panic!("expected JsonParse, got {other:?}"),
        }
    }
}

#[test]
fn b02_empty_team_secret_is_not_serialized_and_round_trips_as_default() {
    let default_toml = to_toml(&Config::default()).unwrap();
    assert!(
        !default_toml.contains("secret"),
        "empty secret should not be serialized: {default_toml}"
    );

    let tmp = TempDir::new("b02-empty-secret");
    let config_path = tmp.config_path();
    let flutter_path = tmp.flutter_path();
    write_file(
        &flutter_path,
        r#"{
          "version": 1,
          "data": {
            "app": {
              "farm.team.enabled": true,
              "farm.team.name": "gorila",
              "farm.team.secret": ""
            }
          }
        }"#,
    );

    let outcome = migrate_if_needed(&config_path, [&flutter_path]).unwrap();
    assert_eq!(outcome, MigrationOutcome::Imported);

    let written_toml = fs::read_to_string(&config_path).unwrap();
    assert!(
        !written_toml.contains("secret"),
        "migrated TOML should omit empty secret: {written_toml}"
    );

    let loaded = load(&config_path).unwrap();
    assert!(loaded.farm.team.enabled);
    assert_eq!(loaded.farm.team.name, "gorila");
    assert_eq!(loaded.farm.team.secret, "");
}
