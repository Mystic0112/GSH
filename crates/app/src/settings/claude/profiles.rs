use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use gsh_config::Config;

use super::widgets::shell_quote;

pub const CONFIG_DIR: &str = "CLAUDE_CONFIG_DIR";

pub fn default_dir() -> Option<PathBuf> {
    std::env::var_os(CONFIG_DIR)
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".claude")))
}

pub fn selected(config: &Config) -> Result<Option<PathBuf>, String> {
    if !cfg!(target_os = "linux") {
        return Ok(None);
    }
    config
        .claude
        .active_profile
        .as_deref()
        .map(|name| directory(config, name))
        .transpose()
}

pub fn directory(config: &Config, name: &str) -> Result<PathBuf, String> {
    let path = config
        .claude
        .profiles
        .get(name)
        .ok_or_else(|| format!("Perfil do Claude desconhecido: {name}"))?;
    validate_name(name)?;
    validate_distinct(config, name, Path::new(path))?;
    verify(Path::new(path), false)
}

pub fn verify_registered(config: &Config, path: &Path) -> Result<PathBuf, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Pasta do perfil indisponível: {error}"))?;
    let registered = config
        .claude
        .profiles
        .values()
        .filter_map(|candidate| Path::new(candidate).canonicalize().ok())
        .any(|candidate| candidate == canonical);
    if !registered {
        return Err("Este perfil Claude não está mais vinculado nas configurações.".into());
    }
    verify(path, false)
}

pub fn register(config: &Config, name: &str, path: &str) -> Result<PathBuf, String> {
    validate_name(name)?;
    if config.claude.profiles.contains_key(name) {
        return Err("Já existe um perfil com esse nome.".into());
    }
    let path = normalize(Path::new(path))?;
    validate_distinct(config, name, &path)?;
    verify(&path, true)
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name != name.trim()
        || name.chars().count() > 64
        || name.chars().any(char::is_control)
    {
        return Err("Use um nome de perfil de 1 a 64 caracteres.".into());
    }
    Ok(())
}

fn normalize(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || !path
            .to_str()
            .is_some_and(|s| !s.chars().any(char::is_control))
        || path.components().any(|part| part == Component::ParentDir)
        || path.parent().is_none()
    {
        return Err("Informe um caminho absoluto para a pasta do perfil, sem '..'.".into());
    }
    Ok(path.components().collect())
}

fn validate_distinct(config: &Config, name: &str, path: &Path) -> Result<(), String> {
    let path = normalize(path)?;
    let conflicts = |other: &Path| {
        let other = other.canonicalize().unwrap_or_else(|_| other.to_path_buf());
        path.starts_with(&other) || other.starts_with(&path)
    };
    if default_dir().as_deref().is_some_and(conflicts)
        || dirs::home_dir().is_some_and(|home| conflicts(&home.join(".claude")))
        || config
            .claude
            .profiles
            .iter()
            .any(|(other, dir)| other != name && conflicts(Path::new(dir)))
    {
        return Err("Use uma pasta separada da conta padrão e dos outros perfis.".into());
    }
    Ok(())
}

pub fn verify(path: &Path, create: bool) -> Result<PathBuf, String> {
    let path = normalize(path)?;
    let io_error =
        |error: io::Error| format!("Pasta do perfil do Claude ({}): {error}", path.display());
    #[cfg(unix)]
    let owner = unsafe { libc::geteuid() };
    for ancestor in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
        let metadata = match fs::symlink_metadata(ancestor) {
            Ok(metadata) => metadata,
            Err(error) if create && ancestor == path && error.kind() == io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    builder.mode(0o700);
                }
                if let Err(error) = builder.create(ancestor) {
                    if error.kind() != io::ErrorKind::AlreadyExists {
                        return Err(io_error(error));
                    }
                }
                fs::symlink_metadata(ancestor).map_err(io_error)?
            }
            Err(error) => return Err(io_error(error)),
        };
        if !metadata.is_dir() {
            return Err(format!("Pasta do perfil inválida: {}", ancestor.display()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if ancestor == path && (metadata.uid() != owner || metadata.mode() & 0o077 != 0) {
                return Err(
                    "A pasta do perfil precisa pertencer a você e ter gravação exclusiva.".into(),
                );
            }
            if (metadata.uid() != 0 && metadata.uid() != owner)
                || (metadata.mode() & 0o022 != 0 && metadata.mode() & 0o1000 == 0)
            {
                return Err(format!(
                    "Outra conta pode substituir esta pasta: {}",
                    ancestor.display()
                ));
            }
        }
    }
    for (entry, directory) in [
        (".credentials.json", false),
        (".claude.json", false),
        ("settings.json", false),
        ("history.jsonl", false),
        ("projects", true),
    ] {
        let entry = path.join(entry);
        let metadata = match fs::symlink_metadata(&entry) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(io_error(error)),
        };
        if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
            return Err(format!(
                "Entrada inválida na pasta do perfil: {}",
                entry.display()
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.uid() != owner || (!directory && metadata.mode() & 0o022 != 0) {
                return Err(format!(
                    "A entrada do perfil precisa pertencer a você e não ser alterável por terceiros: {}",
                    entry.display()
                ));
            }
            if !directory && metadata.nlink() != 1 {
                return Err(format!(
                    "Arquivo compartilhado na pasta do perfil: {}",
                    entry.display()
                ));
            }
        }
    }
    Ok(path)
}

pub fn env(path: &Path) -> Result<Vec<(String, String)>, String> {
    let path = verify(path, false)?;
    Ok(vec![(
        CONFIG_DIR.into(),
        path.to_string_lossy().into_owned(),
    )])
}

pub fn command(path: Option<&Path>, args: &str) -> Result<String, String> {
    let Some(path) = path else {
        return Ok(format!("claude {args}"));
    };
    let path = verify(path, false)?;
    let mut prefix = String::from("env");
    for key in gsh_chat::settings::PROFILE_AUTH_ENV {
        prefix.push_str(&format!(" -u {key}"));
    }
    let scope = shell_quote(&format!("{CONFIG_DIR}={}", path.to_string_lossy()));
    Ok(format!("{prefix} {scope} claude {args}"))
}

pub fn quote_error(error: &str) -> String {
    shell_quote(
        &error
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>(),
    )
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn profile_registration_defends_directory_and_known_auth_files() {
        let scratch = tempfile::tempdir().unwrap();
        fs::set_permissions(scratch.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let profile = scratch.path().join("Equipe ' A");
        let mut config = Config::default();
        let path = register(&config, "Trabalho", profile.to_str().unwrap()).unwrap();
        assert_eq!(path, profile);
        assert_eq!(
            fs::metadata(&profile).unwrap().permissions().mode() & 0o777,
            0o700
        );
        config
            .claude
            .profiles
            .insert("Trabalho".into(), profile.to_string_lossy().into());
        config.claude.active_profile = Some("Trabalho".into());
        assert_eq!(selected(&config), Ok(Some(profile.clone())));
        assert!(register(&config, "Outro", profile.to_str().unwrap()).is_err());
        assert!(register(&config, "Trabalho", "/tmp/outro").is_err());
        assert!(register(&config, "Outro", "/tmp/../tmp").is_err());
        assert!(register(&config, "Outro", "relative").is_err());

        let sibling = scratch.path().join("alias");
        symlink(&profile, &sibling).unwrap();
        assert!(verify(&sibling, false).is_err());
        let elsewhere = scratch.path().join("another-account");
        fs::write(&elsewhere, "do not read").unwrap();
        symlink(elsewhere, profile.join(".credentials.json")).unwrap();
        assert!(selected(&config).is_err());
    }

    #[test]
    fn login_command_preserves_path_and_drops_ambient_authentication() {
        let scratch = tempfile::tempdir().unwrap();
        fs::set_permissions(scratch.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let profile = scratch.path().join("p' $(exit 9)");
        fs::create_dir(&profile).unwrap();
        fs::set_permissions(&profile, fs::Permissions::from_mode(0o700)).unwrap();
        let claude = scratch.path().join("claude");
        fs::write(&claude, "#!/bin/sh\n[ -z \"${ANTHROPIC_API_KEY+x}\" ] || exit 1\nprintf '%s\\n' \"$CLAUDE_CONFIG_DIR\" \"$*\"\n").unwrap();
        fs::set_permissions(&claude, fs::Permissions::from_mode(0o700)).unwrap();
        let output = std::process::Command::new("sh")
            .env(
                "PATH",
                std::env::join_paths([scratch.path(), Path::new("/usr/bin")]).unwrap(),
            )
            .env("ANTHROPIC_API_KEY", "ambient")
            .arg("-c")
            .arg(command(Some(&profile), "auth login").unwrap())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{}\nauth login\n", profile.display())
        );
    }

    #[test]
    fn profile_rejects_group_writable_known_files() {
        let scratch = tempfile::tempdir().unwrap();
        fs::set_permissions(scratch.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let profile = scratch.path().join("private-profile");
        fs::create_dir(&profile).unwrap();
        fs::set_permissions(&profile, fs::Permissions::from_mode(0o700)).unwrap();
        let credentials = profile.join(".credentials.json");
        fs::write(&credentials, "{}").unwrap();
        fs::set_permissions(&credentials, fs::Permissions::from_mode(0o664)).unwrap();
        assert!(verify(&profile, false).is_err());
    }
}
