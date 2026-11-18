//! Pane de terminal do GSH (`gsh-app`): conecta [`TermSession`] ao canvas do
//! `iced`, teclado (`keyboard::listen`), área de transferência
//! (`Ctrl+Shift+C`/`Ctrl+Shift+V`), mouse (scrollback e seleção) e resize.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gsh_term::{Cell, Cursor, GridPos, ShellCommand, TermSession, TermSnapshot};
use iced::widget::{canvas, container, text};
use iced::{clipboard, keyboard, Element, Fill, Subscription, Task};

use crate::labels;
use crate::terminal_canvas::TerminalCanvasProgram;
pub use crate::terminal_grid::{
    compute_grid_size, exceeds_drag_threshold, pixel_to_grid, scroll_delta_to_step, should_resize,
    CellMetrics, ScrollStep, SelectionRange, DEFAULT_COLS, DEFAULT_ROWS, MIN_COLS, MIN_ROWS,
};
pub use crate::terminal_input::{
    classify_key_press, ctrl_char_byte, key_bytes, key_bytes_with_modifiers, named_key_bytes,
    KeyAction,
};
use crate::theme;

/// Intervalo do ticker de sincronização do snapshot (~60 Hz).
pub const TICK_INTERVAL_MS: u64 = 16;

/// Mensagens tratadas pelo [`TerminalPane`].
#[derive(Debug, Clone)]
pub enum TerminalMsg {
    /// Pulso periódico (~16 ms) para sincronizar o snapshot quando houver mudança.
    Tick,
    /// Evento global de teclado vindo de `iced::keyboard::listen`.
    Keyboard(keyboard::Event),
    /// Redimensionamento do grid em `(cols, rows)` calculado pelas métricas da fonte.
    Resize(u16, u16),
    /// Rolagem do histórico para cima em `N` linhas.
    ScrollUp(usize),
    /// Rolagem do histórico para baixo em `N` linhas.
    ScrollDown(usize),
    /// Início de seleção por arraste entre `anchor` e `focus`.
    StartSelection { anchor: GridPos, focus: GridPos },
    /// Extensão da seleção ativa até `focus`.
    ExtendSelection(GridPos),
    /// Limpeza da seleção ativa (clique simples).
    ClearSelection,
    /// Conteúdo lido da área de transferência após `Ctrl+Shift+V`.
    ClipboardLoaded(Option<String>),
}

/// Estado e controlador de um pane de terminal.
pub struct TerminalPane {
    pane_id: String,
    spawn_env: Vec<(String, String)>,
    session: Option<TermSession>,
    snapshot: TermSnapshot,
    selection: Option<SelectionRange>,
    cache: canvas::Cache,
    metrics: CellMetrics,
    cwd: Option<PathBuf>,
    has_spawn_error: bool,
    config: gsh_config::Config,
    pending_agent_enters: VecDeque<Instant>,
}

impl Default for TerminalPane {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalPane {
    /// Inicia um novo pane de terminal com o shell padrão da plataforma.
    pub fn new() -> Self {
        Self::new_in_dir(None)
    }

    /// Inicia um novo pane de terminal no diretório `cwd` informado (ou no
    /// diretório atual do processo quando `None`).
    pub fn new_in_dir(cwd: Option<&Path>) -> Self {
        Self::spawn_with_dims(
            String::new(),
            cwd,
            Vec::new(),
            DEFAULT_COLS,
            DEFAULT_ROWS,
            gsh_config::Config::default(),
        )
    }

    /// Inicia um novo pane de terminal com `pane_id` estável e variáveis de ambiente
    /// do hook (`GSH_PANE_ID`, `GSH_TAB_ID`, `GSH_STATUS_SOCK`) injetadas no processo filho.
    pub fn new_with_env(
        pane_id: impl Into<String>,
        cwd: Option<&Path>,
        spawn_env: Vec<(String, String)>,
    ) -> Self {
        Self::new_with_config(pane_id, cwd, spawn_env, gsh_config::Config::default())
    }

    pub fn new_with_config(
        pane_id: impl Into<String>,
        cwd: Option<&Path>,
        spawn_env: Vec<(String, String)>,
        config: gsh_config::Config,
    ) -> Self {
        Self::spawn_with_dims(
            pane_id.into(),
            cwd,
            spawn_env,
            DEFAULT_COLS,
            DEFAULT_ROWS,
            config,
        )
    }

    /// Reinicia a sessão de terminal no diretório `cwd` preservando `(cols, rows)`,
    /// `pane_id` e `spawn_env` atuais.
    /// Em modo estático de teste (`session.is_none()` e `!has_spawn_error`), mantém o snapshot
    /// e apenas atualiza `cwd`.
    pub fn respawn_in_dir(&mut self, cwd: Option<&Path>) {
        if self.session.is_none() && !self.has_spawn_error {
            self.cwd = cwd.map(Path::to_path_buf);
            return;
        }
        let (cols, rows) = self.size();
        let pane_id = self.pane_id.clone();
        let spawn_env = self.spawn_env.clone();
        *self = Self::spawn_with_dims(pane_id, cwd, spawn_env, cols, rows, self.config.clone());
    }

    fn spawn_with_dims(
        pane_id: String,
        cwd: Option<&Path>,
        spawn_env: Vec<(String, String)>,
        cols: u16,
        rows: u16,
        config: gsh_config::Config,
    ) -> Self {
        let mut cmd = ShellCommand::new(
            config
                .terminal
                .shell
                .as_deref()
                .unwrap_or(gsh_term::default_shell()),
        )
        .scrollback_lines(config.terminal.scrollback.clamp(1000, 100_000));
        let scale =
            (config.effective_terminal_size().clamp(8.0, 32.0) as f32) / theme::TERM_FONT_SIZE;
        if let Some(dir) = cwd {
            cmd = cmd.cwd(dir);
        }
        for (k, v) in &spawn_env {
            cmd = cmd.env(k, v);
        }
        let cwd_owned = cwd.map(Path::to_path_buf);
        match TermSession::spawn(cmd, cols, rows) {
            Ok(session) => {
                let mut pane = Self::with_session(session);
                pane.pane_id = pane_id;
                pane.spawn_env = spawn_env;
                pane.cwd = cwd_owned;
                pane.metrics.cell_width *= scale;
                pane.metrics.cell_height *= scale;
                pane.config = config;
                pane
            }
            Err(_) => Self {
                pane_id,
                spawn_env,
                session: None,
                snapshot: empty_snapshot(cols, rows),
                selection: None,
                cache: canvas::Cache::new(),
                metrics: CellMetrics::default(),
                cwd: cwd_owned,
                has_spawn_error: true,
                config,
                pending_agent_enters: VecDeque::new(),
            },
        }
    }

    /// Constrói um [`TerminalPane`] a partir de uma [`TermSession`] já iniciada.
    pub fn with_session(session: TermSession) -> Self {
        let snapshot = session.snapshot();
        Self {
            pane_id: String::new(),
            spawn_env: Vec::new(),
            session: Some(session),
            snapshot,
            selection: None,
            cache: canvas::Cache::new(),
            metrics: CellMetrics::default(),
            cwd: None,
            has_spawn_error: false,
            config: gsh_config::Config::default(),
            pending_agent_enters: VecDeque::new(),
        }
    }

    /// Constrói um [`TerminalPane`] estático a partir de um [`TermSnapshot`] (útil para testes).
    pub fn from_snapshot(snapshot: TermSnapshot) -> Self {
        Self::from_snapshot_in_dir(snapshot, None)
    }

    /// Constrói um [`TerminalPane`] estático associado a `cwd` (útil para testes).
    pub fn from_snapshot_in_dir(snapshot: TermSnapshot, cwd: Option<PathBuf>) -> Self {
        Self {
            pane_id: String::new(),
            spawn_env: Vec::new(),
            session: None,
            snapshot,
            selection: None,
            cache: canvas::Cache::new(),
            metrics: CellMetrics::default(),
            cwd,
            has_spawn_error: false,
            config: gsh_config::Config::default(),
            pending_agent_enters: VecDeque::new(),
        }
    }

    /// Define o `pane_id` estável desta aba de terminal.
    pub fn with_pane_id(mut self, pane_id: impl Into<String>) -> Self {
        self.pane_id = pane_id.into();
        self
    }

    /// Define as variáveis de ambiente de hook a injetar em respawns futuros.
    pub fn with_spawn_env(mut self, spawn_env: Vec<(String, String)>) -> Self {
        self.spawn_env = spawn_env;
        self
    }

    /// Identificador estável desta aba de terminal (ex.: `"t1"`).
    pub fn pane_id(&self) -> &str {
        &self.pane_id
    }

    /// Variáveis de ambiente injetadas no processo filho desta sessão.
    pub fn spawn_env(&self) -> &[(String, String)] {
        &self.spawn_env
    }

    /// Envia texto ao shell ativo, usado pelo handoff da sessão de chat.
    pub fn send_text(&mut self, value: &str) -> bool {
        let Some(session) = &self.session else {
            return false;
        };
        if session.send(value.as_bytes()).is_err() {
            return false;
        }
        self.sync_snapshot();
        true
    }

    /// Diretório de trabalho configurado para esta aba de terminal.
    pub fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    /// Encerra explicitamente a sessão PTY associada a esta aba, se ativa.
    pub fn close_session(&mut self) {
        let _ = self.session.take();
    }

    /// Envia um comando seguido de quebra de linha para a sessão PTY, se ativa.
    pub fn send_command(&mut self, command: &str) {
        if let Some(session) = &self.session {
            session.scroll_to_bottom();
            let line = format!("{command}\n");
            let _ = session.send(line.as_bytes());
            self.sync_snapshot();
        }
    }

    /// Escreve a mensagem e envia Enter após a pausa que os TUI precisam para
    /// distinguir texto submetido de uma colagem.
    pub fn send_agent_message(&mut self, message: &str) -> bool {
        if !self.send_text(message) {
            return false;
        }
        self.pending_agent_enters
            .push_back(Instant::now() + Duration::from_millis(400));
        true
    }

    /// Fotografia atual do grid visível.
    pub fn snapshot(&self) -> &TermSnapshot {
        &self.snapshot
    }

    /// Seleção visual atualmente ativa, se houver.
    pub fn selection(&self) -> Option<SelectionRange> {
        self.selection
    }

    /// Texto selecionado na sessão do terminal.
    pub fn selected_text(&self) -> Option<String> {
        self.session.as_ref().and_then(TermSession::selected_text)
    }

    /// Dimensões atuais `(cols, rows)` do grid.
    pub fn size(&self) -> (u16, u16) {
        if let Some(session) = &self.session {
            session.size()
        } else {
            (self.snapshot.cols, self.snapshot.rows)
        }
    }

    /// Sincroniza o snapshot com a sessão e invalida o cache somente se houve alteração.
    pub fn sync_snapshot(&mut self) -> bool {
        let Some(session) = &self.session else {
            return false;
        };
        let next = session.snapshot();
        if next != self.snapshot {
            self.snapshot = next;
            self.cache.clear();
            true
        } else {
            false
        }
    }

    pub fn working_directory(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    /// Entrega texto a uma sessão de agente que já foi validada pelo status do harness.
    pub fn deliver_agent_text(&mut self, text: &str) -> Result<(), gsh_term::TermError> {
        let session = self.session.as_ref().ok_or(gsh_term::TermError::Closed)?;
        session.send(text.as_bytes())?;
        self.sync_snapshot();
        Ok(())
    }

    /// Processa uma [`TerminalMsg`] e devolve uma [`Task`] (como leitura/escrita de clipboard).
    pub fn update(&mut self, msg: TerminalMsg) -> Task<TerminalMsg> {
        match msg {
            TerminalMsg::Tick => {
                if self
                    .pending_agent_enters
                    .front()
                    .is_some_and(|due| *due <= Instant::now())
                {
                    self.pending_agent_enters.pop_front();
                    let _ = self.send_text("\r");
                }
                self.sync_snapshot();
                Task::none()
            }
            TerminalMsg::Keyboard(keyboard::Event::KeyPressed {
                key,
                modified_key,
                modifiers,
                ..
            }) => self.handle_key_press(&key, &modified_key, modifiers),
            TerminalMsg::Keyboard(_) => Task::none(),
            TerminalMsg::Resize(cols, rows) => {
                if should_resize(self.size(), (cols, rows)) {
                    if let Some(session) = &self.session {
                        if session.resize(cols, rows).is_ok() {
                            self.snapshot = session.snapshot();
                            self.cache.clear();
                        }
                    } else {
                        self.snapshot = empty_snapshot(cols, rows);
                        self.cache.clear();
                    }
                }
                Task::none()
            }
            TerminalMsg::ScrollUp(lines) => {
                if let Some(session) = &self.session {
                    session.scroll_up(lines);
                    self.sync_snapshot();
                }
                Task::none()
            }
            TerminalMsg::ScrollDown(lines) => {
                if let Some(session) = &self.session {
                    session.scroll_down(lines);
                    self.sync_snapshot();
                }
                Task::none()
            }
            TerminalMsg::StartSelection { anchor, focus } => {
                if let Some(session) = &self.session {
                    session.start_selection(anchor.row, anchor.col);
                    session.extend_selection(focus.row, focus.col);
                }
                let next_sel = Some(SelectionRange::new(anchor, focus));
                if self.selection != next_sel {
                    self.selection = next_sel;
                    self.cache.clear();
                }
                Task::none()
            }
            TerminalMsg::ExtendSelection(focus) => {
                let anchor = self.selection.map_or(focus, |s| s.anchor);
                if let Some(session) = &self.session {
                    if self.selection.is_none() {
                        session.start_selection(anchor.row, anchor.col);
                    }
                    session.extend_selection(focus.row, focus.col);
                }
                let next_sel = Some(SelectionRange::new(anchor, focus));
                if self.selection != next_sel {
                    self.selection = next_sel;
                    self.cache.clear();
                }
                Task::none()
            }
            TerminalMsg::ClearSelection => {
                if let Some(session) = &self.session {
                    session.clear_selection();
                }
                if self.selection.take().is_some() {
                    self.cache.clear();
                }
                Task::none()
            }
            TerminalMsg::ClipboardLoaded(Some(contents)) => {
                if !contents.is_empty() {
                    if let Some(session) = &self.session {
                        session.scroll_to_bottom();
                        let _ = session.send(contents.as_bytes());
                        self.sync_snapshot();
                    }
                }
                Task::none()
            }
            TerminalMsg::ClipboardLoaded(None) => Task::none(),
        }
    }

    fn handle_key_press(
        &mut self,
        key: &keyboard::Key,
        modified_key: &keyboard::Key,
        modifiers: keyboard::Modifiers,
    ) -> Task<TerminalMsg> {
        let Some(action) = classify_key_press(key, modified_key, modifiers) else {
            return Task::none();
        };

        match action {
            KeyAction::Copy => {
                if let Some(selected) = self.selected_text().filter(|s| !s.is_empty()) {
                    return clipboard::write(selected);
                }
                Task::none()
            }
            KeyAction::Paste => clipboard::read().map(TerminalMsg::ClipboardLoaded),
            KeyAction::Send(bytes) => {
                if let Some(session) = &self.session {
                    if self.snapshot.display_offset > 0 {
                        session.scroll_to_bottom();
                    }
                    let _ = session.send(&bytes);
                    self.sync_snapshot();
                }
                Task::none()
            }
        }
    }

    /// Assinatura de eventos do terminal: ticker de ~16 ms e listener de teclado.
    pub fn subscription(&self) -> Subscription<TerminalMsg> {
        let tick =
            iced::time::every(Duration::from_millis(TICK_INTERVAL_MS)).map(|_| TerminalMsg::Tick);
        let keys = keyboard::listen().map(TerminalMsg::Keyboard);
        Subscription::batch([tick, keys])
    }

    /// Renderiza o canvas do terminal ou a mensagem de erro caso o PTY não tenha iniciado.
    pub fn view(&self) -> Element<'_, TerminalMsg> {
        if self.has_spawn_error {
            return container(
                text(labels::TERMINAL_SPAWN_ERROR)
                    .size(13)
                    .color(theme::SHELL_MUTED),
            )
            .padding(16)
            .width(Fill)
            .height(Fill)
            .into();
        }

        let program = TerminalCanvasProgram {
            snapshot: &self.snapshot,
            selection: self.selection,
            cache: &self.cache,
            metrics: self.metrics,
        };

        canvas::Canvas::new(program).width(Fill).height(Fill).into()
    }
}

fn empty_snapshot(cols: u16, rows: u16) -> TermSnapshot {
    let c = usize::from(cols);
    let r = usize::from(rows);
    TermSnapshot {
        cols,
        rows,
        lines: (0..r).map(|_| vec![Cell::default(); c]).collect(),
        cursor: Cursor {
            row: 0,
            col: 0,
            visible: true,
        },
        display_offset: 0,
        history_size: 0,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use gsh_term::{default_shell, ShellCommand};
    use iced::keyboard::key::{Named, NativeCode, Physical};

    use super::*;

    fn key_press_msg(key: keyboard::Key, modifiers: keyboard::Modifiers) -> TerminalMsg {
        TerminalMsg::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn resize_and_selection_update_state_only_when_changed() {
        let mut pane = TerminalPane::from_snapshot(empty_snapshot(80, 24));
        assert_eq!(pane.size(), (80, 24));

        // Mesmo tamanho não altera
        let _ = pane.update(TerminalMsg::Resize(80, 24));
        assert_eq!(pane.size(), (80, 24));

        // Novo tamanho redimensiona
        let _ = pane.update(TerminalMsg::Resize(100, 30));
        assert_eq!(pane.size(), (100, 30));

        // Seleção: iniciar, estender e limpar
        let anchor = GridPos::new(1, 2);
        let focus = GridPos::new(1, 8);
        let _ = pane.update(TerminalMsg::StartSelection { anchor, focus });
        assert_eq!(pane.selection(), Some(SelectionRange::new(anchor, focus)));

        let next_focus = GridPos::new(2, 4);
        let _ = pane.update(TerminalMsg::ExtendSelection(next_focus));
        assert_eq!(
            pane.selection(),
            Some(SelectionRange::new(anchor, next_focus))
        );

        let _ = pane.update(TerminalMsg::ClearSelection);
        assert_eq!(pane.selection(), None);
    }

    #[test]
    fn live_session_receives_keyboard_paste_and_selection_via_pane() {
        let mut cmd = ShellCommand::new(default_shell()).env("PS1", "$ ");
        #[cfg(unix)]
        {
            cmd = cmd.args(["--norc", "--noprofile"]);
        }
        let session = TermSession::spawn(cmd, 80, 24).expect("spawn shell");
        let mut pane = TerminalPane::with_session(session);

        // Cola comando via ClipboardLoaded e envia Enter pelo teclado
        let _ = pane.update(TerminalMsg::ClipboardLoaded(Some(
            "printf 'PANE_UI_%s\\n' OK".to_owned(),
        )));
        let _ = pane.update(key_press_msg(
            keyboard::Key::Named(Named::Enter),
            keyboard::Modifiers::NONE,
        ));

        let deadline = Instant::now() + Duration::from_secs(5);
        while !pane.snapshot().text().contains("PANE_UI_OK") {
            assert!(Instant::now() < deadline, "timeout waiting for PANE_UI_OK");
            std::thread::sleep(Duration::from_millis(16));
            let _ = pane.update(TerminalMsg::Tick);
        }

        // Quando não há saída nova, sync_snapshot retorna false (sem redesenhar à toa)
        let _ = pane.sync_snapshot();
        assert!(!pane.sync_snapshot());

        // Seleciona "PANE_UI_OK" na linha onde apareceu e verifica selected_text
        let (row, col) = pane
            .snapshot()
            .lines
            .iter()
            .enumerate()
            .find_map(|(r, _)| {
                let line = pane.snapshot().line_text(r)?;
                if line.trim_start().starts_with("PANE_UI_OK") {
                    line.find("PANE_UI_OK").map(|c| (r, c))
                } else {
                    None
                }
            })
            .expect("output row found");

        let _ = pane.update(TerminalMsg::StartSelection {
            anchor: GridPos::new(row, col),
            focus: GridPos::new(row, col + "PANE_UI_OK".len() - 1),
        });
        assert_eq!(pane.selected_text().as_deref(), Some("PANE_UI_OK"));

        let _ = pane.update(TerminalMsg::ClearSelection);
        assert_eq!(pane.selected_text(), None);
    }

    #[test]
    fn spawned_shell_inherits_hook_env_variables() {
        let env_vars = vec![
            ("GSH_PANE_ID".to_owned(), "t42".to_owned()),
            ("GSH_TAB_ID".to_owned(), "t42".to_owned()),
            (
                "GSH_STATUS_SOCK".to_owned(),
                "/tmp/gsh-test.sock".to_owned(),
            ),
        ];
        let mut pane = TerminalPane::new_with_env("t42", None, env_vars.clone());
        assert_eq!(pane.pane_id(), "t42");
        assert_eq!(pane.spawn_env(), env_vars.as_slice());

        let _ = pane.update(TerminalMsg::ClipboardLoaded(Some(
            "printf 'HOOK_ENV=%s|%s|%s\\n' \"$GSH_PANE_ID\" \"$GSH_TAB_ID\" \"$GSH_STATUS_SOCK\""
                .to_owned(),
        )));
        let _ = pane.update(key_press_msg(
            keyboard::Key::Named(Named::Enter),
            keyboard::Modifiers::NONE,
        ));

        let deadline = Instant::now() + Duration::from_secs(5);
        while !pane
            .snapshot()
            .text()
            .contains("HOOK_ENV=t42|t42|/tmp/gsh-test.sock")
        {
            assert!(
                Instant::now() < deadline,
                "timeout waiting for HOOK_ENV output; snapshot:\n{}",
                pane.snapshot().text()
            );
            std::thread::sleep(Duration::from_millis(16));
            let _ = pane.update(TerminalMsg::Tick);
        }
    }
}
