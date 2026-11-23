use crate::{
    anchored::Anchored,
    app_menu::{self, Entry, Item, Leading},
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

pub struct StripInput<FWheel, M> {
    pub pane: iced::widget::pane_grid::Pane,
    pub focused: bool,
    pub on_wheel: FWheel,
    pub menu_open: bool,
    pub on_open_menu: M,
    pub on_close_menu: M,
}
use iced::widget::{
    button, column, container, mouse_area, responsive, row, scrollable, text, Space,
};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

pub(crate) fn view_tab_strip<
    'a,
    M,
    FSelect,
    FClose,
    FHover,
    FDrag,
    FIcon,
    FTitle,
    FPaneDrag,
    FWheel,
>(
    model: &'a PaneModel,
    callbacks: ChromeCallbacks<M, FSelect, FClose, FDrag, FIcon, FTitle, FPaneDrag>,
    dirty: &dyn Fn(&PaneTab) -> bool,
    hovered: Option<usize>,
    on_hover: FHover,
    input: StripInput<FWheel, M>,
    tools: Element<'a, M>,
) -> Element<'a, M>
where
    M: Clone + 'a,
    FSelect: Fn(usize) -> M + Clone + 'a,
    FClose: Fn(usize) -> M + Clone + 'a,
    FHover: Fn(Option<usize>) -> M + Clone + 'a,
    FDrag: Fn(usize, TabDragEvent) -> M + Clone + 'a,
    FIcon: Fn(crate::pane_model::TermId) -> Option<(&'static [u8], bool)> + Clone + 'a,
    FTitle: Fn(crate::pane_model::TermId) -> Option<String> + Clone + 'a,
    FWheel: Fn(f32) -> M + Clone + 'a,
{
    let StripInput {
        pane,
        focused,
        on_wheel,
        menu_open,
        on_open_menu,
        on_close_menu,
    } = input;
    let active_idx = model.active_index();
    let tab_count = model.tabs().len();
    let dirty_flags = model.tabs().iter().map(dirty).collect::<Vec<_>>();
    let tabs_model = model;
    let select_for_menu = callbacks.on_select_tab.clone();
    let menu_entries = move || {
        tabs_model
            .tabs()
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                Item::Entry(
                    Entry::new(tabs_model.display_title(tab), select_for_menu(index))
                        .leading(Leading::Glyph(tab.icon()))
                        .selected(active_idx == Some(index)),
                )
            })
            .collect::<Vec<_>>()
    };
    let tab_model = model;
    let select = callbacks.on_select_tab.clone();
    let close = callbacks.on_close_tab.clone();
    let hover = on_hover.clone();
    let drag = std::rc::Rc::new(callbacks.on_tab_drag.clone());
    let icon = callbacks.terminal_icon.clone();
    let title = callbacks.terminal_title.clone();
    let drop_target = callbacks.drop_target;
    let drag_source = callbacks.drag_source;
    let make_tabs = move || {
        let mut tabs_row = row![].height(Fill).align_y(Alignment::Center);
        for (idx, tab) in tab_model.tabs().iter().enumerate() {
            let on_drag_start = std::rc::Rc::clone(&drag);
            let on_drag_hover = std::rc::Rc::clone(&drag);
            let on_drag_drop = std::rc::Rc::clone(&drag);
            let tab_title = match tab {
                PaneTab::Terminal(id) if tab_model.terminal_label(*id).is_none() => {
                    title(*id).unwrap_or_else(|| tab_model.display_title(tab))
                }
                _ => tab_model.display_title(tab),
            };
            tabs_row = tabs_row.push(view_tab(
                tab,
                crate::pane_tab::TabPresentation {
                    title: tab_title,
                    active: active_idx == Some(idx),
                    focused,
                    dirty: dirty_flags.get(idx).copied().unwrap_or(false),
                    hovered: hovered == Some(idx),
                    drop_before: drop_target
                        .filter(|(target, _)| *target == idx)
                        .map(|(_, before)| before),
                    dragging: drag_source == Some(idx),
                },
                select(idx),
                close(idx),
                (hover(Some(idx)), hover(None)),
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
                    PaneTab::Terminal(id) => icon(*id),
                    _ => None,
                },
            ));
        }
        tabs_row
    };
    let overflow_button = move |available_width: f32| -> Element<'a, M> {
        if !has_tab_overflow(tab_count, available_width) {
            return Space::new().width(0.0).into();
        }
        let trigger = button(
            text(labels::ICON_CHEVRON_DOWN)
                .font(theme::FONT_ICON)
                .size(16.0)
                .color(theme::palette().text3),
        )
        .width(28.0)
        .height(28.0)
        .padding(0)
        .on_press(on_open_menu.clone())
        .style(|_: &Theme, status| button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed)
                .then(|| Background::Color(theme::palette().panel3)),
            text_color: theme::palette().text3,
            border: Border {
                radius: 5.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });
        iced::widget::tooltip(
            Anchored::new(
                trigger,
                menu_open.then(|| app_menu::menu(menu_entries(), 220.0)),
                on_close_menu.clone(),
            ),
            labels::PANE_VIEW_ALL_TABS,
            iced::widget::tooltip::Position::Bottom,
        )
        .into()
    };
    let wheel = on_wheel.clone();

    let bar = container(
        // Somente as abas rolam. O Flutter mantém o overflow, o + e as ferramentas
        // fora do SingleChildScrollView, mesmo quando a pane fica estreita.
        row![
            responsive(move |size| {
                let tabs_row = make_tabs();
                row![
                    mouse_area(
                        scrollable(tabs_row)
                            .direction(scrollable::Direction::Horizontal(
                                scrollable::Scrollbar::new().width(3).scroller_width(3),
                            ))
                            .id(scrollable_id(pane))
                            .style(|theme, status| {
                                let mut style = scrollable::default(theme, status);
                                style.horizontal_rail.background = None;
                                style.horizontal_rail.border.radius = 3.0.into();
                                style.horizontal_rail.scroller.background = match status {
                                    scrollable::Status::Active { .. } => {
                                        theme::palette().text4.scale_alpha(0.0).into()
                                    }
                                    scrollable::Status::Hovered {
                                        is_horizontal_scrollbar_disabled: false,
                                        ..
                                    }
                                    | scrollable::Status::Dragged {
                                        is_horizontal_scrollbar_disabled: false,
                                        ..
                                    } => theme::palette().text4.into(),
                                    _ => theme::palette().text4.scale_alpha(0.0).into(),
                                };
                                style.horizontal_rail.scroller.border.radius = 3.0.into();
                                style
                            })
                            .width(Fill)
                            .height(Fill),
                    )
                    .on_scroll({
                        let wheel = wheel.clone();
                        move |delta| {
                            let (x, y) = match delta {
                                iced::mouse::ScrollDelta::Lines { x, y } => (x * 60.0, y * 60.0),
                                iced::mouse::ScrollDelta::Pixels { x, y } => (x, y),
                            };
                            wheel(wheel_delta(x, y))
                        }
                    }),
                    overflow_button(size.width),
                ]
                .width(Fill)
                .height(Fill)
                .align_y(Alignment::Center)
                .into()
            })
            .width(Fill)
            .height(Fill),
            view_add_tab_button(callbacks.on_new_terminal),
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

pub(crate) fn scrollable_id(pane: iced::widget::pane_grid::Pane) -> iced::widget::Id {
    iced::widget::Id::from(format!("pane-tab-strip-{pane:?}"))
}

/// Produz uma operação que mede o viewport real e aplica a regra Flutter de
/// trazer a aba ativa à vista, sem presumir a largura do pane.
pub(crate) fn reveal_active_tab<M: Send + 'static>(
    pane: iced::widget::pane_grid::Pane,
    index: usize,
) -> iced::Task<M> {
    iced_runtime::task::widget(reveal_active_tab_operation(pane, index))
}

pub(crate) fn reveal_active_tab_operation<M>(
    pane: iced::widget::pane_grid::Pane,
    index: usize,
) -> impl iced::advanced::widget::Operation<M> {
    struct Reveal {
        id: iced::widget::Id,
        index: usize,
    }

    impl<M> iced::advanced::widget::Operation<M> for Reveal {
        fn traverse(
            &mut self,
            operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation<M>),
        ) {
            operate(self);
        }

        fn scrollable(
            &mut self,
            id: Option<&iced::widget::Id>,
            bounds: iced::Rectangle,
            content_bounds: iced::Rectangle,
            translation: iced::Vector,
            state: &mut dyn iced::advanced::widget::operation::Scrollable,
        ) {
            if id != Some(&self.id) {
                return;
            }
            if let Some(target) = active_tab_scroll_target(
                self.index,
                translation.x,
                bounds.width,
                (content_bounds.width - bounds.width).max(0.0),
            ) {
                state.scroll_to(iced::widget::scrollable::AbsoluteOffset {
                    x: Some(target),
                    y: None,
                });
            }
        }
    }

    Reveal {
        id: scrollable_id(pane),
        index,
    }
}

/// Calcula o offset que traz uma aba de largura fixa para dentro do viewport,
/// copiando `_scrollActiveIntoView` de `pane_view.dart`.
pub(crate) fn active_tab_scroll_target(
    index: usize,
    viewport_start: f32,
    viewport_width: f32,
    max_scroll: f32,
) -> Option<f32> {
    let start = index as f32 * theme::TAB_WIDTH;
    let end = start + theme::TAB_WIDTH;
    let viewport_end = viewport_start + viewport_width;
    let target = if start < viewport_start {
        Some(start)
    } else if end > viewport_end {
        Some(end - viewport_width)
    } else {
        None
    }?;
    Some(target.clamp(0.0, max_scroll))
}

/// Flutter dá prioridade ao eixo X da roda e usa Y quando X é zero.
pub(crate) fn wheel_delta(x: f32, y: f32) -> f32 {
    if x != 0.0 {
        x
    } else {
        y
    }
}

/// `available_width` is the responsive slot before the optional 28px trigger.
pub(crate) fn has_tab_overflow(tab_count: usize, available_width: f32) -> bool {
    tab_count as f32 * theme::TAB_WIDTH > (available_width - 28.0).max(0.0)
}

#[cfg(test)]
#[path = "pane_strip_tests.rs"]
mod tests;

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

    let btn = iced::widget::tooltip(
        btn,
        labels::PANE_VIEW_NEW_TAB,
        iced::widget::tooltip::Position::Bottom,
    );

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
