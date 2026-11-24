use crate::{db_engine_icon, labels, pane_model::PaneTab, pane_strip::hairline_v, theme};
use gsh_db::Engine;
use iced::widget::{
    button, column, container, mouse_area, responsive, row, svg, text, text_input, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Point, Theme};

pub(crate) fn drag_feedback<'a, M: 'a>(
    tab: &'a PaneTab,
    title: String,
    harness_icon: Option<(&'static [u8], bool)>,
) -> Element<'a, M> {
    let icon: Element<'a, M> = match harness_icon {
        Some((bytes, monochrome)) => {
            let icon = svg(svg::Handle::from_memory(bytes))
                .width(13.0)
                .height(13.0);
            if monochrome {
                icon.style(|_: &Theme, _| svg::Style {
                    color: Some(theme::palette().accent_text),
                })
                .into()
            } else {
                icon.into()
            }
        }
        None => match tab {
            PaneTab::Redis { .. } => db_engine_icon::view(Engine::Redis, 13.0),
            PaneTab::Mongo { .. } => db_engine_icon::view(Engine::Mongo, 13.0),
            _ => text(tab.icon())
                .font(if tab.is_file() {
                    theme::FONT_MONO_BOLD
                } else {
                    theme::FONT_ICON
                })
                .size(13.0)
                .color(theme::palette().accent_text)
                .into(),
        },
    };
    container(
        row![
            icon,
            Space::new().width(7.0),
            container(
                text(title)
                    .font(theme::FONT_TAB)
                    .size(theme::SIZE_TAB)
                    .color(theme::palette().text)
                    .wrapping(text::Wrapping::None),
            )
            .width(Fill)
            .clip(true),
        ]
        .align_y(Alignment::Center),
    )
    .max_width(200.0)
    .padding(iced::Padding {
        top: 8.0,
        right: 11.0,
        bottom: 8.0,
        left: 11.0,
    })
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel2)),
        border: Border {
            color: theme::palette().accent,
            width: 1.0,
            radius: 7.0.into(),
        },
        shadow: iced::Shadow {
            color: theme::palette().shadow,
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    })
    .into()
}

/// Marca de agente à direita do título (`pane_view.dart`: spinner, bolinha amarela, "não visto" e vazada).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabIndicator {
    /// Turno em andamento; verde se um hook trouxe o id da sessão do agente, `accent` senão.
    Spinner { agent: bool },
    /// Agente esperando alguém (permissão): bolinha amarela que pisca.
    Waiting,
    /// Terminou e a aba ainda não foi vista: bolinha `accent`.
    Unseen,
    /// Agente parado e já visto: círculo vazado azul.
    Seen,
}

/// Qual marca a aba mostra. `agent` é o status do hook e só vale com o id da sessão do agente
/// (`claudeSessionId != null` no Flutter); a ordem de prioridade é a de `pane_view.dart`.
pub(crate) fn tab_indicator(
    working: bool,
    agent: Option<gsh_core::status::TurnStatus>,
    unseen_finish: bool,
) -> Option<TabIndicator> {
    use gsh_core::status::TurnStatus;
    if working {
        Some(TabIndicator::Spinner {
            agent: agent.is_some(),
        })
    } else if agent == Some(TurnStatus::Waiting) {
        Some(TabIndicator::Waiting)
    } else if unseen_finish {
        Some(TabIndicator::Unseen)
    } else if agent == Some(TurnStatus::Idle) {
        Some(TabIndicator::Seen)
    } else {
        None
    }
}

impl TabIndicator {
    /// Anima enquanto houver spinner ou pisca.
    pub(crate) fn animates(self) -> bool {
        matches!(self, Self::Spinner { .. } | Self::Waiting)
    }
}

/// Meio ciclo do pisca do status (`_Pulse._every`): todas as bolinhas amarelas piscam juntas.
const PULSE_HALF_MS: u64 = 700;

fn pulse_bright(anim_ms: u64) -> bool {
    (anim_ms / PULSE_HALF_MS).is_multiple_of(2)
}

const INDICATOR_SPINNER_SIZE: f32 = 10.0;
const INDICATOR_SPINNER_STROKE: f32 = 1.5;
const INDICATOR_DOT_SIZE: f32 = 7.0;
const INDICATOR_HOLLOW_STROKE: f32 = 1.4;
/// Espaço entre a marca e o X (`SizedBox(width: 7)`).
const INDICATOR_GAP: f32 = 7.0;

fn indicator_view<'a, M: 'a>(
    indicator: TabIndicator,
    anim_ms: u64,
    drag_alpha: f32,
) -> Element<'a, M> {
    let palette = theme::palette();
    let dot = |fill: Option<Color>, outline: Option<Color>| -> Element<'a, M> {
        container(
            Space::new()
                .width(INDICATOR_DOT_SIZE)
                .height(INDICATOR_DOT_SIZE),
        )
        .style(move |_: &Theme| container::Style {
            background: fill.map(|color| Background::Color(color.scale_alpha(drag_alpha))),
            border: Border {
                color: outline.map_or(Color::TRANSPARENT, |color| color.scale_alpha(drag_alpha)),
                width: if outline.is_some() {
                    INDICATOR_HOLLOW_STROKE
                } else {
                    0.0
                },
                radius: (INDICATOR_DOT_SIZE / 2.0).into(),
            },
            ..Default::default()
        })
        .into()
    };
    match indicator {
        TabIndicator::Spinner { agent } => crate::content_search_panel::spinner_colored(
            INDICATOR_SPINNER_SIZE,
            INDICATOR_SPINNER_STROKE,
            (if agent {
                palette.online
            } else {
                palette.accent
            })
            .scale_alpha(drag_alpha),
            anim_ms,
        ),
        TabIndicator::Waiting => {
            let alpha = if pulse_bright(anim_ms) { 1.0 } else { 0.3 };
            dot(Some(palette.warn.scale_alpha(alpha)), None)
        }
        TabIndicator::Unseen => dot(Some(palette.accent), None),
        TabIndicator::Seen => dot(None, Some(palette.git_untracked)),
    }
}

pub(crate) struct TabPresentation {
    /// Balão com o título completo ao pairar (`AppTooltip(message: displayTitle)`).
    pub show_tooltip: bool,
    pub indicator: Option<TabIndicator>,
    /// Relógio da animação (ms) do spinner e do pisca.
    pub anim_ms: u64,
    pub title: String,
    pub active: bool,
    pub focused: bool,
    pub dirty: bool,
    pub hovered: bool,
    pub drop_before: Option<bool>,
    pub dragging: bool,
}

/// Id do campo de renomear da aba (só existe um por vez).
pub(crate) const RENAME_INPUT_ID: &str = "pane-tab-rename";

/// Campo inline que troca o título da aba ao renomear (`TextField` de `_Tab` em `pane_view.dart`).
#[derive(Clone)]
pub(crate) struct RenameField<M> {
    pub value: String,
    pub on_input: fn(String) -> M,
    pub on_submit: M,
}

/// Mensagens das interações da aba além de selecionar e arrastar.
pub(crate) struct TabActions<M> {
    pub on_close: M,
    pub hover: (M, M),
    /// Botão direito (`onSecondaryTapUp`): abre o menu da aba.
    pub on_context_menu: M,
    /// Botão do meio (`onTertiaryTapUp`): fecha a aba.
    pub on_middle_click: M,
    pub rename: Option<RenameField<M>>,
}

pub(crate) fn view_tab<'a, M, FDragStart, FDragHover, FDragDrop>(
    tab: &PaneTab,
    presentation: TabPresentation,
    on_select: M,
    actions: TabActions<M>,
    drag: (FDragStart, FDragHover, FDragDrop),
    terminal_icon: Option<(&'static [u8], bool)>,
) -> Element<'a, M>
where
    M: Clone + 'a,
    FDragStart: Fn(Point) -> M + 'a,
    FDragHover: Fn(Point, Point) -> M + 'a,
    FDragDrop: Fn(Point) -> M + 'a,
{
    let TabActions {
        on_close,
        hover: hover_messages,
        on_context_menu,
        on_middle_click,
        rename,
    } = actions;
    let TabPresentation {
        show_tooltip,
        indicator,
        anim_ms,
        title,
        active: is_active,
        focused,
        dirty,
        hovered,
        drop_before,
        dragging,
    } = presentation;
    let drag_alpha = if dragging { 0.3_f32 } else { 1.0_f32 };
    let icon_color = (if is_focused_active(is_active, focused) {
        theme::palette().accent_text
    } else if is_active {
        theme::palette().text2
    } else {
        theme::palette().text3
    })
    .scale_alpha(drag_alpha);
    let title_color = (if is_active {
        theme::palette().text
    } else {
        theme::palette().text3
    })
    .scale_alpha(drag_alpha);

    let leading_icon: Element<'a, M> = if let Some((bytes, monochrome)) = terminal_icon {
        let icon = svg::Handle::from_memory(bytes);
        let icon = svg(icon)
            .width(theme::PANE_TAB_ICON_SIZE)
            .height(theme::PANE_TAB_ICON_SIZE)
            .opacity(drag_alpha);
        if monochrome {
            icon.style(move |_: &Theme, _| svg::Style {
                color: Some(icon_color),
            })
            .into()
        } else {
            icon.into()
        }
    } else {
        match tab {
            // `FileTypeIcon.file(nome, size: 15)`: o SVG do tema pelo nome do arquivo
            PaneTab::File(path) => crate::file_icons::view_sized_opacity(
                &crate::pane_model::file_display_name(path),
                false,
                false,
                theme::PANE_FILE_TAB_ICON_SIZE,
                drag_alpha,
            ),
            PaneTab::Redis { .. } => db_engine_icon::view(Engine::Redis, theme::PANE_TAB_ICON_SIZE),
            PaneTab::Mongo { .. } => db_engine_icon::view(Engine::Mongo, theme::PANE_TAB_ICON_SIZE),
            _ => text(tab.icon())
                .font(if tab.is_file() {
                    theme::FONT_MONO_BOLD
                } else {
                    theme::FONT_ICON
                })
                .size(if tab.is_file() {
                    theme::PANE_FILE_TAB_ICON_SIZE
                } else {
                    theme::PANE_TAB_ICON_SIZE
                })
                .color(icon_color)
                .into(),
        }
    };

    let editing = rename.is_some();
    let title_element: Element<'a, M> = match rename {
        Some(field) => text_input("", &field.value)
            .id(RENAME_INPUT_ID)
            .on_input(field.on_input)
            .on_submit(field.on_submit)
            .font(theme::FONT_TAB)
            .size(12.0)
            .padding([2, 6])
            .width(Fill)
            .style(rename_input_style)
            .into(),
        None => {
            let tooltip_title = title.clone();
            // `TextOverflow.ellipsis`: o corte depende da largura que sobra ao título
            let label: Element<'a, M> = responsive(move |size| {
                container(
                    text(ellipsize_to_width(&title, size.width))
                        .font(theme::FONT_TAB)
                        .size(theme::SIZE_TAB)
                        .color(title_color)
                        .wrapping(text::Wrapping::None),
                )
                .height(Fill)
                .align_y(Alignment::Center)
                .into()
            })
            .width(Fill)
            .height(Fill)
            .into();
            if show_tooltip {
                iced::widget::tooltip(
                    label,
                    crate::pane_chrome::tip_bubble(tooltip_title),
                    iced::widget::tooltip::Position::Bottom,
                )
                .into()
            } else {
                label
            }
        }
    };
    let mut tab_label = row![
        leading_icon,
        Space::new().width(theme::PANE_TAB_ICON_GAP),
        title_element,
    ];
    // título, 10, marca, 7, X: a marca fica dentro do botão de seleção (clicar nela seleciona a aba)
    let close_gap = match indicator {
        Some(indicator) if !editing => {
            tab_label = tab_label
                .push(Space::new().width(theme::PANE_TAB_CLOSE_GAP))
                .push(indicator_view(indicator, anim_ms, drag_alpha));
            INDICATOR_GAP
        }
        _ => theme::PANE_TAB_CLOSE_GAP,
    };
    // o button não centra o conteúdo: a linha ocupa a altura toda e centra ícone e título
    let tab_label = tab_label.height(Fill).align_y(Alignment::Center);
    // renomeando, o título é um campo: sem botão de seleção por baixo (nem arraste)
    let select_btn: Element<'a, M> = if editing {
        container(tab_label).width(Fill).height(Fill).into()
    } else {
        button(tab_label)
            .width(Fill)
            .height(Fill)
            .padding(0)
            .on_press(on_select)
            .style(move |_: &Theme, _status| {
                let bg = if is_active {
                    theme::palette().panel
                } else {
                    theme::palette().bg
                };
                button::Style {
                    background: Some(Background::Color(bg.scale_alpha(drag_alpha))),
                    text_color: title_color,
                    border: Border::default(),
                    ..Default::default()
                }
            })
            .into()
    };

    let close_icon: Element<'a, M> = if show_dirty_dot(dirty, hovered) {
        container(
            Space::new()
                .width(theme::PANE_TAB_DIRTY_DOT_SIZE)
                .height(theme::PANE_TAB_DIRTY_DOT_SIZE),
        )
        .style(move |_: &Theme| container::Style {
            background: Some(theme::palette().accent.scale_alpha(drag_alpha).into()),
            border: Border {
                radius: (theme::PANE_TAB_DIRTY_DOT_SIZE / 2.0).into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    } else {
        text(labels::ICON_CLOSE)
            .font(theme::FONT_ICON)
            .size(theme::PANE_TAB_CLOSE_ICON_SIZE)
            .color(theme::palette().text3.scale_alpha(drag_alpha))
            .into()
    };
    let close_btn = button(
        container(close_icon)
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill),
    )
    .width(Length::Fixed(
        theme::PANE_TAB_CLOSE_ICON_SIZE + 2.0 * theme::PANE_TAB_CLOSE_PADDING,
    ))
    .height(Length::Fixed(
        theme::PANE_TAB_CLOSE_ICON_SIZE + 2.0 * theme::PANE_TAB_CLOSE_PADDING,
    ))
    .padding(0)
    .on_press(on_close)
    .style(move |_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(
                    theme::palette().panel3.scale_alpha(drag_alpha),
                ))
            } else {
                None
            },
            text_color: if hovered {
                theme::palette().text.scale_alpha(drag_alpha)
            } else {
                theme::palette().text3.scale_alpha(drag_alpha)
            },
            border: Border {
                radius: theme::RADIUS_BADGE.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    let (drag_start, drag_hover, drag_drop) = drag;
    let select_area = crate::pane_focus::FocusOnPress::new(select_btn, None).on_pointer_event(
        move |point, _, cursor, phase| match phase {
            crate::pane_focus::PointerPhase::Press => drag_start(cursor),
            crate::pane_focus::PointerPhase::Move => drag_hover(point, cursor),
            crate::pane_focus::PointerPhase::Release => drag_drop(cursor),
        },
    );
    let top_area = container(
        row![
            select_area,
            Space::new().width(close_gap),
            mouse_area(close_btn)
                .on_enter(hover_messages.0)
                .on_exit(hover_messages.1)
        ]
        .width(Fill)
        .height(Fill)
        .padding(iced::Padding {
            left: theme::PANE_TAB_LEFT_PADDING,
            right: theme::PANE_TAB_RIGHT_PADDING,
            ..iced::Padding::ZERO
        })
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Fill)
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(if is_active {
            theme::palette().panel.scale_alpha(drag_alpha)
        } else {
            theme::palette().bg.scale_alpha(drag_alpha)
        })),
        ..Default::default()
    });

    let bottom_bar: Element<'a, M> = if is_focused_active(is_active, focused) {
        container(
            Space::new()
                .width(Fill)
                .height(theme::PANE_TAB_INDICATOR_HEIGHT),
        )
        .width(Fill)
        .height(Length::Fixed(theme::PANE_TAB_INDICATOR_HEIGHT))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().accent)),
            ..Default::default()
        })
        .into()
    } else {
        container(
            Space::new()
                .width(Fill)
                .height(theme::PANE_TAB_INDICATOR_HEIGHT),
        )
        .style(|_| container::Style {
            background: Some(theme::palette().bg.into()),
            ..Default::default()
        })
        .into()
    };

    let tab_col = column![top_area, bottom_bar]
        .width(Length::Fixed(theme::TAB_WIDTH - 1.0))
        .height(Fill);

    let tab = row![tab_col, hairline_v()]
        .width(Length::Fixed(theme::TAB_WIDTH))
        .height(Fill);
    let tab: Element<'a, M> = match drop_before {
        Some(before) => with_drop_caret(tab.into(), before),
        None => tab.into(),
    };
    // Flutter: `onSecondaryTapUp` abre o menu e `onTertiaryTapUp` fecha (ambos no soltar)
    mouse_area(tab)
        .on_right_release(on_context_menu)
        .on_middle_release(on_middle_click)
        .into()
}

fn with_drop_caret<'a, M: 'a>(tab: Element<'a, M>, before: bool) -> Element<'a, M> {
    let caret = container(Space::new().width(2.5).height(Fill))
        .padding(iced::Padding {
            top: 6.0,
            bottom: 6.0,
            ..iced::Padding::ZERO
        })
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().accent)),
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    let caret_row = if before {
        row![caret, Space::new().width(Fill)]
    } else {
        row![Space::new().width(Fill), caret]
    }
    .width(Length::Fixed(theme::TAB_WIDTH))
    .height(Fill);
    iced::widget::stack![tab, caret_row]
        .width(Length::Fixed(theme::TAB_WIDTH))
        .height(Fill)
        .into()
}

/// Campo de renomear (`TextField` do shadcn com `borderRadius: 7`): `input_style` do app.
fn rename_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = crate::panel_widgets::input_style(theme, status);
    style.border.radius = 7.0.into();
    style
}

/// Largura em px que `content` ocupa numa linha só com a fonte da aba (o mesmo
/// medidor do `text`, sem precisar de renderer).
fn measure_title(content: &str) -> f32 {
    use iced::advanced::text::{self as adv, Paragraph as _};
    type Para = <iced::Renderer as adv::Renderer>::Paragraph;
    let paragraph = Para::with_text(adv::Text {
        content,
        bounds: iced::Size::INFINITE,
        size: theme::SIZE_TAB.into(),
        line_height: adv::LineHeight::default(),
        font: theme::FONT_TAB,
        align_x: adv::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: adv::Shaping::default(),
        wrapping: adv::Wrapping::None,
    });
    paragraph.min_width()
}

/// Título que cabe em `max_width`: inteiro, ou cortado com "…" no fim (`TextOverflow.ellipsis`).
fn ellipsize_to_width(title: &str, max_width: f32) -> String {
    ellipsize_with(title, max_width, measure_title)
}

fn ellipsize_with(title: &str, max_width: f32, measure: impl Fn(&str) -> f32) -> String {
    if measure(title) <= max_width {
        return title.to_owned();
    }
    let chars: Vec<char> = title.chars().collect();
    let fits = |count: usize| {
        let mut candidate: String = chars.iter().take(count).collect();
        candidate.push('…');
        measure(&candidate) <= max_width
    };
    // maior prefixo que, com a reticência, cabe: busca binária (a largura só cresce com o prefixo)
    let (mut low, mut high) = (0, chars.len());
    while low < high {
        let mid = (low + high).div_ceil(2);
        if fits(mid) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    let mut out: String = chars.iter().take(low).collect();
    out.push('…');
    out
}

fn is_focused_active(active: bool, focused: bool) -> bool {
    active && focused
}

fn show_dirty_dot(dirty: bool, hovered: bool) -> bool {
    dirty && !hovered
}

#[cfg(test)]
mod tests {
    use super::{
        ellipsize_with, is_focused_active, measure_title, pulse_bright, show_dirty_dot,
        tab_indicator, TabIndicator,
    };
    use gsh_core::status::TurnStatus;

    #[test]
    fn indicator_priority_matches_pane_view() {
        use TabIndicator::*;
        // trabalhando: spinner verde com agente, accent sem
        assert_eq!(
            tab_indicator(true, Some(TurnStatus::Working), true),
            Some(Spinner { agent: true })
        );
        assert_eq!(
            tab_indicator(true, None, false),
            Some(Spinner { agent: false })
        );
        assert_eq!(
            tab_indicator(false, Some(TurnStatus::Waiting), true),
            Some(Waiting)
        );
        assert_eq!(tab_indicator(false, None, true), Some(Unseen));
        assert_eq!(
            tab_indicator(false, Some(TurnStatus::Idle), true),
            Some(Unseen)
        );
        assert_eq!(
            tab_indicator(false, Some(TurnStatus::Idle), false),
            Some(Seen)
        );
        // sem agente (sem id de sessão) e sem nada novo: nada
        assert_eq!(tab_indicator(false, None, false), None);
    }

    #[test]
    fn only_spinner_and_waiting_animate() {
        assert!(TabIndicator::Spinner { agent: true }.animates());
        assert!(TabIndicator::Waiting.animates());
        assert!(!TabIndicator::Unseen.animates());
        assert!(!TabIndicator::Seen.animates());
    }

    #[test]
    fn pulse_flips_every_700ms() {
        assert!(pulse_bright(0));
        assert!(pulse_bright(699));
        assert!(!pulse_bright(700));
        assert!(!pulse_bright(1399));
        assert!(pulse_bright(1400));
    }

    #[test]
    fn active_indicator_only_belongs_to_the_focused_pane() {
        assert!(is_focused_active(true, true));
        assert!(!is_focused_active(true, false));
        assert!(!is_focused_active(false, true));
    }

    #[test]
    fn dirty_dot_only_appears_without_hover() {
        assert!(show_dirty_dot(true, false));
        assert!(!show_dirty_dot(true, true));
        assert!(!show_dirty_dot(false, false));
        assert!(!show_dirty_dot(false, true));
    }

    #[test]
    fn title_that_fits_is_kept_whole() {
        assert_eq!(
            ellipsize_with("abc", 30.0, |s| s.chars().count() as f32 * 10.0),
            "abc"
        );
    }

    #[test]
    fn long_title_is_cut_with_an_ellipsis_that_still_fits() {
        // 10 px por caractere, 55 px de espaço: 4 letras + "…" = 50 px
        let cut = ellipsize_with("abcdefgh", 55.0, |s| s.chars().count() as f32 * 10.0);
        assert_eq!(cut, "abcd…");
        // nem a reticência cabe: sobra só ela
        assert_eq!(
            ellipsize_with("abcdefgh", 5.0, |s| s.chars().count() as f32 * 10.0),
            "…"
        );
    }

    #[test]
    fn real_font_measure_grows_with_text() {
        assert!(measure_title("Terminal") > measure_title("Term"));
        assert!(measure_title("") <= 1.0);
    }
}
