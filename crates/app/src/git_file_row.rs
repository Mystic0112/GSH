//! Linha de arquivo alterado com ação de stage ou unstage.
use crate::{git_panel::GitMsg, git_presenter, labels, panel_widgets, theme};
use iced::widget::{button, container, row, text};
use iced::{Border, Element, Fill, Theme};
pub fn view(
    path: &str,
    status: gsh_git::GitFileStatus,
    staged: bool,
    busy: bool,
    selected: bool,
) -> Element<'_, GitMsg> {
    let (dir, name) = path
        .rsplit_once('/')
        .map_or((None, path), |(dir, name)| (Some(dir), name));
    let mut content = row![
        container(
            text(status.badge_code())
                .font(theme::FONT_MONO)
                .size(theme::SIZE_LABEL)
                .color(git_presenter::status_color(status))
        )
        .width(14),
        text(git_presenter::shorten(name, 18))
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text)
    ];
    if let Some(dir) = dir {
        content = content.push(
            text(format!(" {}", git_presenter::shorten(dir, 12)))
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        );
    }
    let select = button(content.clip(true))
        .width(Fill)
        .height(26)
        .padding([2, 3])
        .on_press_maybe((!busy).then(|| GitMsg::Diff(path.to_owned(), staged)))
        .style(move |_: &Theme, state| button::Style {
            background: if selected {
                Some(theme::palette().panel2.into())
            } else if matches!(state, button::Status::Hovered | button::Status::Pressed) {
                Some(theme::palette().panel3.into())
            } else {
                None
            },
            text_color: theme::palette().text,
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    let action = if staged {
        GitMsg::Unstage(path.to_owned())
    } else {
        GitMsg::Stage(path.to_owned())
    };
    row![
        select,
        panel_widgets::ghost(
            if staged {
                labels::ICON_MINUS
            } else {
                labels::ICON_PLUS
            },
            (!busy).then_some(action),
            false
        )
    ]
    .align_y(iced::Alignment::Center)
    .into()
}
