//! Abas do painel direito (`_RightPaneTab` e a barra de 40px de
//! `file_tree_panel.dart`).
//!
//! Cada ícone só aparece se o painel existe; se a aba ativa deixa de existir
//! (saiu do repo git, fechou o workspace), a aba efetiva volta para Arquivos.
//! Para acrescentar busca, banco ou telemetria: uma variante, um item em
//! [`RightTab::ORDER`] (na ordem do Flutter) e um braço em cada `match`.

use iced::widget::{button, container, row, text, tooltip};
use iced::{Alignment, Border, Element, Fill, Length, Padding, Theme};

use crate::{labels, pane_chrome, theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightTab {
    #[default]
    Files,
    Git,
    Gallery,
}

/// O que existe agora e decide quais abas aparecem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Available {
    /// A pasta do workspace é um repositório git.
    pub git: bool,
    /// Há um workspace selecionado.
    pub gallery: bool,
}

impl RightTab {
    /// Ordem dos ícones: arquivos, (busca), git, (banco), (telemetria), galeria.
    pub const ORDER: [Self; 3] = [Self::Files, Self::Git, Self::Gallery];

    fn exists(self, available: Available) -> bool {
        match self {
            Self::Files => true,
            Self::Git => available.git,
            Self::Gallery => available.gallery,
        }
    }

    /// Aba que de fato aparece: esta, ou Arquivos se a condição dela sumiu.
    pub fn effective(self, available: Available) -> Self {
        if self.exists(available) {
            self
        } else {
            Self::Files
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Self::Files => labels::ICON_FOLDER,
            Self::Git => labels::ICON_ACCOUNT_TREE,
            Self::Gallery => labels::ICON_GALLERY,
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            Self::Files => labels::FILES_TOOLTIP,
            Self::Git => labels::SOURCE_CONTROL_TOOLTIP,
            Self::Gallery => labels::GALLERY_TOOLTIP,
        }
    }
}

/// Barra de ícones (40px com a divisória de baixo, que o chamador desenha).
pub fn bar<'a>(active: RightTab, available: Available) -> Element<'a, RightTab> {
    let icons = RightTab::ORDER
        .into_iter()
        .filter(|tab| tab.exists(available))
        .fold(row![].align_y(Alignment::Center), |icons, tab| {
            icons.push(icon(tab, tab == active))
        });
    container(icons)
        .width(Fill)
        .padding(Padding {
            left: theme::RIGHT_TAB_BAR_LEFT,
            right: theme::RIGHT_TAB_BAR_RIGHT,
            ..Padding::ZERO
        })
        .center_y(Length::Fixed(theme::TAB_STRIP_HEIGHT - 1.0))
        .into()
}

fn icon<'a>(tab: RightTab, selected: bool) -> Element<'a, RightTab> {
    let button = button(
        container(
            text(tab.glyph())
                .font(theme::FONT_ICON)
                .size(theme::SIZE_ICON),
        )
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(theme::RIGHT_TAB_BUTTON_SIZE)
    .height(theme::RIGHT_TAB_BUTTON_SIZE)
    .padding(0)
    .on_press(tab)
    .style(move |_: &Theme, status| button::Style {
        background: if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(theme::palette().panel3.into())
        } else if selected {
            Some(theme::palette().panel2.into())
        } else {
            None
        },
        text_color: if selected {
            theme::palette().text
        } else {
            theme::palette().text3
        },
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    });
    tooltip(
        button,
        pane_chrome::tip_bubble(tab.tooltip()),
        tooltip::Position::Bottom,
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTHING: Available = Available {
        git: false,
        gallery: false,
    };
    const ALL: Available = Available {
        git: true,
        gallery: true,
    };

    #[test]
    fn effective_tab_falls_back_to_files_when_its_condition_is_gone() {
        assert_eq!(RightTab::Git.effective(NOTHING), RightTab::Files);
        assert_eq!(RightTab::Gallery.effective(NOTHING), RightTab::Files);
        assert_eq!(RightTab::Git.effective(ALL), RightTab::Git);
        assert_eq!(RightTab::Gallery.effective(ALL), RightTab::Gallery);
        let only_gallery = Available {
            git: false,
            gallery: true,
        };
        assert_eq!(RightTab::Git.effective(only_gallery), RightTab::Files);
        assert_eq!(RightTab::Gallery.effective(only_gallery), RightTab::Gallery);
    }

    #[test]
    fn files_always_exists_and_comes_first() {
        assert!(RightTab::Files.exists(NOTHING));
        assert_eq!(RightTab::ORDER[0], RightTab::Files);
        assert_eq!(RightTab::default(), RightTab::Files);
    }

    #[test]
    fn order_is_files_git_gallery_like_flutter() {
        assert_eq!(
            RightTab::ORDER,
            [RightTab::Files, RightTab::Git, RightTab::Gallery]
        );
    }
}
