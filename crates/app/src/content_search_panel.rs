//! Aba Busca do painel direito (`content_search_panel.dart` do Flutter, modo `fill`).
//!
//! Cabeçalho "BUSCA" com a pílula `N in M` e o spinner, campo com os toggles Aa / ab / .*,
//! mensagens de estado e os resultados agrupados por arquivo (recolhíveis) com os trechos
//! casados em destaque. O estado fica em `content_search_state`; a thread de varredura, em
//! `main.rs`.

use iced::widget::canvas::{self, path::Arc, Path, Stroke};
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, rich_text, row, scrollable, span, text, text_input, tooltip, Space,
};
use iced::{
    keyboard, mouse, Alignment, Background, Border, Color, Element, Fill, Font, Length, Point,
    Radians, Rectangle, Renderer, Theme,
};

use gsh_search::{FileMatches, LineMatch, SearchError};

use crate::content_search_state::{SearchState, Toggle};
use crate::{file_icons, git_presenter, labels, pane_chrome, panel_widgets, theme};

/// Id do campo, para focá-lo (Ctrl+Shift+F, Esc).
pub const INPUT_ID: &str = "content-search-input";

/// Largura média de um caractere em unidades de corpo (aproximação para o "…" das linhas).
const BODY_CHAR_RATIO: f32 = 0.52;
/// JetBrains Mono: avanço de 0,6 em.
const MONO_CHAR_RATIO: f32 = 0.6;

#[derive(Debug, Clone)]
pub enum SearchMsg {
    TermChanged(String),
    /// Passou o debounce do token: busca se ele ainda for o último.
    DebounceElapsed(u64),
    Toggle(Toggle),
    ToggleGroup(String),
    /// Esc no campo: limpa e busca de novo (vazio).
    Clear,
    /// Clique numa linha: caminho relativo e linha (base 1).
    Open(String, usize),
    Batch(u64, Vec<FileMatches>),
    Finished(u64, Result<(), SearchError>),
    /// Quadro do spinner.
    Spin,
}

/// Ctrl+Shift+F ou Cmd+Shift+F (o Flutter liga os dois em qualquer plataforma).
pub fn is_focus_shortcut(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> bool {
    let letter = matches!(key, keyboard::Key::Character(c) if c.eq_ignore_ascii_case("f"));
    letter && modifiers.shift() && (modifiers.control() || modifiers.logo())
}

pub fn view(state: &SearchState, has_root: bool) -> Element<'_, SearchMsg> {
    if !has_root {
        return panel_widgets::empty_label(labels::FILES_NO_FOLDER);
    }
    column![header(state), field(state), results(state)]
        .width(Fill)
        .height(Fill)
        .into()
}

fn header(state: &SearchState) -> Element<'_, SearchMsg> {
    let mut content = row![text(labels::CONTENT_SEARCH_SECTION)
        .font(theme::FONT_BODY)
        .size(theme::FILE_PANEL_HEADER_TITLE_SIZE)
        .color(theme::palette().text3)]
    .spacing(theme::SEARCH_HEADER_GAP)
    .align_y(Alignment::Center);
    if state.has_query() {
        content = content.push(pill(
            labels::content_search_summary(state.total_matches(), state.results.len()),
            theme::SEARCH_PILL_TEXT_SIZE,
            theme::SEARCH_PILL_PADDING,
            theme::SEARCH_PILL_RADIUS,
        ));
    }
    content = content.push(Space::new().width(Fill));
    if state.searching {
        content = content.push(
            canvas::Canvas::new(Spinner {
                elapsed_ms: state.spin_ms,
                color: None,
                stroke: None,
            })
            .width(theme::SEARCH_SPINNER_SIZE)
            .height(theme::SEARCH_SPINNER_SIZE),
        );
    }
    container(content)
        .width(Fill)
        .height(theme::SEARCH_HEADER_HEIGHT)
        .padding(iced::Padding {
            top: 0.0,
            right: theme::SEARCH_HEADER_RIGHT,
            bottom: 0.0,
            left: theme::SEARCH_HEADER_LEFT,
        })
        .center_y(theme::SEARCH_HEADER_HEIGHT)
        .into()
}

/// Cápsula `panel2` com texto `text3` (pílula do cabeçalho e contador do arquivo).
fn pill<'a>(label: String, size: f32, padding: [f32; 2], radius: f32) -> Element<'a, SearchMsg> {
    container(
        text(label)
            .font(theme::FONT_BODY)
            .size(size)
            .color(theme::palette().text3),
    )
    .padding(padding)
    .style(move |_: &Theme| container::Style {
        background: Some(theme::palette().panel2.into()),
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn field(state: &SearchState) -> Element<'_, SearchMsg> {
    let invalid = state.invalid_regex;
    let input = text_input(labels::CONTENT_SEARCH_PLACEHOLDER, &state.term)
        .id(INPUT_ID)
        .on_input(SearchMsg::TermChanged)
        .font(theme::FONT_BODY)
        .size(theme::SEARCH_INPUT_SIZE)
        .line_height(theme::LINE_HEIGHT_BODY)
        .padding(theme::SEARCH_INPUT_PADDING)
        .style(move |_: &Theme, _| {
            let p = theme::palette();
            text_input::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: Border {
                    color: if invalid { p.error } else { p.border },
                    width: 1.0,
                    radius: theme::SEARCH_INPUT_RADIUS.into(),
                },
                icon: p.text3,
                placeholder: p.text4,
                value: p.text,
                selection: p.accent_soft,
            }
        });
    container(
        row![
            input,
            Space::new().width(theme::SEARCH_FIELD_GAP),
            toggle(
                labels::CONTENT_SEARCH_TOGGLE_CASE,
                labels::CONTENT_SEARCH_MATCH_CASE,
                state.case_sensitive,
                false,
                Toggle::CaseSensitive,
            ),
            toggle(
                labels::CONTENT_SEARCH_TOGGLE_WORD,
                labels::CONTENT_SEARCH_WHOLE_WORD,
                state.whole_word,
                true,
                Toggle::WholeWord,
            ),
            toggle(
                labels::CONTENT_SEARCH_TOGGLE_REGEX,
                labels::CONTENT_SEARCH_USE_REGEX,
                state.regex,
                false,
                Toggle::Regex,
            ),
        ]
        .align_y(Alignment::Center),
    )
    .padding(theme::SEARCH_FIELD_PADDING)
    .width(Fill)
    .into()
}

/// Toggle compacto de 26px (`_OptionToggle`): fundo `accent` a 18% quando ligado.
fn toggle(
    label: &'static str,
    tip: &'static str,
    active: bool,
    underline: bool,
    which: Toggle,
) -> Element<'static, SearchMsg> {
    let p = theme::palette();
    let color = if active { p.accent_text } else { p.text3 };
    let glyph: Element<'static, SearchMsg> = rich_text![span::<(), Font>(label)
        .font(theme::FONT_MONO)
        .size(theme::SEARCH_TOGGLE_TEXT_SIZE)
        .color(color)
        .underline(underline)]
    .into();
    let tint = container(glyph)
        .center_x(Length::Fixed(theme::SEARCH_TOGGLE_SIZE))
        .center_y(Length::Fixed(theme::SEARCH_TOGGLE_SIZE))
        .style(move |_: &Theme| container::Style {
            background: active.then(|| {
                Color {
                    a: theme::SEARCH_TOGGLE_ACTIVE_ALPHA,
                    ..theme::palette().accent
                }
                .into()
            }),
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    let hit = button(tint)
        .padding(0)
        .on_press(SearchMsg::Toggle(which))
        .style(|_: &Theme, status| button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed)
                .then(|| theme::palette().panel3.into()),
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    tooltip(hit, pane_chrome::tip_bubble(tip), tooltip::Position::Bottom).into()
}

fn results(state: &SearchState) -> Element<'_, SearchMsg> {
    if state.results.is_empty() {
        let (message, color) = if state.invalid_regex {
            (labels::CONTENT_SEARCH_INVALID_REGEX, theme::palette().error)
        } else if !state.has_query() {
            (
                labels::CONTENT_SEARCH_TYPE_TO_SEARCH,
                theme::palette().text4,
            )
        } else if state.searching {
            (labels::CONTENT_SEARCH_SEARCHING, theme::palette().text4)
        } else {
            (labels::CONTENT_SEARCH_NO_RESULTS, theme::palette().text4)
        };
        return container(
            text(message)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(color),
        )
        .padding(theme::SEARCH_MESSAGE_PADDING)
        .width(Fill)
        .into();
    }
    let list = state
        .results
        .iter()
        .fold(column![].width(Fill), |list, file| {
            list.push(file_group(
                file,
                state.collapsed.contains(&file.relative_path),
            ))
        })
        .padding(iced::Padding {
            bottom: theme::SEARCH_LIST_BOTTOM,
            ..iced::Padding::ZERO
        });
    scrollable(list).width(Fill).height(Fill).into()
}

fn file_group(file: &FileMatches, collapsed: bool) -> Element<'_, SearchMsg> {
    let (dir, name) = file
        .relative_path
        .rsplit_once('/')
        .map_or(("", file.relative_path.as_str()), |(dir, name)| (dir, name));
    let count = file.match_count().to_string();
    let (name_chars, dir_chars) = group_text_budgets(dir, count.chars().count());

    let mut head = row![
        container(
            text(if collapsed {
                labels::ICON_CHEVRON_RIGHT
            } else {
                labels::ICON_CHEVRON_DOWN
            })
            .font(theme::FONT_ICON)
            .size(theme::SEARCH_GROUP_ICON_SIZE)
            .color(theme::palette().text4)
        ),
        Space::new().width(theme::SEARCH_GROUP_CHEVRON_GAP),
        file_icons::view_sized(name, false, false, theme::SEARCH_GROUP_ICON_SIZE),
        Space::new().width(theme::SEARCH_GROUP_GAP),
        text(git_presenter::shorten(name, name_chars))
            .font(theme::FONT_BODY)
            .size(theme::SEARCH_GROUP_NAME_SIZE)
            .line_height(theme::LINE_HEIGHT_BODY)
            .wrapping(Wrapping::None)
            .color(theme::palette().text),
    ]
    .align_y(Alignment::Center);
    if !dir.is_empty() {
        head = head.push(Space::new().width(theme::SEARCH_GROUP_GAP)).push(
            text(git_presenter::shorten(dir, dir_chars))
                .font(theme::FONT_BODY)
                .size(theme::SEARCH_GROUP_DIR_SIZE)
                .wrapping(Wrapping::None)
                .color(theme::palette().text4),
        );
    }
    head = head
        .push(Space::new().width(theme::SEARCH_GROUP_GAP))
        .push(pill(
            count,
            theme::SEARCH_COUNT_TEXT_SIZE,
            theme::SEARCH_COUNT_PADDING,
            theme::SEARCH_COUNT_RADIUS,
        ));
    let header = button(head.clip(true))
        .width(Fill)
        .padding(theme::SEARCH_GROUP_PADDING)
        .on_press(SearchMsg::ToggleGroup(file.relative_path.clone()))
        .style(hover_style(theme::FILE_TREE_ROW_RADIUS));

    let mut group = column![header].width(Fill);
    if !collapsed {
        for line in &file.matches {
            group = group.push(match_row(&file.relative_path, line));
        }
    }
    group.into()
}

fn match_row<'a>(relative_path: &str, line: &'a LineMatch) -> Element<'a, SearchMsg> {
    let text_width = theme::FILES_PANEL_WIDTH
        - 1.0
        - theme::SEARCH_MATCH_PADDING.left
        - theme::SEARCH_MATCH_PADDING.right
        - theme::SEARCH_MATCH_NUMBER_WIDTH
        - theme::SEARCH_MATCH_GAP;
    let max_chars = chars_that_fit(text_width, theme::SEARCH_MATCH_TEXT_SIZE * MONO_CHAR_RATIO);
    let highlight = Color {
        a: theme::SEARCH_MATCH_HIGHLIGHT_ALPHA,
        ..theme::palette().warn
    };
    let spans: Vec<_> = line_segments(line, max_chars)
        .into_iter()
        .map(|segment| {
            let base = span::<(), Font>(segment.text)
                .size(theme::SEARCH_MATCH_TEXT_SIZE)
                .line_height(theme::LINE_HEIGHT_MONO);
            if segment.highlighted {
                base.font(theme::FONT_MONO_BOLD)
                    .color(theme::palette().text)
                    .background(Background::Color(highlight))
            } else {
                base.font(theme::FONT_MONO).color(theme::palette().text2)
            }
        })
        .collect();
    let content = row![
        container(
            text(line.line_number.to_string())
                .font(theme::FONT_MONO)
                .size(theme::SEARCH_MATCH_TEXT_SIZE)
                .line_height(theme::LINE_HEIGHT_MONO)
                .color(theme::palette().text4)
        )
        .width(theme::SEARCH_MATCH_NUMBER_WIDTH)
        .align_right(theme::SEARCH_MATCH_NUMBER_WIDTH),
        Space::new().width(theme::SEARCH_MATCH_GAP),
        container(rich_text(spans).wrapping(Wrapping::None))
            .width(Fill)
            .clip(true),
    ];
    button(content)
        .width(Fill)
        .padding(theme::SEARCH_MATCH_PADDING)
        .on_press(SearchMsg::Open(relative_path.to_owned(), line.line_number))
        .style(hover_style(theme::SEARCH_MATCH_RADIUS))
        .into()
}

/// Fundo `panel` ao passar o mouse (o `hoverColor` das linhas do Flutter).
fn hover_style(radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then(|| theme::palette().panel.into()),
        text_color: theme::palette().text,
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn chars_that_fit(width: f32, char_width: f32) -> usize {
    if char_width <= 0.0 || width <= 0.0 {
        return 0;
    }
    (width / char_width) as usize
}

/// Quantos caracteres cabem em nome e pasta. No Flutter os dois são `Flexible` (cada um
/// recebe até metade do que sobra depois das partes fixas) e terminam em "…".
fn group_text_budgets(dir: &str, count_digits: usize) -> (usize, usize) {
    let count_width = theme::SEARCH_COUNT_PADDING[1] * 2.0
        + count_digits as f32 * theme::SEARCH_COUNT_TEXT_SIZE * BODY_CHAR_RATIO;
    let fixed = theme::SEARCH_GROUP_PADDING[1] * 2.0
        + theme::SEARCH_GROUP_ICON_SIZE * 2.0
        + theme::SEARCH_GROUP_CHEVRON_GAP
        + theme::SEARCH_GROUP_GAP * if dir.is_empty() { 2.0 } else { 3.0 }
        + count_width;
    let free = theme::FILES_PANEL_WIDTH - 1.0 - fixed;
    let name_size = theme::SEARCH_GROUP_NAME_SIZE * BODY_CHAR_RATIO;
    let dir_size = theme::SEARCH_GROUP_DIR_SIZE * BODY_CHAR_RATIO;
    if dir.is_empty() {
        (chars_that_fit(free, name_size), 0)
    } else {
        (
            chars_that_fit(free / 2.0, name_size),
            chars_that_fit(free / 2.0, dir_size),
        )
    }
}

/// Trecho da linha, destacado ou não.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Segment {
    text: String,
    highlighted: bool,
}

/// `_highlightSpan` + reticências: parte a linha nos intervalos casados (limitados ao texto)
/// e corta em `max_chars` caracteres, terminando em "…" quando não cabe.
fn line_segments(line: &LineMatch, max_chars: usize) -> Vec<Segment> {
    let text = line.text.as_str();
    let mut segments = Vec::new();
    let mut cursor = 0;
    for range in &line.ranges {
        let start = range.start.min(text.len());
        let end = range.end.clamp(start, text.len());
        if start > cursor {
            push_segment(&mut segments, text.get(cursor..start), false);
        }
        push_segment(&mut segments, text.get(start..end), true);
        cursor = end;
    }
    if cursor < text.len() {
        push_segment(&mut segments, text.get(cursor..), false);
    }
    ellipsize(segments, max_chars)
}

fn push_segment(segments: &mut Vec<Segment>, piece: Option<&str>, highlighted: bool) {
    if let Some(piece) = piece.filter(|piece| !piece.is_empty()) {
        segments.push(Segment {
            // A fonte não tem glifo útil para TAB: um espaço mantém a coluna legível.
            text: piece.replace('\t', " "),
            highlighted,
        });
    }
}

fn ellipsize(segments: Vec<Segment>, max_chars: usize) -> Vec<Segment> {
    let total: usize = segments.iter().map(|s| s.text.chars().count()).sum();
    if total <= max_chars {
        return segments;
    }
    let mut left = max_chars.saturating_sub(1);
    let mut out = Vec::new();
    for mut segment in segments {
        let len = segment.text.chars().count();
        if len <= left {
            left -= len;
            out.push(segment);
            continue;
        }
        segment.text = segment.text.chars().take(left).collect();
        segment.text.push_str(labels::CONTENT_SEARCH_ELLIPSIS);
        out.push(segment);
        return out;
    }
    out
}

/// Spinner quadrado de `size` px (`CircularProgressIndicator` do shadcn) que gira com `elapsed_ms`.
pub fn spinner<'a, M: 'a>(size: f32, elapsed_ms: u64) -> Element<'a, M> {
    canvas::Canvas::new(Spinner {
        elapsed_ms,
        color: None,
        stroke: None,
    })
    .width(size)
    .height(size)
    .into()
}

/// Spinner com cor e espessura próprias (`CircularProgressIndicator(size:, strokeWidth:, color:)`).
pub fn spinner_colored<'a, M: 'a>(
    size: f32,
    stroke: f32,
    color: Color,
    elapsed_ms: u64,
) -> Element<'a, M> {
    canvas::Canvas::new(Spinner {
        elapsed_ms,
        color: Some(color),
        stroke: Some(stroke),
    })
    .width(size)
    .height(size)
    .into()
}

/// Spinner de 12px (`CircularProgressIndicator(size: 12)` do shadcn): trilho a 20% e arco em
/// `accent` com traço de 1px, que gira e muda de comprimento como o do Material.
struct Spinner {
    elapsed_ms: u64,
    color: Option<Color>,
    stroke: Option<f32>,
}

impl<M> canvas::Program<M> for Spinner {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let width = self
            .stroke
            .unwrap_or(bounds.width.min(bounds.height) / 12.0);
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = (bounds.width.min(bounds.height) - width) / 2.0;
        let accent = self.color.unwrap_or_else(|| theme::palette().accent);
        frame.stroke(
            &Path::circle(center, radius),
            Stroke::default()
                .with_color(Color { a: 0.2, ..accent })
                .with_width(width),
        );
        let (start, sweep) = spinner_arc(self.elapsed_ms);
        let arc = Path::new(|builder| {
            builder.arc(Arc {
                center,
                radius,
                start_angle: Radians(start),
                end_angle: Radians(start + sweep),
            });
        });
        frame.stroke(
            &arc,
            Stroke::default()
                .with_color(accent)
                .with_width(width)
                .with_line_cap(canvas::LineCap::Butt),
        );
        vec![frame.into_geometry()]
    }
}

/// Início e varredura (rad) do arco, no ritmo do indicador indeterminado do Material:
/// ciclo de 1333 ms (cabeça avança na 1ª metade, cauda na 2ª), volta a cada 2222 ms e
/// 90° de deslocamento por ciclo.
fn spinner_arc(elapsed_ms: u64) -> (f32, f32) {
    use std::f32::consts::{FRAC_PI_2, PI, TAU};
    let ease = |t: f32| t * t * (3.0 - 2.0 * t);
    let phase = (elapsed_ms % 1333) as f32 / 1333.0;
    let head = ease((phase / 0.5).min(1.0));
    let tail = ease(((phase - 0.5) / 0.5).clamp(0.0, 1.0));
    let turn = (elapsed_ms % 2222) as f32 / 2222.0;
    let path = ((elapsed_ms / 1333) % 4) as f32;
    let start = -FRAC_PI_2 + tail * 1.5 * PI + turn * TAU + path * FRAC_PI_2;
    let sweep = ((head - tail) * 1.5 * PI).max(0.001);
    (start, sweep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_search::MatchRange;
    use iced::keyboard::Modifiers;

    fn line(text: &str, ranges: &[(usize, usize)]) -> LineMatch {
        LineMatch {
            line_number: 1,
            text: text.to_owned(),
            ranges: ranges
                .iter()
                .map(|&(start, end)| MatchRange { start, end })
                .collect(),
        }
    }

    fn seg(text: &str, highlighted: bool) -> Segment {
        Segment {
            text: text.to_owned(),
            highlighted,
        }
    }

    #[test]
    fn segments_split_around_matches() {
        let got = line_segments(&line("a Clock b clock", &[(2, 7), (10, 15)]), 99);
        assert_eq!(
            got,
            vec![
                seg("a ", false),
                seg("Clock", true),
                seg(" b ", false),
                seg("clock", true)
            ]
        );
    }

    #[test]
    fn segments_clamp_ranges_to_the_truncated_text() {
        // Ranges vêm da linha inteira; o texto pode ter sido cortado.
        let got = line_segments(&line("abc", &[(1, 2), (2, 50), (60, 70)]), 99);
        assert_eq!(got, vec![seg("a", false), seg("b", true), seg("c", true)]);
    }

    #[test]
    fn segments_without_ranges_are_one_plain_piece() {
        assert_eq!(
            line_segments(&line("abc", &[]), 99),
            vec![seg("abc", false)]
        );
        assert!(line_segments(&line("", &[]), 99).is_empty());
    }

    #[test]
    fn long_lines_end_in_an_ellipsis_with_the_style_of_the_cut() {
        let got = line_segments(&line("0123456789", &[(0, 4)]), 6);
        assert_eq!(got, vec![seg("0123", true), seg("4…", false)]);
        // Cabe exatamente: sem reticências.
        assert_eq!(
            line_segments(&line("012345", &[]), 6),
            vec![seg("012345", false)]
        );
        // Corte dentro do trecho destacado, em UTF-8.
        let got = line_segments(&line("ééééé", &[(0, 10)]), 3);
        assert_eq!(got, vec![seg("éé…", true)]);
    }

    #[test]
    fn tabs_become_spaces() {
        assert_eq!(
            line_segments(&line("\tfoo", &[(1, 4)]), 99),
            vec![seg(" ", false), seg("foo", true)]
        );
    }

    #[test]
    fn group_budgets_split_what_is_left_in_half_when_there_is_a_folder() {
        let (name_only, none) = group_text_budgets("", 1);
        assert_eq!(none, 0);
        let (name, dir) = group_text_budgets("src/app", 1);
        assert!(name < name_only);
        assert!(name > 8 && dir > 8, "{name} {dir}");
    }

    #[test]
    fn focus_shortcut_is_ctrl_or_cmd_shift_f() {
        let f = keyboard::Key::Character("f".into());
        let big_f = keyboard::Key::Character("F".into());
        assert!(is_focus_shortcut(&f, Modifiers::CTRL | Modifiers::SHIFT));
        assert!(is_focus_shortcut(
            &big_f,
            Modifiers::CTRL | Modifiers::SHIFT
        ));
        assert!(is_focus_shortcut(&f, Modifiers::LOGO | Modifiers::SHIFT));
        assert!(!is_focus_shortcut(&f, Modifiers::CTRL));
        assert!(!is_focus_shortcut(&f, Modifiers::SHIFT));
        let g = keyboard::Key::Character("g".into());
        assert!(!is_focus_shortcut(&g, Modifiers::CTRL | Modifiers::SHIFT));
    }

    #[test]
    fn spinner_arc_stays_visible_and_moves() {
        let mut starts = std::collections::BTreeSet::new();
        for ms in (0..4000).step_by(33) {
            let (start, sweep) = spinner_arc(ms);
            assert!(sweep > 0.0 && sweep <= 1.5 * std::f32::consts::PI + 0.01);
            starts.insert((start * 100.0) as i32);
        }
        assert!(starts.len() > 10);
    }
}
