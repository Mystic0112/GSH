//! Pane de consulta SQL (PostgreSQL): string de conexão, editor de SQL e grade de resultado.
//!
//! A senha fica só na memória desta sessão: não é gravada em disco.

use gsh_db::{DbError, QueryResult};
use iced::widget::{button, column, container, row, scrollable, text, text_editor, text_input};
use iced::{Element, Fill, Task};

use crate::labels;
use crate::theme;

/// Estado da última execução.
#[derive(Debug, Clone)]
enum Status {
    Idle,
    Running,
    Done(QueryResult),
    Failed(DbError),
}

/// Mensagens do pane de banco.
#[derive(Debug, Clone)]
pub enum Message {
    ConnChanged(String),
    Edit(text_editor::Action),
    Run,
    Finished(Result<QueryResult, DbError>),
}

/// Pane de consulta. Uma instância por aplicativo.
pub struct DbPane {
    conn: String,
    editor: text_editor::Content,
    status: Status,
}

impl Default for DbPane {
    fn default() -> Self {
        Self {
            conn: String::new(),
            editor: text_editor::Content::new(),
            status: Status::Idle,
        }
    }
}

impl DbPane {
    /// Aplica uma mensagem; `Run` dispara a consulta fora da thread de UI.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConnChanged(value) => {
                self.conn = value;
                Task::none()
            }
            Message::Edit(action) => {
                self.editor.perform(action);
                Task::none()
            }
            Message::Run => {
                if matches!(self.status, Status::Running) {
                    return Task::none();
                }
                let conn = self.conn.clone();
                let sql = self.editor.text();
                self.status = Status::Running;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || gsh_db::run_query(&conn, &sql))
                            .await
                            .unwrap_or_else(|_| Err(DbError::Runtime(String::from("join"))))
                    },
                    Message::Finished,
                )
            }
            Message::Finished(result) => {
                self.status = match result {
                    Ok(query) => Status::Done(query),
                    Err(error) => Status::Failed(error),
                };
                Task::none()
            }
        }
    }

    /// Renderiza o pane: conexão, editor de SQL e resultado.
    pub fn view(&self) -> Element<'_, Message> {
        let connection = row![
            text_input(labels::DB_CONN_PLACEHOLDER, &self.conn)
                .on_input(Message::ConnChanged)
                .secure(true)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_LABEL)
                .width(Fill),
            button(text(labels::DB_RUN).size(theme::SIZE_LABEL)).on_press(Message::Run),
        ]
        .spacing(8);

        let editor = text_editor(&self.editor)
            .placeholder(labels::DB_SQL_PLACEHOLDER)
            .on_action(Message::Edit)
            .font(theme::FONT_MONO)
            .size(theme::SIZE_LABEL)
            .height(160);

        column![
            connection,
            editor,
            container(self.view_result()).height(Fill).width(Fill)
        ]
        .spacing(10)
        .padding(12)
        .height(Fill)
        .width(Fill)
        .into()
    }

    fn view_result(&self) -> Element<'_, Message> {
        match &self.status {
            Status::Idle => status_line(labels::DB_IDLE),
            Status::Running => status_line(labels::DB_RUNNING),
            Status::Failed(error) => status_line(&error_label(error)),
            Status::Done(query) => view_grid(query),
        }
    }
}

fn status_line<'a>(message: &str) -> Element<'a, Message> {
    text(message.to_string())
        .font(theme::FONT_BODY)
        .size(theme::SIZE_LABEL)
        .color(theme::TEXT3)
        .into()
}

fn error_label(error: &DbError) -> String {
    match error {
        DbError::InvalidConfig => labels::DB_ERR_INVALID.to_string(),
        DbError::Connect(_) => labels::DB_ERR_CONNECT.to_string(),
        DbError::Query(detail) => format!("{}: {detail}", labels::DB_ERR_QUERY),
        DbError::Timeout => labels::DB_ERR_TIMEOUT.to_string(),
        DbError::Runtime(_) => labels::DB_ERR_RUNTIME.to_string(),
    }
}

fn view_grid(query: &QueryResult) -> Element<'_, Message> {
    if query.columns.is_empty() {
        let summary = match query.affected {
            Some(n) => format!("{} {n}", labels::DB_ROWS_AFFECTED),
            None => labels::DB_NO_ROWS.to_string(),
        };
        return status_line(&summary);
    }

    let header = grid_row(query.columns.iter().map(String::as_str), theme::TEXT);
    let body = query.rows.iter().map(|cells| {
        grid_row(
            cells
                .iter()
                .map(|cell| cell.as_deref().unwrap_or(labels::DB_NULL)),
            theme::TEXT2,
        )
    });

    let mut table = column![header].spacing(2);
    for line in body {
        table = table.push(line);
    }
    if query.truncated {
        table = table.push(status_line(labels::DB_TRUNCATED));
    }
    scrollable(table).width(Fill).height(Fill).into()
}

fn grid_row<'a, I>(cells: I, color: iced::Color) -> Element<'a, Message>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut line = row![].spacing(12);
    for cell in cells {
        line = line.push(
            text(cell.as_ref().to_string())
                .font(theme::FONT_MONO)
                .size(theme::SIZE_LABEL)
                .color(color)
                .width(Fill),
        );
    }
    line.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resultado_com_erro_vira_estado_falho_e_rotulo_tipado() {
        let mut pane = DbPane::default();
        let _ = pane.update(Message::Finished(Err(DbError::Connect(String::from("x")))));
        assert!(matches!(pane.status, Status::Failed(DbError::Connect(_))));
        assert_eq!(
            error_label(&DbError::Connect(String::new())),
            labels::DB_ERR_CONNECT
        );
        assert_eq!(error_label(&DbError::Timeout), labels::DB_ERR_TIMEOUT);
    }

    #[test]
    fn editar_conexao_nao_dispara_consulta() {
        let mut pane = DbPane::default();
        let _ = pane.update(Message::ConnChanged(String::from("postgres://u@h/b")));
        assert_eq!(pane.conn, "postgres://u@h/b");
        assert!(matches!(pane.status, Status::Idle));
    }
}
