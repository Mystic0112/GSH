//! Moldura do pane central (`pane_view.dart` e `pane_tab_leading.dart` do Flutter).
//!
//! Barra de abas de 40px em `BG` com divisória inferior `BORDER` (aba ativa em
//! `PANEL` com indicador inferior `ACCENT` de 2px, abas
//! inativas em `BG` com texto `TEXT3`, botão `×` por aba, botão `+` que abre
//! novo terminal e controles de split/fechar) e corpo em `PANEL` com borda `BORDER`.

use iced::widget::pane_grid;
use iced::widget::{button, canvas, column, container, mouse_area, row, text, tooltip, Space};
use iced::{Background, Border, Element, Fill, Length, Point, Size, Theme};

use crate::labels;
use crate::pane_model::PaneModel;
use crate::pane_strip::view_tab_strip;
use crate::pane_tool_icon::PaneToolIcon;
use crate::theme;

/// Callbacks de interação da barra de abas do pane central.
pub struct ChromeCallbacks<M, FSelect, FClose, FDrag, FIcon, FTitle, FPaneDrag> {
    pub on_select_tab: FSelect,
    pub on_close_tab: FClose,
    pub on_tab_drag: FDrag,
    pub drop_target: Option<(usize, bool)>,
    pub drag_source: Option<usize>,
    pub on_pane_drag: FPaneDrag,
    pub pane_drop_zone: Option<PaneDropZone>,
    pub terminal_icon: FIcon,
    pub terminal_title: FTitle,
    pub on_new_terminal: M,
    pub noop_msg: M,
    pub on_split_right: M,
    pub on_split_down: M,
    /// Pede para fechar o pane (a confirmação é do chamador).
    pub on_close_pane: M,
    /// `false` com um diálogo aberto: a dica é um overlay e ficaria por cima dele.
    pub show_tooltips: bool,
    /// Foca o pane ao clicar no vazio da barra (o conteúdo cuida do próprio clique);
    /// `None` quando o pane já é o focado.
    pub on_focus: Option<M>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneDropZone {
    Strip,
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaneDragEvent {
    Hover { zone: PaneDropZone, cursor: Point },
    Drop { zone: PaneDropZone, cursor: Point },
}

impl PaneDropZone {
    pub fn at(point: Point, size: Size) -> Self {
        const EDGE: f32 = 0.25;
        if size.width <= 0.0 || size.height <= 0.0 {
            return Self::Center;
        }
        let strip_height = theme::TAB_STRIP_HEIGHT;
        if point.y <= strip_height {
            return Self::Strip;
        }
        let body_height = (size.height - strip_height).max(0.0);
        if body_height == 0.0 {
            return Self::Center;
        }
        let fx = (point.x / size.width).clamp(0.0, 1.0);
        let fy = ((point.y - strip_height) / body_height).clamp(0.0, 1.0);
        let mut zone = Self::Center;
        let mut depth = EDGE;
        for (candidate, distance) in [
            (Self::Left, fx),
            (Self::Right, 1.0 - fx),
            (Self::Top, fy),
            (Self::Bottom, 1.0 - fy),
        ] {
            if distance < depth {
                zone = candidate;
                depth = distance;
            }
        }
        zone
    }
}

/// Envolve o conteúdo ativo do pane (terminal ou viewer) com a barra de abas.
pub fn view<'a, M, FSelect, FClose, FHover, FDrag, FIcon, FTitle, FPaneDrag, FWheel>(
    model: &'a PaneModel,
    active_content: Element<'a, M>,
    callbacks: ChromeCallbacks<M, FSelect, FClose, FDrag, FIcon, FTitle, FPaneDrag>,
    dirty: &dyn Fn(&crate::pane_model::PaneTab) -> bool,
    hovered: Option<usize>,
    on_hover: FHover,
    strip: crate::pane_strip::StripInput<FWheel, M>,
) -> Element<'a, M>
where
    M: Clone + 'a,
    FSelect: Fn(usize) -> M + Clone + 'a,
    FClose: Fn(usize) -> M + Clone + 'a,
    FHover: Fn(Option<usize>) -> M + Clone + 'a,
    FDrag: Fn(usize, crate::pane_strip::TabDragEvent) -> M + Clone + 'a,
    FIcon: Fn(crate::pane_model::TermId) -> Option<(&'static [u8], bool)> + Clone + 'a,
    FTitle: Fn(crate::pane_model::TermId) -> Option<String> + Clone + 'a,
    FPaneDrag: Fn(PaneDragEvent) -> M + Clone + 'a,
    FWheel: Fn(f32) -> M + Clone + 'a,
{
    let split_controls = split_controls(
        callbacks.on_split_right.clone(),
        callbacks.on_split_down.clone(),
        callbacks.on_close_pane.clone(),
        callbacks.show_tooltips,
    );
    // as ferramentas ficam dentro da barra (mesmo fundo e borda de baixo), como em pane_view.dart
    let on_focus = callbacks.on_focus.clone();
    let on_pane_drag = std::rc::Rc::new(callbacks.on_pane_drag.clone());
    let pane_drop_zone = callbacks.pane_drop_zone;
    let tab_strip = view_tab_strip(
        model,
        callbacks,
        dirty,
        hovered,
        on_hover,
        strip,
        split_controls,
    );
    // o vazio da barra foca o pane; os botões capturam o próprio clique e já focam o pane
    let tab_strip: Element<'a, M> = match on_focus {
        Some(message) => mouse_area(tab_strip).on_press(message).into(),
        None => tab_strip,
    };

    let body_content: Element<'a, M> = active_content;
    let body = container(body_content)
        .width(Fill)
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel)),
            ..Default::default()
        });

    let hover_callback = on_pane_drag;
    let pane = column![tab_strip, body].width(Fill).height(Fill);
    let pane: Element<'a, M> = match pane_drop_zone {
        Some(PaneDropZone::Strip) => {
            iced::widget::stack![pane, pane_drop_preview(PaneDropZone::Strip)].into()
        }
        Some(zone) => iced::widget::stack![
            pane,
            column![
                Space::new()
                    .width(Fill)
                    .height(Length::Fixed(theme::TAB_STRIP_HEIGHT)),
                pane_drop_preview(zone)
            ]
            .width(Fill)
            .height(Fill)
        ]
        .into(),
        None => pane.into(),
    };
    crate::pane_focus::FocusOnPress::new(pane, None)
        .on_pointer_event(move |point, size, cursor, phase| {
            let zone = PaneDropZone::at(point, size);
            let event = match phase {
                crate::pane_focus::PointerPhase::Press | crate::pane_focus::PointerPhase::Move => {
                    PaneDragEvent::Hover { zone, cursor }
                }
                crate::pane_focus::PointerPhase::Release => PaneDragEvent::Drop { zone, cursor },
            };
            hover_callback(event)
        })
        .into()
}

fn pane_drop_preview<'a, M: 'a>(zone: PaneDropZone) -> Element<'a, M> {
    if zone == PaneDropZone::Strip {
        return column![
            container(
                text(labels::PANE_VIEW_DROP_HERE_TO_MOVE)
                    .size(theme::SIZE_TAB)
                    .color(theme::palette().accent_text),
            )
            .width(Fill)
            .height(Length::Fixed(theme::TAB_STRIP_HEIGHT))
            .center_x(Fill)
            .center_y(Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::palette().accent_soft)),
                border: Border {
                    color: theme::palette().accent,
                    width: 2.0,
                    ..Default::default()
                },
                ..Default::default()
            }),
            Space::new().width(Fill).height(Fill),
        ]
        .width(Fill)
        .height(Fill)
        .into();
    }
    let contents: Element<'a, M> = if zone == PaneDropZone::Center {
        container(
            text(labels::PANE_VIEW_DOCK_AS_TAB)
                .size(theme::SIZE_TAB)
                .color(theme::palette().accent_text),
        )
        .center_x(Fill)
        .center_y(Fill)
        .into()
    } else {
        Space::new().into()
    };
    let preview = container(contents)
        .width(Fill)
        .height(Fill)
        .padding(8)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().accent_soft)),
            border: Border {
                color: theme::palette().accent,
                width: 2.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });
    match zone {
        PaneDropZone::Strip => Space::new().into(),
        PaneDropZone::Center => preview.into(),
        PaneDropZone::Left => row![
            preview.width(Length::FillPortion(1)),
            Space::new().width(Length::FillPortion(1))
        ]
        .width(Fill)
        .height(Fill)
        .into(),
        PaneDropZone::Right => row![
            Space::new().width(Length::FillPortion(1)),
            preview.width(Length::FillPortion(1))
        ]
        .width(Fill)
        .height(Fill)
        .into(),
        PaneDropZone::Top => column![
            preview.height(Length::FillPortion(1)),
            Space::new().height(Length::FillPortion(1))
        ]
        .width(Fill)
        .height(Fill)
        .into(),
        PaneDropZone::Bottom => column![
            Space::new().height(Length::FillPortion(1)),
            preview.height(Length::FillPortion(1))
        ]
        .width(Fill)
        .height(Fill)
        .into(),
    }
}

/// Botões do canto direito da barra, na ordem do Flutter: dividir à direita, dividir abaixo, fechar.
fn split_controls<'a, M: Clone + 'a>(
    on_split_right: M,
    on_split_down: M,
    on_close_pane: M,
    show_tooltips: bool,
) -> Element<'a, M> {
    row![
        tool_button(
            PaneToolIcon::SplitRight,
            labels::PANE_SPLIT_RIGHT,
            on_split_right,
            show_tooltips
        ),
        tool_button(
            PaneToolIcon::SplitDown,
            labels::PANE_SPLIT_DOWN,
            on_split_down,
            show_tooltips
        ),
        tool_button(
            PaneToolIcon::Close,
            labels::PANE_CLOSE,
            on_close_pane,
            show_tooltips
        ),
    ]
    .spacing(theme::PANE_TOOL_GAP)
    .padding(iced::Padding {
        right: theme::PANE_TOOL_SIZE,
        ..iced::Padding::ZERO
    })
    .into()
}

/// Botão fantasma 13px com ícone desenhado e tooltip (`btn` de `_PaneTools`).
fn tool_button<'a, M: Clone + 'a>(
    icon: PaneToolIcon,
    tip: &'static str,
    on_press: M,
    show_tooltip: bool,
) -> Element<'a, M> {
    let size = theme::PANE_TOOL_SIZE;
    let button = button(canvas::Canvas::new(icon).width(size).height(size))
        .width(size)
        .height(size)
        .padding(0)
        .on_press(on_press)
        .style(theme::ghost_button);
    if !show_tooltip {
        return button.into();
    }
    tooltip(button, tip_bubble(tip), tooltip::Position::Bottom).into()
}

/// Balão de dica (tooltip do Flutter): `panel2`, borda `border2`, texto `text`.
pub fn tip_bubble<'a, M: 'a>(
    tip: impl iced::widget::text::IntoFragment<'a>,
) -> iced::widget::Container<'a, M> {
    container(
        text(tip)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text),
    )
    .padding([4, 8])
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel2)),
        border: Border {
            color: theme::palette().border2,
            width: 1.0,
            radius: theme::RADIUS_BUTTON.into(),
        },
        ..Default::default()
    })
}

/// Divisor entre panes (`PaneDivider`): a linha de 1px é `border` (fundo do grid) e vira `accent`
/// ao passar o mouse ou arrastar.
pub fn grid_style(_: &Theme) -> pane_grid::Style {
    let accent = pane_grid::Line {
        color: theme::palette().accent,
        width: theme::PANE_DIVIDER_LINE,
    };
    pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: Background::Color(iced::Color::TRANSPARENT),
            border: Border::default(),
        },
        picked_split: accent,
        hovered_split: accent,
    }
}

/// Estado visual vazio quando todas as abas do pane foram fechadas.
pub fn view_empty_body<'a, M: 'a>() -> Element<'a, M> {
    container(
        text(labels::PANE_EMPTY_STATE)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text3),
    )
    .padding([20, 24])
    .width(Fill)
    .height(Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::PaneDropZone;
    use iced::{Point, Size};

    #[test]
    fn pane_drop_zones_match_flutter_edge_quadrants() {
        let size = Size::new(400.0, 200.0);
        assert_eq!(
            PaneDropZone::at(Point::new(10.0, 100.0), size),
            PaneDropZone::Left
        );
        assert_eq!(
            PaneDropZone::at(Point::new(390.0, 100.0), size),
            PaneDropZone::Right
        );
        assert_eq!(
            PaneDropZone::at(Point::new(200.0, 45.0), size),
            PaneDropZone::Top
        );
        assert_eq!(
            PaneDropZone::at(Point::new(200.0, 190.0), size),
            PaneDropZone::Bottom
        );
        assert_eq!(
            PaneDropZone::at(Point::new(200.0, 100.0), size),
            PaneDropZone::Center
        );
        assert_eq!(
            PaneDropZone::at(Point::new(100.0, 100.0), size),
            PaneDropZone::Center
        );
        assert_eq!(
            PaneDropZone::at(Point::new(200.0, 20.0), size),
            PaneDropZone::Strip
        );
    }
}
