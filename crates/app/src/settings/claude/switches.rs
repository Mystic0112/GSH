//! `ClaudeCodeSettingSwitch`: chave booleana do `~/.claude/settings.json`.
//!
//! A fonte da verdade é o arquivo do Claude Code, não as preferências do GSH. Chave
//! ausente mostra `unset`, o padrão do próprio CLI.

use super::file::{keys, value, Snapshot};
use gsh_harness::Json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    SwitchModels,
    RemoteControl,
    PromptSuggestions,
    Sandbox,
    SandboxStrict,
    ReducedMotion,
    PushInputNeeded,
    PushAgent,
    DisableTelemetry,
    DisableErrorReporting,
}

/// Valor gravado: booleano ou texto (`env.DISABLE_*` liga com `"1"`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lit {
    Bool(bool),
    One,
}

impl Lit {
    fn json(self) -> Json {
        match self {
            Self::Bool(b) => Json::Bool(b),
            Self::One => Json::Str(String::from("1")),
        }
    }
}

pub struct Spec {
    pub key: &'static str,
    /// Estado mostrado com a chave ausente.
    unset: bool,
    /// Voltar ao padrão apaga a chave em vez de gravar o valor. Para chaves em que o CLI
    /// distingue "nunca escolhido" de "escolhido", fica `false`.
    remove_when_default: bool,
    /// Valor gravado com o switch ligado / desligado (`None` = desligar remove a chave).
    on: Lit,
    off: Option<Lit>,
}

const fn spec(key: &'static str, unset: bool, remove_when_default: bool) -> Spec {
    Spec {
        key,
        unset,
        remove_when_default,
        on: Lit::Bool(true),
        off: Some(Lit::Bool(false)),
    }
}

impl Switch {
    pub fn spec(self) -> Spec {
        match self {
            // Gravado explícito: o CLI só pergunta a preferência enquanto a chave nunca foi escolhida.
            Self::SwitchModels => spec(keys::SWITCH_MODELS_ON_FLAG, true, false),
            Self::RemoteControl => spec(keys::REMOTE_CONTROL_AT_STARTUP, false, true),
            Self::PromptSuggestions => spec(keys::PROMPT_SUGGESTION, true, true),
            Self::Sandbox => spec(keys::SANDBOX_ENABLED, false, true),
            // Estrito = `allowUnsandboxedCommands: false`.
            Self::SandboxStrict => Spec {
                on: Lit::Bool(false),
                off: Some(Lit::Bool(true)),
                ..spec(keys::SANDBOX_ALLOW_UNSANDBOXED, false, true)
            },
            Self::ReducedMotion => spec(keys::REDUCED_MOTION, false, true),
            Self::PushInputNeeded => spec(keys::INPUT_NEEDED_NOTIF, false, false),
            Self::PushAgent => spec(keys::AGENT_PUSH_NOTIF, false, false),
            Self::DisableTelemetry => Spec {
                on: Lit::One,
                off: None,
                ..spec(keys::DISABLE_TELEMETRY, false, true)
            },
            Self::DisableErrorReporting => Spec {
                on: Lit::One,
                off: None,
                ..spec(keys::DISABLE_ERROR_REPORTING, false, true)
            },
        }
    }

    /// Estado do switch no `settings.json` lido.
    pub fn is_on(self, snapshot: &Snapshot) -> bool {
        let spec = self.spec();
        match value(snapshot, spec.key).filter(|v| !v.is_null()) {
            Some(v) if *v == spec.on.json() => true,
            Some(v) if spec.off.is_some_and(|off| *v == off.json()) => false,
            _ => spec.unset,
        }
    }

    /// O que gravar em `key` ao levar o switch para `on` (`None` apaga a chave).
    pub fn written(self, on: bool) -> Option<Json> {
        let spec = self.spec();
        if spec.remove_when_default && on == spec.unset {
            None
        } else if on {
            Some(spec.on.json())
        } else {
            spec.off.map(Lit::json)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Switch; 10] = [
        Switch::SwitchModels,
        Switch::RemoteControl,
        Switch::PromptSuggestions,
        Switch::Sandbox,
        Switch::SandboxStrict,
        Switch::ReducedMotion,
        Switch::PushInputNeeded,
        Switch::PushAgent,
        Switch::DisableTelemetry,
        Switch::DisableErrorReporting,
    ];

    fn nested(key: &str, value: Json) -> Json {
        key.split('.').rev().fold(value, |inner, part| {
            Json::Obj(vec![(part.to_owned(), inner)])
        })
    }

    #[test]
    fn written_value_reads_back_as_the_same_state_for_every_switch() {
        for switch in ALL {
            for on in [true, false] {
                let root = match switch.written(on) {
                    Some(v) => nested(switch.spec().key, v),
                    None => Json::Obj(Vec::new()),
                };
                assert_eq!(switch.is_on(&Ok(root)), on, "{switch:?} {on}");
            }
        }
    }

    #[test]
    fn absent_key_shows_the_cli_default() {
        let empty: Snapshot = Ok(Json::Obj(Vec::new()));
        assert!(Switch::SwitchModels.is_on(&empty));
        assert!(Switch::PromptSuggestions.is_on(&empty));
        assert!(!Switch::Sandbox.is_on(&empty));
        assert!(!Switch::SandboxStrict.is_on(&empty));
        assert!(!Switch::DisableTelemetry.is_on(&Err(String::from("ilegível"))));
    }

    #[test]
    fn what_gets_written_matches_the_flutter_rules() {
        // `switchModelsOnFlag` nunca apaga: o CLI distingue "nunca escolhido"
        assert_eq!(Switch::SwitchModels.written(true), Some(Json::Bool(true)));
        // voltar ao padrão apaga
        assert_eq!(Switch::Sandbox.written(false), None);
        assert_eq!(Switch::PromptSuggestions.written(true), None);
        assert_eq!(Switch::Sandbox.written(true), Some(Json::Bool(true)));
        // estrito liga gravando `false`
        assert_eq!(Switch::SandboxStrict.written(true), Some(Json::Bool(false)));
        // flag de env liga com "1" e desligar remove
        assert_eq!(
            Switch::DisableTelemetry.written(true),
            Some(Json::Str(String::from("1")))
        );
        assert_eq!(Switch::DisableTelemetry.written(false), None);
        // chaves sem remoção gravam o desligado
        assert_eq!(Switch::PushAgent.written(false), Some(Json::Bool(false)));
    }
}
