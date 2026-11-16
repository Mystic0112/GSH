//! Diálogo único de conexão (`db_connection_dialog.dart`): criar (o motor vem do menu do "+") e
//! editar (clicar na conexão; motor fixo). Testar valida antes de salvar; Excluir (só ao editar)
//! fica na extrema esquerda.
//!
//! Túnel SSH fica fora desta fase: o bloco `ssh` de uma conexão editada atravessa o diálogo
//! intacto, mas não há campos para ele.

use std::path::{Path, PathBuf};

use gsh_db::url::{self, ParsedUrl};
use gsh_db::{Access, Connection, DbError, Endpoint, Engine, Origin, SecretStore};
use iced::widget::{
    button, canvas, column, container, row, text, text_editor, text_input, Space, Stack,
};
use iced::{
    alignment::Vertical, mouse, Alignment, Background, Border, Element, Fill, Length, Padding,
    Rectangle, Renderer, Task, Theme,
};

use crate::drag_handle::{Axis, DragHandle};
use crate::pane_close_dialog::{self, ButtonKind};
use crate::{db_engine_icon, labels, theme};

#[derive(Debug, Clone)]
pub enum DialogMsg {
    Name(String),
    PickFile,
    FilePicked(Option<PathBuf>),
    Uri(text_editor::Action),
    UriResize(f32),
    UriResizeReset,
    SavePassword(bool),
    Password(String),
    AllowWrites(bool),
    VisibleToAgents(bool),
    Test,
    Tested(Option<String>),
    Submit,
    Delete,
    Cancel,
    Noop,
}

/// O que o diálogo decidiu; quem o abriu persiste.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Saved {
        connection: Connection,
        /// Senha digitada ou extraída da URL; `None` = não mexer na guardada.
        password: Option<String>,
        previous_name: Option<String>,
    },
    Deleted(Connection),
    Cancelled,
}

#[derive(Debug)]
pub struct ConnectionDialog {
    engine: Engine,
    initial: Option<Connection>,
    name: String,
    file: String,
    uri: text_editor::Content,
    uri_height: f32,
    save_password: bool,
    password: String,
    allow_writes: bool,
    visible_to_agents: bool,
    testing: bool,
    /// `Some(Ok)` / `Some(Err(mensagem))` depois de testar.
    test: Option<Result<(), String>>,
}

/// Conexão montada a partir dos campos.
struct Built {
    connection: Connection,
    password: Option<String>,
}

impl ConnectionDialog {
    /// Dialog novo (motor do menu do "+") ou de edição (`initial`).
    pub fn new(engine: Engine, initial: Option<Connection>) -> Self {
        let mut dialog = Self {
            engine,
            name: String::new(),
            file: String::new(),
            uri: text_editor::Content::new(),
            uri_height: theme::DB_DIALOG_URI_HEIGHT,
            save_password: initial.as_ref().is_some_and(|c| c.save_password),
            password: String::new(),
            allow_writes: initial
                .as_ref()
                .is_some_and(|c| c.access == Access::ReadWrite),
            visible_to_agents: initial.as_ref().is_none_or(|c| c.agents),
            testing: false,
            test: None,
            initial: None,
        };
        if let Some(connection) = &initial {
            dialog.name.clone_from(&connection.name);
            if engine == Engine::Sqlite {
                dialog.file = connection.sqlite_path().to_string();
            } else {
                dialog.uri = text_editor::Content::with_text(&connection.url);
            }
        }
        dialog.initial = initial;
        dialog
    }

    fn editing(&self) -> bool {
        self.initial.is_some()
    }

    fn name_or_default(&self) -> String {
        let name = self.name.trim();
        if name.is_empty() {
            labels::DB_DEFAULT_CONNECTION_NAME.to_string()
        } else {
            name.to_string()
        }
    }

    fn access(&self) -> Access {
        if self.allow_writes {
            Access::ReadWrite
        } else {
            Access::Read
        }
    }

    /// Conexão do modo ativo, ou `None` se a connection string não vale (Salvar e Testar ficam
    /// desabilitados).
    fn try_build(&self) -> Option<Built> {
        if self.engine == Engine::Sqlite {
            return Some(Built {
                connection: Connection::sqlite(
                    &self.name_or_default(),
                    self.file.trim(),
                    Origin::Registered,
                    self.access(),
                    self.visible_to_agents,
                ),
                password: None,
            });
        }
        let raw = self.uri.text();
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        // "Salvar senha" ligado com senha na URI: tira do texto e manda para o cofre
        let (url, extracted) = if self.save_password {
            split_password(raw)
        } else {
            (raw.to_string(), None)
        };
        let password = if self.password.is_empty() {
            extracted
        } else {
            Some(self.password.clone())
        };
        let mut json = serde_json::json!({
            "name": self.name_or_default(),
            "url": url,
            "savePassword": self.save_password,
            "access": self.access().name(),
            "agents": self.visible_to_agents,
        });
        if let (Some(ssh), Some(map)) = (
            self.initial.as_ref().and_then(|c| c.ssh.as_ref()),
            json.as_object_mut(),
        ) {
            map.insert(
                "ssh".into(),
                serde_json::json!({
                    "host": ssh.host, "port": ssh.port, "user": ssh.user,
                    "keyPath": ssh.key_path, "savePassphrase": ssh.save_passphrase,
                }),
            );
        }
        let connection = Connection::from_json(&json, Origin::Registered).ok()?;
        Some(Built {
            connection,
            password,
        })
    }

    /// Texto vermelho abaixo do campo: a URL não vale.
    fn url_invalid(&self) -> bool {
        self.engine != Engine::Sqlite
            && !self.uri.text().trim().is_empty()
            && self.try_build().is_none()
    }

    fn clear_test(&mut self) {
        self.test = None;
    }

    pub fn update(
        &mut self,
        message: DialogMsg,
        root: &Path,
        secrets: &SecretStore,
    ) -> (Task<DialogMsg>, Option<Outcome>) {
        match message {
            DialogMsg::Name(value) => self.name = value,
            DialogMsg::PickFile => {
                let start = root.to_path_buf();
                return (
                    Task::perform(
                        async move {
                            rfd::AsyncFileDialog::new()
                                .set_title(labels::DB_DIALOG_CHOOSE_FILE_TITLE)
                                .set_directory(start)
                                .pick_file()
                                .await
                                .map(|handle| handle.path().to_path_buf())
                        },
                        DialogMsg::FilePicked,
                    ),
                    None,
                );
            }
            DialogMsg::FilePicked(Some(path)) => {
                // dentro do workspace guarda o caminho relativo (portável entre máquinas do time)
                let relative = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                if self.name.trim().is_empty() {
                    self.name = relative.rsplit('/').next().unwrap_or(&relative).to_string();
                }
                self.file = relative;
            }
            DialogMsg::FilePicked(None) | DialogMsg::Noop => {}
            DialogMsg::Uri(action) => {
                let edit = action.is_edit();
                self.uri.perform(action);
                if edit {
                    self.clear_test();
                }
            }
            DialogMsg::UriResize(delta) => {
                self.uri_height = (self.uri_height + delta)
                    .clamp(theme::DB_DIALOG_URI_MIN, theme::DB_DIALOG_URI_MAX);
            }
            DialogMsg::UriResizeReset => self.uri_height = theme::DB_DIALOG_URI_HEIGHT,
            DialogMsg::SavePassword(value) => self.save_password = value,
            DialogMsg::Password(value) => self.password = value,
            DialogMsg::AllowWrites(value) => self.allow_writes = value,
            DialogMsg::VisibleToAgents(value) => self.visible_to_agents = value,
            DialogMsg::Test => {
                if self.testing {
                    return (Task::none(), None);
                }
                let Some(built) = self.try_build() else {
                    self.test = Some(Err(labels::DB_DIALOG_INVALID_URL.to_string()));
                    return (Task::none(), None);
                };
                self.testing = true;
                self.test = None;
                // edição com senha no cofre: o teste usa a guardada quando nada foi digitado
                // (a chave é o nome ORIGINAL; renomear só migra a senha ao salvar)
                let stored = self
                    .initial
                    .as_ref()
                    .filter(|c| c.save_password)
                    .map(|c| c.name.clone());
                let (root, secrets) = (root.to_path_buf(), secrets.clone());
                return (
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                test_connection(
                                    &built.connection,
                                    &root,
                                    built.password,
                                    stored,
                                    &secrets,
                                )
                            })
                            .await
                            .unwrap_or_else(|_| {
                                Some(labels::DB_DIALOG_CONNECTION_FAILED.to_string())
                            })
                        },
                        DialogMsg::Tested,
                    ),
                    None,
                );
            }
            DialogMsg::Tested(error) => {
                self.testing = false;
                self.test = Some(error.map_or(Ok(()), Err));
            }
            DialogMsg::Submit => {
                if let Some(built) = self.try_build() {
                    return (
                        Task::none(),
                        Some(Outcome::Saved {
                            connection: built.connection,
                            password: built.password,
                            previous_name: self.initial.as_ref().map(|c| c.name.clone()),
                        }),
                    );
                }
            }
            DialogMsg::Delete => {
                if let Some(initial) = self.initial.clone() {
                    return (Task::none(), Some(Outcome::Deleted(initial)));
                }
            }
            DialogMsg::Cancel => return (Task::none(), Some(Outcome::Cancelled)),
        }
        (Task::none(), None)
    }

    pub fn view(&self) -> Element<'_, DialogMsg> {
        let palette = theme::palette();
        let title = row![
            db_engine_icon::view(self.engine, theme::DB_DIALOG_TITLE_ICON),
            Space::new().width(theme::DIALOG_TITLE_GAP),
            text(if self.editing() {
                labels::DB_DIALOG_EDIT_TITLE
            } else {
                labels::DB_DIALOG_NEW_TITLE
            })
            .font(theme::FONT_TITLE)
            .size(theme::DIALOG_TITLE_SIZE)
            .color(palette.text),
        ]
        .align_y(Alignment::Center);

        let mut body = column![field(
            labels::DB_DIALOG_NAME,
            text_input(labels::DB_DIALOG_NAME_HINT, &self.name)
                .on_input(DialogMsg::Name)
                .font(theme::FONT_MONO)
                .size(theme::DB_DIALOG_INPUT_SIZE)
                .padding(theme::DB_DIALOG_INPUT_PADDING)
                .style(input_style(false))
                .into(),
        )];
        if self.engine == Engine::Sqlite {
            body = body.push(self.file_field());
        } else {
            body = body.push(self.uri_field()).push(switch_row(
                labels::DB_DIALOG_SAVE_PASSWORD,
                self.save_password,
                DialogMsg::SavePassword,
                None,
            ));
            if self.save_password {
                let hint = if self.initial.as_ref().is_some_and(|c| c.save_password) {
                    "*******"
                } else {
                    ""
                };
                body = body.push(field(
                    labels::DB_DIALOG_PASSWORD,
                    text_input(hint, &self.password)
                        .on_input(DialogMsg::Password)
                        .secure(true)
                        .font(theme::FONT_MONO)
                        .size(theme::DB_DIALOG_INPUT_SIZE)
                        .padding(theme::DB_DIALOG_INPUT_PADDING)
                        .style(input_style(false))
                        .into(),
                ));
            }
        }
        body = body
            .push(switch_row(
                labels::DB_DIALOG_ALLOW_WRITES,
                self.allow_writes,
                DialogMsg::AllowWrites,
                Some(labels::DB_DIALOG_ALLOW_WRITES_HINT),
            ))
            .push(switch_row(
                labels::DB_DIALOG_VISIBLE_TO_AGENTS,
                self.visible_to_agents,
                DialogMsg::VisibleToAgents,
                Some(labels::DB_DIALOG_VISIBLE_TO_AGENTS_HINT),
            ));
        if let Some(status) = self.test_row() {
            body = body.push(status);
        }
        let body = container(body).width(Length::Fixed(theme::DB_DIALOG_CONTENT_WIDTH));

        let can_build = self.try_build().is_some();
        let mut actions = row![]
            .spacing(theme::DIALOG_ACTIONS_GAP)
            .align_y(Alignment::Center);
        if self.editing() {
            actions = actions.push(pane_close_dialog::action(
                labels::DB_DIALOG_DELETE,
                ButtonKind::Danger,
                Some(DialogMsg::Delete),
            ));
        }
        actions = actions
            .push(Space::new().width(Fill))
            .push(pane_close_dialog::action(
                labels::DIALOG_CANCEL,
                ButtonKind::Ghost,
                Some(DialogMsg::Cancel),
            ))
            .push(pane_close_dialog::action(
                labels::DB_DIALOG_TEST,
                ButtonKind::Outline,
                (!self.testing && can_build).then_some(DialogMsg::Test),
            ))
            .push(pane_close_dialog::action(
                if self.editing() {
                    labels::DB_DIALOG_SAVE
                } else {
                    labels::DB_DIALOG_ADD
                },
                ButtonKind::Primary,
                can_build.then_some(DialogMsg::Submit),
            ));
        pane_close_dialog::form(
            title.into(),
            body.into(),
            actions.into(),
            Some(theme::DB_DIALOG_CONTENT_WIDTH),
            DialogMsg::Cancel,
            DialogMsg::Noop,
        )
    }

    /// Campo do SQLite: não se digita, o clique abre o seletor de arquivo.
    fn file_field(&self) -> Element<'_, DialogMsg> {
        let palette = theme::palette();
        let has_file = !self.file.is_empty();
        let picker = button(
            row![
                container(
                    text(if has_file {
                        self.file.as_str()
                    } else {
                        labels::DB_DIALOG_CHOOSE_FILE_PLACEHOLDER
                    })
                    .font(theme::FONT_MONO)
                    .size(theme::DB_DIALOG_INPUT_SIZE)
                    .color(if has_file {
                        palette.text
                    } else {
                        palette.text4
                    })
                    .wrapping(text::Wrapping::None)
                )
                .width(Fill)
                .clip(true),
                Space::new().width(theme::DB_DIALOG_SWITCH_GAP),
                text(labels::ICON_FOLDER_OPEN)
                    .font(theme::FONT_ICON)
                    .size(theme::DB_DIALOG_FILE_ICON)
                    .color(palette.text3),
            ]
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding(theme::DB_DIALOG_INPUT_PADDING)
        .on_press(DialogMsg::PickFile)
        .style(|_: &Theme, status| button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed)
                .then(|| Background::Color(theme::palette().panel3)),
            text_color: theme::palette().text,
            border: Border {
                color: theme::palette().border,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        });
        field(labels::DB_DIALOG_FILE, picker.into())
    }

    /// Connection string: a URL inteira, do jeito que o driver a recebe. A altura se arrasta pela
    /// alça do canto (duplo clique volta ao padrão).
    fn uri_field(&self) -> Element<'_, DialogMsg> {
        let invalid = self.url_invalid();
        let editor = text_editor(&self.uri)
            .placeholder(format!(
                "{}://user:password@host:{}/database",
                self.engine.scheme(),
                self.engine.default_port()
            ))
            .on_action(DialogMsg::Uri)
            .font(theme::FONT_MONO)
            .size(theme::DB_DIALOG_URI_SIZE)
            .padding(theme::DB_DIALOG_INPUT_PADDING)
            .height(Length::Fixed(self.uri_height))
            .style(move |_: &Theme, _| {
                let palette = theme::palette();
                text_editor::Style {
                    background: Background::Color(iced::Color::TRANSPARENT),
                    border: Border {
                        color: if invalid {
                            palette.error
                        } else {
                            palette.border
                        },
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    placeholder: palette.text4,
                    value: palette.text,
                    selection: theme::TERM_SELECTION_BG,
                }
            });
        let grip = DragHandle::new(
            container(
                canvas(Grip)
                    .width(theme::DB_DIALOG_GRIP_SIZE)
                    .height(theme::DB_DIALOG_GRIP_SIZE),
            )
            .padding(theme::DB_DIALOG_GRIP_PADDING),
            Axis::Vertical,
            DialogMsg::UriResize,
        )
        .on_reset(DialogMsg::UriResizeReset);
        let stacked = Stack::new().push(editor).push(
            container(grip)
                .width(Fill)
                .height(Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(Vertical::Bottom)
                .padding(theme::DB_DIALOG_GRIP_INSET),
        );
        let mut content = column![stacked];
        if invalid {
            content = content
                .push(Space::new().height(theme::DB_DIALOG_INVALID_GAP))
                .push(
                    text(labels::DB_DIALOG_INVALID_URL)
                        .font(theme::FONT_BODY)
                        .size(theme::DB_DIALOG_HINT_SIZE)
                        .color(theme::palette().error),
                );
        }
        field(labels::DB_DIALOG_CONNECTION_STRING, content.into())
    }

    /// Linha do resultado do teste: "Testando conexão…", "Conexão OK" ou o erro.
    fn test_row(&self) -> Option<Element<'_, DialogMsg>> {
        let palette = theme::palette();
        let (glyph, color, message) = if self.testing {
            (
                labels::ICON_MORE_HORIZ,
                palette.text3,
                labels::DB_DIALOG_TESTING,
            )
        } else {
            match &self.test {
                None => return None,
                Some(Ok(())) => (
                    labels::ICON_CHECK_CIRCLE,
                    palette.online,
                    labels::DB_DIALOG_CONNECTION_OK,
                ),
                Some(Err(message)) => (
                    labels::ICON_ERROR_OUTLINE,
                    palette.error,
                    if message.is_empty() {
                        labels::DB_DIALOG_CONNECTION_FAILED
                    } else {
                        message.as_str()
                    },
                ),
            }
        };
        Some(
            container(
                row![
                    text(glyph)
                        .font(theme::FONT_ICON)
                        .size(theme::DB_DIALOG_TEST_ICON)
                        .color(color),
                    Space::new().width(theme::DB_DIALOG_TEST_GAP),
                    text(message)
                        .font(theme::FONT_BODY)
                        .size(theme::DB_DIALOG_TEST_SIZE)
                        .color(color)
                        .width(Fill),
                ]
                .align_y(Alignment::Start),
            )
            .padding(Padding {
                top: theme::DB_DIALOG_TEST_TOP,
                ..Padding::ZERO
            })
            .into(),
        )
    }
}

/// Tira a senha do `userinfo` da URL; devolve a URL sem ela e a senha decodificada.
fn split_password(raw: &str) -> (String, Option<String>) {
    let Ok(ParsedUrl { user_info, .. }) = url::parse(raw) else {
        return (raw.to_string(), None);
    };
    let Some((user, password)) = user_info.split_once(':') else {
        return (raw.to_string(), None);
    };
    let stripped = raw.replacen(&user_info, user, 1);
    (stripped, Some(url::decode_component(password)))
}

/// Testa a conexão como a execução real: senha digitada, depois a do cofre (pelo nome original),
/// depois a embutida na URL. `None` = OK.
fn test_connection(
    connection: &Connection,
    root: &Path,
    typed: Option<String>,
    stored_name: Option<String>,
    secrets: &SecretStore,
) -> Option<String> {
    let typed = typed.filter(|password| !password.is_empty());
    let password = typed
        .or_else(|| stored_name.and_then(|name| secrets.read(&root.to_string_lossy(), &name)))
        .or_else(|| connection.url_password());
    let result =
        Endpoint::resolve(connection, root, password).and_then(|e| gsh_db::driver::ping(&e));
    result
        .err()
        .map(|error: DbError| labels::db_error_message(&error))
}

/// Rótulo em cima e o controle embaixo, com 10 de respiro.
fn field<'a>(label: &'static str, control: Element<'a, DialogMsg>) -> Element<'a, DialogMsg> {
    container(
        column![
            text(label)
                .font(theme::FONT_BODY)
                .size(theme::DB_DIALOG_LABEL_SIZE)
                .color(theme::palette().text3),
            Space::new().height(theme::DB_DIALOG_LABEL_GAP),
            control,
        ]
        .width(Fill),
    )
    .padding(Padding {
        bottom: theme::DB_DIALOG_FIELD_GAP,
        ..Padding::ZERO
    })
    .into()
}

/// Switch + rótulo (+ dica à direita), como o "Save Password".
fn switch_row<'a>(
    label: &'static str,
    value: bool,
    on_toggle: fn(bool) -> DialogMsg,
    hint: Option<&'static str>,
) -> Element<'a, DialogMsg> {
    let palette = theme::palette();
    let mut line = row![
        // altura fixa: o switch centraliza com `Fill` e se esticaria sem um limite
        container(crate::settings::view::switch(value, on_toggle(!value)))
            .height(theme::SETTINGS_SWITCH_HEIGHT),
        Space::new().width(theme::DB_DIALOG_SWITCH_GAP),
        text(label)
            .font(theme::FONT_BODY)
            .size(theme::DB_DIALOG_SWITCH_LABEL_SIZE)
            .color(palette.text2),
    ]
    .align_y(Alignment::Center);
    if let Some(hint) = hint {
        line = line
            .push(Space::new().width(theme::DB_DIALOG_SWITCH_GAP))
            .push(
                container(
                    text(hint)
                        .font(theme::FONT_BODY)
                        .size(theme::DB_DIALOG_HINT_SIZE)
                        .color(palette.text4)
                        .wrapping(text::Wrapping::None),
                )
                .width(Fill)
                .clip(true),
            );
    }
    container(line)
        .padding(Padding {
            bottom: theme::DB_DIALOG_SWITCH_ROW_BOTTOM,
            ..Padding::ZERO
        })
        .into()
}

/// Campo de uma linha: fundo transparente e borda `border` (ou `error`), raio 6.
fn input_style(invalid: bool) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, _| {
        let palette = theme::palette();
        text_input::Style {
            background: Background::Color(iced::Color::TRANSPARENT),
            border: Border {
                color: if invalid {
                    palette.error
                } else {
                    palette.border
                },
                width: 1.0,
                radius: 6.0.into(),
            },
            icon: palette.text3,
            placeholder: palette.text4,
            value: palette.text,
            selection: theme::TERM_SELECTION_BG,
        }
    }
}

/// Duas riscas diagonais no canto: a convenção de "arrastável" do textarea.
struct Grip;

impl<M> canvas::Program<M> for Grip {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let stroke = canvas::Stroke::default()
            .with_color(theme::palette().text4)
            .with_width(1.0)
            .with_line_cap(canvas::LineCap::Round);
        for inset in [0.0_f32, 4.0] {
            frame.stroke(
                &canvas::Path::line(
                    iced::Point::new(bounds.width - inset, bounds.height),
                    iced::Point::new(bounds.width, bounds.height - inset),
                ),
                stroke,
            );
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secrets() -> (tempfile::TempDir, SecretStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = SecretStore::new(dir.path().join("s.json"));
        (dir, store)
    }

    fn typed(dialog: &mut ConnectionDialog, text: &str) {
        dialog.uri = text_editor::Content::with_text(text);
    }

    #[test]
    fn sqlite_builds_without_validation_and_defaults_the_name() {
        let mut dialog = ConnectionDialog::new(Engine::Sqlite, None);
        let built = dialog.try_build().unwrap();
        assert_eq!(built.connection.name, "new-connection");
        assert_eq!(built.connection.url, "sqlite:");
        dialog.file = " data/app.db ".into();
        dialog.name = "app".into();
        dialog.allow_writes = true;
        let built = dialog.try_build().unwrap();
        assert_eq!(built.connection.url, "sqlite:data/app.db");
        assert_eq!(built.connection.access, Access::ReadWrite);
        assert!(built.connection.agents);
    }

    #[test]
    fn network_needs_a_valid_connection_string() {
        let mut dialog = ConnectionDialog::new(Engine::Postgres, None);
        assert!(dialog.try_build().is_none() && !dialog.url_invalid());
        typed(&mut dialog, "not a url");
        assert!(dialog.try_build().is_none() && dialog.url_invalid());
        typed(&mut dialog, "postgres://u@h:5432/db");
        let built = dialog.try_build().unwrap();
        assert_eq!(built.connection.engine, Engine::Postgres);
        assert_eq!(built.connection.name, "new-connection");
        assert!(built.password.is_none());
    }

    #[test]
    fn save_password_moves_the_url_password_to_the_vault() {
        let mut dialog = ConnectionDialog::new(Engine::Postgres, None);
        dialog.save_password = true;
        typed(
            &mut dialog,
            "postgres://ana:s%40t@h:5432/db?sslmode=disable",
        );
        let built = dialog.try_build().unwrap();
        assert_eq!(
            built.connection.url,
            "postgres://ana@h:5432/db?sslmode=disable"
        );
        assert!(built.connection.save_password);
        assert_eq!(built.password.as_deref(), Some("s@t"));
        // a digitada no campo vence a da URL
        dialog.password = "typed".into();
        assert_eq!(
            dialog.try_build().unwrap().password.as_deref(),
            Some("typed")
        );
        // sem "Salvar senha", a URL fica como veio (senha em claro no arquivo, escolha do usuário)
        dialog.save_password = false;
        dialog.password.clear();
        let built = dialog.try_build().unwrap();
        assert_eq!(
            built.connection.url,
            "postgres://ana:s%40t@h:5432/db?sslmode=disable"
        );
        assert!(built.password.is_none());
    }

    #[test]
    fn editing_keeps_the_ssh_block_and_reports_the_previous_name() {
        let original = Connection::from_json(
            &serde_json::json!({
                "name": "old", "url": "postgres://u@h/db", "savePassword": true,
                "ssh": {"host":"b","user":"u","keyPath":"k","port":2200,"savePassphrase":true}
            }),
            Origin::Registered,
        )
        .unwrap();
        let mut dialog = ConnectionDialog::new(Engine::Postgres, Some(original.clone()));
        dialog.name = "new".into();
        let (_dir, store) = secrets();
        let (_task, outcome) = dialog.update(DialogMsg::Submit, Path::new("/w"), &store);
        let Some(Outcome::Saved {
            connection,
            previous_name,
            ..
        }) = outcome
        else {
            panic!("esperava Saved");
        };
        assert_eq!(previous_name.as_deref(), Some("old"));
        assert_eq!(connection.name, "new");
        assert_eq!(connection.ssh, original.ssh);
    }

    #[test]
    fn delete_only_exists_when_editing_and_cancel_always() {
        let (_dir, store) = secrets();
        let mut fresh = ConnectionDialog::new(Engine::Sqlite, None);
        assert_eq!(
            fresh.update(DialogMsg::Delete, Path::new("/w"), &store).1,
            None
        );
        assert_eq!(
            fresh.update(DialogMsg::Cancel, Path::new("/w"), &store).1,
            Some(Outcome::Cancelled)
        );
    }

    #[test]
    fn picking_a_file_inside_the_workspace_stores_a_relative_path_and_names_the_connection() {
        let (_dir, store) = secrets();
        let mut dialog = ConnectionDialog::new(Engine::Sqlite, None);
        let _ = dialog.update(
            DialogMsg::FilePicked(Some(PathBuf::from("/w/data/app.db"))),
            Path::new("/w"),
            &store,
        );
        assert_eq!(dialog.file, "data/app.db");
        assert_eq!(dialog.name, "app.db");
        let _ = dialog.update(
            DialogMsg::FilePicked(Some(PathBuf::from("/elsewhere/x.db"))),
            Path::new("/w"),
            &store,
        );
        assert_eq!(dialog.file, "/elsewhere/x.db");
        assert_eq!(dialog.name, "app.db");
    }

    #[test]
    fn test_runs_the_real_ping_and_reports_the_driver_error() {
        let dir = tempfile::tempdir().unwrap();
        let (_d2, store) = secrets();
        let db = dir.path().join("t.db");
        rusqlite::Connection::open(&db).unwrap();
        let ok = Connection::sqlite("t", "t.db", Origin::Registered, Access::Read, true);
        assert_eq!(test_connection(&ok, dir.path(), None, None, &store), None);
        let missing = Connection::sqlite("m", "none.db", Origin::Registered, Access::Read, true);
        assert!(test_connection(&missing, dir.path(), None, None, &store).is_some());
    }

    #[test]
    fn view_builds_in_every_mode() {
        for (engine, initial) in [(Engine::Sqlite, None), (Engine::Postgres, None)] {
            let mut dialog = ConnectionDialog::new(engine, initial);
            dialog.test = Some(Err("boom".into()));
            let _ = dialog.view();
            dialog.testing = true;
            let _ = dialog.view();
        }
    }
}
