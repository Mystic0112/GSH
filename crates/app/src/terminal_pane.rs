//! Pane de terminal do GSH (`gsh-app`): conecta [`TermSession`] ao canvas do
//! `iced`, teclado (`keyboard::listen`), área de transferência
//! (`Ctrl+Shift+C`/`Ctrl+Shift+V`), mouse (scrollback e seleção) e resize.

use std::path::Path;
use std::time::Duration;

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
    session: Option<TermSession>,
    snapshot: TermSnapshot,
    selection: Option<SelectionRange>,
    cache: canvas::Cache,
    metrics: CellMetrics,
    has_spawn_error: bool,
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
        Self::spawn_with_dims(cwd, DEFAULT_COLS, DEFAULT_ROWS)
    }

    /// Reinicia a sessão de terminal no diretório `cwd` preservando `(cols, rows)` atuais.
    /// Em modo estático de teste (`session.is_none()` e `!has_spawn_error`), mantém o snapshot.
    pub fn respawn_in_dir(&mut self, cwd: Option<&Path>) {
        if self.session.is_none() && !self.has_spawn_error {
            return;
        }
        let (cols, rows) = self.size();
        *self = Self::spawn_with_dims(cwd, cols, rows);
    }

    fn spawn_with_dims(cwd: Option<&Path>, cols: u16, rows: u16) -> Self {
        let mut cmd = ShellCommand::default();
        if let Some(dir) = cwd {
            cmd = cmd.cwd(dir);
        }
        match TermSession::spawn(cmd, cols, rows) {
            Ok(session) => Self::with_session(session),
            Err(_) => Self {
                session: None,
                snapshot: empty_snapshot(cols, rows),
                selection: None,
                cache: canvas::Cache::new(),
                metrics: CellMetrics::default(),
                has_spawn_error: true,
            },
        }
    }

    /// Constrói um [`TerminalPane`] a partir de uma [`TermSession`] já iniciada.
    pub fn with_session(session: TermSession) -> Self {
        let snapshot = session.snapshot();
        Self {
            session: Some(session),
            snapshot,
            selection: None,
            cache: canvas::Cache::new(),
            metrics: CellMetrics::default(),
            has_spawn_error: false,
        }
    }

    /// Constrói um [`TerminalPane`] estático a partir de um [`TermSnapshot`] (útil para testes).
    pub fn from_snapshot(snapshot: TermSnapshot) -> Self {
        Self {
            session: None,
            snapshot,
            selection: None,
            cache: canvas::Cache::new(),
            metrics: CellMetrics::default(),
            has_spawn_error: false,
        }
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

    /// Processa uma [`TerminalMsg`] e devolve uma [`Task`] (como leitura/escrita de clipboard).
    pub fn update(&mut self, msg: TerminalMsg) -> Task<TerminalMsg> {
        match msg {
            TerminalMsg::Tick => {
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
}
