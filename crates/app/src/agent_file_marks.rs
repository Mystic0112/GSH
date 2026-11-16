//! "Quem está mexendo no quê" no painel Arquivos (`agent_file_marks.dart` do Flutter): botão de
//! filtro por agente no cabeçalho, bolinhas coloridas por linha da árvore e a lista plana dos
//! arquivos do agente filtrado.
//!
//! Só leitura: o estado vive em [`AgentFileActivity`] (núcleo) e o filtro escolhido, no `Shell`.

use std::path::Path;

use gsh_core::file_activity::{recent_touchers, AgentFileActivity, FileTouch};
use iced::widget::{
    button, column, container, mouse_area, rich_text, row, scrollable, span, text, tooltip, Space,
};
use iced::{Alignment, Background, Border, Element, Fill, Font, Length, Padding, Theme};

use crate::files_panel::FilesMsg;
use crate::{clock, labels, pane_chrome, theme};

/// Uma aba de agente do menu: terminal com harness ativo ou com arquivos tocados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTab {
    pub id: String,
    pub label: String,
    /// O harness reporta os arquivos por hook. Sem isso (e sem arquivos) o rastreio não existe.
    pub by_hook: bool,
}

/// O que os widgets deste módulo leem a cada quadro.
#[derive(Clone, Copy)]
pub struct AgentFiles<'a> {
    pub activity: &'a AgentFileActivity,
    pub tabs: &'a [AgentTab],
    /// Filtro já validado por [`effective_filter`].
    pub filter: Option<&'a str>,
    pub now_ms: u64,
}

/// Filtro de uma aba que fechou volta para "todos" (`agentFilter` em `file_tree_panel.dart`).
pub fn effective_filter<'a>(
    selected: Option<&'a str>,
    activity: &AgentFileActivity,
    tabs: &[AgentTab],
) -> Option<&'a str> {
    selected.filter(|id| activity.touched_count(id) > 0 || tabs.iter().any(|t| t.id == *id))
}

/// Rótulo da aba `tab_id` (o id cru se a aba já não existe), `agentLabelOf`.
fn label_of<'a>(tabs: &'a [AgentTab], tab_id: &'a str) -> &'a str {
    tabs.iter()
        .find(|t| t.id == tab_id)
        .map_or(tab_id, |t| t.label.as_str())
}

/// "Editado por X · hh:mm", com o sufixo de inferido (`_touchLine`).
fn touch_line(label: &str, touch: FileTouch) -> String {
    let line = labels::format_edited_by(label, &clock::hhmm(touch.at_ms));
    if touch.inferred {
        format!("{line} {}", labels::INFERRED_SUFFIX)
    } else {
        line
    }
}

fn hover_style(radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then(|| Background::Color(theme::palette().panel3)),
        text_color: theme::palette().text,
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn dot<M: 'static>(color: iced::Color) -> Element<'static, M> {
    container(
        Space::new()
            .width(theme::AGENT_DOT_SIZE)
            .height(theme::AGENT_DOT_SIZE),
    )
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: (theme::AGENT_DOT_SIZE / 2.0).into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn icon<M: 'static>(glyph: &'static str, size: f32, color: iced::Color) -> Element<'static, M> {
    text(glyph)
        .font(theme::FONT_ICON)
        .size(size)
        .color(color)
        .into()
}

/// Botão robô do cabeçalho: cinza sem filtro; com filtro, mais claro e com a bolinha do agente.
pub fn filter_button(ctx: AgentFiles<'_>) -> Element<'static, FilesMsg> {
    let palette = theme::palette();
    let (icon_color, tip) = match ctx.filter {
        None => (palette.text3, labels::AGENT_FILTER_TOOLTIP.to_string()),
        Some(id) => (palette.text, label_of(ctx.tabs, id).to_string()),
    };
    let mut content = row![icon(
        labels::ICON_SMART_TOY,
        theme::AGENT_FILTER_ICON_SIZE,
        icon_color
    )]
    .align_y(Alignment::Center);
    if let Some(id) = ctx.filter {
        content = content
            .push(Space::new().width(theme::AGENT_FILTER_DOT_GAP))
            .push(dot(theme::agent_color(id)));
    }
    let trigger = button(content)
        .padding(theme::AGENT_FILTER_PADDING)
        .on_press(FilesMsg::ToggleAgentMenu)
        .style(hover_style(theme::AGENT_FILTER_RADIUS));
    tooltip(
        trigger,
        pane_chrome::tip_bubble(tip),
        tooltip::Position::Bottom,
    )
    .into()
}

/// Menu do filtro: "Todos os agentes" e uma entrada por aba de agente (`AgentFilterButton._open`).
pub fn menu(ctx: AgentFiles<'_>) -> Element<'static, FilesMsg> {
    let palette = theme::palette();
    let mut items = column![menu_item(
        container(icon(
            labels::ICON_GROUPS,
            theme::AGENT_MENU_ICON_SIZE,
            palette.text3
        ))
        .width(theme::AGENT_MENU_LEADING_WIDTH)
        .into(),
        text(labels::ALL_AGENTS)
            .font(theme::FONT_BODY)
            .size(theme::AGENT_MENU_TEXT_SIZE)
            .color(palette.text)
            .into(),
        ctx.filter.is_none(),
        Some(FilesMsg::SetAgentFilter(None)),
    )];
    if !ctx.tabs.is_empty() {
        items = items.push(
            container(crate::panel_widgets::hairline_h())
                .padding([theme::AGENT_MENU_DIVIDER_VERTICAL, 0.0]),
        );
    }
    for tab in ctx.tabs {
        items = items.push(tab_item(ctx, tab));
    }
    container(items)
        .width(theme::AGENT_MENU_MIN_WIDTH)
        .padding(Padding {
            top: theme::AGENT_MENU_PADDING,
            right: theme::AGENT_MENU_PADDING,
            bottom: theme::AGENT_MENU_PADDING,
            left: theme::AGENT_MENU_PADDING,
        })
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

fn tab_item(ctx: AgentFiles<'_>, tab: &AgentTab) -> Element<'static, FilesMsg> {
    let palette = theme::palette();
    let count = ctx.activity.touched_count(&tab.id);
    let unavailable = count == 0 && !tab.by_hook;
    let leading = container(
        row![
            dot(theme::agent_color(&tab.id)),
            Space::new().width(theme::AGENT_MENU_DOT_GAP),
            icon(
                labels::ICON_TERMINAL,
                theme::AGENT_MENU_TAB_ICON_SIZE,
                palette.text3
            ),
        ]
        .align_y(Alignment::Center),
    )
    // bolinha + ícone passam dos 24 do slot fixo; o MenuButton separa o rótulo por 8
    .padding(Padding {
        right: theme::AGENT_MENU_GAP,
        ..Padding::ZERO
    });
    let name = container(
        text(tab.label.clone())
            .font(theme::FONT_BODY)
            .size(theme::AGENT_MENU_TEXT_SIZE)
            .color(if unavailable {
                palette.text4
            } else {
                palette.text
            })
            .wrapping(text::Wrapping::None),
    )
    .max_width(theme::AGENT_MENU_LABEL_MAX)
    .clip(true);
    let tail = if unavailable {
        text(labels::TRACKING_UNAVAILABLE)
    } else {
        text(count.to_string())
    }
    .font(theme::FONT_BODY)
    .size(theme::SIZE_LABEL)
    .color(palette.text4);
    menu_item(
        leading.into(),
        row![name, Space::new().width(theme::AGENT_MENU_GAP), tail]
            .align_y(Alignment::Center)
            .into(),
        ctx.filter == Some(tab.id.as_str()),
        (!unavailable).then(|| FilesMsg::SetAgentFilter(Some(tab.id.clone()))),
    )
}

/// Uma linha do menu (`MenuButton`): ícone à esquerda, rótulo, check à direita se selecionada.
fn menu_item(
    leading: Element<'static, FilesMsg>,
    label: Element<'static, FilesMsg>,
    selected: bool,
    on_press: Option<FilesMsg>,
) -> Element<'static, FilesMsg> {
    let mut content = row![leading, label].align_y(Alignment::Center);
    if selected {
        content = content.push(Space::new().width(Fill)).push(icon(
            labels::ICON_CHECK,
            theme::AGENT_MENU_CHECK_SIZE,
            theme::palette().accent_text,
        ));
    }
    button(content)
        .width(Fill)
        .padding(Padding {
            top: theme::AGENT_MENU_ITEM_VERTICAL,
            right: theme::AGENT_MENU_ITEM_RIGHT,
            bottom: theme::AGENT_MENU_ITEM_VERTICAL,
            left: theme::AGENT_MENU_ITEM_LEFT,
        })
        .on_press_maybe(on_press)
        .style(hover_style(theme::AGENT_MENU_RADIUS))
        .into()
}

/// Fundo transparente que fecha o menu ao clicar fora dele.
pub fn menu_backdrop() -> Element<'static, FilesMsg> {
    mouse_area(
        container(Space::new().width(Fill).height(Fill))
            .width(Fill)
            .height(Fill),
    )
    .on_press(FilesMsg::CloseAgentMenu)
    .into()
}

/// Bolinhas por agente numa linha da árvore (máx. 3, depois `+n`), com o aviso de edição
/// concorrente e o tooltip de quem editou e quando. `None` se ninguém tocou o arquivo.
pub fn marks(ctx: AgentFiles<'_>, path: &Path) -> Option<Element<'static, FilesMsg>> {
    let touchers = ctx.activity.touchers_of(path.to_str()?);
    if touchers.is_empty() {
        return None;
    }
    let palette = theme::palette();
    let concurrent = recent_touchers(&touchers, ctx.now_ms) >= 2;
    let mut lines: Vec<String> = touchers
        .iter()
        .map(|t| touch_line(label_of(ctx.tabs, t.tab_id), t.touch))
        .collect();
    if concurrent {
        lines.push(labels::CONCURRENT_AGENT_EDITS.to_string());
    }

    let mut content = row![].align_y(Alignment::Center);
    if concurrent {
        content = content.push(
            container(icon(
                labels::ICON_WARNING_AMBER,
                theme::AGENT_MARK_WARN_SIZE,
                palette.warn,
            ))
            .padding(Padding {
                right: theme::AGENT_MARK_WARN_GAP,
                ..Default::default()
            }),
        );
    }
    for toucher in touchers.iter().take(theme::AGENT_MAX_DOTS) {
        content = content.push(container(dot(theme::agent_color(toucher.tab_id))).padding(
            Padding {
                left: theme::AGENT_MARK_DOT_GAP,
                ..Default::default()
            },
        ));
    }
    if touchers.len() > theme::AGENT_MAX_DOTS {
        content = content.push(
            container(
                text(format!("+{}", touchers.len() - theme::AGENT_MAX_DOTS))
                    .font(theme::FONT_BODY)
                    .size(theme::AGENT_MARK_MORE_SIZE)
                    .color(palette.text3),
            )
            .padding(Padding {
                left: theme::AGENT_MARK_MORE_GAP,
                ..Default::default()
            }),
        );
    }
    Some(
        tooltip(
            content,
            pane_chrome::tip_bubble(lines.join("\n")),
            tooltip::Position::Bottom,
        )
        .into(),
    )
}

/// Lista plana dos arquivos tocados pelo agente filtrado, do mais novo ao mais antigo
/// (`AgentTouchedList`). Clicar abre o arquivo; apagado vem riscado e sem clique.
// ponytail: a existência do arquivo é conferida a cada quadro (o Flutter também); com milhares
// de arquivos tocados vale cachear o resultado por lote do hook.
pub fn touched_list(
    ctx: AgentFiles<'_>,
    tab_id: &str,
    root: Option<&Path>,
) -> Element<'static, FilesMsg> {
    let palette = theme::palette();
    let entries = ctx.activity.touched_by_recent(tab_id);
    if entries.is_empty() {
        return container(
            text(labels::NO_TOUCHED_FILES)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(palette.text3)
                .align_x(Alignment::Center),
        )
        .center_x(Fill)
        .center_y(Fill)
        .into();
    }
    let label = label_of(ctx.tabs, tab_id);
    let mut list = column![].padding(Padding {
        top: theme::AGENT_LIST_PADDING_VERTICAL,
        right: theme::AGENT_LIST_PADDING_HORIZONTAL,
        bottom: theme::AGENT_LIST_PADDING_VERTICAL,
        left: theme::AGENT_LIST_PADDING_HORIZONTAL,
    });
    for (path, touch) in entries {
        list = list.push(touched_row(path, touch, label, root));
    }
    scrollable(list.width(Fill)).width(Fill).height(Fill).into()
}

fn touched_row(
    path: &str,
    touch: FileTouch,
    label: &str,
    root: Option<&Path>,
) -> Element<'static, FilesMsg> {
    let palette = theme::palette();
    let abs = Path::new(path);
    let exists = abs.exists();
    // Dentro da raiz mostra o caminho relativo; fora (ou igual a ela) o absoluto.
    let shown = root
        .and_then(|r| abs.strip_prefix(r).ok())
        .filter(|rel| !rel.as_os_str().is_empty())
        .unwrap_or(abs)
        .to_string_lossy()
        .into_owned();
    let basename = abs
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
    let font = if touch.inferred {
        Font {
            style: iced::font::Style::Italic,
            ..theme::FONT_BODY
        }
    } else {
        theme::FONT_BODY
    };
    let name = rich_text![span::<(), Font>(shown)
        .font(font)
        .size(theme::AGENT_LIST_NAME_SIZE)
        .color(if exists { palette.text2 } else { palette.text4 })
        .strikethrough(!exists)]
    .wrapping(text::Wrapping::None);
    let content = row![
        crate::file_icons::view(&basename, false, false),
        Space::new().width(theme::AGENT_LIST_ICON_GAP),
        container(name).width(Fill).clip(true),
        text(clock::hhmm(touch.at_ms))
            .font(theme::FONT_BODY)
            .size(theme::AGENT_LIST_TIME_SIZE)
            .color(palette.text4),
    ]
    .align_y(Alignment::Center);
    let open = button(content)
        .width(Fill)
        .height(Length::Fixed(theme::AGENT_LIST_ROW_HEIGHT))
        .padding([0.0, theme::AGENT_LIST_ROW_PADDING])
        .on_press_maybe(exists.then(|| FilesMsg::SelectFile(abs.to_path_buf())))
        .style(|_: &Theme, status| button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed)
                .then(|| Background::Color(theme::palette().panel)),
            text_color: theme::palette().text,
            border: Border {
                radius: theme::AGENT_LIST_ROW_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    tooltip(
        open,
        pane_chrome::tip_bubble(touch_line(label, touch)),
        tooltip::Position::Bottom,
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(id: &str) -> AgentTab {
        AgentTab {
            id: id.into(),
            label: format!("Terminal {id}"),
            by_hook: true,
        }
    }

    #[test]
    fn filtro_de_aba_que_sumiu_volta_para_todos() {
        let mut activity = AgentFileActivity::default();
        activity.record("t1", &["/p/a".to_string()], false, 1_000);
        let tabs = [tab("t2")];
        // t1 sem aba mas com arquivos: vale. t2 com aba e sem arquivos: vale. t3: some.
        assert_eq!(effective_filter(Some("t1"), &activity, &tabs), Some("t1"));
        assert_eq!(effective_filter(Some("t2"), &activity, &tabs), Some("t2"));
        assert_eq!(effective_filter(Some("t3"), &activity, &tabs), None);
        assert_eq!(effective_filter(None, &activity, &tabs), None);
        activity.forget_tab("t1");
        assert_eq!(effective_filter(Some("t1"), &activity, &tabs), None);
    }

    #[test]
    fn rotulo_cai_no_id_cru_sem_aba() {
        let tabs = [tab("t1")];
        assert_eq!(label_of(&tabs, "t1"), "Terminal t1");
        assert_eq!(label_of(&tabs, "t9"), "t9");
    }

    #[test]
    fn linha_de_toque_marca_o_inferido() {
        let hook = FileTouch {
            at_ms: 0,
            inferred: false,
        };
        let inferido = FileTouch {
            at_ms: 0,
            inferred: true,
        };
        let a = touch_line("Terminal", hook);
        assert!(a.starts_with("Editado por Terminal · "));
        assert!(!a.ends_with(labels::INFERRED_SUFFIX));
        assert_eq!(touch_line("Terminal", inferido), format!("{a} (inferido)"));
    }
}
