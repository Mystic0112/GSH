use crate::{
    editor_highlight,
    editor_pane::{EditorMsg, EditorPane},
    editor_state::EditorNotice,
    labels, theme,
};
use iced::widget::text_editor;
use iced::{Element, Fill, Theme};

impl EditorPane {
    fn body(&self, height: f32, width: f32) -> Element<'_, EditorMsg> {
        use iced::widget::{container, row, scrollable};
        let count = self.content.line_count();
        let active = self.content.cursor().position.line;
        let ext = self
            .state
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned();
        let line_height = theme::SIZE_MONO * theme::LINE_HEIGHT_MONO;
        let viewport_height = if height.is_finite() && height > 0.0 {
            height
        } else {
            theme::EDITOR_VIEWPORT_FALLBACK
        };
        self.viewport_height.set(viewport_height);
        let page_lines =
            ((viewport_height / line_height) as usize).clamp(1, theme::EDITOR_PAGE_MAX_LINES);
        let editor_width = if self.config.word_wrap {
            width.max(1.0)
        } else {
            (self.longest_line as f32 * theme::EDITOR_COLUMN_WIDTH).max(width)
        };
        let estimated_lines = if self.config.word_wrap {
            self.state
                .buffer
                .text()
                .split('\n')
                .map(|line| {
                    let cells: usize = line
                        .chars()
                        .map(|ch| {
                            if ch == '\t' {
                                4
                            } else if ch.is_ascii() {
                                1
                            } else {
                                2
                            }
                        })
                        .sum();
                    ((cells as f32 * theme::EDITOR_COLUMN_WIDTH / editor_width).ceil() as usize)
                        .max(1)
                        .saturating_mul(2)
                })
                .sum::<usize>()
                .max(count)
        } else {
            count
        };
        let tab_size = self.config.tab_size;
        let submit_shortcut = self.submit_shortcut;
        let editor = iced::widget::text_editor(&self.content)
            .width(editor_width)
            .font(theme::FONT_MONO)
            .size(theme::SIZE_MONO)
            .line_height(theme::LINE_HEIGHT_MONO)
            .padding(0)
            .height(
                (estimated_lines as f32 * line_height + theme::EDITOR_EXTRA_HEIGHT)
                    .max(theme::EDITOR_MIN_HEIGHT),
            )
            .wrapping(if self.config.word_wrap {
                iced::widget::text::Wrapping::WordOrGlyph
            } else {
                iced::widget::text::Wrapping::None
            })
            .on_action(EditorMsg::Action)
            .key_binding(move |key| {
                crate::editor_keys::bind_key(key, page_lines, tab_size, submit_shortcut)
            })
            .style(|_: &Theme, _| text_editor::Style {
                background: theme::palette().panel.into(),
                border: iced::Border::default(),
                placeholder: theme::palette().text2,
                // iced 0.14 uses the value color for the caret as well as unstyled text.
                value: theme::SYNTAX_BASE,
                selection: theme::TERM_SELECTION_BG,
            })
            .highlight_with::<editor_highlight::SyntaxHighlighter>(
                editor_highlight::Settings::new(
                    if self.config.syntax_highlight {
                        ext
                    } else {
                        String::new()
                    },
                    self.highlight_text.clone(),
                ),
                editor_highlight::format,
            );
        let content = if self.config.word_wrap {
            container(row![editor]).padding([theme::EDITOR_PAD, 0.0])
        } else {
            container(row![self.gutter.view(active), editor]).padding([theme::EDITOR_PAD, 0.0])
        };
        scrollable(content)
            .direction(scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::default(),
                horizontal: scrollable::Scrollbar::default(),
            })
            .id(self.scroll_id.clone())
            .on_scroll(|viewport| {
                EditorMsg::Viewport(viewport.absolute_offset().y, viewport.bounds().height)
            })
            .height(Fill)
            .into()
    }
    pub fn view(&self) -> Element<'_, EditorMsg> {
        use iced::widget::{column, container, responsive, text};
        let body = responsive(move |size| self.body(size.height, size.width));
        let mut content = column![body].height(Fill);
        let is_ckp = self
            .state
            .path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("ckp"));
        if is_ckp {
            use iced::widget::{button, row};
            let mut tools =
                row![button(labels::EDITOR_APPLY_CKP_LAYOUT).on_press(EditorMsg::ApplyCkpLayout)]
                    .spacing(8);
            if let Some(error) = &self.ckp_error {
                tools = tools.push(
                    text(error)
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().error),
                );
            }
            content = content.push(container(tools).padding([6, 12]));
        }
        #[cfg(target_os = "linux")]
        if let Some(lsp) = &self.lsp {
            use iced::widget::{button, row, scrollable};
            let mut tools = row![].spacing(8);
            if lsp.formatting || lsp.external.is_some() {
                let label = if lsp.pending {
                    labels::EDITOR_FORMATTING
                } else {
                    labels::EDITOR_FORMAT
                };
                tools = tools.push(button(label).on_press_maybe(
                    (!lsp.pending && self.state.notice.is_none()).then_some(EditorMsg::Format),
                ));
            }
            if let Some(error) = &lsp.error {
                tools = tools.push(
                    text(error)
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().error),
                );
            }
            content = content.push(tools);
            if !lsp.diagnostics.is_empty() {
                let messages =
                    lsp.diagnostics
                        .iter()
                        .fold(column![].spacing(2), |items, diagnostic| {
                            let line = diagnostic.range.start.line.saturating_add(1);
                            let (severity, color) = match diagnostic.severity {
                                Some(1) => {
                                    (labels::EDITOR_DIAGNOSTIC_ERROR, theme::palette().error)
                                }
                                Some(2) => {
                                    (labels::EDITOR_DIAGNOSTIC_WARNING, theme::palette().warn)
                                }
                                Some(4) => (labels::EDITOR_DIAGNOSTIC_HINT, theme::palette().text2),
                                _ => (labels::EDITOR_DIAGNOSTIC_INFO, theme::palette().text2),
                            };
                            items.push(
                                button(
                                    text(format!("{severity} · L{line}  {}", diagnostic.message))
                                        .size(theme::SIZE_LABEL)
                                        .color(color),
                                )
                                .on_press(EditorMsg::RevealDiagnostic(line as usize)),
                            )
                        });
                content = content
                    .push(container(scrollable(messages).height(iced::Shrink)).max_height(160));
            }
        }
        if let Some(notice) = &self.state.notice {
            let message = match notice {
                EditorNotice::ModifiedOnDisk => labels::EDITOR_EXTERNAL_CHANGE.to_owned(),
                EditorNotice::SaveError(error) => labels::format_editor_error(error),
            };
            content = content.push(
                container(text(message).size(theme::SIZE_LABEL).color(notice.color()))
                    .padding([8, 12])
                    .width(Fill)
                    .style(|_: &Theme| container::Style {
                        background: Some(theme::palette().panel2.into()),
                        ..Default::default()
                    }),
            );
            if matches!(notice, EditorNotice::ModifiedOnDisk) {
                content = content.push(
                    iced::widget::button(labels::EDITOR_OVERWRITE).on_press(EditorMsg::Overwrite),
                );
            }
        }
        container(content)
            .height(Fill)
            .width(Fill)
            .style(|_: &Theme| container::Style {
                background: Some(theme::palette().panel.into()),
                ..Default::default()
            })
            .into()
    }
}
