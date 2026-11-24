//! Barra superior do shell (`gsh_topbar.dart` / `WindowTitleBar` do Flutter).
//!
//! Altura 46px, superfície `BG`, linha divisória inferior de 1px `BORDER`,
//! botão de recolher o rail à esquerda + título do workspace ativo, pílula da
//! `AgentIsland` centralizada e botão do painel de arquivos à direita.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::widget::{button, container, row, stack, text, tooltip, Space};
use iced::{
    mouse, Alignment, Background, Border, Color, Element, Event, Fill, Length, Point, Rectangle,
    Size, Theme, Vector,
};

use crate::labels;
use crate::theme;
use crate::{anchored::Anchored, app_menu};

/// Dados de entrada para renderizar a topbar sem acoplamento ao estado global.
#[derive(Debug, Clone, Copy)]
pub struct TopbarProps<'a> {
    pub workspace_title: &'a str,
    pub rail_open: bool,
    pub files_open: bool,
    pub counts: crate::farm::FarmCounts,
    /// Largura visível da pílula da ilha; `0.0` quando a ilha está oculta/desativada.
    pub island_width: f32,
}

/// Ações dos botões da topbar (mensagens do shell).
pub struct TopbarActions<M> {
    pub on_toggle_rail: M,
    pub on_toggle_files: M,
    pub on_chat: Option<M>,
    pub on_farm: Option<M>,
    pub on_browser: Option<M>,
    /// Abre a aba Harness do workspace; `None` esconde o botão (sem pasta local ou recurso desligado).
    pub on_harness: Option<M>,
    pub on_agent_graph: Option<M>,
    pub on_localsend: Option<M>,
    pub app_menus: Vec<AppMenu<M>>,
    pub app_menu_open: bool,
    pub on_toggle_app_menu: M,
    pub on_close_app_menu: M,
}

pub struct AppMenu<M> {
    pub items: Vec<app_menu::Item<M>>,
}

/// Renderiza a barra superior do shell com a pílula da ilha centralizada.
pub fn view<'a, M: Clone + 'a>(
    props: TopbarProps<'a>,
    island_pill: impl Fn() -> Element<'a, M> + 'a,
    actions: TopbarActions<M>,
) -> Element<'a, M> {
    let mut menu_items = Vec::new();
    for menu in actions.app_menus {
        if !menu_items.is_empty() {
            menu_items.push(app_menu::Item::Divider);
        }
        menu_items.extend(menu.items);
    }
    let menu_button = icon_toggle_button(
        labels::ICON_MORE_VERT,
        actions.app_menu_open,
        actions.on_toggle_app_menu,
    );
    let menu = Anchored::new(
        menu_button,
        actions
            .app_menu_open
            .then(|| app_menu::menu(menu_items, 180.0)),
        actions.on_close_app_menu.clone(),
    )
    .on_escape(actions.on_close_app_menu.clone());
    let menu = tooltip(
        menu,
        crate::pane_chrome::tip_bubble("Menu"),
        tooltip::Position::Bottom,
    );
    let left = row![Space::new().width(12.0), menu]
        .push(tooltip(
            icon_toggle_button(
                labels::ICON_SIDEBAR_LEFT,
                !props.rail_open,
                actions.on_toggle_rail,
            ),
            crate::pane_chrome::tip_bubble(labels::TOPBAR_COLLAPSE_SIDEBAR),
            tooltip::Position::Bottom,
        ))
        .push(Space::new().width(8.0))
        .push(
            text(props.workspace_title)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TOPBAR_TITLE)
                .color(theme::palette().text),
        )
        .align_y(Alignment::Center);

    let mut right = row![];
    if let Some(on_farm) = actions.on_farm.clone() {
        right = right.push(farm_strip(props.counts, on_farm));
    }
    if let Some(on_chat) = actions.on_chat {
        right = right.push(tooltip(
            icon_toggle_button(labels::ICON_CHAT, false, on_chat),
            crate::pane_chrome::tip_bubble(labels::TOPBAR_OPEN_CHAT),
            tooltip::Position::Bottom,
        ));
    }
    if let Some(on_farm) = actions.on_farm {
        right = right.push(tooltip(
            icon_toggle_button(labels::ICON_FARM, false, on_farm),
            crate::pane_chrome::tip_bubble(labels::TOPBAR_OPEN_FARM),
            tooltip::Position::Bottom,
        ));
    }
    if let Some(on_browser) = actions.on_browser {
        right = right.push(tooltip(
            icon_toggle_button(labels::ICON_LANGUAGE, false, on_browser),
            crate::pane_chrome::tip_bubble(labels::SETTINGS_FEATURE_BROWSER),
            tooltip::Position::Bottom,
        ));
    }
    if let Some(on_harness) = actions.on_harness {
        right = right.push(tooltip(
            icon_toggle_button(labels::ICON_TUNE, false, on_harness),
            crate::pane_chrome::tip_bubble(labels::TOPBAR_OPEN_HARNESS),
            tooltip::Position::Bottom,
        ));
    }
    if let Some(on_agent_graph) = actions.on_agent_graph {
        right = right.push(tooltip(
            icon_toggle_button(labels::ICON_HUB_OUTLINED, false, on_agent_graph),
            crate::pane_chrome::tip_bubble(labels::TOPBAR_VISUALIZE_AGENTS),
            tooltip::Position::Bottom,
        ));
    }
    if let Some(on_localsend) = actions.on_localsend {
        right = right.push(tooltip(
            icon_toggle_button(labels::ICON_SEND, false, on_localsend),
            crate::pane_chrome::tip_bubble(labels::TOPBAR_LOCALSEND_SEND),
            tooltip::Position::Bottom,
        ));
    }
    // ponytail: o grafo de agentes entra com o painel dele
    let right = right
        .push(tooltip(
            icon_toggle_button(
                labels::ICON_SIDEBAR_RIGHT,
                !props.files_open,
                actions.on_toggle_files,
            ),
            crate::pane_chrome::tip_bubble(labels::TOPBAR_TOGGLE_FILES),
            tooltip::Position::Bottom,
        ))
        .push(Space::new().width(4.0))
        .spacing(4)
        .align_y(Alignment::Center);

    let bar: Element<'a, M> = TopbarLayout {
        children: [left.into(), right.into(), island_pill()],
        trailing_width: theme::WINDOW_CONTROLS_WIDTH,
        pill_width: props.island_width,
    }
    .into();

    let bottom_hairline = container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(Length::Fixed(1.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        });

    container(stack![
        bar,
        container(bottom_hairline)
            .width(Fill)
            .height(Fill)
            .align_y(Alignment::End),
    ])
    .width(Fill)
    .height(Length::Fixed(theme::TOPBAR_HEIGHT))
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().bg)),
        ..Default::default()
    })
    .into()
}

/// Controles de janela à direita (`WindowControlsTrailing` / `_WinButton` em `window_controls.dart`):
/// 3 botões quadrados 46×46 sem raio (Minimizar, Maximizar, Fechar), ícone 16px `text2`,
/// hover `panel3` e hover vermelho `#E81123` com ícone branco no Fechar.
pub fn window_controls<'a, M: Clone + 'a>(
    on_minimize: M,
    on_toggle_maximize: M,
    on_close: M,
) -> Element<'a, M> {
    let buttons = row![
        window_control_button(
            labels::ICON_WINDOW_MINIMIZE,
            labels::WINDOW_CONTROL_MINIMIZE,
            false,
            on_minimize,
        ),
        window_control_button(
            labels::ICON_WINDOW_MAXIMIZE,
            labels::WINDOW_CONTROL_MAXIMIZE,
            false,
            on_toggle_maximize,
        ),
        window_control_button(
            labels::ICON_CLOSE,
            labels::WINDOW_CONTROL_CLOSE,
            true,
            on_close,
        ),
    ];
    let bottom_hairline = container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(Length::Fixed(1.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        });
    container(stack![
        buttons,
        container(bottom_hairline)
            .width(Fill)
            .height(Fill)
            .align_y(Alignment::End),
    ])
    .width(Length::Fixed(theme::WINDOW_CONTROLS_WIDTH))
    .height(Length::Fixed(theme::TOPBAR_HEIGHT))
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().bg)),
        ..Default::default()
    })
    .into()
}

fn window_control_style(danger: bool, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = if hovered {
        Some(Background::Color(if danger {
            theme::WINDOW_CLOSE_HOVER
        } else {
            theme::palette().panel3
        }))
    } else {
        None
    };
    let text_color = if hovered && danger {
        Color::WHITE
    } else {
        theme::palette().text2
    };
    button::Style {
        background,
        text_color,
        border: Border::default(),
        ..Default::default()
    }
}

fn window_control_button<'a, M: Clone + 'a>(
    glyph: &'static str,
    tip: &'static str,
    danger: bool,
    on_press: M,
) -> Element<'a, M> {
    let btn = button(
        container(
            text(glyph)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_ICON)
                .line_height(text::LineHeight::Relative(1.0)),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(theme::WINDOW_CONTROL_BUTTON_SIZE))
    .height(Length::Fixed(theme::WINDOW_CONTROL_BUTTON_SIZE))
    .padding(0)
    .on_press(on_press)
    .style(move |_: &Theme, status| window_control_style(danger, status));
    tooltip(
        btn,
        crate::pane_chrome::tip_bubble(tip),
        tooltip::Position::Bottom,
    )
    .into()
}

/// Verifica se a pílula da ilha, centrada horizontalmente na janela inteira,
/// cabe entre o grupo da esquerda e o grupo da direita (incluindo os controles
/// da janela) sem invadir nenhum dos dois lados.
fn island_fits(
    window_width: f32,
    left_width: f32,
    right_total_width: f32,
    pill_width: f32,
) -> bool {
    if pill_width <= 0.0 {
        return false;
    }
    let window_center = window_width / 2.0;
    let half_needed = pill_width / 2.0 + theme::TOPBAR_ISLAND_GAP;
    window_center - half_needed >= left_width
        && window_center + half_needed <= window_width - right_total_width
}

/// Layout da topbar com três filhos fixos (`[left, right, center]`):
/// - `left` fica colado à esquerda e centrado verticalmente;
/// - `right` fica colado à direita (antes dos controles de janela) e centrado verticalmente;
/// - `center` (pílula da ilha) é posicionado no centro horizontal da janela inteira
///   (`bar_width + trailing_width`), e só é desenhado/interativo quando [`island_fits`]
///   garante que não invade nem `left` nem `right`.
struct TopbarLayout<'a, Message> {
    children: [Element<'a, Message>; 3],
    trailing_width: f32,
    pill_width: f32,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for TopbarLayout<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let bar_size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let child_limits = layout::Limits::new(Size::ZERO, bar_size);

        let [left, right, center] = &mut self.children;
        let mut trees = tree.children.iter_mut();
        let (Some(left_tree), Some(right_tree), Some(center_tree)) =
            (trees.next(), trees.next(), trees.next())
        else {
            return layout::Node::new(bar_size);
        };

        let mut left_node = left
            .as_widget_mut()
            .layout(left_tree, renderer, &child_limits);
        let left_size = left_node.size();
        left_node.move_to_mut(Point::new(
            0.0,
            ((bar_size.height - left_size.height) / 2.0).max(0.0),
        ));

        let mut right_node = right
            .as_widget_mut()
            .layout(right_tree, renderer, &child_limits);
        let right_size = right_node.size();
        let right_x = (bar_size.width - right_size.width).max(left_size.width);
        right_node.move_to_mut(Point::new(
            right_x,
            ((bar_size.height - right_size.height) / 2.0).max(0.0),
        ));

        let window_width = bar_size.width + self.trailing_width;
        let right_total_width = right_size.width + self.trailing_width;
        let center_node = if island_fits(
            window_width,
            left_size.width,
            right_total_width,
            self.pill_width,
        ) {
            let mut node = center
                .as_widget_mut()
                .layout(center_tree, renderer, &child_limits);
            let center_size = node.size();
            let center_x = window_width / 2.0 - center_size.width / 2.0;
            let center_y = ((bar_size.height - center_size.height) / 2.0).max(0.0);
            node.move_to_mut(Point::new(center_x, center_y));
            node
        } else {
            layout::Node::new(Size::ZERO)
        };

        layout::Node::with_children(bar_size, vec![left_node, right_node, center_node])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((child, state), child_layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                if child_layout.bounds().width > 0.0 {
                    child
                        .as_widget_mut()
                        .operate(state, child_layout, renderer, operation);
                }
            }
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        mut cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let [left, right, center] = &mut self.children;
        let mut trees = tree.children.iter_mut();
        let mut layouts = layout.children();
        let (
            Some(left_tree),
            Some(right_tree),
            Some(center_tree),
            Some(left_layout),
            Some(right_layout),
            Some(center_layout),
        ) = (
            trees.next(),
            trees.next(),
            trees.next(),
            layouts.next(),
            layouts.next(),
            layouts.next(),
        )
        else {
            return;
        };

        if center_layout.bounds().width > 0.0 {
            center.as_widget_mut().update(
                center_tree,
                event,
                center_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            if shell.is_event_captured() {
                return;
            }
            if cursor.is_over(layout.bounds())
                && !cursor.is_levitating()
                && center.as_widget().mouse_interaction(
                    center_tree,
                    center_layout,
                    cursor,
                    viewport,
                    renderer,
                ) != mouse::Interaction::None
            {
                cursor = cursor.levitate();
            }
        }

        left.as_widget_mut().update(
            left_tree,
            event,
            left_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        if shell.is_event_captured() {
            return;
        }

        right.as_widget_mut().update(
            right_tree,
            event,
            right_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let [left, right, center] = &self.children;
        let mut trees = tree.children.iter();
        let mut layouts = layout.children();
        let (
            Some(left_tree),
            Some(right_tree),
            Some(center_tree),
            Some(left_layout),
            Some(right_layout),
            Some(center_layout),
        ) = (
            trees.next(),
            trees.next(),
            trees.next(),
            layouts.next(),
            layouts.next(),
            layouts.next(),
        )
        else {
            return mouse::Interaction::None;
        };

        if center_layout.bounds().width > 0.0 {
            let interaction = center.as_widget().mouse_interaction(
                center_tree,
                center_layout,
                cursor,
                viewport,
                renderer,
            );
            if interaction != mouse::Interaction::None {
                return interaction;
            }
        }

        let left_interaction =
            left.as_widget()
                .mouse_interaction(left_tree, left_layout, cursor, viewport, renderer);
        if left_interaction != mouse::Interaction::None {
            return left_interaction;
        }

        right
            .as_widget()
            .mouse_interaction(right_tree, right_layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let [left, right, center] = &self.children;
        let mut trees = tree.children.iter();
        let mut layouts = layout.children();
        let (
            Some(left_tree),
            Some(right_tree),
            Some(center_tree),
            Some(left_layout),
            Some(right_layout),
            Some(center_layout),
        ) = (
            trees.next(),
            trees.next(),
            trees.next(),
            layouts.next(),
            layouts.next(),
            layouts.next(),
        )
        else {
            return;
        };

        left.as_widget().draw(
            left_tree,
            renderer,
            theme,
            style,
            left_layout,
            cursor,
            viewport,
        );
        right.as_widget().draw(
            right_tree,
            renderer,
            theme,
            style,
            right_layout,
            cursor,
            viewport,
        );
        if center_layout.bounds().width > 0.0 {
            center.as_widget().draw(
                center_tree,
                renderer,
                theme,
                style,
                center_layout,
                cursor,
                viewport,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<TopbarLayout<'a, Message>> for Element<'a, Message> {
    fn from(layout: TopbarLayout<'a, Message>) -> Self {
        Self::new(layout)
    }
}

fn icon_toggle_button<'a, M: Clone + 'a>(
    glyph: &'static str,
    selected: bool,
    on_press: M,
) -> Element<'a, M> {
    button(
        container(
            text(glyph)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_TOPBAR_ICON)
                .line_height(text::LineHeight::Relative(1.0))
                .color(if selected {
                    theme::palette().accent_text
                } else {
                    theme::palette().text3
                }),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(theme::TOPBAR_ICON_BUTTON_SIZE))
    .height(Length::Fixed(theme::TOPBAR_ICON_BUTTON_SIZE))
    .padding(0)
    .on_press(on_press)
    .style(move |_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let bg = if selected {
            Some(Background::Color(theme::palette().accent_soft))
        } else if hovered {
            Some(Background::Color(theme::palette().panel3))
        } else {
            None
        };
        button::Style {
            background: bg,
            text_color: if selected {
                theme::palette().accent_text
            } else if hovered {
                theme::palette().text2
            } else {
                theme::palette().text3
            },
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}

fn counter<'a, M: 'a>(count: usize, color: iced::Color) -> Element<'a, M> {
    row![
        counter_dot(color),
        text(count.to_string())
            .font(theme::FONT_MONO)
            .size(theme::SIZE_LABEL)
            .color(if count == 0 {
                theme::palette().text4
            } else {
                theme::palette().text2
            })
    ]
    .spacing(theme::FARM_STRIP_DOT_GAP)
    .align_y(Alignment::Center)
    .into()
}

/// Contadores sempre visíveis, mesmo zerados: o Flutter só os esconde sem loops, mas
/// aqui os loops vêm só dos terminais observados por hook, e esconder deixaria a barra vazia.
fn farm_strip<'a, M: Clone + 'a>(counts: crate::farm::FarmCounts, on_open: M) -> Element<'a, M> {
    let waiting = counts.waiting;
    let counters = button(
        container(
            row![
                counter(counts.working, theme::palette().online),
                counter(waiting, theme::palette().warn),
                counter(counts.idle, theme::palette().git_untracked),
            ]
            .spacing(theme::FARM_STRIP_COUNTER_GAP)
            .align_y(Alignment::Center),
        )
        .height(Fill)
        .center_y(Fill),
    )
    .height(theme::TOPBAR_ICON_BUTTON_SIZE)
    .padding([0.0, theme::FARM_STRIP_BUTTON_PADDING])
    .on_press(on_open.clone())
    .style(|_: &Theme, status| button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then_some(Background::Color(theme::palette().panel3)),
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let summary = labels::format_farm_summary(counts.working, waiting, counts.idle);
    let tooltip = tooltip(
        counters,
        text(summary).size(theme::SIZE_LABEL),
        tooltip::Position::Bottom,
    );
    let mut strip = row![tooltip].spacing(4).align_y(Alignment::Center);
    if waiting > 0 {
        let pill = button(
            container(
                row![
                    counter_dot(theme::palette().warn),
                    text(labels::farm_need_you(waiting))
                        .font(theme::FONT_BODY_SEMIBOLD)
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().warn),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .height(Fill)
            .center_y(Fill),
        )
        .height(theme::FARM_STRIP_PILL_HEIGHT)
        .padding([0.0, theme::FARM_STRIP_PILL_PADDING])
        .on_press(on_open)
        .style(|_: &Theme, status| {
            let alpha = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                0.24
            } else {
                0.14
            };
            button::Style {
                background: Some(Background::Color(iced::Color {
                    a: alpha,
                    ..theme::palette().warn
                })),
                border: Border {
                    radius: theme::FARM_STRIP_PILL_RADIUS.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });
        strip = strip.push(pill);
    }
    // Flutter: 8px depois do strip; o `spacing(4)` da barra já cobre metade.
    container(strip)
        .padding(iced::Padding {
            right: theme::FARM_STRIP_TRAILING_GAP,
            ..iced::Padding::ZERO
        })
        .into()
}

fn counter_dot<'a, M: 'a>(color: iced::Color) -> Element<'a, M> {
    container(Space::new())
        .width(theme::FARM_STRIP_DOT_SIZE)
        .height(theme::FARM_STRIP_DOT_SIZE)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                radius: (theme::FARM_STRIP_DOT_SIZE / 2.0).into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn island_fits_checks_both_groups_around_window_center() {
        let gap = theme::TOPBAR_ISLAND_GAP;
        let pill = 50.0;
        let half = pill / 2.0 + gap; // 31.0

        // Janela 1025 (centro 512.5), esquerda 109.5, direita total 466.0 (início em 559.0): cabe com folga.
        assert!(super::island_fits(1025.0, 109.5, 466.0, pill));
        // Limite exato à direita: 512.5 + half == 1025.0 - right_total
        assert!(super::island_fits(1025.0, 109.5, 512.5 - half, pill));
        assert!(!super::island_fits(
            1025.0,
            109.5,
            512.5 - half + 0.01,
            pill
        ));
        // Limite exato à esquerda: 512.5 - half == left
        assert!(super::island_fits(1025.0, 512.5 - half, 466.0, pill));
        assert!(!super::island_fits(
            1025.0,
            512.5 - half + 0.01,
            466.0,
            pill
        ));
        // Largura estreita (700) onde o grupo direito cruza o centro: não cabe.
        assert!(!super::island_fits(700.0, 109.5, 466.0, pill));
        // Pílula vazia/desativada: não cabe.
        assert!(!super::island_fits(1920.0, 109.5, 466.0, 0.0));
        // Pílula larga (3 contadores = 129.5) em 1025 com navegador (466): esconde para não cobrir.
        assert!(!super::island_fits(1025.0, 109.5, 466.0, 129.5));
        assert!(super::island_fits(1400.0, 109.5, 466.0, 129.5));
    }

    #[test]
    fn window_close_button_hover_uses_flutter_red_and_white_icon() {
        theme::set_active(theme::ThemeVariant::Dark);
        let close_hover = window_control_style(true, button::Status::Hovered);
        assert_eq!(
            close_hover.background,
            Some(Background::Color(theme::WINDOW_CLOSE_HOVER))
        );
        assert_eq!(close_hover.text_color, Color::WHITE);
        assert_eq!(close_hover.border.radius, 0.0.into());

        let normal_hover = window_control_style(false, button::Status::Hovered);
        assert_eq!(
            normal_hover.background,
            Some(Background::Color(theme::palette().panel3))
        );
        assert_eq!(normal_hover.text_color, theme::palette().text2);
    }

    #[test]
    fn chat_button_precedes_farm_button_and_window_controls_fit_flutter_metrics() {
        assert_eq!(theme::WINDOW_CONTROL_BUTTON_SIZE, 46.0);
        assert_eq!(theme::WINDOW_CONTROLS_WIDTH, 138.0);

        let bar = view(
            TopbarProps {
                workspace_title: labels::WORKSPACE_GSH,
                rail_open: true,
                files_open: true,
                counts: crate::farm::FarmCounts::default(),
                island_width: 0.0,
            },
            || Space::new().into(),
            TopbarActions {
                on_toggle_rail: (),
                on_toggle_files: (),
                on_chat: Some(()),
                on_farm: Some(()),
                on_browser: None,
                on_harness: Some(()),
                on_agent_graph: Some(()),
                on_localsend: None,
                app_menus: Vec::new(),
                app_menu_open: false,
                on_toggle_app_menu: (),
                on_close_app_menu: (),
            },
        );
        let mut ui = iced_test::simulator(bar);
        let chat_bounds = ui
            .find(labels::ICON_CHAT)
            .expect("chat icon present")
            .visible_bounds()
            .expect("chat bounds");
        let farm_bounds = ui
            .find(labels::ICON_FARM)
            .expect("farm icon present")
            .visible_bounds()
            .expect("farm bounds");
        assert!(
            chat_bounds.x < farm_bounds.x,
            "Chat ({}) deve vir antes de Fazenda ({})",
            chat_bounds.x,
            farm_bounds.x
        );
    }

    #[test]
    fn topbar_vertical_centering() {
        use std::path::Path;

        use iced::{Settings, Size};
        use iced_test::simulator::Simulator;

        theme::set_active(theme::ThemeVariant::Dark);

        let island = crate::island::AgentIsland::new(vec![crate::island::IslandAgent::new(
            "t1",
            "claude",
            "style",
            crate::island::IslandAgentState::Working,
        )]);

        let bar = view(
            TopbarProps {
                workspace_title: "style",
                rail_open: true,
                files_open: true,
                counts: crate::farm::FarmCounts {
                    working: 0,
                    waiting: 0,
                    idle: 0,
                },
                island_width: island.pill_width(),
            },
            || island.view_pill().map(|_| ()),
            TopbarActions {
                on_toggle_rail: (),
                on_toggle_files: (),
                on_chat: Some(()),
                on_farm: Some(()),
                on_browser: None,
                on_harness: Some(()),
                on_agent_graph: Some(()),
                on_localsend: Some(()),
                app_menus: Vec::new(),
                app_menu_open: false,
                on_toggle_app_menu: (),
                on_close_app_menu: (),
            },
        );
        let full_topbar: Element<'_, ()> =
            row![container(bar).width(Fill), window_controls((), (), ()),]
                .align_y(Alignment::Center)
                .into();

        std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
        let settings = Settings {
            fonts: theme::EMBEDDED_FONTS
                .iter()
                .map(|font| (*font).into())
                .collect(),
            default_font: theme::FONT_BODY,
            ..Default::default()
        };
        let mut simulator = Simulator::with_size(settings, Size::new(1280.0, 120.0), full_topbar);
        let target = Path::new("/home/heliodev/projetos/GSH-wt/shots/gsh-topbar-centro.png");
        let generated = target.with_file_name("gsh-topbar-centro-tiny-skia.png");
        std::fs::create_dir_all(target.parent().expect("shots dir")).expect("create shots dir");
        let _ = std::fs::remove_file(target);
        let _ = std::fs::remove_file(&generated);
        simulator
            .snapshot(&iced::Theme::Dark)
            .expect("snapshot render")
            .matches_image(target)
            .expect("snapshot write");
        std::fs::rename(&generated, target).expect("snapshot rename");

        let img = image::open(target).expect("open topbar png").to_rgba8();
        let viewport_w = 1280.0_f32;
        let scale = img.width() as f32 / viewport_w;
        let [bg_r, bg_g, bg_b, _] = img.get_pixel(0, 0).0;
        let bar_h_px = (theme::TOPBAR_HEIGHT * scale).round() as u32;
        let hairline_px = scale.round().max(1.0) as u32;
        let scan_h = bar_h_px - hairline_px; // exclui a hairline inferior de 1px lógico

        let differs = |p: &image::Rgba<u8>| -> bool {
            let [r, g, b, _] = p.0;
            let dr = r.abs_diff(bg_r);
            let dg = g.abs_diff(bg_g);
            let db = b.abs_diff(bg_b);
            dr.max(dg).max(db) > 4
        };

        let active_cols: Vec<bool> = (0..img.width())
            .map(|x| (0..scan_h).any(|y| differs(img.get_pixel(x, y))))
            .collect();
        let is_active = |x: u32| -> bool { active_cols.get(x as usize).copied().unwrap_or(false) };

        let group_spans = |max_gap: u32| -> Vec<(u32, u32)> {
            let mut spans: Vec<(u32, u32)> = Vec::new();
            let mut x = 0;
            while x < img.width() {
                if !is_active(x) {
                    x += 1;
                    continue;
                }
                let start = x;
                let mut end = x;
                while x < img.width() {
                    if is_active(x) {
                        end = x;
                        x += 1;
                    } else {
                        let gap_start = x;
                        while x < img.width() && !is_active(x) {
                            x += 1;
                        }
                        if x < img.width() && x - gap_start <= max_gap {
                            continue;
                        }
                        break;
                    }
                }
                spans.push((start, end));
            }
            spans
        };

        // Gap de 7px lógicos: agrupa letras de "style", bolinha+dígito de cada contador (gap=5px)
        // e mantém separados os 3 contadores (gap=10px) e cada ícone/botão.
        let spans = group_spans((7.0 * scale).round() as u32);
        let target_center = theme::TOPBAR_HEIGHT / 2.0;
        let expected_names = [
            "menu (⋮)",
            "rail",
            "título (style)",
            "ilha (pílula)",
            "contador trabalhando",
            "contador esperando",
            "contador parado",
            "chat",
            "fazenda",
            "harness",
            "grafo",
            "localsend",
            "arquivos",
            "minimizar",
            "maximizar",
            "fechar",
        ];

        eprintln!(
            "=== Medição vertical da topbar (img={}x{}, scale={scale:.1}, bar_h={}, target_center={target_center:.2}, spans={}) ===",
            img.width(),
            img.height(),
            theme::TOPBAR_HEIGHT,
            spans.len()
        );
        let mut failures = Vec::new();
        for (idx, &(x0, x1)) in spans.iter().enumerate() {
            let mut first_y = None;
            let mut last_y = None;
            for y in 0..scan_h {
                if (x0..=x1).any(|x| differs(img.get_pixel(x, y))) {
                    if first_y.is_none() {
                        first_y = Some(y);
                    }
                    last_y = Some(y);
                }
            }
            let fy = first_y.expect("span must have active pixels");
            let ly = last_y.expect("span must have active pixels");
            let fy_log = fy as f32 / scale;
            let ly_log = (ly + 1) as f32 / scale;
            let center_y = (fy_log + ly_log) / 2.0;
            let delta = center_y - target_center;
            let name = expected_names.get(idx).copied().unwrap_or("desconhecido");
            eprintln!(
                "  [{idx:02}] {name:<22} x={:6.1}..{:6.1}  y={fy_log:5.2}..{ly_log:5.2} (h={:5.2})  center={center_y:5.2}  delta={delta:+5.2}px",
                x0 as f32 / scale,
                (x1 + 1) as f32 / scale,
                ly_log - fy_log,
            );
            if delta.abs() > 1.0 {
                failures.push(format!(
                    "{name}: y={fy_log:.2}..{ly_log:.2}, center={center_y:.2}, target={target_center:.2}, delta={delta:+.2}px"
                ));
            }
        }

        assert_eq!(
            spans.len(),
            expected_names.len(),
            "esperava {} elementos na topbar, encontrou {}",
            expected_names.len(),
            spans.len()
        );
        assert!(
            failures.is_empty(),
            "elementos fora do eixo vertical central (tolerância 1.0px):\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn topbar_island_horizontal_window_centering() {
        use std::path::Path;

        use iced::{Settings, Size};
        use iced_test::simulator::Simulator;

        theme::set_active(theme::ThemeVariant::Dark);
        std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");

        let measure = |window_w: f32, file_name: &str| -> (Vec<(f32, f32)>, f32) {
            let island = crate::island::AgentIsland::new(Vec::new());
            let bar = view(
                TopbarProps {
                    workspace_title: "style",
                    rail_open: true,
                    files_open: true,
                    counts: crate::farm::FarmCounts {
                        working: 0,
                        waiting: 0,
                        idle: 0,
                    },
                    island_width: island.pill_width(),
                },
                || island.view_pill().map(|_| ()),
                TopbarActions {
                    on_toggle_rail: (),
                    on_toggle_files: (),
                    on_chat: Some(()),
                    on_farm: Some(()),
                    on_browser: Some(()),
                    on_harness: Some(()),
                    on_agent_graph: Some(()),
                    on_localsend: Some(()),
                    app_menus: Vec::new(),
                    app_menu_open: false,
                    on_toggle_app_menu: (),
                    on_close_app_menu: (),
                },
            );
            let full_topbar: Element<'_, ()> =
                row![container(bar).width(Fill), window_controls((), (), ()),]
                    .align_y(Alignment::Center)
                    .into();

            let settings = Settings {
                fonts: theme::EMBEDDED_FONTS
                    .iter()
                    .map(|font| (*font).into())
                    .collect(),
                default_font: theme::FONT_BODY,
                ..Default::default()
            };
            let mut simulator =
                Simulator::with_size(settings, Size::new(window_w, 120.0), full_topbar);
            let target = Path::new("/home/heliodev/projetos/GSH-wt/shots").join(file_name);
            let stem = target
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("file stem");
            let generated = target.with_file_name(format!("{stem}-tiny-skia.png"));
            std::fs::create_dir_all(target.parent().expect("shots dir")).expect("create shots dir");
            let _ = std::fs::remove_file(&target);
            let _ = std::fs::remove_file(&generated);
            simulator
                .snapshot(&iced::Theme::Dark)
                .expect("snapshot render")
                .matches_image(&target)
                .expect("snapshot write");
            std::fs::rename(&generated, &target).expect("snapshot rename");

            let img = image::open(&target).expect("open topbar png").to_rgba8();
            let scale = img.width() as f32 / window_w;
            let [bg_r, bg_g, bg_b, _] = img.get_pixel(0, 0).0;
            let bar_h_px = (theme::TOPBAR_HEIGHT * scale).round() as u32;
            let hairline_px = scale.round().max(1.0) as u32;
            let scan_h = bar_h_px - hairline_px;

            let differs = |p: &image::Rgba<u8>| -> bool {
                let [r, g, b, _] = p.0;
                r.abs_diff(bg_r).max(g.abs_diff(bg_g)).max(b.abs_diff(bg_b)) > 4
            };
            let active_cols: Vec<bool> = (0..img.width())
                .map(|x| (0..scan_h).any(|y| differs(img.get_pixel(x, y))))
                .collect();
            let is_active =
                |x: u32| -> bool { active_cols.get(x as usize).copied().unwrap_or(false) };
            let max_gap = (7.0 * scale).round() as u32;

            let mut spans = Vec::new();
            let mut x = 0;
            while x < img.width() {
                if !is_active(x) {
                    x += 1;
                    continue;
                }
                let start = x;
                let mut end = x;
                while x < img.width() {
                    if is_active(x) {
                        end = x;
                        x += 1;
                    } else {
                        let gap_start = x;
                        while x < img.width() && !is_active(x) {
                            x += 1;
                        }
                        if x < img.width() && x - gap_start <= max_gap {
                            continue;
                        }
                        break;
                    }
                }
                spans.push((start as f32 / scale, (end + 1) as f32 / scale));
            }
            (spans, scale)
        };

        let mut failures = Vec::new();
        for (window_w, file_name) in [
            (1025.0_f32, "gsh-topbar-janela-1025.png"),
            (1400.0_f32, "gsh-topbar-janela-1400.png"),
            (1920.0_f32, "gsh-topbar-janela-1920.png"),
        ] {
            let (spans, _scale) = measure(window_w, file_name);
            let window_center = window_w / 2.0;
            // 0: menu, 1: rail, 2: título, 3: ilha, 4..=16: contadores + 7 ícones + 3 controles janela
            assert_eq!(
                spans.len(),
                17,
                "largura {window_w}: esperava 17 spans (com ilha visível), encontrou {}: {:?}",
                spans.len(),
                spans
            );
            let left_end = spans.get(2).expect("title span").1;
            let (pill_left, pill_right) = *spans.get(3).expect("island pill span");
            let right_start = spans.get(4).expect("right group span").0;
            let pill_center = (pill_left + pill_right) / 2.0;
            let delta = pill_center - window_center;
            eprintln!(
                "  W={window_w:6.1} | left_end={left_end:5.1} | pill={pill_left:6.1}..{pill_right:6.1} (center={pill_center:6.2}, target={window_center:6.2}, delta={delta:+6.2}px) | right_start={right_start:6.1}"
            );
            if delta.abs() > 1.0 {
                failures.push(format!(
                    "W={window_w}: centro da pílula={pill_center:.2} vs centro da janela={window_center:.2} (delta={delta:+.2}px)"
                ));
            }
            assert!(
                pill_left > left_end,
                "W={window_w}: pílula ({pill_left}) invadiu grupo esquerdo ({left_end})"
            );
            assert!(
                pill_right < right_start,
                "W={window_w}: pílula ({pill_right}) invadiu grupo direito ({right_start})"
            );
        }

        // Largura estreita: ilha deve sumir sem sobrepor nada.
        let (narrow_spans, _scale) = measure(700.0, "gsh-topbar-janela-700.png");
        let narrow_left_end = narrow_spans.get(2).expect("narrow title span").1;
        let narrow_right_start = narrow_spans.get(3).expect("narrow right group span").0;
        eprintln!(
            "  W= 700.0 | spans={} (esperado 16, ilha oculta) | left_end={narrow_left_end:5.1} | right_start={narrow_right_start:5.1}",
            narrow_spans.len(),
        );
        assert_eq!(
            narrow_spans.len(),
            16,
            "W=700: ilha deveria estar oculta (16 spans), encontrou {}: {:?}",
            narrow_spans.len(),
            narrow_spans
        );
        assert!(
            narrow_left_end < narrow_right_start,
            "W=700: grupo esquerdo ({narrow_left_end}) sobrepôs grupo direito ({narrow_right_start})"
        );

        assert!(
            failures.is_empty(),
            "pílula fora do centro da janela (tolerância 1.0px):\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn topbar_window_drag_and_island_click_through() {
        use iced::widget::mouse_area;
        use iced::{Settings, Size};
        use iced_test::simulator::Simulator;

        #[derive(Debug, Clone, PartialEq, Eq)]
        enum TestMsg {
            Drag,
            ToggleMaximize,
            Minimize,
            Close,
            ToggleRail,
            ToggleFiles,
            Chat,
            Farm,
            Browser,
            Harness,
            Graph,
            LocalSend,
            ToggleMenu,
            CloseMenu,
            Island(crate::island::IslandMsg),
        }

        theme::set_active(theme::ThemeVariant::Dark);
        let island = crate::island::AgentIsland::new(Vec::new());
        let build_bar = || -> Element<'_, TestMsg> {
            let bar = view(
                TopbarProps {
                    workspace_title: "style",
                    rail_open: true,
                    files_open: true,
                    counts: crate::farm::FarmCounts::default(),
                    island_width: island.pill_width(),
                },
                || island.view_pill().map(TestMsg::Island),
                TopbarActions {
                    on_toggle_rail: TestMsg::ToggleRail,
                    on_toggle_files: TestMsg::ToggleFiles,
                    on_chat: Some(TestMsg::Chat),
                    on_farm: Some(TestMsg::Farm),
                    on_browser: Some(TestMsg::Browser),
                    on_harness: Some(TestMsg::Harness),
                    on_agent_graph: Some(TestMsg::Graph),
                    on_localsend: Some(TestMsg::LocalSend),
                    app_menus: Vec::new(),
                    app_menu_open: false,
                    on_toggle_app_menu: TestMsg::ToggleMenu,
                    on_close_app_menu: TestMsg::CloseMenu,
                },
            );
            row![
                mouse_area(container(bar).width(Fill))
                    .on_press(TestMsg::Drag)
                    .on_double_click(TestMsg::ToggleMaximize),
                window_controls(TestMsg::Minimize, TestMsg::ToggleMaximize, TestMsg::Close),
            ]
            .align_y(Alignment::Center)
            .into()
        };

        let settings = Settings {
            fonts: theme::EMBEDDED_FONTS
                .iter()
                .map(|font| (*font).into())
                .collect(),
            default_font: theme::FONT_BODY,
            ..Default::default()
        };

        // 1. Clique em área vazia da barra inicia arraste da janela.
        let mut ui = Simulator::with_size(settings.clone(), Size::new(1025.0, 120.0), build_bar());
        ui.point_at(Point::new(300.0, 23.0));
        let _ = ui.simulate(iced_test::simulator::click());
        let msgs: Vec<TestMsg> = ui.into_messages().collect();
        assert_eq!(msgs, vec![TestMsg::Drag]);

        // 2. Clique no centro da janela (512.5, 23.0) acerta a pílula e não arrasta a janela.
        let mut ui = Simulator::with_size(settings.clone(), Size::new(1025.0, 120.0), build_bar());
        ui.point_at(Point::new(512.5, 23.0));
        let _ = ui.simulate(iced_test::simulator::click());
        let msgs: Vec<TestMsg> = ui.into_messages().collect();
        assert_eq!(
            msgs,
            vec![TestMsg::Island(crate::island::IslandMsg::TogglePanel)]
        );

        // 3. Clique no farm_strip (x=575, dentro da margem transparente do canvas da pílula)
        // atravessa até o botão da fazenda sem abrir a ilha nem arrastar a janela.
        let mut ui = Simulator::with_size(settings.clone(), Size::new(1025.0, 120.0), build_bar());
        ui.point_at(Point::new(575.0, 23.0));
        let _ = ui.simulate(iced_test::simulator::click());
        let msgs: Vec<TestMsg> = ui.into_messages().collect();
        assert_eq!(msgs, vec![TestMsg::Farm]);

        // 4. Clique no botão do rail à esquerda aciona ToggleRail sem arrastar a janela.
        let mut ui = Simulator::with_size(settings, Size::new(1025.0, 120.0), build_bar());
        ui.point_at(Point::new(54.0, 23.0));
        let _ = ui.simulate(iced_test::simulator::click());
        let msgs: Vec<TestMsg> = ui.into_messages().collect();
        assert_eq!(msgs, vec![TestMsg::ToggleRail]);
    }
}
