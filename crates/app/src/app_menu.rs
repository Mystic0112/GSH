//! Menu popup compacto (`showAppMenu` de `app_menu.dart`, sobre o `DropdownMenu` do shadcn):
//! ícone à esquerda, rótulo, check à direita no selecionado, cor de erro no destrutivo.
//!
//! A moldura e os itens têm as medidas do menu do filtro por agente (`AGENT_MENU_*`), que é o
//! mesmo componente no Flutter. O popup em si (posição, clique fora, Esc) é do [`crate::anchored`].

use gsh_db::Engine;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Padding, Theme};

use crate::{db_engine_icon, labels, panel_widgets, theme};

/// Ícone à esquerda do item.
#[derive(Debug, Clone, Copy)]
pub enum Leading {
    Glyph(&'static str),
    Engine(Engine),
}

/// Entrada do menu.
pub struct Entry<M> {
    pub label: String,
    pub leading: Option<Leading>,
    pub selected: bool,
    pub danger: bool,
    /// `None` = item cinza, não clicável.
    pub on_press: Option<M>,
}

pub enum Item<M> {
    Entry(Entry<M>),
    Divider,
}

impl<M> Entry<M> {
    pub fn new(label: impl Into<String>, on_press: M) -> Self {
        Self {
            label: label.into(),
            leading: None,
            selected: false,
            danger: false,
            on_press: Some(on_press),
        }
    }

    pub fn leading(mut self, leading: Leading) -> Self {
        self.leading = Some(leading);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn disabled(on_press: Option<M>) -> Self
    where
        M: Clone,
    {
        Self {
            label: String::new(),
            leading: None,
            selected: false,
            danger: false,
            on_press,
        }
    }
}

/// Largura média de um caractere do rótulo (13px, Hanken Grotesk).
const CHAR_WIDTH: f32 = 7.0;

/// Folga do item além do rótulo: ícone à esquerda, check à direita e o respiro das bordas.
const ITEM_EXTRA: f32 = 70.0;

/// Monta a caixa do menu; `min_width` é o `minWidth` do `showAppMenu` (o teto é o
/// `AGENT_MENU_MAX_WIDTH`). A largura é fixa e estimada pelo maior rótulo: o iced não mede o
/// texto antes do layout, e item `Fill` num menu sem largura se esticaria até a janela.
pub fn menu<'a, M: Clone + 'a>(items: Vec<Item<M>>, min_width: f32) -> Element<'a, M> {
    let longest = items
        .iter()
        .filter_map(|item| match item {
            Item::Entry(entry) => Some(entry.label.chars().count()),
            Item::Divider => None,
        })
        .max()
        .unwrap_or(0);
    let width =
        (longest as f32 * CHAR_WIDTH + ITEM_EXTRA).clamp(min_width, theme::AGENT_MENU_MAX_WIDTH);
    let mut list = column![];
    for item in items {
        list = list.push(match item {
            Item::Divider => container(panel_widgets::hairline_h())
                .padding([theme::AGENT_MENU_DIVIDER_VERTICAL, 0.0])
                .into(),
            Item::Entry(entry) => entry_view(entry),
        });
    }
    container(list)
        .width(width)
        .padding(theme::AGENT_MENU_PADDING)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel)),
            border: Border {
                color: theme::palette().border,
                width: 1.0,
                radius: theme::AGENT_MENU_RADIUS.into(),
            },
            shadow: iced::Shadow {
                color: theme::palette().shadow,
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..Default::default()
        })
        .into()
}

fn entry_view<'a, M: Clone + 'a>(entry: Entry<M>) -> Element<'a, M> {
    let palette = theme::palette();
    let enabled = entry.on_press.is_some();
    let icon_color = if !enabled {
        palette.text4
    } else if entry.danger {
        palette.error
    } else {
        palette.text3
    };
    let label_color = if !enabled {
        palette.text4
    } else if entry.danger {
        palette.error
    } else {
        palette.text
    };
    let mut content = row![].align_y(Alignment::Center);
    if let Some(leading) = entry.leading {
        let icon: Element<'a, M> = match leading {
            Leading::Glyph(glyph) => text(glyph)
                .font(theme::FONT_ICON)
                .size(theme::AGENT_MENU_ICON_SIZE)
                .color(icon_color)
                .into(),
            Leading::Engine(engine) => db_engine_icon::view(engine, theme::AGENT_MENU_ICON_SIZE),
        };
        content = content.push(container(icon).width(theme::AGENT_MENU_LEADING_WIDTH));
    }
    content = content.push(
        text(entry.label)
            .font(theme::FONT_BODY)
            .size(theme::AGENT_MENU_TEXT_SIZE)
            .color(label_color)
            .wrapping(text::Wrapping::None),
    );
    if entry.selected {
        content = content.push(Space::new().width(Fill)).push(
            text(labels::ICON_CHECK)
                .font(theme::FONT_ICON)
                .size(theme::AGENT_MENU_CHECK_SIZE)
                .color(palette.accent_text),
        );
    }
    button(content)
        .width(Length::Fill)
        .padding(Padding {
            top: theme::AGENT_MENU_ITEM_VERTICAL,
            right: theme::AGENT_MENU_ITEM_RIGHT,
            bottom: theme::AGENT_MENU_ITEM_VERTICAL,
            left: theme::AGENT_MENU_ITEM_LEFT,
        })
        .on_press_maybe(entry.on_press)
        .style(|_: &Theme, status| button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed)
                .then(|| Background::Color(theme::palette().panel3)),
            text_color: theme::palette().text,
            border: Border {
                radius: theme::AGENT_MENU_RADIUS.into(),
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
    fn builds_with_every_kind_of_item() {
        let items = vec![
            Item::Entry(Entry::new("Editar", 1).leading(Leading::Glyph(labels::ICON_EDIT))),
            Item::Entry(
                Entry::new("SQLite", 2)
                    .leading(Leading::Engine(Engine::Sqlite))
                    .selected(true),
            ),
            Item::Divider,
            Item::Entry(Entry::new("Excluir", 3).danger()),
            Item::Entry(Entry {
                label: "Nenhuma conexão SQL".into(),
                ..Entry::disabled(None::<u8>)
            }),
        ];
        let _: Element<'_, u8> = menu(items, 200.0);
    }
}
