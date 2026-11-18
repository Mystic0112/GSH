use crate::{
    labels,
    pane_chrome::ChromeCallbacks,
    pane_model::{PaneModel, PaneTab},
    pane_tab::view_tab,
    theme,
};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

pub(crate) fn view_tab_strip<'a, M, FSelect, FClose, FHover>(
    model: &'a PaneModel,
    callbacks: ChromeCallbacks<M, FSelect, FClose>,
    dirty: &dyn Fn(&PaneTab) -> bool,
    hovered: Option<usize>,
    on_hover: FHover,
    tools: Element<'a, M>,
) -> Element<'a, M>
where
    M: Clone + 'a,
    FSelect: Fn(usize) -> M + 'a,
    FClose: Fn(usize) -> M + 'a,
    FHover: Fn(Option<usize>) -> M,
{
    let active_idx = model.active_index();
    let mut tabs_row = row![].height(Fill).align_y(Alignment::Center);
    for (idx, tab) in model.tabs().iter().enumerate() {
        let is_active = active_idx == Some(idx);
        tabs_row = tabs_row.push(view_tab(
            tab,
            crate::pane_tab::TabPresentation {
                title: model.display_title(tab),
                active: is_active,
                dirty: dirty(tab),
                hovered: hovered == Some(idx),
            },
            (callbacks.on_select_tab)(idx),
            (callbacks.on_close_tab)(idx),
            (on_hover(Some(idx)), on_hover(None)),
        ));
    }

    let add_tab = view_add_tab_button(callbacks.on_new_terminal);
    let bar = container(
        // abas e "+" numa faixa com rolagem horizontal que encolhe; as ferramentas ficam
        // sempre visíveis à direita (o Expanded + SingleChildScrollView de pane_view.dart)
        row![
            scrollable(row![tabs_row, add_tab].height(Fill))
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new().width(0).scroller_width(0),
                ))
                .width(Fill)
                .height(Fill),
            tools
        ]
        .width(Fill)
        .height(Length::Fixed(theme::TAB_STRIP_HEIGHT - 1.0))
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(theme::TAB_STRIP_HEIGHT - 1.0))
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().bg)),
        ..Default::default()
    });

    let bottom_hairline = container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(Length::Fixed(1.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        });

    column![bar, bottom_hairline]
        .width(Fill)
        .height(Length::Fixed(theme::TAB_STRIP_HEIGHT))
        .into()
}

fn view_add_tab_button<'a, M: Clone + 'a>(on_new_terminal: M) -> Element<'a, M> {
    let btn = button(
        container(
            text(labels::ICON_PLUS)
                .font(theme::FONT_ICON)
                .size(theme::PANE_TAB_ADD_ICON_SIZE)
                .color(theme::palette().text4),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(theme::PANE_TAB_ADD_WIDTH))
    .height(Fill)
    .padding(0)
    .on_press(on_new_terminal)
    .style(|_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(theme::palette().panel3))
            } else {
                None
            },
            text_color: if hovered {
                theme::palette().text2
            } else {
                theme::palette().text4
            },
            border: Border::default(),
            ..Default::default()
        }
    });

    row![btn, hairline_v()]
        .width(Length::Fixed(theme::PANE_TAB_ADD_WIDTH + 1.0))
        .height(Fill)
        .into()
}

pub(crate) fn hairline_v<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(1.0).height(Fill))
        .width(Length::Fixed(1.0))
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        })
        .into()
}
