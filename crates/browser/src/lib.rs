mod actions;
mod cdp;
mod daemon;
mod install;
mod obscura;
mod snapshot;
mod url;

pub use actions::{run_action, Action, BrowserState};
pub use cdp::{BrowserError, Cdp};
pub use daemon::{client_command, daemon_main, Endpoint};
pub use install::{asset_name, install, verify_sha256};
pub use obscura::{find_binary, start_obscura, Obscura};
pub use snapshot::{snapshot, truncate, Snapshot};
pub use url::validate_url;

pub fn cli(args: &[String]) -> Result<String, BrowserError> {
    if args.first().map(String::as_str) == Some("install") {
        install()?;
        return Ok("Obscura installed.".into());
    }
    client_command(args)
}
