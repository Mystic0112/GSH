//! Aba Galeria do painel direito (`gallery_panel.dart` do Flutter): vitrine dos
//! documentos especiais do GSH. Um cartão por modelo (ícone, título, caminho
//! e descrição); clicar pede a criação do arquivo na raiz do workspace.

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Border, Element, Fill, Padding, Theme};

use crate::gallery_template::GalleryTemplate;
use crate::{file_icons, labels, panel_widgets, theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryMsg {
    /// Cartão clicado: cria o arquivo do modelo e abre.
    Create(GalleryTemplate),
}

/// Corpo da aba: cabeçalho "GALERIA" e a lista de cartões, ou a mensagem de
/// "sem pasta" quando o workspace não tem pasta.
pub fn view<'a>(has_root: bool) -> Element<'a, GalleryMsg> {
    if !has_root {
        return panel_widgets::empty_label(labels::FILES_NO_FOLDER);
    }
    let intro = container(
        text(labels::GALLERY_INTRO)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text3),
    )
    .padding(Padding {
        top: theme::GALLERY_INTRO_PADDING_TOP,
        right: theme::GALLERY_INTRO_PADDING_SIDE,
        bottom: theme::GALLERY_INTRO_PADDING_BOTTOM,
        left: theme::GALLERY_INTRO_PADDING_SIDE,
    });
    let cards = GalleryTemplate::ALL
        .into_iter()
        .fold(column![].spacing(theme::GALLERY_CARD_GAP), |list, t| {
            list.push(card(t))
        });
    let list = column![intro, cards];
    column![
        panel_widgets::section_header(labels::GALLERY_SECTION_UPPER, Vec::new()),
        scrollable(container(list).width(Fill).padding(Padding {
            top: theme::GALLERY_PADDING_TOP,
            right: theme::GALLERY_PADDING_SIDE,
            bottom: theme::GALLERY_PADDING_BOTTOM,
            left: theme::GALLERY_PADDING_SIDE,
        }))
        .width(Fill)
        .height(Fill),
    ]
    .into()
}

fn card<'a>(template: GalleryTemplate) -> Element<'a, GalleryMsg> {
    let title_row = row![
        text(labels::gallery_title(template))
            .font(theme::FONT_BODY_SEMIBOLD)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text)
            .width(Fill),
        text(template.card_path())
            .font(theme::FONT_BODY)
            .size(theme::GALLERY_PATH_SIZE)
            .color(theme::palette().text3),
    ];
    let description = text(labels::gallery_description(template))
        .font(theme::FONT_BODY)
        .size(theme::GALLERY_DESCRIPTION_SIZE)
        .color(theme::palette().text3);
    let content = row![
        file_icons::asset_view(template.icon(), theme::GALLERY_ICON_SIZE),
        Space::new().width(theme::GALLERY_ICON_GAP),
        column![title_row, description]
            .spacing(theme::GALLERY_DESCRIPTION_GAP)
            .width(Fill),
    ]
    .align_y(Alignment::Start);
    button(content)
        .width(Fill)
        .padding(theme::GALLERY_CARD_PADDING)
        .on_press(GalleryMsg::Create(template))
        .style(|_: &Theme, status| {
            let palette = theme::palette();
            button::Style {
                background: Some(
                    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                        palette.panel3
                    } else {
                        palette.panel2
                    }
                    .into(),
                ),
                text_color: palette.text,
                border: Border {
                    color: palette.border,
                    width: 1.0,
                    radius: theme::GALLERY_CARD_RADIUS.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_card_per_template_each_with_title_and_description() {
        assert_eq!(GalleryTemplate::ALL.len(), 9);
        for template in GalleryTemplate::ALL {
            assert!(!labels::gallery_title(template).is_empty(), "{template:?}");
            assert!(
                !labels::gallery_description(template).is_empty(),
                "{template:?}"
            );
            let _ = card(template);
        }
        let _ = view(true);
        let _ = view(false);
    }

    #[test]
    fn kanban_card_shows_its_extension() {
        assert_eq!(GalleryTemplate::Kanban.card_path(), ".kanban");
        assert_eq!(
            labels::gallery_title(GalleryTemplate::Kanban),
            "Quadro kanban"
        );
    }
}
