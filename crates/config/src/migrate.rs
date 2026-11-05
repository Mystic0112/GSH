//! Migração única do `settings.json` do GSH em Flutter para o
//! `~/.gsh/config.toml`.
//!
//! ## Caminhos do `settings.json` no Flutter (`StorageLocation`)
//!
//! Em `lib/app/core/data/setup/storage_location.dart` e
//! `lib/app/core/data/repositories/json_settings_store.dart`, o Flutter grava
//! um `JsonStateStore` em `<raiz>/<subdir>/state/settings.json` (onde
//! `<subdir>` é `gsh` em release e `gsh-debug` em debug), no envelope:
//!
//! ```json
//! {"version": 1, "data": {"app": { ... }}}
//! ```
//!
//! Ordem das raízes candidatas ([`default_flutter_candidates`]):
//!
//! 1. **Pasta escolhida pelo usuário (qualquer SO)**: caminho absoluto gravado
//!    no ponteiro fixo `~/.gsh/storage_root` (`StorageLocation.overrideRoot`).
//! 2. **Windows** (`StorageLocation.effectiveRoot`):
//!    - `%APPDATA%\Gorila Software House\GSH Desktop`
//!      (`getApplicationSupportDirectory()` — `RoamingAppData\CompanyName\ProductName`
//!      de `windows/runner/Runner.rc`; o Flutter usa `AppData\Roaming` como
//!      padrão no Windows para evitar locks do OneDrive sobre `Documents`);
//!    - `%USERPROFILE%\Documents` (fallback legado em
//!      `StorageLocation.legacyHiveDirCandidates`).
//! 3. **macOS**: `~/Documents` (`getApplicationDocumentsDirectory()`, com
//!    app-sandbox desligado).
//! 4. **Linux**: `$XDG_DOCUMENTS_DIR` (ou `~/Documents` quando `user-dirs.dirs`
//!    não define `XDG_DOCUMENTS_DIR`, espelhando `xdg-user-dir DOCUMENTS`).
//!
//! Para cada raiz, tenta `gsh/state/settings.json` antes de
//! `gsh-debug/state/settings.json`.
//!
//! ## Mapeamento de chaves (`data.app` JSON → `config.toml`)
//!
//! As chaves de topo do JSON permanecem com o mesmo nome no TOML; as chaves com
//! ponto no JSON (`secao.chave` ou `secao.sub.chave`) viram tabelas TOML:
//!
//! | Chave em `data.app` (`settings.json`) | Tabela / chave em `config.toml` | Campo em [`Config`] |
//! |---|---|---|
//! | `themeMode` | `themeMode` | `theme_mode` |
//! | `themeId` | `themeId` | `theme_id` |
//! | `interfaceFont` | `interfaceFont` | `interface_font` |
//! | `interfaceSize` | `interfaceSize` | `interface_size` |
//! | `codeFont` | `codeFont` | `code_font` |
//! | `codeSize` | `codeSize` | `code_size` |
//! | `terminalFont` | `terminalFont` | `terminal_font` |
//! | `terminalSize` | `terminalSize` | `terminal_size` |
//! | `terminalFontWeight` | `terminalFontWeight` | `terminal_font_weight` |
//! | `pinUserMessage` | `pinUserMessage` | `pin_user_message` |
//! | `lspCommands` | `[lspCommands]` | `lsp_commands` |
//! | `lspFormatters` | `[lspFormatters]` | `lsp_formatters` |
//! | `formatOnSave` | `formatOnSave` | `format_on_save` |
//! | `closeTabOnShellExit` | `closeTabOnShellExit` | `close_tab_on_shell_exit` |
//! | `notificationsEnabled` | `notificationsEnabled` | `notifications_enabled` |
//! | `swapSidePanels` | `swapSidePanels` | `swap_side_panels` |
//! | `showGsh` | `showGsh` | `show_gsh` |
//! | `developerMode` | `developerMode` | `developer_mode` |
//! | `launchAtStartup` | `launchAtStartup` | `launch_at_startup` |
//! | `locale` | `locale` | `locale` |
//! | `updateCheckFrequency` | `updateCheckFrequency` | `update_check_frequency` |
//! | `editor.engine` | `[editor] engine` | `editor.engine` |
//! | `terminal.default_profile_id` | `[terminal] default_profile_id` | `terminal.default_profile_id` |
//! | `terminal.engine` | `[terminal] engine` | `terminal.engine` |
//! | `sound.volume` | `[sound] volume` | `sound.volume` |
//! | `sound.events` | `[sound.events]` | `sound.events` |
//! | `sound.onActiveTab` | `[sound.onActiveTab]` | `sound.on_active_tab` |
//! | `sourceControl.viewMode` | `[sourceControl] viewMode` | `source_control.view_mode` |
//! | `automation.harnessId` | `[automation] harnessId` | `automation.harness_id` |
//! | `automation.modelId` | `[automation] modelId` | `automation.model_id` |
//! | `chat.classifySessionStates` | `[chat] classifySessionStates` | `chat.classify_session_states` |
//! | `chat.remoteControl` | `[chat] remoteControl` | `chat.remote_control` |
//! | `chat.transcriptView` | `[chat] transcriptView` | `chat.transcript_view` |
//! | `chat.font` | `[chat] font` | `chat.font` |
//! | `chat.transcriptWidth` | `[chat] transcriptWidth` | `chat.transcript_width` |
//! | `code.themeLight` | `[code] themeLight` | `code.theme_light` |
//! | `code.themeDark` | `[code] themeDark` | `code.theme_dark` |
//! | `features.hidden` | `[features] hidden` | `features.hidden` |
//! | `links.openInApp` | `[links] openInApp` | `links.open_in_app` |
//! | `farm.scenery` | `[farm] scenery` | `farm.scenery` |
//! | `farm.team.enabled` | `[farm.team] enabled` | `farm.team.enabled` |
//! | `farm.team.name` | `[farm.team] name` | `farm.team.name` |
//! | `farm.team.me` | `[farm.team] me` | `farm.team.me` |
//! | `farm.team.showTitles` | `[farm.team] showTitles` | `farm.team.show_titles` |
//! | `farm.team.network` | `[farm.team] network` | `farm.team.network` |
//! | `farm.team.secret` | `[farm.team] secret` | `farm.team.secret` |
//! | `farm.team.doorAsk` | `[farm.team] doorAsk` | `farm.team.door_ask` |
//! | `farm.team.doorAllow` | `[farm.team] doorAllow` | `farm.team.door_allow` |
//!
//! ## O que fica de fora (ignorado na migração)
//!
//! - **Estado de UI/sessão gravado pelo próprio app** (não é configuração de
//!   usuário): `lastOpenAppId`, `searchPanelHeight`, `tasksPanelHeight`,
//!   `railVisible`, `treeVisible`, `lastUpdateCheckTime`,
//!   `chat.permissionMode`, `chat.effort`, `farm.plots`, além de chaves irmãs
//!   de `data.app` no mesmo `settings.json` (`dismissed_update_version` do
//!   `JsonDismissedUpdateStore` e `ssh_known_host_keys` do
//!   `JsonSshHostKeyStore`).
//! - **`sound.overrides`** (`soundOverrides`): aponta para cópias de áudio
//!   gerenciadas na subpasta `sounds/` do storage do Flutter (`SoundLibrary`).
//! - **Chaves de migrações antigas do Flutter** (já absorvidas pelo Flutter):
//!   `soundEnabled`, `editor.neovim.enabled`, `syntaxTheme`, `enableAgent`.
//!   (Os nomes antigos de harness em `automation.harnessId` — `claude`,
//!   `copilot`, `opencode`, `codex`, `pi` — **são** convertidos pelo
//!   deserializer de [`crate::schema::Automation`]; `gemini` cai em `None`).

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

use serde_json::{Map, Value};

use crate::error::{ConfigError, JsonParseCause};
use crate::schema::Config;

const STORAGE_ROOT_POINTER: &str = "storage_root";
const SUBDIRS: &[&str] = &["gsh", "gsh-debug"];
const STATE_DIR: &str = "state";
const SETTINGS_FILE: &str = "settings.json";
const WINDOWS_COMPANY: &str = "Gorila Software House";
const WINDOWS_PRODUCT: &str = "GSH Desktop";

/// Tamanho máximo (1 MiB) aceito para `settings.json` e `storage_root`.
pub const MAX_MIGRATION_FILE_BYTES: u64 = 1024 * 1024;

/// Profundidade máxima (número de segmentos separados por `.`) de uma chave em
/// `data.app` do `settings.json`.
pub const MAX_KEY_DEPTH: usize = 32;

/// Resultado tipado de [`migrate_if_needed`], sem texto de UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationOutcome {
    /// `config.toml` não existia; um `settings.json` do Flutter foi lido e
    /// gravado como TOML em `config_path`.
    Imported,
    /// `config.toml` não existia e nenhum `settings.json` candidato existia.
    NothingToImport,
    /// `config.toml` já existia; nada foi lido nem sobrescrito.
    AlreadyMigrated,
}

/// Caminhos candidatos ao `settings.json` do Flutter no SO atual, em ordem de
/// precedência:
///
/// 1. Raiz customizada em `~/.gsh/storage_root` (se existir e não for vazia;
///    deve ser arquivo regular, <= 1 MiB e conter caminho absoluto);
/// 2. No Windows: `%APPDATA%\Gorila Software House\GSH Desktop`
///    (`getApplicationSupportDirectory()`) e depois `Documents`;
/// 3. No macOS e Linux: `Documents` (`dirs::document_dir()`, com fallback para
///    `~/Documents` no Linux quando `XDG_DOCUMENTS_DIR` não está definido).
///
/// Para cada raiz, `gsh/state/settings.json` vem antes de
/// `gsh-debug/state/settings.json`.
pub fn default_flutter_candidates() -> Result<Vec<PathBuf>, ConfigError> {
    let home = dirs::home_dir();
    let override_root = match home.as_deref() {
        Some(h) => read_storage_root(&h.join(".gsh").join(STORAGE_ROOT_POINTER))?,
        None => None,
    };
    let data_dir = dirs::data_dir();
    let document_dir = dirs::document_dir().or_else(|| {
        if cfg!(target_os = "linux") {
            home.as_ref().map(|h| h.join("Documents"))
        } else {
            None
        }
    });
    Ok(candidates_for_platform(
        cfg!(windows),
        override_root.as_deref(),
        data_dir.as_deref(),
        document_dir.as_deref(),
    ))
}

/// Lê e valida o ponteiro `storage_root` em `pointer`.
///
/// - Arquivo ausente ou em branco: `Ok(None)`.
/// - Não é arquivo regular ou excede [`MAX_MIGRATION_FILE_BYTES`]: erro tipado.
/// - Caminho relativo: [`ConfigError::RelativeStorageRoot`].
pub fn read_storage_root(pointer: &Path) -> Result<Option<PathBuf>, ConfigError> {
    let Some(raw) = read_migration_file(pointer)? else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let root = PathBuf::from(trimmed);
    if !root.is_absolute() {
        return Err(ConfigError::RelativeStorageRoot {
            path: pointer.to_owned(),
        });
    }
    Ok(Some(root))
}

/// Lê um arquivo de migração (`settings.json` ou `storage_root`) somente após
/// checar que é arquivo regular (`metadata.is_file()`) e que não excede
/// [`MAX_MIGRATION_FILE_BYTES`]. Devolve `Ok(None)` quando `NotFound`.
fn read_migration_file(path: &Path) -> Result<Option<String>, ConfigError> {
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_owned(),
                source,
            });
        }
    };
    if !meta.is_file() {
        return Err(ConfigError::NotRegularFile {
            path: path.to_owned(),
        });
    }
    if meta.len() > MAX_MIGRATION_FILE_BYTES {
        return Err(ConfigError::FileTooLarge {
            path: path.to_owned(),
            size: meta.len(),
            max: MAX_MIGRATION_FILE_BYTES,
        });
    }

    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_owned(),
                source,
            });
        }
    };
    let fd_meta = file.metadata().map_err(|source| ConfigError::Read {
        path: path.to_owned(),
        source,
    })?;
    if !fd_meta.is_file() {
        return Err(ConfigError::NotRegularFile {
            path: path.to_owned(),
        });
    }
    if fd_meta.len() > MAX_MIGRATION_FILE_BYTES {
        return Err(ConfigError::FileTooLarge {
            path: path.to_owned(),
            size: fd_meta.len(),
            max: MAX_MIGRATION_FILE_BYTES,
        });
    }

    let mut text = String::new();
    file.take(MAX_MIGRATION_FILE_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|source| ConfigError::Read {
            path: path.to_owned(),
            source,
        })?;
    if (text.len() as u64) > MAX_MIGRATION_FILE_BYTES {
        return Err(ConfigError::FileTooLarge {
            path: path.to_owned(),
            size: text.len() as u64,
            max: MAX_MIGRATION_FILE_BYTES,
        });
    }
    Ok(Some(text))
}

/// Monta a lista de candidatos sem consultar o ambiente (pura, testável para
/// Windows e Unix em qualquer host).
pub(crate) fn candidates_for_platform(
    is_windows: bool,
    override_root: Option<&Path>,
    data_dir: Option<&Path>,
    document_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(custom) = override_root {
        push_unique(&mut roots, custom.to_owned());
    }
    if is_windows {
        if let Some(roaming) = data_dir {
            push_unique(
                &mut roots,
                roaming.join(WINDOWS_COMPANY).join(WINDOWS_PRODUCT),
            );
        }
    }
    if let Some(docs) = document_dir {
        push_unique(&mut roots, docs.to_owned());
    }

    let mut files = Vec::with_capacity(roots.len() * SUBDIRS.len());
    for root in &roots {
        for subdir in SUBDIRS {
            files.push(root.join(subdir).join(STATE_DIR).join(SETTINGS_FILE));
        }
    }
    files
}

fn push_unique(list: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !list.contains(&candidate) {
        list.push(candidate);
    }
}

/// Lê o `settings.json` do Flutter em `path` e devolve [`Config`] normalizado.
pub fn load_flutter_json(path: &Path) -> Result<Config, ConfigError> {
    let Some(text) = read_migration_file(path)? else {
        return Err(ConfigError::Read {
            path: path.to_owned(),
            source: io::Error::from(io::ErrorKind::NotFound),
        });
    };
    parse_flutter_json(path, &text)
}

/// Decodifica o conteúdo de um `settings.json` do Flutter (`{"version": 1,
/// "data": {"app": {...}}}`) usando o mesmo [`Config`] e a mesma normalização
/// do `fromJson`. `path` só entra no erro.
pub fn parse_flutter_json(path: &Path, text: &str) -> Result<Config, ConfigError> {
    let root: Value = serde_json::from_str(text).map_err(|e| ConfigError::JsonParse {
        path: path.to_owned(),
        line: nonzero(e.line()),
        column: nonzero(e.column()),
        cause: JsonParseCause::from_category(e.classify()),
    })?;

    let app_map = extract_app_map(path, root)?;
    let expanded = expand_dotted_keys(path, app_map)?;

    let config: Config =
        serde_json::from_value(Value::Object(expanded)).map_err(|e| ConfigError::JsonParse {
            path: path.to_owned(),
            line: nonzero(e.line()),
            column: nonzero(e.column()),
            cause: JsonParseCause::from_category(e.classify()),
        })?;

    config
        .normalized()
        .map_err(|(key, value)| ConfigError::NonFinite {
            path: path.to_owned(),
            key,
            value,
        })
}

/// Serializa [`Config`] para texto TOML pronto para gravar em `config.toml`.
pub fn to_toml(config: &Config) -> Result<String, ConfigError> {
    let normalized =
        config
            .clone()
            .normalized()
            .map_err(|(key, value)| ConfigError::NonFinite {
                path: PathBuf::from("config.toml"),
                key,
                value,
            })?;
    toml::to_string_pretty(&normalized).map_err(|e| ConfigError::Parse {
        path: PathBuf::from("config.toml"),
        line: None,
        column: None,
        cause: e.to_string(),
    })
}

/// Migra o primeiro `settings.json` existente em `flutter_candidates` para
/// `config_path`, **somente** quando `config_path` ainda não existe.
///
/// - Se `config_path` já existe: não lê candidatos nem sobrescreve, devolve
///   [`MigrationOutcome::AlreadyMigrated`].
/// - Se nenhum candidato existe: não cria `config_path`, devolve
///   [`MigrationOutcome::NothingToImport`].
/// - Se o candidato existe e é válido: grava `config_path` sem apagar nem
///   alterar o `settings.json` original e devolve [`MigrationOutcome::Imported`].
/// - Se o candidato existe mas é inválido: não cria `config_path` e devolve
///   [`ConfigError`] com o caminho do `settings.json` e a causa.
pub fn migrate_if_needed<I, P>(
    config_path: &Path,
    flutter_candidates: I,
) -> Result<MigrationOutcome, ConfigError>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    match fs::metadata(config_path) {
        Ok(_) => return Ok(MigrationOutcome::AlreadyMigrated),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(ConfigError::Read {
                path: config_path.to_owned(),
                source,
            });
        }
    }

    let override_root = match config_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => {
            read_storage_root(&parent.join(STORAGE_ROOT_POINTER))?
        }
        _ => None,
    };

    let mut found: Option<(PathBuf, String)> = None;
    if let Some(custom_root) = &override_root {
        for subdir in SUBDIRS {
            let candidate_path = custom_root.join(subdir).join(STATE_DIR).join(SETTINGS_FILE);
            if let Some(text) = read_migration_file(&candidate_path)? {
                found = Some((candidate_path, text));
                break;
            }
        }
    }
    if found.is_none() {
        for candidate in flutter_candidates {
            let candidate_path = candidate.as_ref();
            if let Some(text) = read_migration_file(candidate_path)? {
                found = Some((candidate_path.to_owned(), text));
                break;
            }
        }
    }

    let Some((source_path, text)) = found else {
        return Ok(MigrationOutcome::NothingToImport);
    };

    let config = parse_flutter_json(&source_path, &text)?;
    let toml_text = toml::to_string_pretty(&config).map_err(|e| ConfigError::Parse {
        path: source_path.clone(),
        line: None,
        column: None,
        cause: e.to_string(),
    })?;

    if let Some(parent) = config_path.parent() {
        if !parent.as_os_str().is_empty() {
            create_config_dir(parent).map_err(|source| ConfigError::Write {
                path: config_path.to_owned(),
                source,
            })?;
        }
    }
    write_config_exclusive(config_path, &toml_text)?;

    Ok(MigrationOutcome::Imported)
}

/// Cria o diretório de configuração (com modo `0700` no Unix) sem alterar as
/// permissões caso o diretório já exista.
fn create_config_dir(dir: &Path) -> io::Result<()> {
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(dir)
}

/// Grava `config.toml` com criação exclusiva (`create_new`, modo `0600` no
/// Unix). Se o arquivo surgiu entre a checagem inicial e a abertura, devolve
/// [`ConfigError::AlreadyExists`] sem sobrescrever.
fn write_config_exclusive(config_path: &Path, toml_text: &str) -> Result<(), ConfigError> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    opts.mode(0o600);

    let mut file = match opts.open(config_path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            return Err(ConfigError::AlreadyExists {
                path: config_path.to_owned(),
            });
        }
        Err(source) => {
            return Err(ConfigError::Write {
                path: config_path.to_owned(),
                source,
            });
        }
    };

    file.write_all(toml_text.as_bytes())
        .map_err(|source| ConfigError::Write {
            path: config_path.to_owned(),
            source,
        })
}

fn nonzero(n: usize) -> Option<usize> {
    if n == 0 {
        None
    } else {
        Some(n)
    }
}

/// Valida o envelope `{"version": 1, "data": {"app": {...}}}` do
/// `JsonStateStore` e devolve o mapa de `"app"`.
fn extract_app_map(path: &Path, root: Value) -> Result<Map<String, Value>, ConfigError> {
    let Value::Object(mut root_obj) = root else {
        return Err(envelope_error(path, JsonParseCause::RootNotObject));
    };

    if let Some(version) = root_obj.get("version") {
        if version.as_u64() != Some(1) {
            return Err(envelope_error(path, JsonParseCause::UnsupportedVersion));
        }
    }

    let Some(data_val) = root_obj.remove("data") else {
        return Err(envelope_error(path, JsonParseCause::MissingData));
    };
    let Value::Object(mut data_obj) = data_val else {
        return Err(envelope_error(path, JsonParseCause::DataNotObject));
    };

    match data_obj.remove("app") {
        None | Some(Value::Null) => Ok(Map::new()),
        Some(Value::Object(app)) => Ok(app),
        Some(_) => Err(envelope_error(path, JsonParseCause::AppNotObject)),
    }
}

fn envelope_error(path: &Path, cause: JsonParseCause) -> ConfigError {
    ConfigError::JsonParse {
        path: path.to_owned(),
        line: None,
        column: None,
        cause,
    }
}

/// Expande chaves com ponto do mapa `"app"` (`"chat.font"` → `{"chat": {"font": ...}}`,
/// `"farm.team.name"` → `{"farm": {"team": {"name": ...}}}`) e descarta entradas
/// com valor `null` (no Flutter, `null` = ausente/padrão).
///
/// Limita a profundidade a [`MAX_KEY_DEPTH`] segmentos de forma iterativa.
fn expand_dotted_keys(
    path: &Path,
    app: Map<String, Value>,
) -> Result<Map<String, Value>, ConfigError> {
    let mut root = Map::new();
    for (full_key, value) in app {
        if value.is_null() {
            continue;
        }
        if full_key.split('.').nth(MAX_KEY_DEPTH).is_some() {
            return Err(ConfigError::KeyDepthExceeded {
                path: path.to_owned(),
                max: MAX_KEY_DEPTH,
            });
        }
        let mut parts = full_key.split('.').peekable();
        let mut current = &mut root;
        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                if current.contains_key(part) {
                    return Err(envelope_error(path, JsonParseCause::ConflictingKey));
                }
                current.insert(part.to_owned(), value);
                break;
            }
            let entry = current
                .entry(part.to_owned())
                .or_insert_with(|| Value::Object(Map::new()));
            match entry {
                Value::Object(map) => current = map,
                _ => return Err(envelope_error(path, JsonParseCause::ConflictingKey)),
            }
        }
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_candidates_prioritize_override_then_appdata_then_documents() {
        let custom = Path::new(r"D:\Sync\GSH");
        let roaming = Path::new(r"C:\Users\helio\AppData\Roaming");
        let docs = Path::new(r"C:\Users\helio\Documents");

        let list = candidates_for_platform(true, Some(custom), Some(roaming), Some(docs));
        let expected_app_data = roaming.join("Gorila Software House").join("GSH Desktop");

        assert_eq!(
            list,
            vec![
                custom.join("gsh").join("state").join("settings.json"),
                custom.join("gsh-debug").join("state").join("settings.json"),
                expected_app_data
                    .join("gsh")
                    .join("state")
                    .join("settings.json"),
                expected_app_data
                    .join("gsh-debug")
                    .join("state")
                    .join("settings.json"),
                docs.join("gsh").join("state").join("settings.json"),
                docs.join("gsh-debug").join("state").join("settings.json"),
            ]
        );
    }

    #[test]
    fn unix_candidates_use_override_then_documents_and_deduplicate() {
        let docs = Path::new("/home/helio/Documents");
        let list = candidates_for_platform(
            false,
            Some(docs),
            Some(Path::new("/home/helio/.local/share")),
            Some(docs),
        );

        assert_eq!(
            list,
            vec![
                docs.join("gsh").join("state").join("settings.json"),
                docs.join("gsh-debug").join("state").join("settings.json"),
            ]
        );
    }
}
