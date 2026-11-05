//! Máquina de turno de uma aba com agente por hook.
//!
//! Espelha `TerminalSession.applyClaudeStatus` / `_setStatus` / `_maybeInterrupt`
//! do Flutter. "Terminou" **não é um quarto estado**: é o efeito de assentar em
//! `Idle` ou `Waiting` por [`FINISH_DEBOUNCE_MS`] sem mudar (o app usa para
//! badge, som e notificação do SO). O debounce absorve o `idle -> working` de
//! um redesenho rápido.
//!
//! Sem relógio: todo método recebe `now_ms` (ms monotônicos, de qualquer
//! origem fixa), e quem dirige o tempo consulta [`TurnTracker::next_deadline`]
//! e chama [`TurnTracker::poll_finished`].
//!
//! Transições (`working` mid-turn = `PreToolUse`/`PostToolUse` e afins, isto é,
//! qualquer `working` que não seja `UserPromptSubmit`):
//!
//! | Chega                                   | Efeito                                                    |
//! |-----------------------------------------|-----------------------------------------------------------|
//! | `working` + `UserPromptSubmit`          | turno ativo; status `Working` (nunca descartado)          |
//! | `working` mid-turn, turno ativo         | status `Working`                                          |
//! | `working` mid-turn, sem turno ativo, < 5 s após o último `Idle` | descartado (`Applied::Stale`)         |
//! | `waiting`                               | status `Waiting`; agenda "terminou"                       |
//! | `idle` (Stop, SessionStart, SessionEnd) | turno encerrado; status `Idle`; agenda "terminou"         |
//! | mesmo status que o atual                | `Applied::Unchanged` (não reagenda nem registra o idle)   |
//! | ESC cru em `Working` ([`TurnTracker::interrupt`]) | `Idle` silencioso: não agenda "terminou"        |

use super::StatusUpdate;

/// Janela em que um `working` mid-turn sem turno ativo, logo após um `Idle`, é
/// tratado como hook reordenado do turno que já acabou (cada hook é um processo
/// separado, sem ordem garantida). Um turno novo legítimo vem como
/// `UserPromptSubmit`, que nunca é filtrado.
pub const STALE_WORKING_GUARD_MS: u64 = 5_000;

/// Quanto o status precisa ficar em `Idle`/`Waiting` para valer "terminou".
pub const FINISH_DEBOUNCE_MS: u64 = 50;

/// Status de turno de uma aba. Só existem estes três no wire (`st`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TurnStatus {
    #[default]
    Idle,
    Working,
    Waiting,
}

impl TurnStatus {
    /// `working` e `waiting` são os nomes; qualquer outro texto (não vazio) vira
    /// `Idle`, como o app original.
    pub fn from_wire(s: &str) -> TurnStatus {
        match s {
            "working" => TurnStatus::Working,
            "waiting" => TurnStatus::Waiting,
            _ => TurnStatus::Idle,
        }
    }

    pub fn as_wire(self) -> &'static str {
        match self {
            TurnStatus::Idle => "idle",
            TurnStatus::Working => "working",
            TurnStatus::Waiting => "waiting",
        }
    }
}

/// Resultado de aplicar um status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    /// `working` tardio de um turno que já acabou: descartado.
    Stale,
    /// Já estava nesse status.
    Unchanged,
    /// Mudou; `from` é o status anterior (a UI repinta).
    Changed { from: TurnStatus },
}

/// Estado de turno de uma aba.
#[derive(Debug, Clone, Default)]
pub struct TurnTracker {
    status: TurnStatus,
    /// `true` entre `UserPromptSubmit` e o próximo `idle`.
    turn_active: bool,
    /// Quando o status *mudou* para `Idle` pela última vez (ou interrupt).
    last_idle_ms: Option<u64>,
    /// Instante a partir do qual vale "terminou"; `None` = nada agendado.
    finish_at_ms: Option<u64>,
}

impl TurnTracker {
    pub fn new() -> TurnTracker {
        TurnTracker::default()
    }

    pub fn status(&self) -> TurnStatus {
        self.status
    }

    pub fn turn_active(&self) -> bool {
        self.turn_active
    }

    /// Aplica uma atualização do socket.
    pub fn apply(&mut self, update: &StatusUpdate, now_ms: u64) -> Applied {
        self.apply_status(update.status, update.starts_turn(), now_ms)
    }

    /// Mesmo que [`apply`](Self::apply), sem o envelope (`turn_start` =
    /// `UserPromptSubmit`).
    pub fn apply_status(&mut self, status: TurnStatus, turn_start: bool, now_ms: u64) -> Applied {
        if turn_start {
            self.turn_active = true;
        }
        if status == TurnStatus::Idle {
            self.turn_active = false;
        }
        let late = self
            .last_idle_ms
            .is_some_and(|t| now_ms.saturating_sub(t) < STALE_WORKING_GUARD_MS);
        if status == TurnStatus::Working && !turn_start && !self.turn_active && late {
            return Applied::Stale;
        }
        self.set(status, now_ms)
    }

    fn set(&mut self, next: TurnStatus, now_ms: u64) -> Applied {
        if next == self.status {
            return Applied::Unchanged;
        }
        let from = self.status;
        self.status = next;
        if next == TurnStatus::Idle {
            self.last_idle_ms = Some(now_ms);
        }
        // Toda mudança cancela o agendamento anterior; só Idle/Waiting agendam.
        self.finish_at_ms = match next {
            TurnStatus::Idle | TurnStatus::Waiting => {
                Some(now_ms.saturating_add(FINISH_DEBOUNCE_MS))
            }
            TurnStatus::Working => None,
        };
        Applied::Changed { from }
    }

    /// ESC cru do usuário enquanto `Working`. O Claude não emite `Stop` num
    /// interrupt, então sem isto o último hook seria um `working` e o spinner
    /// giraria para sempre. Vai a `Idle` **sem** agendar "terminou" (o usuário
    /// acabou de agir; não há o que avisar). `true` se mudou.
    pub fn interrupt(&mut self, now_ms: u64) -> bool {
        if self.status != TurnStatus::Working {
            return false;
        }
        self.turn_active = false;
        self.finish_at_ms = None;
        self.status = TurnStatus::Idle;
        self.last_idle_ms = Some(now_ms);
        true
    }

    /// Quando chamar [`poll_finished`](Self::poll_finished) de novo.
    pub fn next_deadline(&self) -> Option<u64> {
        self.finish_at_ms
    }

    /// "Terminou": devolve `Idle` ou `Waiting` uma única vez, quando o status
    /// assentou por [`FINISH_DEBOUNCE_MS`]. `Waiting` é "precisa do usuário"
    /// (permissão, pergunta, plano) e usa o mesmo caminho do fim de turno; o
    /// consumidor distingue pelo valor.
    pub fn poll_finished(&mut self, now_ms: u64) -> Option<TurnStatus> {
        match self.finish_at_ms {
            Some(at) if now_ms >= at => {
                self.finish_at_ms = None;
                Some(self.status)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TurnStatus::{Idle, Waiting, Working};

    /// `working` de início de turno / mid-turn.
    fn prompt(t: &mut TurnTracker, now: u64) -> Applied {
        t.apply_status(Working, true, now)
    }
    fn tool(t: &mut TurnTracker, now: u64) -> Applied {
        t.apply_status(Working, false, now)
    }
    fn stop(t: &mut TurnTracker, now: u64) -> Applied {
        t.apply_status(Idle, false, now)
    }

    #[test]
    fn ciclo_normal_de_um_turno() {
        let mut t = TurnTracker::new();
        assert_eq!(t.status(), Idle);
        assert_eq!(prompt(&mut t, 0), Applied::Changed { from: Idle });
        assert!(t.turn_active());
        assert_eq!(tool(&mut t, 10), Applied::Unchanged);
        assert_eq!(stop(&mut t, 20), Applied::Changed { from: Working });
        assert!(!t.turn_active());
        assert_eq!(t.status(), Idle);
    }

    #[test]
    fn working_tardio_depois_do_stop_e_descartado() {
        let mut t = TurnTracker::new();
        prompt(&mut t, 0);
        stop(&mut t, 100);
        assert_eq!(tool(&mut t, 150), Applied::Stale);
        assert_eq!(t.status(), Idle);
        // Na borda da janela (5 s) já vale.
        assert_eq!(
            tool(&mut t, 100 + STALE_WORKING_GUARD_MS - 1),
            Applied::Stale
        );
        assert_eq!(
            tool(&mut t, 100 + STALE_WORKING_GUARD_MS),
            Applied::Changed { from: Idle }
        );
    }

    #[test]
    fn user_prompt_submit_logo_apos_idle_nunca_e_filtrado() {
        let mut t = TurnTracker::new();
        prompt(&mut t, 0);
        stop(&mut t, 100);
        assert_eq!(prompt(&mut t, 101), Applied::Changed { from: Idle });
        assert!(t.turn_active());
        // E o mid-turn desse turno novo vale.
        assert_eq!(tool(&mut t, 102), Applied::Unchanged);
    }

    #[test]
    fn working_orfao_sem_idle_anterior_vale() {
        // `last_idle` só existe depois de um idle que mudou o status: o app que
        // abre com o agente já no meio do turno aceita o primeiro working.
        let mut t = TurnTracker::new();
        assert_eq!(tool(&mut t, 0), Applied::Changed { from: Idle });
    }

    #[test]
    fn idle_repetido_nao_registra_last_idle() {
        let mut t = TurnTracker::new();
        // SessionStart como primeiro evento: já é Idle, nada muda.
        assert_eq!(stop(&mut t, 1_000), Applied::Unchanged);
        assert_eq!(t.next_deadline(), None);
        // Como o Idle inicial não registrou instante, o working órfão passa.
        assert_eq!(tool(&mut t, 1_001), Applied::Changed { from: Idle });
    }

    #[test]
    fn waiting_vai_e_volta_para_working() {
        let mut t = TurnTracker::new();
        prompt(&mut t, 0);
        assert_eq!(
            t.apply_status(Waiting, false, 10),
            Applied::Changed { from: Working }
        );
        assert!(t.turn_active(), "waiting não encerra o turno");
        // Aprovado: o PostToolUse segue o mesmo turno.
        assert_eq!(tool(&mut t, 20), Applied::Changed { from: Waiting });
    }

    #[test]
    fn terminou_dispara_uma_vez_apos_o_debounce() {
        let mut t = TurnTracker::new();
        prompt(&mut t, 0);
        stop(&mut t, 100);
        assert_eq!(t.next_deadline(), Some(100 + FINISH_DEBOUNCE_MS));
        assert_eq!(t.poll_finished(100), None);
        assert_eq!(t.poll_finished(100 + FINISH_DEBOUNCE_MS - 1), None);
        assert_eq!(t.poll_finished(100 + FINISH_DEBOUNCE_MS), Some(Idle));
        assert_eq!(t.poll_finished(1_000), None);
        assert_eq!(t.next_deadline(), None);
    }

    #[test]
    fn terminou_em_waiting_avisa_acao_necessaria() {
        let mut t = TurnTracker::new();
        prompt(&mut t, 0);
        t.apply_status(Waiting, false, 10);
        assert_eq!(t.poll_finished(10 + FINISH_DEBOUNCE_MS), Some(Waiting));
    }

    #[test]
    fn flicker_idle_working_cancela_o_terminou() {
        let mut t = TurnTracker::new();
        prompt(&mut t, 0);
        stop(&mut t, 100);
        // Turno enfileirado começa dentro do debounce.
        prompt(&mut t, 120);
        assert_eq!(t.next_deadline(), None);
        assert_eq!(t.poll_finished(10_000), None);
    }

    #[test]
    fn repeticao_do_mesmo_status_nao_reagenda() {
        let mut t = TurnTracker::new();
        prompt(&mut t, 0);
        t.apply_status(Waiting, false, 10);
        // Notification duplicada 30 ms depois: não empurra o prazo.
        assert_eq!(t.apply_status(Waiting, false, 40), Applied::Unchanged);
        assert_eq!(t.next_deadline(), Some(10 + FINISH_DEBOUNCE_MS));
    }

    #[test]
    fn waiting_para_idle_reagenda_e_troca_o_valor() {
        let mut t = TurnTracker::new();
        prompt(&mut t, 0);
        t.apply_status(Waiting, false, 10);
        stop(&mut t, 30);
        assert_eq!(t.poll_finished(60), None, "prazo do waiting foi cancelado");
        assert_eq!(t.poll_finished(30 + FINISH_DEBOUNCE_MS), Some(Idle));
    }

    #[test]
    fn interrupt_vai_a_idle_sem_avisar_e_ativa_o_guard() {
        let mut t = TurnTracker::new();
        prompt(&mut t, 0);
        assert!(t.interrupt(100));
        assert_eq!(t.status(), Idle);
        assert!(!t.turn_active());
        assert_eq!(t.next_deadline(), None);
        // Hook tardio do turno interrompido é descartado...
        assert_eq!(tool(&mut t, 150), Applied::Stale);
        // ...e um prompt novo reacende.
        assert_eq!(prompt(&mut t, 200), Applied::Changed { from: Idle });
    }

    #[test]
    fn interrupt_fora_de_working_e_noop() {
        let mut t = TurnTracker::new();
        assert!(!t.interrupt(0));
        prompt(&mut t, 0);
        t.apply_status(Waiting, false, 10);
        assert!(!t.interrupt(20));
        assert_eq!(t.status(), Waiting);
    }

    #[test]
    fn apply_usa_o_evento_do_update() {
        use crate::status::{decode_line, Incoming};
        let line = |ev: &str, st: &str| {
            let raw = format!(r#"{{"paneId":"t","st":"{st}","ev":"{ev}"}}"#);
            match decode_line(&raw, None) {
                Ok(Incoming::Status(u)) => u,
                other => panic!("{other:?}"),
            }
        };
        let mut t = TurnTracker::new();
        t.apply(&line("UserPromptSubmit", "working"), 0);
        t.apply(&line("Stop", "idle"), 10);
        assert_eq!(t.apply(&line("PostToolUse", "working"), 20), Applied::Stale);
        assert_eq!(
            t.apply(&line("UserPromptSubmit", "working"), 30),
            Applied::Changed { from: Idle }
        );
    }

    #[test]
    fn status_wire_ida_e_volta() {
        for s in [Idle, Working, Waiting] {
            assert_eq!(TurnStatus::from_wire(s.as_wire()), s);
        }
        assert_eq!(TurnStatus::from_wire("qualquer"), Idle);
    }
}
