//! Numeração de linhas e decoração da margem do editor.
use crate::theme;
use iced::widget::{column, container, row, text, Space};
use iced::{Element, Fill, Theme};

#[derive(Debug, Default)]
pub struct Gutter {
    numbers: Vec<String>,
}
impl Gutter {
    pub fn update(&mut self, count: usize) {
        if count == self.numbers.len() {
            return;
        }
        let digits = count.to_string().len().max(2);
        self.numbers = (1..=count).map(|n| format!("{n:>digits$}")).collect();
    }
    pub fn view<'a, M: 'a>(&'a self, active: usize) -> Element<'a, M> {
        let mut numbers = column![];
        for (index, value) in self.numbers.iter().enumerate() {
            numbers = numbers.push(
                text(value)
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_MONO)
                    .line_height(theme::LINE_HEIGHT_MONO)
                    .color(if index == active {
                        theme::TEXT
                    } else {
                        theme::EDITOR_GUTTER
                    }),
            );
        }
        let divider =
            container(Space::new().width(1.0).height(Fill)).style(|_: &Theme| container::Style {
                background: Some(theme::BORDER.into()),
                ..Default::default()
            });
        row![
            Space::new().width(theme::GUTTER_ICON),
            container(numbers).padding([0.0, theme::GUTTER_PAD]),
            divider
        ]
        .into()
    }
}
