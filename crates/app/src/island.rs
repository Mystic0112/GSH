//! Ilha de agentes no topo da janela e painel expansível (`agent_island.dart` e
//! `agent_island_panel.dart` do app Flutter).
//!
//! - Pílula no topo, centralizada, desenhada em `iced::widget::canvas` com o
//!   gorila na pose do estado mais prioritário e os contadores por estado.
//! - Largura da pílula interpolada suavemente por frame ao trocar de estado.
//! - Clique na pílula abre/fecha o painel com as quatro seções do Flutter na
//!   mesma ordem: "Pedem permissão", "Esperando você", "Terminaram" e
//!   "Trabalhando".

use std::cell::Cell;

use gsh_core::status::{AgentHarness, HookEvent, StatusUpdate, TurnStatus};
use iced::time::{Duration, Instant};
use iced::widget::canvas::{self, Action, Event, Path, Stroke};
use iced::widget::text;
use iced::{
    alignment, border, mouse, window, Color, Element, Length, Point, Rectangle, Renderer, Size,
    Theme,
};

use crate::agent_island_panel;
use crate::agent_status::IslandServerError;
use crate::gorilla::{draw_gorilla, GorillaMood, GORILLA_HEIGHT, GORILLA_WIDTH};
use crate::labels;
use crate::theme;

// Paleta da ilha e do painel (reexportada de `crate::theme` — fonte única de tokens).
pub const COLOR_BG: Color = theme::BG;
pub const COLOR_PANEL: Color = theme::PANEL;
pub const COLOR_PANEL2: Color = theme::PANEL2;
pub const COLOR_PANEL3: Color = theme::PANEL3;
pub const COLOR_BORDER: Color = theme::BORDER;
pub const COLOR_BORDER2: Color = theme::BORDER2;
pub const COLOR_TEXT: Color = theme::TEXT;
pub const COLOR_TEXT2: Color = theme::TEXT2;
pub const COLOR_TEXT3: Color = theme::TEXT3;
pub const COLOR_ACCENT: Color = theme::ACCENT;
pub const COLOR_OK: Color = theme::OK;
pub const COLOR_WARN: Color = theme::WARN;
pub const COLOR_ERROR: Color = theme::ERROR;
pub const COLOR_ONLINE: Color = theme::ONLINE;
pub const COLOR_SHADOW: Color = theme::SHADOW;

/// Altura da pílula (`SizedBox(height: 30)` em `agent_island.dart`).
pub const PILL_HEIGHT: f32 = 30.0;
/// Raio da borda da pílula (`BorderRadius.circular(15)`).
pub const PILL_RADIUS: f32 = 15.0;
/// Padding horizontal interno da pílula (`EdgeInsets.symmetric(horizontal: 9)`).
pub const PILL_PADDING_H: f32 = 9.0;
/// Largura do painel aberto (`AgentIslandPanel.width = 380.0`).
pub const PANEL_WIDTH: f32 = 380.0;

/// Espaço à esquerda de cada contador na pílula (`EdgeInsets.only(left: 8)`).
const COUNT_LEFT_GAP: f32 = 8.0;
/// Diâmetro da bolinha de estado na pílula (`width: 7, height: 7`).
const COUNT_DOT_SIZE: f32 = 7.0;
/// Espaço entre a bolinha e o número (`SizedBox(width: 4)`).
const COUNT_TEXT_GAP: f32 = 4.0;
/// Largura por dígito do contador em `Font::MONOSPACE` 12px.
const COUNT_DIGIT_WIDTH: f32 = 7.5;

/// Duração da interpolação suave da largura da pílula em milissegundos.
pub const WIDTH_ANIM_DURATION_MS: f32 = 180.0;

/// Duração do festejo (`GorillaMood::Cheer`) quando um agente termina
/// (`Duration(milliseconds: 1400)` em `agent_island.dart`).
pub const CHEER_DURATION_MS: u64 = 1_400;

/// Estado de um agente exibido na ilha, ordenado do menos prioritário para o
/// mais prioritário:
/// `AskingPermission > Waiting > Working > Finished`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IslandAgentState {
    /// Terminou o turno e ainda não foi visto.
    Finished,
    /// Trabalhando no turno.
    Working,
    /// Esperando resposta ou ação do usuário.
    Waiting,
    /// Pedindo permissão de ferramenta.
    AskingPermission,
}

impl IslandAgentState {
    /// Converte um [`TurnStatus`] e [`HookEvent`] vindos do `gsh-core` no estado
    /// correspondente da ilha, sem reimplementar a máquina de turno.
    pub fn from_turn_status(
        status: TurnStatus,
        event: Option<&HookEvent>,
        unseen_finish: bool,
    ) -> Option<Self> {
        match status {
            TurnStatus::Waiting => {
                if matches!(event, Some(HookEvent::PermissionRequest)) {
                    Some(IslandAgentState::AskingPermission)
                } else {
                    Some(IslandAgentState::Waiting)
                }
            }
            TurnStatus::Working => Some(IslandAgentState::Working),
            TurnStatus::Idle => {
                if unseen_finish {
                    Some(IslandAgentState::Finished)
                } else {
                    None
                }
            }
        }
    }

    /// Converte um [`StatusUpdate`] do `gsh-core` no estado da ilha.
    pub fn from_status_update(update: &StatusUpdate, unseen_finish: bool) -> Option<Self> {
        Self::from_turn_status(update.status, update.event.as_ref(), unseen_finish)
    }
}

/// Decisão tomada pelo usuário num card da seção "Pedem permissão"
/// (`AgentPermissionDecision` no Flutter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    AllowAlways,
    Deny,
    Pass,
}

/// Dados do pedido de permissão de uma ferramenta (`AgentPermissionAsk` no
/// Flutter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionAsk {
    pub id: String,
    pub tool_name: String,
    pub input_summary: String,
    pub always_rule: Option<String>,
}

/// Um agente listado na ilha (`IslandAgent` no Flutter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IslandAgent {
    pub tab_id: String,
    pub name: String,
    pub workspace: String,
    pub state: IslandAgentState,
    pub harness: Option<AgentHarness>,
    pub ask: Option<PermissionAsk>,
}

impl IslandAgent {
    pub fn new(
        tab_id: impl Into<String>,
        name: impl Into<String>,
        workspace: impl Into<String>,
        state: IslandAgentState,
    ) -> Self {
        Self {
            tab_id: tab_id.into(),
            name: name.into(),
            workspace: workspace.into(),
            state,
            harness: None,
            ask: None,
        }
    }

    pub fn with_harness(mut self, harness: AgentHarness) -> Self {
        self.harness = Some(harness);
        self
    }

    pub fn with_ask(mut self, ask: PermissionAsk) -> Self {
        self.ask = Some(ask);
        self
    }
}

/// Contadores de agentes por estado para a pílula e para o painel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IslandCounts {
    pub asking: usize,
    pub waiting: usize,
    pub working: usize,
    pub finished: usize,
}

impl IslandCounts {
    pub fn from_agents(agents: &[IslandAgent]) -> Self {
        let mut counts = Self::default();
        for agent in agents {
            match agent.state {
                IslandAgentState::AskingPermission => {
                    counts.asking = counts.asking.saturating_add(1);
                }
                IslandAgentState::Waiting => {
                    counts.waiting = counts.waiting.saturating_add(1);
                }
                IslandAgentState::Working => {
                    counts.working = counts.working.saturating_add(1);
                }
                IslandAgentState::Finished => {
                    counts.finished = counts.finished.saturating_add(1);
                }
            }
        }
        counts
    }

    /// Contador âmbar exibido na pílula: soma quem espera por você e quem pede
    /// permissão (`count(waiting) + asks.length` em `agent_island.dart`).
    pub fn waiting_badge(&self) -> usize {
        self.waiting.saturating_add(self.asking)
    }
}

/// Retorna o estado mais importante presente na lista de agentes, seguindo a
/// prioridade: `AskingPermission > Waiting > Working > Finished`.
pub fn top_agent_state(agents: &[IslandAgent]) -> Option<IslandAgentState> {
    agents.iter().map(|a| a.state).max()
}

/// Escolhe a pose do gorila (`GorillaMood`) pela ordem de prioridade dos
/// estados (`pedindo permissão > esperando você > trabalhando > terminou`) e
/// pelo estado de expansão/festejo (`_mood` em `agent_island.dart`).
pub fn choose_gorilla_mood(agents: &[IslandAgent], expanded: bool, cheering: bool) -> GorillaMood {
    match top_agent_state(agents) {
        Some(IslandAgentState::AskingPermission | IslandAgentState::Waiting) => GorillaMood::Wave,
        Some(_) if cheering => GorillaMood::Cheer,
        Some(IslandAgentState::Working) => GorillaMood::Typing,
        Some(IslandAgentState::Finished) => {
            if expanded {
                GorillaMood::Breathe
            } else {
                GorillaMood::Rest
            }
        }
        None => GorillaMood::Sleep,
    }
}

fn digit_count(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut count = 0;
    let mut val = n;
    while val > 0 {
        count += 1;
        val /= 10;
    }
    count
}

fn single_counter_width(n: usize) -> f32 {
    COUNT_LEFT_GAP + COUNT_DOT_SIZE + COUNT_TEXT_GAP + (digit_count(n) as f32) * COUNT_DIGIT_WIDTH
}

/// Calcula a largura alvo da pílula para os contadores atuais.
pub fn target_pill_width(counts: IslandCounts) -> f32 {
    let mut width = PILL_PADDING_H * 2.0 + GORILLA_WIDTH;
    for n in [counts.working, counts.waiting_badge(), counts.finished] {
        if n > 0 {
            width += single_counter_width(n);
        }
    }
    width
}

/// Curva `Curves.easeOutCubic` (`1 - (1 - t)^3`) para transição suave.
pub fn ease_out_cubic(t: f32) -> f32 {
    let clamped = t.clamp(0.0, 1.0);
    let inv = 1.0 - clamped;
    1.0 - inv * inv * inv
}

/// Interpola a largura da pílula entre `from` e `to` para o progresso
/// normalizado `progress` (`0.0..=1.0`) usando `ease_out_cubic`.
pub fn interpolate_width(from: f32, to: f32, progress: f32) -> f32 {
    let eased = ease_out_cubic(progress);
    from + (to - from) * eased
}

/// Controlador de interpolação por frame da largura da pílula.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidthTween {
    from: f32,
    current: f32,
    target: f32,
    elapsed_ms: f32,
    duration_ms: f32,
}

impl WidthTween {
    pub fn new(initial_width: f32) -> Self {
        Self {
            from: initial_width,
            current: initial_width,
            target: initial_width,
            elapsed_ms: WIDTH_ANIM_DURATION_MS,
            duration_ms: WIDTH_ANIM_DURATION_MS,
        }
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    pub fn is_animating(&self) -> bool {
        (self.current - self.target).abs() > 0.05 && self.elapsed_ms < self.duration_ms
    }

    /// Define um novo alvo de largura. Se diferente do alvo atual, inicia uma
    /// nova interpolação suave a partir da largura instantânea (`current`).
    pub fn set_target(&mut self, next_target: f32) {
        if (next_target - self.target).abs() <= 0.05 {
            return;
        }
        self.from = self.current;
        self.target = next_target;
        self.elapsed_ms = 0.0;
    }

    /// Avança a interpolação em `dt_ms` milissegundos e devolve a largura atual.
    pub fn step(&mut self, dt_ms: f32) -> f32 {
        if !self.is_animating() {
            self.current = self.target;
            return self.current;
        }
        let dt = dt_ms.max(0.0);
        self.elapsed_ms = (self.elapsed_ms + dt).min(self.duration_ms);
        let progress = if self.duration_ms > 0.0 {
            self.elapsed_ms / self.duration_ms
        } else {
            1.0
        };
        self.current = interpolate_width(self.from, self.target, progress);
        if (self.elapsed_ms - self.duration_ms).abs() <= f32::EPSILON {
            self.current = self.target;
        }
        self.current
    }
}

/// Mensagens emitidas pela ilha e pelo painel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IslandMsg {
    /// Clique na pílula: abre ou fecha o painel.
    TogglePanel,
    /// Clique fora do painel: fecha o painel se estiver aberto.
    ClosePanel,
    /// Clique numa linha de agente: vai até a aba e fecha o painel.
    RevealTab(String),
    /// Clique num botão de permissão ("Permitir", "Sempre", "Negar", "No terminal").
    DecidePermission {
        tab_id: String,
        decision: PermissionDecision,
    },
}

/// Estado completo da ilha de agentes no app.
#[derive(Debug, Clone)]
pub struct AgentIsland {
    agents: Vec<IslandAgent>,
    pinned: bool,
    cheering: bool,
    width_tween: WidthTween,
    active_tab_id: Option<String>,
    /// Conta quantas vezes agentes em `Working` passaram a terminar, para o
    /// canvas disparar a janela de `CHEER_DURATION_MS` (1400 ms).
    cheer_generation: u64,
    /// Erro tipado do `StatusServer` caso o socket de status tenha falhado ao iniciar.
    server_error: Option<IslandServerError>,
}

impl Default for AgentIsland {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// Lista de demonstração de agentes (usada em testes unitários da ilha).
pub fn demo_agents() -> Vec<IslandAgent> {
    vec![
        IslandAgent::new(
            labels::DEMO_TAB_1_ID,
            labels::DEMO_TAB_1_NAME,
            labels::DEMO_TAB_1_WORKSPACE,
            IslandAgentState::AskingPermission,
        )
        .with_harness(AgentHarness::Claude)
        .with_ask(PermissionAsk {
            id: labels::DEMO_ASK_1_ID.to_string(),
            tool_name: labels::DEMO_ASK_1_TOOL.to_string(),
            input_summary: labels::DEMO_ASK_1_INPUT.to_string(),
            always_rule: Some(labels::DEMO_ASK_1_RULE.to_string()),
        }),
        IslandAgent::new(
            labels::DEMO_TAB_2_ID,
            labels::DEMO_TAB_2_NAME,
            labels::DEMO_TAB_2_WORKSPACE,
            IslandAgentState::Working,
        )
        .with_harness(AgentHarness::Codex),
        IslandAgent::new(
            labels::DEMO_TAB_3_ID,
            labels::DEMO_TAB_3_NAME,
            labels::DEMO_TAB_3_WORKSPACE,
            IslandAgentState::Finished,
        )
        .with_harness(AgentHarness::Claude),
    ]
}

impl AgentIsland {
    pub fn new(agents: Vec<IslandAgent>) -> Self {
        let counts = IslandCounts::from_agents(&agents);
        let initial_width = target_pill_width(counts);
        Self {
            agents,
            pinned: false,
            cheering: false,
            width_tween: WidthTween::new(initial_width),
            active_tab_id: None,
            cheer_generation: 0,
            server_error: None,
        }
    }

    pub fn agents(&self) -> &[IslandAgent] {
        &self.agents
    }

    pub fn server_error(&self) -> Option<&IslandServerError> {
        self.server_error.as_ref()
    }

    pub fn set_server_error(&mut self, error: Option<IslandServerError>) {
        self.server_error = error;
    }

    pub fn is_expanded(&self) -> bool {
        self.pinned
    }

    pub fn active_tab_id(&self) -> Option<&str> {
        self.active_tab_id.as_deref()
    }

    pub fn counts(&self) -> IslandCounts {
        IslandCounts::from_agents(&self.agents)
    }

    pub fn mood(&self) -> GorillaMood {
        choose_gorilla_mood(&self.agents, self.is_expanded(), self.cheering)
    }

    pub fn width_tween(&self) -> &WidthTween {
        &self.width_tween
    }

    pub fn step_width(&mut self, dt_ms: f32) -> f32 {
        self.width_tween.step(dt_ms)
    }

    /// Substitui a lista de agentes, detectando se algum agente que trabalhava
    /// terminou (sem passar a esperar) para acionar o pulo de comemoração
    /// (`didUpdateWidget` em `agent_island.dart`).
    pub fn set_agents(&mut self, next_agents: Vec<IslandAgent>) {
        let finished_now = self.agents.iter().any(|old| {
            old.state == IslandAgentState::Working
                && !next_agents.iter().any(|n| {
                    n.tab_id == old.tab_id
                        && matches!(
                            n.state,
                            IslandAgentState::Working
                                | IslandAgentState::Waiting
                                | IslandAgentState::AskingPermission
                        )
                })
        });
        if finished_now {
            self.cheering = true;
            self.cheer_generation = self.cheer_generation.saturating_add(1);
        }
        self.agents = next_agents;
        let next_width = target_pill_width(self.counts());
        self.width_tween.set_target(next_width);
    }

    pub fn update(&mut self, msg: IslandMsg) {
        match msg {
            IslandMsg::TogglePanel => {
                self.pinned = !self.pinned;
            }
            IslandMsg::ClosePanel => {
                self.pinned = false;
            }
            IslandMsg::RevealTab(tab_id) => {
                self.pinned = false;
                self.active_tab_id = Some(tab_id.clone());
                let mut next = self.agents.clone();
                // Ao visitar uma aba que já tinha terminado, o aviso "não visto"
                // é consumido.
                next.retain(|a| !(a.tab_id == tab_id && a.state == IslandAgentState::Finished));
                self.set_agents(next);
            }
            IslandMsg::DecidePermission { tab_id, decision } => {
                let mut next = self.agents.clone();
                for agent in &mut next {
                    if agent.tab_id == tab_id {
                        agent.ask = None;
                        match decision {
                            PermissionDecision::Allow | PermissionDecision::AllowAlways => {
                                agent.state = IslandAgentState::Working;
                            }
                            PermissionDecision::Deny => {
                                agent.state = IslandAgentState::Finished;
                            }
                            PermissionDecision::Pass => {
                                agent.state = IslandAgentState::Waiting;
                            }
                        }
                    }
                }
                if decision == PermissionDecision::Pass {
                    self.pinned = false;
                    self.active_tab_id = Some(tab_id);
                }
                self.set_agents(next);
            }
        }
    }

    /// Renderiza a pílula da ilha em canvas no topo da janela.
    pub fn view_pill(&self) -> Element<'_, IslandMsg> {
        let counts = self.counts();
        let target_w = target_pill_width(counts);
        // Largura fixa do canvas acomoda a pílula com folga para interpolar
        // suavemente do centro para as bordas sem deslocar o layout vizinho.
        let canvas_width = target_w.max(self.width_tween.current()).max(160.0) + 24.0;
        let program = IslandPillProgram {
            counts,
            base_mood: choose_gorilla_mood(&self.agents, self.is_expanded(), false),
            cheer_generation: self.cheer_generation,
            target_width: target_w,
            has_server_error: self.server_error.is_some(),
        };
        canvas::Canvas::new(program)
            .width(Length::Fixed(canvas_width))
            .height(Length::Fixed(PILL_HEIGHT))
            .into()
    }

    /// Renderiza o painel aberto com as seções do Flutter na mesma ordem:
    /// "Pedem permissão", "Esperando você", "Terminaram", "Trabalhando".
    pub fn view_panel(&self) -> Element<'_, IslandMsg> {
        agent_island_panel::view_panel(&self.agents, self.server_error.as_ref())
    }
}

/// Estado interno do `canvas::Program` da pílula, persistido entre quadros.
#[derive(Debug)]
struct PillCanvasState {
    initialized: bool,
    width_tween: WidthTween,
    last_frame_at: Option<Instant>,
    gorilla_tick: u64,
    last_pose_step_at: Option<Instant>,
    last_mood: Option<GorillaMood>,
    seen_cheer_generation: u64,
    cheer_until: Option<Instant>,
    /// Largura mais recente efetivamente desenhada (para hit-test de mouse).
    drawn_width: Cell<f32>,
}

impl Default for PillCanvasState {
    fn default() -> Self {
        let initial = PILL_PADDING_H * 2.0 + GORILLA_WIDTH;
        Self {
            initialized: false,
            width_tween: WidthTween::new(initial),
            last_frame_at: None,
            gorilla_tick: 0,
            last_pose_step_at: None,
            last_mood: None,
            seen_cheer_generation: 0,
            cheer_until: None,
            drawn_width: Cell::new(initial),
        }
    }
}

impl PillCanvasState {
    fn effective_mood(&self, base_mood: GorillaMood, now: Option<Instant>) -> GorillaMood {
        if base_mood == GorillaMood::Wave {
            return GorillaMood::Wave;
        }
        let cheering = match (self.cheer_until, now) {
            (Some(until), Some(t)) => t < until,
            (Some(_), None) => true,
            (None, _) => false,
        };
        if cheering {
            GorillaMood::Cheer
        } else {
            base_mood
        }
    }
}

struct IslandPillProgram {
    counts: IslandCounts,
    base_mood: GorillaMood,
    cheer_generation: u64,
    target_width: f32,
    has_server_error: bool,
}

impl IslandPillProgram {
    fn pill_rect(&self, bounds: Rectangle, current_width: f32) -> Rectangle {
        let w = current_width.min(bounds.width);
        let x = ((bounds.width - w) / 2.0).max(0.0);
        let y = ((bounds.height - PILL_HEIGHT) / 2.0).max(0.0);
        Rectangle {
            x,
            y,
            width: w,
            height: PILL_HEIGHT,
        }
    }
}

impl canvas::Program<IslandMsg> for IslandPillProgram {
    type State = PillCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<IslandMsg>> {
        if !state.initialized {
            state.initialized = true;
            state.width_tween = WidthTween::new(self.target_width);
            state.drawn_width.set(self.target_width);
            state.seen_cheer_generation = self.cheer_generation;
        }

        if (state.width_tween.target() - self.target_width).abs() > 0.05 {
            state.width_tween.set_target(self.target_width);
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pill = self.pill_rect(bounds, state.drawn_width.get());
                let local_pill = Rectangle {
                    x: bounds.x + pill.x,
                    y: bounds.y + pill.y,
                    width: pill.width,
                    height: pill.height,
                };
                if cursor.is_over(local_pill) {
                    return Some(Action::publish(IslandMsg::TogglePanel).and_capture());
                }
                None
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if self.cheer_generation > state.seen_cheer_generation {
                    state.seen_cheer_generation = self.cheer_generation;
                    state.cheer_until = Some(*now + Duration::from_millis(CHEER_DURATION_MS));
                }
                if state.cheer_until.is_some_and(|until| *now >= until) {
                    state.cheer_until = None;
                }

                let dt_ms = state
                    .last_frame_at
                    .map(|prev| now.saturating_duration_since(prev).as_secs_f32() * 1000.0)
                    .unwrap_or(16.0)
                    .clamp(1.0, 100.0);
                state.last_frame_at = Some(*now);

                let current_w = state.width_tween.step(dt_ms);
                state.drawn_width.set(current_w);

                let mood = state.effective_mood(self.base_mood, Some(*now));
                if state.last_mood != Some(mood) {
                    state.last_mood = Some(mood);
                    state.gorilla_tick = 0;
                    state.last_pose_step_at = Some(*now);
                }

                let mut next_redraw: Option<Instant> = state.cheer_until;

                if let Some(interval_ms) = mood.frame_interval_ms() {
                    let interval = Duration::from_millis(interval_ms);
                    let last_step = state.last_pose_step_at.unwrap_or(*now);
                    if now.saturating_duration_since(last_step) >= interval {
                        state.gorilla_tick = state.gorilla_tick.saturating_add(1);
                        state.last_pose_step_at = Some(*now);
                    }
                    let pose_deadline = state.last_pose_step_at.unwrap_or(*now) + interval;
                    next_redraw = Some(match next_redraw {
                        Some(existing) if existing < pose_deadline => existing,
                        _ => pose_deadline,
                    });
                }

                if state.width_tween.is_animating() {
                    Some(Action::request_redraw())
                } else {
                    next_redraw.map(Action::request_redraw_at)
                }
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let current_width = if state.initialized {
            state.width_tween.current()
        } else {
            self.target_width
        };
        state.drawn_width.set(current_width);

        let pill = self.pill_rect(bounds, current_width);
        let global_pill = Rectangle {
            x: bounds.x + pill.x,
            y: bounds.y + pill.y,
            width: pill.width,
            height: pill.height,
        };
        let hovered = cursor.is_over(global_pill);

        let bg_color = if hovered { COLOR_PANEL3 } else { COLOR_PANEL2 };
        let waiting_total = self.counts.waiting_badge();
        let border_color = if self.has_server_error {
            COLOR_ERROR
        } else if waiting_total > 0 {
            COLOR_WARN
        } else {
            COLOR_BORDER
        };

        let pill_path = Path::rounded_rectangle(
            Point::new(pill.x, pill.y),
            Size::new(pill.width, pill.height),
            border::Radius::from(PILL_RADIUS),
        );
        frame.fill(&pill_path, bg_color);

        let mood = state.effective_mood(self.base_mood, state.last_frame_at);
        let tick = state.gorilla_tick;

        // Desenha no mesmo `frame` (após o fundo da pílula e antes da borda) usando
        // coordenadas relativas a `pill.x`/`pill.y`. Evita `with_clip`, que em
        // `iced_wgpu` insere os meshes do sub-frame antes do buffer do frame pai
        // (deixando o gorila atrás da pílula) e não translada a origem para `pill.x`.
        let gorilla_rect = gorilla_bounds_in_pill(pill);
        draw_gorilla(
            &mut frame,
            gorilla_rect.position(),
            mood,
            tick,
            COLOR_WARN,
            COLOR_BG,
        );

        let mut cursor_x = gorilla_rect.x + GORILLA_WIDTH;
        let center_y = pill.y + PILL_HEIGHT / 2.0;
        let max_right = pill.x + pill.width - PILL_PADDING_H + 0.5;

        for (n, dot_color) in [
            (self.counts.working, COLOR_ACCENT),
            (waiting_total, COLOR_WARN),
            (self.counts.finished, COLOR_ONLINE),
        ] {
            if n == 0 {
                continue;
            }
            let item_w = single_counter_width(n);
            if cursor_x + item_w > max_right {
                break;
            }
            cursor_x += COUNT_LEFT_GAP;
            let dot_center = Point::new(cursor_x + COUNT_DOT_SIZE / 2.0, center_y);
            let dot = Path::circle(dot_center, COUNT_DOT_SIZE / 2.0);
            frame.fill(&dot, dot_color);
            cursor_x += COUNT_DOT_SIZE + COUNT_TEXT_GAP;

            frame.fill_text(canvas::Text {
                content: n.to_string(),
                position: Point::new(cursor_x, center_y),
                max_width: 48.0,
                color: COLOR_TEXT2,
                size: theme::SIZE_LABEL.into(),
                line_height: text::LineHeight::default(),
                font: theme::FONT_MONO,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Center,
                shaping: text::Shaping::Basic,
            });

            cursor_x += (digit_count(n) as f32) * COUNT_DIGIT_WIDTH;
        }

        frame.stroke(
            &pill_path,
            Stroke::default().with_color(border_color).with_width(1.0),
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let pill = self.pill_rect(bounds, state.drawn_width.get());
        let global_pill = Rectangle {
            x: bounds.x + pill.x,
            y: bounds.y + pill.y,
            width: pill.width,
            height: pill.height,
        };
        if cursor.is_over(global_pill) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

/// Calcula o retângulo (`32 x 25`) ocupado pelo gorila dentro do retângulo da pílula.
pub fn gorilla_bounds_in_pill(pill: Rectangle) -> Rectangle {
    Rectangle {
        x: pill.x + PILL_PADDING_H,
        y: pill.y + (PILL_HEIGHT - GORILLA_HEIGHT) / 2.0,
        width: GORILLA_WIDTH,
        height: GORILLA_HEIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gorilla::{clawd_hd, frame_spec, zed_sprite};

    fn agent(id: &str, state: IslandAgentState) -> IslandAgent {
        IslandAgent::new(id, format!("Agent {id}"), "GSH", state)
    }

    #[test]
    fn prioridade_dos_estados_segue_permissao_esperando_trabalhando_terminou() {
        assert!(IslandAgentState::AskingPermission > IslandAgentState::Waiting);
        assert!(IslandAgentState::Waiting > IslandAgentState::Working);
        assert!(IslandAgentState::Working > IslandAgentState::Finished);

        // Todos juntos -> AskingPermission vence
        let all = vec![
            agent("1", IslandAgentState::Finished),
            agent("2", IslandAgentState::Working),
            agent("3", IslandAgentState::Waiting),
            agent("4", IslandAgentState::AskingPermission),
        ];
        assert_eq!(
            top_agent_state(&all),
            Some(IslandAgentState::AskingPermission)
        );
        assert_eq!(choose_gorilla_mood(&all, false, false), GorillaMood::Wave);
        assert_eq!(choose_gorilla_mood(&all, true, true), GorillaMood::Wave);

        // Sem AskingPermission -> Waiting vence Working e Finished
        let without_ask = vec![
            agent("1", IslandAgentState::Finished),
            agent("2", IslandAgentState::Working),
            agent("3", IslandAgentState::Waiting),
        ];
        assert_eq!(
            top_agent_state(&without_ask),
            Some(IslandAgentState::Waiting)
        );
        assert_eq!(
            choose_gorilla_mood(&without_ask, false, false),
            GorillaMood::Wave
        );

        // Sem Waiting -> Working vence Finished
        let working_and_finished = vec![
            agent("1", IslandAgentState::Finished),
            agent("2", IslandAgentState::Working),
        ];
        assert_eq!(
            top_agent_state(&working_and_finished),
            Some(IslandAgentState::Working)
        );
        assert_eq!(
            choose_gorilla_mood(&working_and_finished, false, false),
            GorillaMood::Typing
        );
        // Se um agente acabou de terminar enquanto outro trabalha, comemora por 1400ms
        assert_eq!(
            choose_gorilla_mood(&working_and_finished, false, true),
            GorillaMood::Cheer
        );

        // Só Finished -> Rest (fechada), Breathe (aberta) ou Cheer (recém-terminado)
        let only_finished = vec![agent("1", IslandAgentState::Finished)];
        assert_eq!(
            top_agent_state(&only_finished),
            Some(IslandAgentState::Finished)
        );
        assert_eq!(
            choose_gorilla_mood(&only_finished, false, false),
            GorillaMood::Rest
        );
        assert_eq!(
            choose_gorilla_mood(&only_finished, true, false),
            GorillaMood::Breathe
        );
        assert_eq!(
            choose_gorilla_mood(&only_finished, false, true),
            GorillaMood::Cheer
        );

        // Lista vazia -> None / Sleep
        let empty: Vec<IslandAgent> = Vec::new();
        assert_eq!(top_agent_state(&empty), None);
        assert_eq!(
            choose_gorilla_mood(&empty, false, false),
            GorillaMood::Sleep
        );
    }

    #[test]
    fn mapeia_status_do_core_para_estado_da_ilha() {
        let ask_update = StatusUpdate {
            pane_id: "t1".into(),
            status: TurnStatus::Waiting,
            event: Some(HookEvent::PermissionRequest),
            session_id: None,
            transcript_path: None,
            harness: Some(AgentHarness::Claude),
            files: Vec::new(),
        };
        assert_eq!(
            IslandAgentState::from_status_update(&ask_update, false),
            Some(IslandAgentState::AskingPermission)
        );

        let wait_update = StatusUpdate {
            event: Some(HookEvent::Notification),
            ..ask_update.clone()
        };
        assert_eq!(
            IslandAgentState::from_status_update(&wait_update, false),
            Some(IslandAgentState::Waiting)
        );

        let work_update = StatusUpdate {
            status: TurnStatus::Working,
            event: Some(HookEvent::PreToolUse),
            ..ask_update.clone()
        };
        assert_eq!(
            IslandAgentState::from_status_update(&work_update, false),
            Some(IslandAgentState::Working)
        );

        let idle_update = StatusUpdate {
            status: TurnStatus::Idle,
            event: Some(HookEvent::Stop),
            ..ask_update
        };
        assert_eq!(
            IslandAgentState::from_status_update(&idle_update, true),
            Some(IslandAgentState::Finished)
        );
        assert_eq!(
            IslandAgentState::from_status_update(&idle_update, false),
            None
        );
    }

    #[test]
    fn interpolacao_da_largura_da_pilula_e_suave_e_monotonica() {
        let empty_w = target_pill_width(IslandCounts::default());
        let one_w = target_pill_width(IslandCounts {
            working: 1,
            ..Default::default()
        });
        let two_w = target_pill_width(IslandCounts {
            working: 1,
            waiting: 1,
            ..Default::default()
        });
        let three_w = target_pill_width(IslandCounts {
            working: 1,
            waiting: 1,
            finished: 1,
            asking: 1,
        });

        assert!(empty_w < one_w);
        assert!(one_w < two_w);
        assert!(two_w < three_w);

        // Extremos e clamp da função pura
        assert!((interpolate_width(50.0, 110.0, -0.5) - 50.0).abs() < f32::EPSILON);
        assert!((interpolate_width(50.0, 110.0, 0.0) - 50.0).abs() < f32::EPSILON);
        assert!((interpolate_width(50.0, 110.0, 1.0) - 110.0).abs() < f32::EPSILON);
        assert!((interpolate_width(50.0, 110.0, 1.5) - 110.0).abs() < f32::EPSILON);

        // Em t = 0.5 com ease_out_cubic (1 - 0.5^3 = 0.875), já avançou 87.5% do percurso
        let mid = interpolate_width(50.0, 130.0, 0.5);
        assert!((mid - 120.0).abs() < 0.01);

        // Simulação frame a frame com WidthTween crescendo e depois encolhendo
        let mut tween = WidthTween::new(empty_w);
        assert!(!tween.is_animating());

        tween.set_target(three_w);
        assert!(tween.is_animating());

        let mut last = tween.current();
        for _ in 0..20 {
            let next = tween.step(16.0);
            assert!(next >= last, "largura deve crescer monotonicamente");
            last = next;
        }
        assert!(!tween.is_animating());
        assert!((tween.current() - three_w).abs() < 0.01);

        // Encolhendo de volta para 1 contador
        tween.set_target(one_w);
        assert!(tween.is_animating());
        let mut last = tween.current();
        for _ in 0..20 {
            let next = tween.step(16.0);
            assert!(next <= last, "largura deve diminuir monotonicamente");
            last = next;
        }
        assert!(!tween.is_animating());
        assert!((tween.current() - one_w).abs() < 0.01);
    }

    #[test]
    fn poses_do_gorila_tem_ritmo_e_sprites_fieis_ao_flutter() {
        assert_eq!(GorillaMood::Sleep.frame_interval_ms(), None);
        assert_eq!(GorillaMood::Rest.frame_interval_ms(), None);
        assert_eq!(GorillaMood::Breathe.frame_interval_ms(), Some(650));
        assert_eq!(GorillaMood::Typing.frame_interval_ms(), Some(140));
        assert_eq!(GorillaMood::Wave.frame_interval_ms(), Some(280));
        assert_eq!(GorillaMood::Cheer.frame_interval_ms(), Some(90));

        for mood in [
            GorillaMood::Sleep,
            GorillaMood::Rest,
            GorillaMood::Breathe,
            GorillaMood::Typing,
            GorillaMood::Wave,
            GorillaMood::Cheer,
        ] {
            let spec = frame_spec(mood, 1);
            let pix = clawd_hd(spec);
            assert!(pix.filled_count() > 200, "pose {mood:?} vazia");
        }
        assert!(zed_sprite().filled_count() > 10);
    }

    #[test]
    fn gorila_fica_inteiro_dentro_do_contorno_da_pilula_em_todas_as_poses() {
        let program = IslandPillProgram {
            counts: IslandCounts {
                asking: 1,
                working: 1,
                finished: 1,
                waiting: 0,
            },
            base_mood: GorillaMood::Wave,
            cheer_generation: 0,
            target_width: target_pill_width(IslandCounts {
                asking: 1,
                working: 1,
                finished: 1,
                waiting: 0,
            }),
            has_server_error: false,
        };
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: PILL_HEIGHT,
        };
        let pill = program.pill_rect(bounds, program.target_width);
        let g = gorilla_bounds_in_pill(pill);

        assert!(g.x >= pill.x + PILL_PADDING_H - f32::EPSILON);
        assert!(g.x + g.width <= pill.x + pill.width - PILL_PADDING_H + f32::EPSILON);
        assert!(g.y >= pill.y);
        assert!(g.y + g.height <= pill.y + pill.height);

        for mood in [
            GorillaMood::Sleep,
            GorillaMood::Rest,
            GorillaMood::Breathe,
            GorillaMood::Typing,
            GorillaMood::Wave,
            GorillaMood::Cheer,
        ] {
            for tick in 0..20 {
                let spec = frame_spec(mood, tick);
                // Topo da cabeça no recorte (y_rel >= 1.0) somado a origin.y (2.5) + dy nunca sai do topo (0.0)
                let top_y = g.y + 1.0 + spec.dy;
                let bottom_y = g.y + (GORILLA_HEIGHT - 1.0) + spec.dy;
                assert!(
                    top_y >= pill.y,
                    "pose {mood:?} tick {tick} vazou pelo topo: {top_y} < {}",
                    pill.y
                );
                assert!(
                    bottom_y <= pill.y + pill.height,
                    "pose {mood:?} tick {tick} vazou por baixo: {bottom_y} > {}",
                    pill.y + pill.height
                );
            }
        }
    }
}
