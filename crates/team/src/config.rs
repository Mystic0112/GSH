//! Preferencias do time (`TeamFarmConfig` de team_farm.dart).

use std::fmt;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct TeamConfig {
    pub enabled: bool,
    pub team: String,
    pub name: String,
    /// Manda os titulos das sessoes (podem ter trecho de prompt).
    pub show_titles: bool,
    /// Faixa da rede do trabalho (`192.168.10.0/24`); vazio = qualquer rede.
    pub network: String,
    /// Senha do time: cifra os pacotes e as batidas. Vazia = pacote aberto, sem correio.
    pub secret: String,
    pub ask_before_opening: bool,
    /// Quem pode bater (nomes separados por virgula). Vazio = todo o time.
    pub allowed_knockers: String,
}

impl TeamConfig {
    pub fn active(&self) -> bool {
        self.enabled && !self.team.trim().is_empty()
    }

    pub fn mail_active(&self) -> bool {
        self.active() && !self.secret.is_empty()
    }

    /// `name` (de quem bateu) esta liberado?
    pub fn allows(&self, name: &str) -> bool {
        let wanted = name.trim().to_lowercase();
        let mut list = self
            .allowed_knockers
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .peekable();
        list.peek().is_none() || list.any(|s| s == wanted)
    }
}

/// A senha nunca vai para log.
impl fmt::Debug for TeamConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TeamConfig")
            .field("enabled", &self.enabled)
            .field("team", &self.team)
            .field("name", &self.name)
            .field("show_titles", &self.show_titles)
            .field("network", &self.network)
            .field("secret", &(!self.secret.is_empty()).then_some("<set>"))
            .field("ask_before_opening", &self.ask_before_opening)
            .field("allowed_knockers", &self.allowed_knockers)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_e_mail_active() {
        let mut c = TeamConfig::default();
        assert!(!c.active());
        c.enabled = true;
        assert!(!c.active(), "sem nome de time nao ativa");
        c.team = "  ".into();
        assert!(!c.active());
        c.team = "gorila".into();
        assert!(c.active() && !c.mail_active());
        c.secret = "x".into();
        assert!(c.mail_active());
    }

    #[test]
    fn allows_ignora_caixa_e_espacos() {
        let mut c = TeamConfig::default();
        assert!(c.allows("qualquer"));
        c.allowed_knockers = " Ana , ,bob".into();
        assert!(c.allows("ana") && c.allows(" BOB "));
        assert!(!c.allows("carlos"));
    }

    #[test]
    fn debug_nao_vaza_a_senha() {
        let c = TeamConfig {
            secret: "super-segredo".into(),
            ..TeamConfig::default()
        };
        assert!(!format!("{c:?}").contains("super-segredo"));
    }
}
