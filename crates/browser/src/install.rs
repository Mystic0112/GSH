use crate::BrowserError;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Read;
pub fn verify_sha256(bytes: &[u8], expected: &str) -> bool {
    format!("{:x}", Sha256::digest(bytes)).eq_ignore_ascii_case(expected.trim())
}
pub fn asset_name(os: &str, arch: &str) -> Result<&'static str, BrowserError> {
    match (os, arch) {
        ("linux", "x86_64") => Ok("obscura-linux-x86_64.tar.gz"),
        ("linux", "aarch64") => Ok("obscura-linux-aarch64.tar.gz"),
        ("macos", "x86_64") => Ok("obscura-macos-x86_64.tar.gz"),
        ("macos", "aarch64") => Ok("obscura-macos-aarch64.tar.gz"),
        ("windows", "x86_64") => Ok("obscura-windows-x86_64.zip"),
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
    if !verify_sha256(&bytes, expected) {
        return Err(BrowserError::Message(
            "Obscura checksum mismatch; refusing install".into(),
        ));
    }
    Err(BrowserError::Message(
        "verified release; safe archive extraction is not available".into(),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_and_assets() {
        assert!(verify_sha256(
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ));
        assert_eq!(
            asset_name("windows", "x86_64").ok(),
            Some("obscura-windows-x86_64.zip")
        );
        assert!(asset_name("freebsd", "x86_64").is_err())
    }
}
