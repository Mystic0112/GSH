use iced::{Element, Point, Size};
use iced_test::simulator::Simulator;

use crate::{agent_graph, theme};

#[test]
fn clicking_new_agent_in_empty_graph_publishes_create() {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = iced::Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    let view: Element<'_, agent_graph::Message> =
        agent_graph::view(Vec::new(), Vec::new(), Vec::new(), None, None);
    let mut ui = Simulator::with_size(settings, Size::new(800.0, 600.0), view);
    assert!(ui.click(crate::labels::AGENT_GRAPH_NEW_AGENT).is_ok());
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages.contains(&agent_graph::Message::Create),
        "{messages:?}"
    );
}

#[test]
fn creating_agent_from_dialog_publishes_create() {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = iced::Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    let directory = tempfile::tempdir().expect("tempdir");
    let mut draft = crate::agent_graph_dialog::Draft::new(directory.path(), Vec::new());
    draft.name = "Revisor".into();
    let dialog: Element<'_, crate::agent_graph_dialog::Message> =
        crate::agent_graph_dialog::view(&draft, &[], None);
    let mut ui = Simulator::with_size(settings, Size::new(800.0, 700.0), dialog);
    assert!(ui.click("Criar").is_ok());
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages.contains(&crate::agent_graph_dialog::Message::Create),
        "{messages:?}"
    );
}

#[test]
fn dragging_from_agent_port_to_another_agent_connects_them() {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = iced::Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    let nodes = vec![
        agent_graph::Node {
            id: "t1".into(),
            label: "Um".into(),
            detail: "projeto".into(),
            status: "Ocioso".into(),
            status_pulsing: false,
            color: iced::Color::from_rgb8(1, 2, 3),
            maestro: false,
            queued: 0,
            harness_icon: None,
            harness_icon_monochrome: false,
        },
        agent_graph::Node {
            id: "t2".into(),
            label: "Dois".into(),
            detail: "projeto".into(),
            status: "Ocioso".into(),
            status_pulsing: false,
            color: iced::Color::from_rgb8(4, 5, 6),
            maestro: false,
            queued: 0,
            harness_icon: None,
            harness_icon_monochrome: false,
        },
    ];
    let view = agent_graph::view(
        nodes,
        vec![("t1".into(), 80.0, 80.0), ("t2".into(), 400.0, 80.0)],
        Vec::new(),
        None,
        None,
    );
    let mut ui = Simulator::with_size(settings, Size::new(800.0, 500.0), view);
    ui.point_at(Point::new(300.0, 124.0));
    let _ = ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
        iced::mouse::Button::Left,
    ))]);
    ui.point_at(Point::new(420.0, 124.0));
    let _ = ui.simulate([
        iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: Point::new(420.0, 124.0),
        }),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
    ]);
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages.contains(&agent_graph::Message::Connect("t1".into(), "t2".into())),
        "{messages:?}"
    );
}

#[test]
fn dragging_agent_node_moves_it_without_selecting_away_from_the_graph() {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = iced::Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    let nodes = vec![agent_graph::Node {
        id: "t1".into(),
        label: "Um".into(),
        detail: "projeto".into(),
        status: "Ocioso".into(),
        status_pulsing: false,
        color: iced::Color::from_rgb8(1, 2, 3),
        maestro: false,
        queued: 0,
        harness_icon: None,
        harness_icon_monochrome: false,
    }];
    let view = agent_graph::view(
        nodes,
        vec![("t1".into(), 80.0, 80.0)],
        Vec::new(),
        None,
        None,
    );
    let mut ui = Simulator::with_size(settings, Size::new(800.0, 500.0), view);
    ui.point_at(Point::new(100.0, 100.0));
    let _ = ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
        iced::mouse::Button::Left,
    ))]);
    ui.point_at(Point::new(140.0, 130.0));
    let _ = ui.simulate([
        iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: Point::new(140.0, 130.0),
        }),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
    ]);
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages.contains(&agent_graph::Message::Move("t1".into(), 120.0, 110.0)),
        "{messages:?}"
    );
    assert!(
        !messages.contains(&agent_graph::Message::Select("t1".into())),
        "dragging must leave the graph active: {messages:?}"
    );
    let mut store = crate::agent_graph_store::AgentGraphStore::default();
    for message in messages {
        if let agent_graph::Message::Move(id, x, y) = message {
            store.set_position(&id, x, y);
        }
    }
    assert_eq!(store.position("t1"), Some((120.0, 110.0)));
}

#[test]
fn right_clicking_empty_graph_publishes_creation_at_the_scene_point() {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = iced::Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    let nodes = vec![agent_graph::Node {
        id: "t1".into(),
        label: "Um".into(),
        detail: "projeto".into(),
        status: "Ocioso".into(),
        status_pulsing: false,
        queued: 0,
        color: iced::Color::from_rgb8(1, 2, 3),
        maestro: false,
        harness_icon: None,
        harness_icon_monochrome: false,
    }];
    let view = agent_graph::view(
        nodes,
        vec![("t1".into(), 80.0, 80.0)],
        Vec::new(),
        None,
        None,
    );
    let mut ui = Simulator::with_size(settings, Size::new(800.0, 500.0), view);
    ui.point_at(Point::new(600.0, 300.0));
    let _ = ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
        iced::mouse::Button::Right,
    ))]);
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages.contains(&agent_graph::Message::CreateAt(600.0, 300.0)),
        "{messages:?}"
    );
}

#[test]
fn every_material_icon_codepoint_in_labels_has_an_embedded_glyph() {
    let face = ttf_parser::Face::parse(theme::MATERIAL_SYMBOLS_TTF, 0)
        .expect("embedded Material Symbols font is valid");
    let source = include_str!("labels.rs");
    let mut checked = 0;
    for line in source
        .lines()
        .filter(|line| line.contains("pub const ICON_"))
    {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let mut remaining = value;
        while let Some(start) = remaining.find("\\u{") {
            remaining = &remaining[start + 3..];
            let Some(end) = remaining.find('}') else {
                panic!("invalid Unicode escape in {name}");
            };
            let codepoint = u32::from_str_radix(&remaining[..end], 16)
                .expect("ICON Unicode escape is hexadecimal");
            let character = char::from_u32(codepoint).expect("ICON codepoint is valid");
            assert!(
                face.glyph_index(character).is_some(),
                "{name} U+{codepoint:04X} is missing from embedded Material Symbols font"
            );
            checked += 1;
            remaining = &remaining[end + 1..];
        }
    }
    assert!(checked > 0);
}
