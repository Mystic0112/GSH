//! Vigia de conectividade do Modo Bolsa no Linux.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const MIN_PREFERRED_WIFI_SIGNAL: u8 = 40;
pub const POLL_INTERVAL: Duration = Duration::from_secs(30);
pub const HOTSPOT_RETRY_INTERVAL: Duration = Duration::from_secs(120);

#[derive(Default)]
pub(crate) struct BagModeState {
    pub config: BagModeConfig,
    pub preferred_draft: String,
    pub work: Option<Arc<AtomicBool>>,
    pub status: Option<Result<String, String>>,
    pub poll_at: Option<Instant>,
    pub preferred_retry_at: Option<Instant>,
}

impl BagModeState {
    pub fn is_editable(&self) -> bool {
        !self.config.enabled && self.work.is_none()
    }

    pub fn cancel_work(&self) {
        if let Some(work) = &self.work {
            work.store(true, Ordering::Relaxed);
        }
    }
}

impl Drop for BagModeState {
    fn drop(&mut self) {
        self.cancel_work();
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BagModeConfig {
    pub enabled: bool,
    pub hotspot_ssid: String,
    pub preferred_wifi_ssids: Vec<String>,
}

impl BagModeConfig {
    pub fn validate(&self) -> Result<(), String> {
        let hotspot = sanitize_ssid(&self.hotspot_ssid)
            .ok_or_else(|| "Informe um SSID de hotspot válido (até 32 bytes).".to_owned())?;
        if self
            .preferred_wifi_ssids
            .iter()
            .any(|ssid| sanitize_ssid(ssid).is_none_or(|ssid| ssid == hotspot))
        {
            return Err("Informe SSIDs de Wi-Fi válidos e diferentes do hotspot.".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleWifi {
    pub in_use: bool,
    pub ssid: String,
    pub signal: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Connectivity {
    Full,
    Offline,
    Unknown,
}

impl Connectivity {
    fn from_nmcli(stdout: &str) -> Self {
        match stdout.trim() {
            "full" => Self::Full,
            "none" | "portal" | "limited" => Self::Offline,
            _ => Self::Unknown,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Full => "internet disponível",
            Self::Offline => "sem acesso à internet",
            Self::Unknown => "acesso à internet não confirmado",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BagPoll {
    pub status: String,
    pub switched_to_hotspot: bool,
}

fn unescape_nmcli_terse(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            out.push(chars.next().unwrap_or('\\'));
        } else {
            out.push(ch);
        }
    }
    out
}

fn sanitize_ssid(ssid: &str) -> Option<&str> {
    let trimmed = ssid.trim();
    if trimmed.is_empty() || trimmed.len() > 32 || ssid.chars().any(char::is_control) {
        return None;
    }
    Some(trimmed)
}

pub fn parse_nmcli_wifi_list(stdout: &str) -> Vec<VisibleWifi> {
    stdout
        .lines()
        .filter_map(|line| {
            let (in_use, rest) = line.split_once(':')?;
            let (name, signal) = rest.rsplit_once(':')?;
            let signal = signal.trim().parse::<u8>().ok().filter(|s| *s <= 100)?;
            let name = unescape_nmcli_terse(name);
            Some(VisibleWifi {
                in_use: in_use.trim() == "*",
                ssid: sanitize_ssid(&name)?.to_owned(),
                signal,
            })
        })
        .collect()
}

fn evaluate_wifi_switch<'a>(
    config: &'a BagModeConfig,
    visible: &'a [VisibleWifi],
    connectivity: Connectivity,
    defer_preferred: bool,
) -> Option<&'a str> {
    if !config.enabled {
        return None;
    }
    let hotspot = sanitize_ssid(&config.hotspot_ssid)?;
    let on_hotspot = visible
        .iter()
        .any(|wifi| wifi.in_use && wifi.ssid == hotspot);
    let disconnected = !visible.iter().any(|wifi| wifi.in_use);
    let needs_recovery = connectivity == Connectivity::Offline
        || (connectivity == Connectivity::Unknown && disconnected);

    if needs_recovery && !on_hotspot && visible.iter().any(|wifi| wifi.ssid == hotspot) {
        return Some(hotspot);
    }

    if (on_hotspot && defer_preferred) || (!needs_recovery && !on_hotspot) {
        return None;
    }
    visible
        .iter()
        .filter(|wifi| {
            !visible
                .iter()
                .any(|active| active.in_use && active.ssid == wifi.ssid)
                && wifi.ssid != hotspot
                && wifi.signal >= MIN_PREFERRED_WIFI_SIGNAL
                && config
                    .preferred_wifi_ssids
                    .iter()
                    .any(|ssid| sanitize_ssid(ssid) == Some(wifi.ssid.as_str()))
        })
        .max_by_key(|wifi| wifi.signal)
        .map(|wifi| wifi.ssid.as_str())
}

fn nmcli(cancelled: &AtomicBool, args: &[&str]) -> Result<String, String> {
    if cancelled.load(Ordering::Relaxed) {
        return Err("Operação do Modo bolsa cancelada.".to_owned());
    }

    let output = Command::new("nmcli")
        .env("LC_ALL", "C.UTF-8")
        .args(["--wait", "15"])
        .args(args)
        .output()
        .map_err(|err| format!("Falha ao executar nmcli: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "nmcli falhou ({}): {}",
            output.status,
            stderr.trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|err| format!("Resposta inválida do nmcli: {err}"))
}

fn wifi_profile_uuid(ssid: &str, cancelled: &AtomicBool) -> Result<String, String> {
    let profiles = nmcli(cancelled, &["-t", "-f", "UUID,TYPE", "connection", "show"])?;
    for line in profiles.lines() {
        let Some((uuid, "802-11-wireless")) = line.split_once(':') else {
            continue;
        };
        // shortcut: usa o primeiro perfil salvo para o SSID, revise se a UI passar a selecionar perfis duplicados.
        let name = nmcli(
            cancelled,
            &[
                "--escape",
                "no",
                "-g",
                "802-11-wireless.ssid",
                "connection",
                "show",
                "uuid",
                uuid,
            ],
        )?;
        if name.strip_suffix('\n').unwrap_or(&name) == ssid {
            return Ok(uuid.to_owned());
        }
    }
    Err(format!(
        "Conecte-se a “{ssid}” no Linux para salvar o perfil Wi-Fi antes de usar o Modo bolsa."
    ))
}

pub fn configure_hotspot_blocking(
    config: &BagModeConfig,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    config.validate()?;
    let uuid = wifi_profile_uuid(config.hotspot_ssid.trim(), cancelled)?;
    nmcli(
        cancelled,
        &[
            "connection",
            "modify",
            "uuid",
            &uuid,
            "connection.metered",
            "yes",
            "connection.autoconnect",
            "yes",
            "connection.autoconnect-priority",
            "100",
        ],
    )
    .map(|_| ())
}

fn probe_connectivity(cancelled: &AtomicBool) -> Result<Connectivity, String> {
    nmcli(cancelled, &["networking", "connectivity", "check"])
        .map(|status| Connectivity::from_nmcli(&status))
}

pub fn poll_and_heal_bag_mode_blocking(
    config: &BagModeConfig,
    defer_preferred: bool,
    cancelled: &AtomicBool,
) -> Result<BagPoll, String> {
    config.validate()?;
    let visible = parse_nmcli_wifi_list(&nmcli(
        cancelled,
        &["-t", "-f", "IN-USE,SSID,SIGNAL", "device", "wifi", "list"],
    )?);
    let connectivity = probe_connectivity(cancelled)?;
    let target = evaluate_wifi_switch(config, &visible, connectivity, defer_preferred);
    let (network, connectivity) = if let Some(ssid) = target {
        let uuid = wifi_profile_uuid(ssid, cancelled)?;
        nmcli(cancelled, &["connection", "up", "uuid", &uuid])?;
        (
            ssid,
            probe_connectivity(cancelled).unwrap_or(Connectivity::Unknown),
        )
    } else {
        let network = visible
            .iter()
            .find(|wifi| wifi.in_use)
            .map_or("sem Wi-Fi", |wifi| wifi.ssid.as_str());
        (network, connectivity)
    };
    Ok(BagPoll {
        status: format!("{network} · {}", connectivity.label()),
        switched_to_hotspot: target == Some(config.hotspot_ssid.trim()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nmcli_terse_wifi_output_including_colons_in_ssid() {
        let sample = "\
*:iPhone de Helio:88
 :CasaWiFi_5G:74
 :Escritorio\\:Corp:62
 :Vizinho:35
 :   :90
malformed_line
";

        let parsed = parse_nmcli_wifi_list(sample);
        assert_eq!(
            parsed,
            vec![
                VisibleWifi {
                    in_use: true,
                    ssid: "iPhone de Helio".to_string(),
                    signal: 88,
                },
                VisibleWifi {
                    in_use: false,
                    ssid: "CasaWiFi_5G".to_string(),
                    signal: 74,
                },
                VisibleWifi {
                    in_use: false,
                    ssid: "Escritorio:Corp".to_string(),
                    signal: 62,
                },
                VisibleWifi {
                    in_use: false,
                    ssid: "Vizinho".to_string(),
                    signal: 35,
                },
            ]
        );
    }

    #[test]
    fn switches_from_hotspot_to_strongest_preferred_wifi() {
        let config = BagModeConfig {
            enabled: true,
            hotspot_ssid: "HotspotCelular".to_string(),
            preferred_wifi_ssids: vec!["Casa_2G".to_string(), "Casa_5G".to_string()],
        };
        let visible = vec![
            VisibleWifi {
                in_use: true,
                ssid: "HotspotCelular".to_string(),
                signal: 92,
            },
            VisibleWifi {
                in_use: false,
                ssid: "Casa_2G".to_string(),
                signal: 58,
            },
            VisibleWifi {
                in_use: false,
                ssid: "Casa_5G".to_string(),
                signal: 81,
            },
        ];

        assert_eq!(
            evaluate_wifi_switch(&config, &visible, Connectivity::Full, false),
            Some("Casa_5G")
        );
    }

    #[test]
    fn does_not_switch_when_already_on_preferred_or_signal_too_weak_or_disabled() {
        let mut config = BagModeConfig {
            enabled: true,
            hotspot_ssid: "HotspotCelular".to_string(),
            preferred_wifi_ssids: vec!["CasaWiFi".to_string()],
        };

        let already_home = vec![
            VisibleWifi {
                in_use: false,
                ssid: "HotspotCelular".to_string(),
                signal: 90,
            },
            VisibleWifi {
                in_use: true,
                ssid: "CasaWiFi".to_string(),
                signal: 75,
            },
        ];
        assert_eq!(
            evaluate_wifi_switch(&config, &already_home, Connectivity::Full, false),
            None
        );

        let weak_home = vec![
            VisibleWifi {
                in_use: true,
                ssid: "HotspotCelular".to_string(),
                signal: 85,
            },
            VisibleWifi {
                in_use: false,
                ssid: "CasaWiFi".to_string(),
                signal: 39,
            },
        ];
        assert_eq!(
            evaluate_wifi_switch(&config, &weak_home, Connectivity::Full, false),
            None
        );

        config.enabled = false;
        let strong_home = vec![
            VisibleWifi {
                in_use: true,
                ssid: "HotspotCelular".to_string(),
                signal: 85,
            },
            VisibleWifi {
                in_use: false,
                ssid: "CasaWiFi".to_string(),
                signal: 80,
            },
        ];
        assert_eq!(
            evaluate_wifi_switch(&config, &strong_home, Connectivity::Full, false),
            None
        );
    }

    #[test]
    fn fails_over_to_hotspot_when_wifi_cannot_reach_internet() {
        let config = BagModeConfig {
            enabled: true,
            hotspot_ssid: "Hotspot".into(),
            preferred_wifi_ssids: vec!["Casa".into()],
        };
        let wifi = parse_nmcli_wifi_list("*:Casa:75\n:Hotspot:88");
        assert_eq!(
            evaluate_wifi_switch(&config, &wifi, Connectivity::Offline, false),
            Some("Hotspot")
        );
        assert_eq!(
            evaluate_wifi_switch(&config, &wifi, Connectivity::Unknown, false),
            None
        );

        let disconnected = parse_nmcli_wifi_list(":Casa:75\n:Hotspot:88");
        assert_eq!(
            evaluate_wifi_switch(&config, &disconnected, Connectivity::Unknown, false),
            Some("Hotspot")
        );
        assert_eq!(
            evaluate_wifi_switch(&config, &disconnected, Connectivity::Full, false),
            None
        );
    }

    #[test]
    fn holds_hotspot_after_failover_until_it_disconnects() {
        let config = BagModeConfig {
            enabled: true,
            hotspot_ssid: "Hotspot".into(),
            preferred_wifi_ssids: vec!["Casa".into()],
        };
        let wifi = parse_nmcli_wifi_list("*:Hotspot:88\n:Casa:75");
        assert_eq!(
            evaluate_wifi_switch(&config, &wifi, Connectivity::Full, true),
            None
        );
        assert_eq!(
            evaluate_wifi_switch(&config, &wifi, Connectivity::Offline, true),
            None
        );
        let disconnected = parse_nmcli_wifi_list(":Casa:75");
        assert_eq!(
            evaluate_wifi_switch(&config, &disconnected, Connectivity::Offline, true),
            Some("Casa")
        );
    }

    #[test]
    fn validates_wifi_ssids_and_connectivity_response() {
        let mut config = BagModeConfig::default();
        assert!(config.validate().is_err());
        config.hotspot_ssid = "MeuHotspot".into();
        assert!(config.validate().is_ok());
        for ssid in ["MeuHotspot", "Casa\nWiFi", "Casa\0WiFi", &"á".repeat(17)] {
            config.preferred_wifi_ssids = vec![ssid.to_owned()];
            assert!(config.validate().is_err(), "accepted {ssid:?}");
        }
        for response in ["none", "limited", "portal"] {
            assert!(Connectivity::from_nmcli(response) == Connectivity::Offline);
        }
        assert!(Connectivity::from_nmcli("full\n") == Connectivity::Full);
        assert!(Connectivity::from_nmcli("unknown") == Connectivity::Unknown);
        assert!(nmcli(&AtomicBool::new(true), &[]).is_err());
    }
}
