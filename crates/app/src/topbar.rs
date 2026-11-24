//! Barra superior do shell (`gsh_topbar.dart` / `WindowTitleBar` do Flutter).
//!
//! Altura 46px, superfície `BG`, linha divisória inferior de 1px `BORDER`,
//! botão de recolher o rail à esquerda + título do workspace ativo, pílula da
//! `AgentIsland` centralizada e botão do painel de arquivos à direita.

use iced::widget::{button, container, responsive, row, stack, text, tooltip, Space};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Theme};

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

    let center_island = responsive(move |size| {
        if island_fits(size.width) {
            container(island_pill())
                .center_x(Fill)
                .center_y(Fill)
                .into()
        } else {
            Space::new().into()
        }
    });

    let bar = container(
        row![left, center_island, right]
            .width(Fill)
            .height(Fill)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Fill);

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

fn island_fits(width: f32) -> bool {
    width >= theme::TOPBAR_CENTER_MIN_WIDTH
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
    fn island_requires_160_pixels_of_free_width() {
        assert!(!super::island_fits(159.99));
        assert!(super::island_fits(160.0));
        assert!(super::island_fits(161.0));
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
}
