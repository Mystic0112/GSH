use crate::BrowserError;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Cursor, Read, Write},
    path::{Component, Path},
};
pub fn verify_sha256(bytes: &[u8], expected: &str) -> bool {
    format!("{:x}", Sha256::digest(bytes)).eq_ignore_ascii_case(expected.trim())
}
pub fn asset_name(os: &str, arch: &str) -> Result<&'static str, BrowserError> {
    match (os, arch) {
        ("linux", "x86_64") => Ok("obscura-x86_64-linux.tar.gz"),
        ("linux", "aarch64") => Ok("obscura-aarch64-linux.tar.gz"),
        ("macos", "x86_64" | "x64") => Ok("obscura-x86_64-macos.tar.gz"),
        ("macos", "aarch64" | "arm64") => Ok("obscura-aarch64-macos.tar.gz"),
        ("windows", "x86_64") => Ok("obscura-x86_64-windows.zip"),
        _ => Err(BrowserError::Message(format!(
            "unsupported Obscura platform: {os}/{arch}"
        ))),
    }
}
pub fn install() -> Result<(), BrowserError> {
    let asset = asset_name(std::env::consts::OS, std::env::consts::ARCH)?;
    let client = reqwest::blocking::Client::builder()
        .build()
        .map_err(|e| BrowserError::Message(e.to_string()))?;
    let release: Value = client
        .get("https://api.github.com/repos/h4ckf0r0day/obscura/releases/latest")
        .header("User-Agent", "gsh-browser")
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|e| BrowserError::Message(e.to_string()))?;
    let a = release["assets"]
        .as_array()
        .and_then(|xs| xs.iter().find(|x| x["name"] == asset))
        .ok_or_else(|| BrowserError::Message(format!("release asset not found: {asset}")))?;
    let url = a["browser_download_url"]
        .as_str()
        .ok_or_else(|| BrowserError::Message("missing download URL".into()))?;
    let expected = a["digest"]
        .as_str()
        .and_then(|s| s.strip_prefix("sha256:"))
        .ok_or_else(|| {
            BrowserError::Message("release API has no SHA-256 digest; refusing install".into())
        })?;
    let mut bytes = Vec::new();
    client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| BrowserError::Message(e.to_string()))?
        .take(512 * 1024 * 1024)
        .read_to_end(&mut bytes)?;
    let version = release["tag_name"]
        .as_str()
        .and_then(|s| s.strip_prefix('v'))
        .ok_or_else(|| BrowserError::Message("release has no version tag".into()))?;
    let root = dirs::data_dir()
        .ok_or_else(|| BrowserError::Message("cannot locate data directory".into()))?
        .join("work.gorila.gshdesktop/obscura");
    let temp = root.join(format!(".{version}.tmp-{}", std::process::id()));
    if temp.exists() {
        fs::remove_dir_all(&temp)?;
    }
    fs::create_dir_all(&temp)?;
    let extraction = extract_verified(&bytes, expected, &temp, asset.ends_with(".zip"));
    if let Err(e) = extraction {
        let _ = fs::remove_dir_all(&temp);
        return Err(e);
    }
    let dest = root.join(version);
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    fs::rename(&temp, &dest)?;
    Ok(())
}

fn extract_verified(
    bytes: &[u8],
    expected: &str,
    dest: &Path,
    windows: bool,
) -> Result<(), BrowserError> {
    if !verify_sha256(bytes, expected) {
        return Err(BrowserError::Message(
            "Obscura checksum mismatch; refusing install".into(),
        ));
    }
    if windows {
        extract_zip(bytes, dest)
    } else {
        extract_tar(bytes, dest)
    }
}

fn safe_name(path: &Path, windows: bool) -> Result<String, BrowserError> {
    if path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(BrowserError::Message("unsafe archive path".into()));
    }
    let s = path
        .to_str()
        .ok_or_else(|| BrowserError::Message("archive path is not UTF-8".into()))?;
    if s.contains('\\') {
        return Err(BrowserError::Message("unsafe archive path".into()));
    }
    let name = s.rsplit('/').next().unwrap_or("");
    if windows
        && (name.ends_with('.')
            || name.ends_with(' ')
            || name.contains(':')
            || ["CON", "PRN", "AUX", "NUL"]
                .iter()
                .any(|r| name.split('.').next().unwrap_or("").eq_ignore_ascii_case(r))
            || (name.len() == 4
                && ["COM", "LPT"].iter().any(|r| {
                    name.get(..3)
                        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(r))
                        && name.as_bytes()[3].is_ascii_digit()
                })))
    {
        return Err(BrowserError::Message(
            "reserved Windows archive path".into(),
        ));
    }
    let accepted = if windows {
        name == "obscura.exe"
            || name == "obscura-worker.exe"
            || name.to_ascii_lowercase().ends_with(".dll")
    } else {
        name == "obscura" || name == "obscura-worker"
    };
    if !accepted || path.components().count() != 1 {
        return Err(BrowserError::Message(
            "unexpected file in Obscura archive".into(),
        ));
    }
    Ok(name.into())
}

fn extract_tar(bytes: &[u8], dest: &Path) -> Result<(), BrowserError> {
    let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
    let mut archive = tar::Archive::new(decoder);
    let mut files = Vec::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let ty = entry.header().entry_type();
        if !ty.is_file() {
            return Err(BrowserError::Message(
                "archive links and non-files are rejected".into(),
            ));
        }
        let name = safe_name(&entry.path()?, false)?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        files.push((name, data));
    }
    validate_required(&files, false)?;
    write_files(dest, files, false)
}
fn extract_zip(bytes: &[u8], dest: &Path) -> Result<(), BrowserError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| BrowserError::Message(e.to_string()))?;
    let mut files = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| BrowserError::Message(e.to_string()))?;
        if entry.is_dir() {
            let _ = safe_name(Path::new(entry.name().trim_end_matches('/')), true)?;
            continue;
        }
        if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err(BrowserError::Message(
                "archive symlinks are rejected".into(),
            ));
        }
        let name = safe_name(Path::new(entry.name()), true)?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        files.push((name, data));
    }
    validate_required(&files, true)?;
    write_files(dest, files, true)
}
fn validate_required(files: &[(String, Vec<u8>)], windows: bool) -> Result<(), BrowserError> {
    let required = if windows {
        ["obscura.exe", "obscura-worker.exe"]
    } else {
        ["obscura", "obscura-worker"]
    };
    for name in required {
        if !files.iter().any(|(n, _)| n == name) {
            return Err(BrowserError::Message(format!("archive is missing {name}")));
        }
    }
    Ok(())
}
fn write_files(
    dest: &Path,
    files: Vec<(String, Vec<u8>)>,
    windows: bool,
) -> Result<(), BrowserError> {
    for (name, data) in files {
        let path = dest.join(&name);
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        f.write_all(&data)?;
        #[cfg(unix)]
        if !windows {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn archive(entries: &[(&str, tar::EntryType, &[u8])]) -> Vec<u8> {
        let gz = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = tar::Builder::new(gz);
        for (path, kind, data) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_entry_type(*kind);
            h.set_size(data.len() as u64);
            h.set_mode(0o755);
            h.set_cksum();
            tar.append_data(&mut h, path, *data).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }
    fn malicious_archive(path: &str) -> Vec<u8> {
        let bytes = archive(&[
            ("obscura", tar::EntryType::Regular, b"bin"),
            ("obscura-worker", tar::EntryType::Regular, b"worker"),
            ("evil", tar::EntryType::Regular, b"bad"),
        ]);
        let mut raw = Vec::new();
        flate2::read::GzDecoder::new(Cursor::new(bytes))
            .read_to_end(&mut raw)
            .unwrap();
        let header = &mut raw[1024..1536];
        header[..100].fill(0);
        header[..path.len()].copy_from_slice(path.as_bytes());
        header[148..156].fill(b' ');
        let sum: usize = header.iter().map(|b| usize::from(*b)).sum();
        let checksum = format!("{sum:06o}\0 ");
        header[148..156].copy_from_slice(checksum.as_bytes());
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        gz.write_all(&raw).unwrap();
        gz.finish().unwrap()
    }
    fn temp() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "gsh-browser-install-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn hash_and_assets() {
        assert!(verify_sha256(
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ));
        assert_eq!(
            asset_name("windows", "x86_64").ok(),
            Some("obscura-x86_64-windows.zip")
        );
        assert_eq!(
            asset_name("linux", "x86_64").ok(),
            Some("obscura-x86_64-linux.tar.gz")
        );
        assert_eq!(
            asset_name("linux", "aarch64").ok(),
            Some("obscura-aarch64-linux.tar.gz")
        );
        assert_eq!(
            asset_name("macos", "aarch64").ok(),
            Some("obscura-aarch64-macos.tar.gz")
        );
        assert_eq!(
            asset_name("macos", "x86_64").ok(),
            Some("obscura-x86_64-macos.tar.gz")
        );
        assert_eq!(
            asset_name("macos", "arm64").ok(),
            Some("obscura-aarch64-macos.tar.gz")
        );
        assert_eq!(
            asset_name("macos", "x64").ok(),
            Some("obscura-x86_64-macos.tar.gz")
        );
        assert!(asset_name("freebsd", "x86_64").is_err())
    }
    #[test]
    fn extrai_tar_e_define_execucao() {
        let p = temp();
        let b = archive(&[
            ("obscura", tar::EntryType::Regular, b"bin"),
            ("obscura-worker", tar::EntryType::Regular, b"worker"),
        ]);
        extract_tar(&b, &p).unwrap();
        assert_eq!(fs::read(p.join("obscura")).unwrap(), b"bin");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(p.join("obscura"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o755
            );
        }
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn rejeita_traversal_e_absoluto_sem_gravar() {
        for path in ["../evil", "/tmp/evil"] {
            let p = temp();
            let b = malicious_archive(path);
            assert!(extract_tar(&b, &p).is_err());
            assert_eq!(fs::read_dir(&p).unwrap().count(), 0);
            fs::remove_dir_all(p).unwrap();
        }
    }
    #[test]
    fn rejeita_link_simbolico() {
        let p = temp();
        let b = archive(&[
            ("obscura", tar::EntryType::Regular, b"bin"),
            ("obscura-worker", tar::EntryType::Regular, b"worker"),
            ("evil", tar::EntryType::Symlink, b"/tmp/evil"),
        ]);
        assert!(extract_tar(&b, &p).is_err());
        assert_eq!(fs::read_dir(&p).unwrap().count(), 0);
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn hash_invalido_impede_extracao() {
        let p = temp();
        let b = archive(&[
            ("obscura", tar::EntryType::Regular, b"bin"),
            ("obscura-worker", tar::EntryType::Regular, b"worker"),
        ]);
        assert!(extract_verified(&b, "00", &p, false).is_err());
        assert_eq!(fs::read_dir(&p).unwrap().count(), 0);
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    #[ignore = "baixa 72 MB da internet"]
    fn download_release_real_e_confere_checksum() {
        let asset = asset_name("linux", "x86_64").unwrap();
        let client = reqwest::blocking::Client::new();
        let release: Value = client
            .get("https://api.github.com/repos/h4ckf0r0day/obscura/releases/latest")
            .header("User-Agent", "gsh-browser-test")
            .send()
            .unwrap()
            .json()
            .unwrap();
        let item = release["assets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["name"] == asset)
            .unwrap();
        let expected = item["digest"]
            .as_str()
            .unwrap()
            .strip_prefix("sha256:")
            .unwrap();
        let bytes = client
            .get(item["browser_download_url"].as_str().unwrap())
            .send()
            .unwrap()
            .bytes()
            .unwrap();
        assert!(verify_sha256(&bytes, expected));
        let p = temp();
        extract_tar(&bytes, &p).unwrap();
        fs::remove_dir_all(p).unwrap();
    }
}
