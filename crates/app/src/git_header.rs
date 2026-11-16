//! Cabeçalho e abas do controle de versão (a barra de abas do painel é `right_tabs`).
use crate::{git_panel::GitMsg, git_state::GitState, labels, panel_widgets, theme};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Border, Element, Fill, Theme};
fn tab<'a>(label: &'static str, active: bool, msg: GitMsg) -> Element<'a, GitMsg> {
    button(text(label).font(theme::FONT_TAB).size(theme::SIZE_LABEL))
        .padding([6, 10])
        .on_press(msg)
        .style(move |_: &Theme, status| button::Style {
            background: if active {
                Some(theme::palette().accent_soft.into())
            } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                Some(theme::palette().panel3.into())
            } else {
                None
            },
            text_color: if active {
                theme::palette().text
            } else {
                theme::palette().text3
            },
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
pub fn view(state: &GitState) -> Element<'_, GitMsg> {
    let title = row![
        text(labels::SOURCE_CONTROL_SECTION_UPPER)
            .font(theme::FONT_TITLE)
            .size(theme::SIZE_SECTION_HEADER)
            .color(theme::palette().text3),
        Space::new().width(Fill),
        panel_widgets::ghost(
            labels::ICON_REFRESH,
            (!state.busy).then_some(GitMsg::Refresh),
            false
        )
    ]
    .align_y(iced::Alignment::Center);
    let tabs = row![
        tab(
            labels::GIT_TAB_CHANGES,
            !state.history_open,
            GitMsg::Changes
        ),
        tab(labels::GIT_TAB_HISTORY, state.history_open, GitMsg::History)
    ]
    .spacing(4);
    container(column![title, tabs].spacing(6))
        .padding([6, 14])
        .into()
}
