//! "Quem está mexendo no quê": arquivos tocados por agente (aba) (`AgentFileActivity` em
//! `agent_file_activity.dart` do Flutter).
//!
//! Em memória e sem relógio: quem chama passa o instante (epoch em ms). A aba some ao fechar.
//! O rastreio por inferência de saída (`OutputTurnFileTracker`) fica de fora: só o hook
//! (`inferred = false`) alimenta isto por enquanto, mas o campo já existe para o dia em que
//! houver outra origem.

use std::collections::BTreeMap;

/// Janela em que dois agentes no mesmo arquivo viram aviso (10 minutos).
pub const CONCURRENT_WINDOW_MS: u64 = 10 * 60 * 1000;

/// Um arquivo tocado por um agente: quando, e se foi inferido em vez de reportado pelo hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileTouch {
    pub at_ms: u64,
    pub inferred: bool,
}

/// Quem tocou um arquivo e como.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toucher<'a> {
    pub tab_id: &'a str,
    pub touch: FileTouch,
}

/// Arquivos tocados por aba (caminho absoluto -> toque).
#[derive(Debug, Clone, Default)]
pub struct AgentFileActivity {
    by_tab: BTreeMap<String, BTreeMap<String, FileTouch>>,
}

impl AgentFileActivity {
    /// Registra `paths` (absolutos) como tocados por `tab_id` em `at_ms`. O toque mais novo
    /// de um mesmo caminho substitui o anterior. Lista vazia não faz nada.
    pub fn record(&mut self, tab_id: &str, paths: &[String], inferred: bool, at_ms: u64) {
        if paths.is_empty() {
            return;
        }
        let touch = FileTouch { at_ms, inferred };
        let files = self.by_tab.entry(tab_id.to_owned()).or_default();
        for path in paths {
            files.insert(path.clone(), touch);
        }
    }

    /// Aba fechada: some com o que ela tocou. `true` se havia algo registrado.
    pub fn forget_tab(&mut self, tab_id: &str) -> bool {
        self.by_tab.remove(tab_id).is_some()
    }

    /// Quantos arquivos `tab_id` tocou.
    pub fn touched_count(&self, tab_id: &str) -> usize {
        self.by_tab.get(tab_id).map_or(0, BTreeMap::len)
    }

    /// Arquivos tocados por `tab_id`, do toque mais novo ao mais antigo.
    pub fn touched_by_recent(&self, tab_id: &str) -> Vec<(&str, FileTouch)> {
        let mut out: Vec<(&str, FileTouch)> = self.by_tab.get(tab_id).map_or_else(Vec::new, |f| {
            f.iter()
                .map(|(path, touch)| (path.as_str(), *touch))
                .collect()
        });
        out.sort_by(|a, b| b.1.at_ms.cmp(&a.1.at_ms).then_with(|| a.0.cmp(b.0)));
        out
    }

    /// Quem tocou `path`, do toque mais novo ao mais antigo.
    pub fn touchers_of(&self, path: &str) -> Vec<Toucher<'_>> {
        let mut out: Vec<Toucher<'_>> = self
            .by_tab
            .iter()
            .filter_map(|(tab_id, files)| {
                files.get(path).map(|touch| Toucher {
                    tab_id: tab_id.as_str(),
                    touch: *touch,
                })
            })
            .collect();
        out.sort_by(|a, b| {
            b.touch
                .at_ms
                .cmp(&a.touch.at_ms)
                .then_with(|| a.tab_id.cmp(b.tab_id))
        });
        out
    }
}

/// Quantos de `touchers` mexeram há menos de [`CONCURRENT_WINDOW_MS`]; 2 ou mais é aviso de
/// edição concorrente.
pub fn recent_touchers(touchers: &[Toucher<'_>], now_ms: u64) -> usize {
    touchers
        .iter()
        .filter(|t| now_ms.saturating_sub(t.touch.at_ms) < CONCURRENT_WINDOW_MS)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn acumula_entre_lotes_e_o_toque_novo_vence() {
        let mut a = AgentFileActivity::default();
        a.record("t1", &paths(&["/p/a.rs"]), false, 1_000);
        a.record("t1", &paths(&["/p/b.rs", "/p/a.rs"]), false, 2_000);
        assert_eq!(a.touched_count("t1"), 2);
        let lista = a.touched_by_recent("t1");
        assert_eq!(lista.first().map(|(_, t)| t.at_ms), Some(2_000));
        assert!(lista.iter().all(|(_, t)| t.at_ms == 2_000));
    }

    #[test]
    fn lista_vazia_nao_registra_a_aba() {
        let mut a = AgentFileActivity::default();
        a.record("t1", &[], false, 1_000);
        assert_eq!(a.touched_count("t1"), 0);
        assert!(!a.forget_tab("t1"));
    }

    #[test]
    fn touched_by_recent_ordena_do_mais_novo() {
        let mut a = AgentFileActivity::default();
        a.record("t1", &paths(&["/p/velho"]), false, 1_000);
        a.record("t1", &paths(&["/p/novo"]), true, 3_000);
        a.record("t1", &paths(&["/p/meio"]), false, 2_000);
        let nomes: Vec<&str> = a.touched_by_recent("t1").iter().map(|(p, _)| *p).collect();
        assert_eq!(nomes, ["/p/novo", "/p/meio", "/p/velho"]);
        assert_eq!(
            a.touched_by_recent("t1").first().map(|(_, t)| t.inferred),
            Some(true)
        );
    }

    #[test]
    fn touchers_of_ordena_do_mais_novo_e_ignora_quem_nao_tocou() {
        let mut a = AgentFileActivity::default();
        a.record("t1", &paths(&["/p/x"]), false, 1_000);
        a.record("t2", &paths(&["/p/x"]), false, 5_000);
        a.record("t3", &paths(&["/p/outro"]), false, 9_000);
        let quem: Vec<&str> = a.touchers_of("/p/x").iter().map(|t| t.tab_id).collect();
        assert_eq!(quem, ["t2", "t1"]);
        assert!(a.touchers_of("/p/nada").is_empty());
    }

    #[test]
    fn esquecer_a_aba_tira_o_que_ela_tocou() {
        let mut a = AgentFileActivity::default();
        a.record("t1", &paths(&["/p/x"]), false, 1_000);
        a.record("t2", &paths(&["/p/x"]), false, 2_000);
        assert!(a.forget_tab("t1"));
        assert_eq!(a.touched_count("t1"), 0);
        let quem: Vec<&str> = a.touchers_of("/p/x").iter().map(|t| t.tab_id).collect();
        assert_eq!(quem, ["t2"]);
        assert!(!a.forget_tab("t1"));
    }

    #[test]
    fn concorrencia_conta_so_quem_mexeu_na_janela() {
        let mut a = AgentFileActivity::default();
        let agora = 10 * CONCURRENT_WINDOW_MS;
        a.record("t1", &paths(&["/p/x"]), false, agora - CONCURRENT_WINDOW_MS);
        a.record(
            "t2",
            &paths(&["/p/x"]),
            false,
            agora - CONCURRENT_WINDOW_MS + 1,
        );
        a.record("t3", &paths(&["/p/x"]), false, agora);
        // t1 saiu da janela (exatamente 10 min não vale); t2 e t3 valem.
        assert_eq!(recent_touchers(&a.touchers_of("/p/x"), agora), 2);
        // 1 ms depois, t2 completa 10 min e sai também.
        assert_eq!(recent_touchers(&a.touchers_of("/p/x"), agora + 1), 1);
        assert_eq!(
            recent_touchers(&a.touchers_of("/p/x"), agora + CONCURRENT_WINDOW_MS),
            0
        );
    }
}
