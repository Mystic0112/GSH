use crate::{db_engine_icon, labels, pane_model::PaneTab, pane_strip::hairline_v, theme};
use gsh_db::Engine;
use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

pub(crate) fn view_tab<'a, M: Clone + 'a>(
    tab: &PaneTab,
    is_active: bool,
    dirty: bool,
    hovered: bool,
    on_select: M,
    on_close: M,
    hover_messages: (M, M),
) -> Element<'a, M> {
    let icon_color = if is_active {
        theme::palette().accent_text
    } else {
        theme::palette().text3
    };
    let title_color = if is_active {
        theme::palette().text
    } else {
        theme::palette().text3
    };

    let leading_icon: Element<'a, M> = match tab {
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
    };

    let tab_label = row![
        leading_icon,
        Space::new().width(theme::PANE_TAB_ICON_GAP),
        container(
            text(tab.title())
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
                background: Some(Background::Color(bg)),
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
        .style(|_: &Theme| container::Style {
            background: Some(theme::palette().accent.into()),
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
            .color(theme::palette().text3)
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
    .style(|_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(theme::palette().panel3))
            } else {
                None
            },
            text_color: if hovered {
                theme::palette().text
            } else {
                theme::palette().text3
            },
            border: Border {
                radius: theme::RADIUS_BADGE.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    let top_area = container(
        row![
            select_btn,
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
            theme::palette().panel
        } else {
            theme::palette().bg
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

    row![tab_col, hairline_v()]
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
