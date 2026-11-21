use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const VERSION: &str = "2.0";
pub const MULTICAST: std::net::Ipv4Addr = std::net::Ipv4Addr::new(224, 0, 0, 167);
pub const PORT: u16 = 53317;
pub const API: &str = "/api/localsend/v2";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub alias: String,
    pub version: String,
    pub device_model: Option<String>,
    pub device_type: Option<String>,
    pub fingerprint: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub download: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Announcement {
    #[serde(flatten)]
    pub info: DeviceInfo,
    #[serde(default)]
    pub announce: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub id: String,
    pub file_name: String,
    pub size: u64,
    pub file_type: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareUpload {
    pub info: DeviceInfo,
    pub files: BTreeMap<String, FileMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadSession {
    pub session_id: String,
    pub files: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announcement_uses_protocol_field_names() {
        let payload = r#"{"alias":"Desk","version":"2.0","deviceModel":null,"deviceType":"desktop","fingerprint":"abc","port":53317,"protocol":"https","download":true,"announce":true}"#;
        let value: Announcement = serde_json::from_str(payload).unwrap();
        assert_eq!(value.info.alias, "Desk");
        assert!(value.announce);
        let encoded = serde_json::to_value(value).unwrap();
        assert_eq!(encoded["deviceType"], "desktop");
    }

    #[test]
    fn prepare_upload_round_trips_protocol_metadata() {
        let file = FileMetadata {
            id: "a".into(),
            file_name: "x.txt".into(),
            size: 3,
            file_type: "text/plain".into(),
            sha256: None,
            preview: None,
            metadata: None,
        };
        let encoded = serde_json::to_value(file).unwrap();
        assert_eq!(encoded["fileName"], "x.txt");
        assert_eq!(encoded["size"], 3);
    }
}
