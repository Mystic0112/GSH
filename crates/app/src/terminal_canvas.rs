//! Renderização do grid do [`TermSnapshot`] em `iced::widget::canvas` e
//! captura de eventos de mouse (scrollback, clique e seleção por arraste) e
//! redimensionamento do pane.

use gsh_term::{GridPos, TermSnapshot};
use iced::widget::canvas::{self, Action, Event};
use iced::widget::text;
use iced::{alignment, mouse, Point, Rectangle, Renderer, Size, Theme};

use crate::terminal_grid::{
    compute_grid_size, exceeds_drag_threshold, pixel_to_grid, scroll_delta_to_step, should_resize,
    CellMetrics, ScrollStep, SelectionRange,
};
use crate::terminal_pane::TerminalMsg;
use crate::theme;
use gsh_term::TerminalInputModes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalLinkTarget {
    Url(String),
    File {
        path: std::path::PathBuf,
        line: Option<usize>,
    },
}

pub fn terminal_link_at(
    snapshot: &TermSnapshot,
    position: GridPos,
    cwd: Option<&std::path::Path>,
) -> Option<TerminalLinkTarget> {
    let line = snapshot.lines.get(position.row)?;
    let chars = line.iter().map(|cell| cell.ch).collect::<Vec<_>>();
    let is_delimiter = |ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\'' | '`'
            )
    };
    if chars.get(position.col).is_some_and(|ch| is_delimiter(*ch)) {
        return None;
    }
    let token_start = chars
        .iter()
        .enumerate()
        .take(position.col.saturating_add(1))
        .rev()
        .find_map(|(index, ch)| is_delimiter(*ch).then_some(index.saturating_add(1)))
        .unwrap_or(0);
    let token_end = chars
        .iter()
        .enumerate()
        .skip(position.col)
        .find_map(|(index, ch)| is_delimiter(*ch).then_some(index))
        .unwrap_or(chars.len());
    if position.col < token_start || position.col >= token_end {
        return None;
    }
    let token = chars
        .get(token_start..token_end)?
        .iter()
        .collect::<String>();
    let trimmed = token.trim_end_matches(['.', ',', ';', ':', '!', '?']);
    let trimmed_end = token_start.saturating_add(trimmed.chars().count());
    if position.col >= trimmed_end {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        let host = trimmed.split_once("://")?.1;
        if !host.is_empty() {
            return Some(TerminalLinkTarget::Url(trimmed.to_owned()));
        }
    }
    if lower.starts_with("www.") {
        return Some(TerminalLinkTarget::Url(format!("https://{trimmed}")));
    }

    let raw = trimmed.trim_end_matches([')', ']', '}']);
    let (path_text, line_number) = raw
        .rsplit_once(':')
        .and_then(|(path, line)| line.parse::<usize>().ok().map(|line| (path, Some(line))))
        .unwrap_or((raw, None));
    let anchored = path_text.starts_with('/')
        || path_text.starts_with("./")
        || path_text.starts_with("../")
        || path_text.starts_with("~/");
    if !path_text.contains('/') || (!anchored && !path_text.contains('.')) {
        return None;
    }
    let expanded = if let Some(rest) = path_text.strip_prefix("~/") {
        std::env::var_os("HOME")
            .map(std::path::PathBuf::from)?
            .join(rest)
    } else {
        std::path::PathBuf::from(path_text)
    };
    let resolved = if expanded.is_absolute() {
        expanded
    } else {
        cwd?.join(expanded)
    };
    resolved.exists().then_some(TerminalLinkTarget::File {
        path: resolved,
        line: line_number,
    })
}

/// Estado efêmero de interação de ponteiro e última dimensão observada no canvas.
#[derive(Debug, Default)]
pub struct TerminalCanvasState {
    last_grid_size: Option<(u16, u16)>,
    press_origin: Option<Point>,
    anchor_pos: Option<GridPos>,
    dragging: bool,
    last_focus: Option<GridPos>,
    forwarding_mouse: bool,
}

/// Programa de `Canvas` que desenha o [`TermSnapshot`] usando um [`canvas::Cache`].
pub struct TerminalCanvasProgram<'a> {
    pub snapshot: &'a TermSnapshot,
    pub selection: Option<SelectionRange>,
    pub cache: &'a canvas::Cache,
    pub metrics: CellMetrics,
    pub input_modes: TerminalInputModes,
    pub modifiers: iced::keyboard::Modifiers,
    pub cwd: Option<&'a std::path::Path>,
}

impl canvas::Program<TerminalMsg> for TerminalCanvasProgram<'_> {
    type State = TerminalCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<TerminalMsg>> {
        if let Event::Mouse(mouse_event) = event {
            if let Some(action) = self.handle_mouse(state, mouse_event, bounds, cursor) {
                return Some(action);
            }
        }

        if bounds.width > 1.0 && bounds.height > 1.0 {
            let next_size = compute_grid_size(bounds.width, bounds.height, self.metrics);
            if state.last_grid_size != Some(next_size) {
                state.last_grid_size = Some(next_size);
                let current = (self.snapshot.cols, self.snapshot.rows);
                if should_resize(current, next_size) {
                    return Some(Action::publish(TerminalMsg::Resize(
                        next_size.0,
                        next_size.1,
                    )));
                }
            }
        }

        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            draw_snapshot_frame(
                frame,
                bounds.size(),
                self.snapshot,
                self.selection,
                self.metrics,
            );
        });
        vec![geometry]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if self.modifiers.command() {
            if let Some(position) = cursor.position_in(bounds) {
                let cell = pixel_to_grid(
                    position.x,
                    position.y,
                    self.metrics,
                    self.snapshot.cols,
                    self.snapshot.rows,
                );
                if terminal_link_at(self.snapshot, cell, self.cwd).is_some() {
                    return mouse::Interaction::Pointer;
                }
            }
        }
        if cursor.is_over(bounds) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }
}

impl TerminalCanvasProgram<'_> {
    fn handle_mouse(
        &self,
        state: &mut TerminalCanvasState,
        mouse_event: &mouse::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<TerminalMsg>> {
        match mouse_event {
            mouse::Event::WheelScrolled { delta } if cursor.is_over(bounds) => {
                if self.mouse_reporting() {
                    let step = scroll_delta_to_step(*delta, self.metrics.cell_height)?;
                    let (button, count) = match step {
                        ScrollStep::Up(lines) => (64, lines),
                        ScrollStep::Down(lines) => (65, lines),
                    };
                    let pos = cursor.position_in(bounds)?;
                    let cell = pixel_to_grid(
                        pos.x,
                        pos.y,
                        self.metrics,
                        self.snapshot.cols,
                        self.snapshot.rows,
                    );
                    let mut bytes = Vec::new();
                    for _ in 0..count.min(32) {
                        bytes.extend(crate::terminal_input::mouse_report_bytes(
                            button,
                            cell.col,
                            cell.row,
                            crate::terminal_input::MouseReportOptions {
                                release: false,
                                motion: false,
                                modifiers: self.modifiers,
                                utf8: self.input_modes.mouse_utf8,
                                sgr: self.input_modes.mouse_sgr,
                            },
                        ));
                    }
                    return Some(Action::publish(TerminalMsg::MouseReport(bytes)).and_capture());
                }
                let step = scroll_delta_to_step(*delta, self.metrics.cell_height)?;
                let msg = match step {
                    ScrollStep::Up(lines) => TerminalMsg::ScrollUp(lines),
                    ScrollStep::Down(lines) => TerminalMsg::ScrollDown(lines),
                };
                Some(Action::publish(msg).and_capture())
            }
            mouse::Event::ButtonPressed(button) => {
                let local_pt = cursor.position_in(bounds)?;
                let pos = pixel_to_grid(
                    local_pt.x,
                    local_pt.y,
                    self.metrics,
                    self.snapshot.cols,
                    self.snapshot.rows,
                );
                if matches!(button, mouse::Button::Right) {
                    if let Some(target) = terminal_link_at(self.snapshot, pos, self.cwd) {
                        return Some(
                            Action::publish(TerminalMsg::ShowLinkMenu {
                                target,
                                position: local_pt,
                            })
                            .and_capture(),
                        );
                    }
                }
                if matches!(button, mouse::Button::Left) && self.modifiers.command() {
                    if let Some(target) = terminal_link_at(self.snapshot, pos, self.cwd) {
                        return Some(
                            Action::publish(TerminalMsg::OpenTerminalLink(target)).and_capture(),
                        );
                    }
                }
                state.press_origin = Some(local_pt);
                state.anchor_pos = Some(pos);
                state.dragging = false;
                state.last_focus = None;
                if self.mouse_reporting() && !self.modifiers.alt() {
                    let code = match button {
                        mouse::Button::Left => 0,
                        mouse::Button::Middle => 1,
                        mouse::Button::Right => 2,
                        _ => return None,
                    };
                    state.forwarding_mouse = true;
                    return Some(
                        Action::publish(TerminalMsg::MouseReport(
                            crate::terminal_input::mouse_report_bytes(
                                code,
                                pos.col,
                                pos.row,
                                crate::terminal_input::MouseReportOptions {
                                    release: false,
                                    motion: false,
                                    modifiers: self.modifiers,
                                    utf8: self.input_modes.mouse_utf8,
                                    sgr: self.input_modes.mouse_sgr,
                                },
                            ),
                        ))
                        .and_capture(),
                    );
                }
                if !matches!(button, mouse::Button::Left) {
                    return None;
                }
                Some(Action::publish(TerminalMsg::ClearSelection).and_capture())
            }
            mouse::Event::CursorMoved { .. } => {
                if state.forwarding_mouse
                    && (self.input_modes.mouse_drag || self.input_modes.mouse_motion)
                {
                    let global = cursor.position()?;
                    let local = Point::new(global.x - bounds.x, global.y - bounds.y);
                    let pos = pixel_to_grid(
                        local.x,
                        local.y,
                        self.metrics,
                        self.snapshot.cols,
                        self.snapshot.rows,
                    );
                    let button = if self.input_modes.mouse_motion && !self.input_modes.mouse_drag {
                        3
                    } else {
                        0
                    };
                    let bytes = crate::terminal_input::mouse_report_bytes(
                        button,
                        pos.col,
                        pos.row,
                        crate::terminal_input::MouseReportOptions {
                            release: false,
                            motion: true,
                            modifiers: self.modifiers,
                            utf8: self.input_modes.mouse_utf8,
                            sgr: self.input_modes.mouse_sgr,
                        },
                    );
                    return Some(Action::publish(TerminalMsg::MouseReport(bytes)).and_capture());
                }
                let anchor = state.anchor_pos?;
                let origin = state.press_origin?;
                let global_pt = cursor.position()?;
                let local_pt = Point::new(global_pt.x - bounds.x, global_pt.y - bounds.y);
                let focus = pixel_to_grid(
                    local_pt.x,
                    local_pt.y,
                    self.metrics,
                    self.snapshot.cols,
                    self.snapshot.rows,
                );

                if !state.dragging && exceeds_drag_threshold(origin, local_pt) {
                    state.dragging = true;
                    state.last_focus = Some(focus);
                    return Some(
                        Action::publish(TerminalMsg::StartSelection { anchor, focus })
                            .and_capture(),
                    );
                }

                if state.dragging && state.last_focus != Some(focus) {
                    state.last_focus = Some(focus);
                    return Some(
                        Action::publish(TerminalMsg::ExtendSelection(focus)).and_capture(),
                    );
                }
                None
            }
            mouse::Event::ButtonReleased(button) => {
                if state.forwarding_mouse {
                    state.forwarding_mouse = false;
                    let local = cursor.position_in(bounds)?;
                    let pos = pixel_to_grid(
                        local.x,
                        local.y,
                        self.metrics,
                        self.snapshot.cols,
                        self.snapshot.rows,
                    );
                    let code = match button {
                        mouse::Button::Left => 0,
                        mouse::Button::Middle => 1,
                        mouse::Button::Right => 2,
                        _ => 3,
                    };
                    return Some(
                        Action::publish(TerminalMsg::MouseReport(
                            crate::terminal_input::mouse_report_bytes(
                                code,
                                pos.col,
                                pos.row,
                                crate::terminal_input::MouseReportOptions {
                                    release: true,
                                    motion: false,
                                    modifiers: self.modifiers,
                                    utf8: self.input_modes.mouse_utf8,
                                    sgr: self.input_modes.mouse_sgr,
                                },
                            ),
                        ))
                        .and_capture(),
                    );
                }
                if !matches!(button, mouse::Button::Left) {
                    return None;
                }
                state.press_origin = None;
                state.anchor_pos = None;
                state.dragging = false;
                state.last_focus = None;
                None
            }
            _ => None,
        }
    }

    fn mouse_reporting(&self) -> bool {
        self.input_modes.mouse_click || self.input_modes.mouse_drag || self.input_modes.mouse_motion
    }
}

/// Desenha o conteúdo completo do [`TermSnapshot`] num [`canvas::Frame`].
pub fn draw_snapshot_frame(
    frame: &mut canvas::Frame,
    size: Size,
    snapshot: &TermSnapshot,
    selection: Option<SelectionRange>,
    metrics: CellMetrics,
) {
    frame.fill_rectangle(Point::ORIGIN, size, theme::terminal_bg());

    let cw = metrics.cell_width;
    let ch = metrics.cell_height;

    for (row_idx, row_cells) in snapshot.lines.iter().enumerate() {
        let cell_y = metrics.padding_y + (row_idx as f32) * ch;

        for (col_idx, cell) in row_cells.iter().enumerate() {
            let (_, bg_opt) = theme::effective_cell_colors(cell);
            if let Some(bg_color) = bg_opt {
                let cell_x = metrics.padding_x + (col_idx as f32) * cw;
                frame.fill_rectangle(Point::new(cell_x, cell_y), Size::new(cw, ch), bg_color);
            }
        }

        if let Some(sel) = selection {
            if let Some((c0, c1)) = sel.row_span(row_idx, snapshot.cols) {
                let sel_x = metrics.padding_x + (c0 as f32) * cw;
                let sel_w = (c1.saturating_sub(c0).saturating_add(1) as f32) * cw;
                frame.fill_rectangle(
                    Point::new(sel_x, cell_y),
                    Size::new(sel_w, ch),
                    theme::terminal_selection(),
                );
            }
        }

        if snapshot.cursor.visible && snapshot.cursor.row == row_idx {
            let cur_x = metrics.padding_x + (snapshot.cursor.col as f32) * cw;
            frame.fill_rectangle(
                Point::new(cur_x, cell_y),
                Size::new(cw, ch),
                theme::terminal_cursor(),
            );
        }

        for (col_idx, cell) in row_cells.iter().enumerate() {
            let cell_x = metrics.padding_x + (col_idx as f32) * cw;
            let is_cursor = snapshot.cursor.visible
                && snapshot.cursor.row == row_idx
                && snapshot.cursor.col == col_idx;
            let (base_fg, _) = theme::effective_cell_colors(cell);
            let fg_color = if is_cursor {
                theme::terminal_cursor_text()
            } else {
                base_fg
            };

            if cell.ch != ' ' && cell.ch != '\0' {
                let mut utf8_buf = [0u8; 4];
                let content = cell.ch.encode_utf8(&mut utf8_buf).to_owned();
                frame.fill_text(canvas::Text {
                    content,
                    position: Point::new(cell_x, cell_y + ch * 0.5),
                    max_width: cw * 2.0,
                    color: fg_color,
                    size: (theme::TERM_FONT_SIZE * ch / theme::TERM_CELL_HEIGHT).into(),
                    line_height: text::LineHeight::Absolute(ch.into()),
                    font: theme::cell_font(cell.bold),
                    align_x: text::Alignment::Left,
                    align_y: alignment::Vertical::Center,
                    shaping: text::Shaping::Advanced,
                });
            }

            if cell.underline {
                frame.fill_rectangle(
                    Point::new(cell_x, cell_y + ch - 2.0),
                    Size::new(cw, 1.0),
                    fg_color,
                );
            }
        }
    }

    if snapshot.display_offset > 0 && snapshot.history_size > 0 {
        let track_h = (size.height - metrics.padding_y * 2.0).max(8.0);
        let thumb_h = (track_h * 0.15).clamp(12.0, track_h);
        let ratio = (snapshot.display_offset as f32 / snapshot.history_size as f32).clamp(0.0, 1.0);
        let thumb_y = metrics.padding_y + (1.0 - ratio) * (track_h - thumb_h);
        frame.fill_rectangle(
            Point::new((size.width - 5.0).max(0.0), thumb_y),
            Size::new(3.0, thumb_h),
            theme::TERM_SCROLLBAR,
        );
    }
}
