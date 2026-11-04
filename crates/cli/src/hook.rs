//! `gsh hook`: helper que os agentes (Claude Code, Codex CLI) invocam nos hooks
//! de ciclo de vida. Lê o envelope JSON do stdin, traduz em status de turno e
//! manda pro app pelo socket de status. A tradução é de `gsh_core::hook`.
//!
//! Os agentes rodam os hooks SEM terminal controlador, por isso o canal é o
//! socket (env `GSH_STATUS_SOCK`/`GSH_PANE_ID`, herdadas da PTY) e não OSC.
//! Sessão fora do GSH não tem essas envs: nada vai pro socket.
//!
//! **Fazenda:** com ou sem aba, o último status de cada sessão vai para
//! `~/.gsh/farm/<session_id>.json`. É o único jeito de a fazenda saber de um
//! pedido de permissão, que não aparece no transcript.
//!
//! **Nunca escreve no stdout** (participa do protocolo de hook), salvo a
//! decisão de permissão do app, e **nunca falha barulhento**: qualquer erro é
//! engolido pra não atrapalhar o turno. A única exceção é o guard de hosts
//! protegidos, que bloqueia o comando (exit 2).

use std::io::Read;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gsh_core::hook::{
    farm_action, guard_hit, protected_hosts, status_for, status_payload, str_field, FarmAction,
};
use gsh_core::wire::{
    permission_ask_request, ENV_PANE_ID, ENV_STATUS_PORT, ENV_STATUS_SOCK, ENV_STATUS_TOKEN,
};
use serde_json::Value;

use crate::env::{env_non_empty, home_dir};
use crate::error::CliError;
use crate::transport;

/// O app segura a resposta do `permission-ask` até alguém decidir (o hook tem
/// timeout longo no settings do harness); 24 h é só o teto do socket.
const PERMISSION_TIMEOUT: Duration = Duration::from_secs(86_400);

pub fn run(harness: &str) -> Result<Option<String>, CliError> {
    let Some(envelope) = read_envelope() else {
        return Ok(None);
    };
    let event = str_field(&envelope, "hook_event_name");
    let home = home_dir();

    // Hosts protegidos: comando que cite um deles é bloqueado antes de rodar,
    // em qualquer sessão da máquina (não só numa aba do GSH).
    if event == "PreToolUse" {
        if let Some(home) = &home {
            let path = Path::new(home).join(".gsh").join("protected-hosts.json");
            let hosts = std::fs::read_to_string(path)
                .map(|raw| protected_hosts(&raw))
                .unwrap_or_default();
            if let Some(host) = guard_hit(&envelope, &hosts) {
                return Err(CliError::Blocked(format!(
                    "GSH Desktop bloqueou este comando: ele cita o host protegido \"{host}\". \
                     Peça ao usuário para rodar à mão ou remover o host em Configurações > Claude Code > Segurança."
                )));
            }
        }
    }

    let status = status_for(&event, &envelope);
    if let Some(home) = &home {
        let dir = Path::new(home).join(".gsh").join("farm");
        record_farm(
            &dir,
            &farm_action(
                &str_field(&envelope, "session_id"),
                &event,
                status,
                harness,
                now_ms(),
            ),
        );
    }
    let Some(status) = status else {
        return Ok(None); // evento que não nos interessa
    };

    let Some(pane_id) = env_non_empty(ENV_PANE_ID) else {
        return Ok(None); // não é sessão do GSH
    };
    let sock = env_non_empty(ENV_STATUS_SOCK);
    let port = env_non_empty(ENV_STATUS_PORT).and_then(|p| p.parse::<u16>().ok());
    if sock.is_none() && port.is_none() {
        return Ok(None);
    }

    // Token só importa no TCP; no UDS a permissão do socket já protege.
    let token = std::env::var(ENV_STATUS_TOKEN).ok();
    let payload = status_payload(
        &pane_id,
        status,
        &event,
        &envelope,
        harness,
        token.as_deref(),
    );
    let _ = transport::send_status(&format!("{payload}\n"), sock.as_deref(), port);

    // Pedido de permissão do Claude: pergunta ao app (ilha/notificação) e espera
    // a decisão. Sem resposta, "pass" ou app fora do ar, não imprime nada e o
    // Claude mostra o prompt de sempre no terminal.
    if event == "PermissionRequest" && harness == "claude" {
        if let Some(out) = ask_permission(&pane_id, &envelope) {
            return Ok(Some(format!("{out}\n")));
        }
    }
    Ok(None)
}

fn read_envelope() -> Option<Value> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    serde_json::from_str::<Value>(&raw)
        .ok()
        .filter(Value::is_object)
}

/// Manda o `permission-ask` e devolve a saída do hook que o app montou
/// (`data.output`), ou `None` = deixar o terminal perguntar.
fn ask_permission(pane_id: &str, envelope: &Value) -> Option<String> {
    let resp = transport::request(
        permission_ask_request(pane_id, envelope),
        PERMISSION_TIMEOUT,
    )
    .ok()?;
    resp.hook_output().map(Value::to_string)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// Aplica a decisão da fazenda. Escrita atômica (tmp + rename): a fazenda lê a
/// cada 2 s e nunca pode ver meio JSON. Falha de disco é engolida.
fn record_farm(dir: &Path, action: &FarmAction) {
    match action {
        FarmAction::Skip => {}
        FarmAction::Remove { file_name } => {
            let _ = std::fs::remove_file(dir.join(file_name));
        }
        FarmAction::Write { file_name, body } => {
            if std::fs::create_dir_all(dir).is_err() {
                return;
            }
            // Mesmo nome do original: `.<sid>.<pid>.tmp`.
            let stem = file_name.strip_suffix(".json").unwrap_or(file_name);
            let tmp = dir.join(format!(".{stem}.{}.tmp", std::process::id()));
            if std::fs::write(&tmp, body).is_ok()
                && std::fs::rename(&tmp, dir.join(file_name)).is_err()
            {
                let _ = std::fs::remove_file(&tmp);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fazenda_grava_atomico_e_apaga() {
        let dir = std::env::temp_dir().join(format!("gsh-farm-test-{}", std::process::id()));
        let write = FarmAction::Write {
            file_name: "abc-123.json".into(),
            body: r#"{"st":"waiting"}"#.into(),
        };
        record_farm(&dir, &write);
        assert_eq!(
            std::fs::read_to_string(dir.join("abc-123.json")).unwrap(),
            r#"{"st":"waiting"}"#
        );
        // Nada de .tmp sobrando.
        let left: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert_eq!(left.len(), 1);
        record_farm(&dir, &FarmAction::Skip);
        assert!(dir.join("abc-123.json").exists());
        record_farm(
            &dir,
            &FarmAction::Remove {
                file_name: "abc-123.json".into(),
            },
        );
        assert!(!dir.join("abc-123.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
