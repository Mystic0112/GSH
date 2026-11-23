use super::{
    active_tab_scroll_target, has_tab_overflow, reveal_active_tab_operation, wheel_delta,
    StripInput, TabDragEvent,
};
use crate::theme;

#[test]
fn active_tab_target_moves_tabs_before_and_after_viewport() {
    let width = theme::TAB_WIDTH;
    assert_eq!(
        active_tab_scroll_target(0, width, width * 2.0, width * 5.0),
        Some(0.0)
    );
    assert_eq!(
        active_tab_scroll_target(5, 0.0, width * 2.0, width * 5.0),
        Some(width * 4.0)
    );
}

#[test]
fn active_tab_target_keeps_visible_tab_in_place() {
    let width = theme::TAB_WIDTH;
    assert_eq!(
        active_tab_scroll_target(2, width, width * 2.0, width * 5.0),
        None
    );
}

#[test]
fn wheel_delta_prefers_horizontal_axis_then_vertical() {
    assert_eq!(wheel_delta(12.0, 34.0), 12.0);
    assert_eq!(wheel_delta(0.0, 34.0), 34.0);
    assert_eq!(wheel_delta(0.0, 0.0), 0.0);
}

#[test]
fn overflow_button_tracks_real_width_and_exact_boundary() {
    let three_tabs_width = theme::TAB_WIDTH * 3.0;
    assert!(!has_tab_overflow(3, three_tabs_width + 28.0));
    assert!(has_tab_overflow(3, three_tabs_width + 27.0));
    assert!(!has_tab_overflow(3, three_tabs_width + 28.001));
}

#[test]
#[ignore = "captures active-first and active-last pane strips for visual review"]
fn capture_pane_tab_strip_states() {
    use crate::pane_chrome::{self, ChromeCallbacks};
    use crate::pane_model::{PaneModel, PaneTab};
    use iced::advanced::renderer::{Headless, Style};
    use iced::widget::{container, pane_grid, row, Space};
    use iced::{Element, Length, Size, Theme};
    use iced_runtime::user_interface::{Cache, UserInterface};
    use iced_test::renderer::Renderer;
    use std::path::Path;

    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    theme::set_active(theme::ThemeVariant::Dark);
    let tabs = vec![
        PaneTab::Terminal(1),
        PaneTab::Chat(1),
        PaneTab::Farm,
        PaneTab::File("/tmp/fazenda.rs".into()),
        PaneTab::Settings,
        PaneTab::TelemetryCase {
            fingerprint: "case-6".into(),
            title: "Caso 6".into(),
        },
    ];
    let first = PaneModel::restore(tabs.clone(), Some(0), Vec::new());
    let last = PaneModel::restore(tabs, Some(5), Vec::new());
    let (mut grid, first_pane) = pane_grid::State::new(first);
    let (last_pane, _) = grid
        .split(pane_grid::Axis::Vertical, first_pane, last)
        .expect("split test panes");
    let first_model = grid.get(first_pane).expect("first model");
    let last_model = grid.get(last_pane).expect("last model");
    let dirty = |_: &PaneTab| false;
    let view: Element<'_, (), Theme, Renderer> = row![
        container(pane_chrome::view(
            first_model,
            Space::new().into(),
            ChromeCallbacks {
                on_select_tab: |_: usize| (),
                on_close_tab: |_: usize| (),
                on_tab_drag: |_: usize, _: TabDragEvent| (),
                drop_target: None,
                drag_source: None,
                on_pane_drag: |_: pane_chrome::PaneDragEvent| (),
                pane_drop_zone: None,
                terminal_icon: |_: u64| None,
                terminal_title: |_: u64| None,
                on_new_terminal: (),
                noop_msg: (),
                on_split_right: (),
                on_split_down: (),
                on_close_pane: (),
                show_tooltips: false,
                on_focus: None,
            },
            &dirty,
            None,
            |_| (),
            StripInput {
                pane: first_pane,
                focused: true,
                on_wheel: |_| (),
                menu_open: false,
                on_open_menu: (),
                on_close_menu: (),
                on_tab_menu: std::rc::Rc::new(|_| ()),
                tab_menu: None,
                rename: None,
            },
        ))
        .width(Length::Fixed(1360.0)),
        container(pane_chrome::view(
            last_model,
            Space::new().into(),
            ChromeCallbacks {
                on_select_tab: |_: usize| (),
                on_close_tab: |_: usize| (),
                on_tab_drag: |_: usize, _: TabDragEvent| (),
                drop_target: None,
                drag_source: None,
                on_pane_drag: |_: pane_chrome::PaneDragEvent| (),
                pane_drop_zone: None,
                terminal_icon: |_: u64| None,
                terminal_title: |_: u64| None,
                on_new_terminal: (),
                noop_msg: (),
                on_split_right: (),
                on_split_down: (),
                on_close_pane: (),
                show_tooltips: false,
                on_focus: None,
            },
            &dirty,
            None,
            |_| (),
            StripInput {
                pane: last_pane,
                focused: false,
                on_wheel: |_| (),
                menu_open: true,
                on_open_menu: (),
                on_close_menu: (),
                on_tab_menu: std::rc::Rc::new(|_| ()),
                tab_menu: None,
                rename: None,
            },
        ))
        .width(Length::Fixed(640.0)),
    ]
    .width(Length::Fixed(2000.0))
    .height(Length::Fixed(480.0))
    .into();
    let mut renderer = iced_runtime::futures::futures::executor::block_on(Renderer::new(
        theme::FONT_BODY,
        theme::SIZE_BODY.into(),
        Some("tiny-skia"),
    ))
    .expect("create tiny-skia renderer");
    let mut ui = UserInterface::build(
        view,
        Size::new(2000.0, 480.0),
        Cache::default(),
        &mut renderer,
    );
    let mut reveal = reveal_active_tab_operation::<()>(last_pane, 5);
    ui.operate(&renderer, &mut reveal);
    ui.draw(
        &mut renderer,
        &Theme::Dark,
        &Style {
            text_color: theme::palette().text,
        },
        iced::mouse::Cursor::Unavailable,
    );
    let output = Headless::screenshot(
        &mut renderer,
        Size::new(4000, 960),
        2.0,
        theme::palette().bg,
    );
    image::save_buffer(
        Path::new("/home/heliodev/projetos/GSH-wt/shots/rust-tabstrip-fixed2.png"),
        &output,
        4000,
        960,
        image::ExtendedColorType::Rgba8,
    )
    .expect("write pane strip snapshot");
}
