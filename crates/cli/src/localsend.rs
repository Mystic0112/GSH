use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use gsh_localsend::{send_files, Discovery, Identity};

use crate::error::CliError;

pub fn run(args: &[String]) -> Result<Option<String>, CliError> {
    let Some(action) = args.first().map(String::as_str) else {
        return Err(CliError::usage("gsh localsend: expected peers or send"));
    };
    match action {
        "peers" => peers(args.get(1..).unwrap_or_default()),
        "send" => send(args.get(1..).unwrap_or_default()),
        _ => Err(CliError::usage(format!(
            "gsh localsend: unknown action: {action}"
        ))),
    }
}

fn peers(args: &[String]) -> Result<Option<String>, CliError> {
    let json = args.iter().any(|arg| arg == "--json");
    if args.iter().any(|arg| arg != "--json") {
        return Err(CliError::usage("gsh localsend peers: expected only --json"));
    }
    let identity =
        Identity::load_or_create(53317).map_err(|error| CliError::App(error.to_string()))?;
    let discovery = Discovery::start(identity).map_err(|error| CliError::App(error.to_string()))?;
    thread::sleep(Duration::from_secs(2));
    let mut peers = discovery.peers();
    peers.sort_by(|left, right| {
        left.info
            .alias
            .to_lowercase()
            .cmp(&right.info.alias.to_lowercase())
    });
    if json {
        return serde_json::to_string_pretty(&peers)
            .map(|out| Some(format!("{out}\n")))
            .map_err(|error| CliError::App(error.to_string()));
    }
    let out = if peers.is_empty() {
        "(none)\n".to_owned()
    } else {
        peers
            .iter()
            .map(|peer| {
                format!(
                    "{}\t{}:{}\t{}\t{}",
                    peer.info.alias,
                    peer.ip,
                    peer.info.port.unwrap_or(53317),
                    peer.info.device_model.as_deref().unwrap_or(""),
                    peer.info.fingerprint
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };
    Ok(Some(out))
}

fn send(args: &[String]) -> Result<Option<String>, CliError> {
    let Some(target) = args.first() else {
        return Err(CliError::usage("gsh localsend send: missing <alias|ip>"));
    };
    let paths = args
        .get(1..)
        .unwrap_or_default()
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Err(CliError::usage("gsh localsend send: missing file"));
    }
    let identity =
        Identity::load_or_create(53317).map_err(|error| CliError::App(error.to_string()))?;
    let discovery =
        Discovery::start(identity.clone()).map_err(|error| CliError::App(error.to_string()))?;
    thread::sleep(Duration::from_secs(2));
    let ip = target.parse::<Ipv4Addr>().ok();
    let matching = discovery
        .peers()
        .into_iter()
        .filter(|peer| peer.info.alias.eq_ignore_ascii_case(target) || Some(peer.ip) == ip)
        .collect::<Vec<_>>();
    let peer = match matching.as_slice() {
        [peer] => peer.clone(),
        [] => return Err(CliError::App(format!("LocalSend peer not found: {target}"))),
        _ => {
            return Err(CliError::App(format!(
                "LocalSend peer target is ambiguous: {target}"
            )))
        }
    };
    let mut last_percent = u64::MAX;
    send_files(
        &peer,
        &identity,
        &paths,
        2 * 1024 * 1024 * 1024,
        |path, sent, total| {
            let percent = sent.saturating_mul(100).checked_div(total).unwrap_or(100);
            if percent != last_percent {
                eprintln!("{}: {percent}%", path.display());
                last_percent = percent;
            }
        },
        || false,
    )
    .map_err(|error| CliError::App(error.to_string()))?;
    Ok(Some(format!(
        "Sent {} file(s) to {}.\n",
        paths.len(),
        peer.info.alias
    )))
}
