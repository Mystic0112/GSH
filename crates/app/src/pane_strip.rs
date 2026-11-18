use crate::{
    labels,
    pane_chrome::ChromeCallbacks,
    pane_model::{PaneModel, PaneTab},
    pane_tab::view_tab,
    theme,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabDragEvent {
    Start {
        index: usize,
        cursor: iced::Point,
    },
    Hover {
        index: usize,
        before: bool,
        cursor: iced::Point,
    },
    Drop {
        index: usize,
        cursor: iced::Point,
    },
}
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

pub(crate) fn view_tab_strip<'a, M, FSelect, FClose, FHover, FDrag, FIcon, FTitle, FPaneDrag>(
    model: &'a PaneModel,
    callbacks: ChromeCallbacks<M, FSelect, FClose, FDrag, FIcon, FTitle, FPaneDrag>,
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
    FDrag: Fn(usize, TabDragEvent) -> M + 'a,
    FIcon: Fn(crate::pane_model::TermId) -> Option<(&'static [u8], bool)>,
    FTitle: Fn(crate::pane_model::TermId) -> Option<String>,
{
    let active_idx = model.active_index();
    let mut tabs_row = row![].height(Fill).align_y(Alignment::Center);
    let on_drag = std::rc::Rc::new(callbacks.on_tab_drag);
    for (idx, tab) in model.tabs().iter().enumerate() {
        let is_active = active_idx == Some(idx);
        let on_drag_start = std::rc::Rc::clone(&on_drag);
        let on_drag_hover = std::rc::Rc::clone(&on_drag);
        let on_drag_drop = std::rc::Rc::clone(&on_drag);
        tabs_row = tabs_row.push(view_tab(
            tab,
            crate::pane_tab::TabPresentation {
                title: match tab {
                    PaneTab::Terminal(id) if model.terminal_label(*id).is_none() => {
                        (callbacks.terminal_title)(*id).unwrap_or_else(|| model.display_title(tab))
                    }
                    _ => model.display_title(tab),
                },
                active: is_active,
                dirty: dirty(tab),
                hovered: hovered == Some(idx),
                drop_before: callbacks
                    .drop_target
                    .filter(|(target, _)| *target == idx)
                    .map(|(_, before)| before),
                dragging: callbacks.drag_source == Some(idx),
            },
            (callbacks.on_select_tab)(idx),
            (callbacks.on_close_tab)(idx),
            (on_hover(Some(idx)), on_hover(None)),
            (
                move |cursor| on_drag_start(idx, TabDragEvent::Start { index: idx, cursor }),
                move |point, cursor| {
                    on_drag_hover(
                        idx,
                        TabDragEvent::Hover {
                            index: idx,
                            before: point.x < theme::TAB_WIDTH / 2.0,
                            cursor,
                        },
                    )
                },
                move |point| {
                    on_drag_drop(
                        idx,
                        TabDragEvent::Drop {
                            index: idx,
                            cursor: point,
                        },
                    )
                },
            ),
            match tab {
                PaneTab::Terminal(id) => (callbacks.terminal_icon)(*id),
                _ => None,
            },
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
