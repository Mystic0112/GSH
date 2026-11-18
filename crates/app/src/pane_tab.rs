use crate::{db_engine_icon, labels, pane_model::PaneTab, pane_strip::hairline_v, theme};
use gsh_db::Engine;
use iced::widget::{button, column, container, mouse_area, row, svg, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Point, Theme};

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

pub(crate) struct TabPresentation {
    pub title: String,
    pub active: bool,
    pub dirty: bool,
    pub hovered: bool,
    pub drop_before: Option<bool>,
    pub dragging: bool,
}

pub(crate) fn view_tab<'a, M, FDragStart, FDragHover, FDragDrop>(
    tab: &PaneTab,
    presentation: TabPresentation,
    on_select: M,
    on_close: M,
    hover_messages: (M, M),
    drag: (FDragStart, FDragHover, FDragDrop),
    terminal_icon: Option<(&'static [u8], bool)>,
) -> Element<'a, M>
where
    M: Clone + 'a,
    FDragStart: Fn(Point) -> M + 'a,
    FDragHover: Fn(Point, Point) -> M + 'a,
    FDragDrop: Fn(Point) -> M + 'a,
{
    let TabPresentation {
        title,
        active: is_active,
        dirty,
        hovered,
        drop_before,
        dragging,
    } = presentation;
    let drag_alpha = if dragging { 0.3_f32 } else { 1.0_f32 };
    let icon_color = (if is_active {
        theme::palette().accent_text
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

    let tab_label = row![
        leading_icon,
        Space::new().width(theme::PANE_TAB_ICON_GAP),
        container(
            text(title)
                .font(theme::FONT_TAB)
                .size(theme::SIZE_TAB)
                .color(title_color)
                .wrapping(text::Wrapping::None),
        )
        .width(Fill)
        .clip(true),
    ];
    // o button não centra o conteúdo: a linha ocupa a altura toda e centra ícone e título
    let select_btn = button(tab_label.height(Fill).align_y(Alignment::Center))
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
        });

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
            Space::new().width(theme::PANE_TAB_CLOSE_GAP),
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

    let bottom_bar: Element<'a, M> = if is_active {
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
    let Some(before) = drop_before else {
        return tab.into();
    };
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

fn show_dirty_dot(dirty: bool, hovered: bool) -> bool {
    dirty && !hovered
}

#[cfg(test)]
mod tests {
    use super::show_dirty_dot;

    #[test]
    fn dirty_dot_only_appears_without_hover() {
        assert!(show_dirty_dot(true, false));
        assert!(!show_dirty_dot(true, true));
        assert!(!show_dirty_dot(false, false));
        assert!(!show_dirty_dot(false, true));
    }
}
