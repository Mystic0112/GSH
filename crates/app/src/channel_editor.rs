use std::fmt;

use gsh_core::workspace::{
    valid_channel_name, Channel, ChannelDraft, ChannelId, ChannelMember, ChannelReplyRule,
    WorkspaceList,
};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, text, text_editor, text_input, tooltip,
};
use iced::{Alignment, Border, Element, Fill};

use crate::{labels, pane_chrome, theme};

#[derive(Debug, Clone)]
pub enum ChannelAction {
    Select(Option<ChannelId>),
    New,
    Edit(ChannelId),
    Close,
    Name(String),
    Instructions(text_editor::Action),
    PickFolder,
    ClearFolder,
    ToggleMember(ChannelMember),
    ReplyRule(ChannelReplyRule),
    Save,
    SetArchived(ChannelId, bool),
    AssignWorkspace(ChannelId),
}

pub struct ChannelEditor {
    pub id: Option<ChannelId>,
    pub draft: ChannelDraft,
    pub instructions: text_editor::Content,
    pub error: Option<String>,
}

impl ChannelEditor {
    pub fn new(channel: Option<&Channel>) -> Self {
        let draft = channel
            .map(|channel| channel.draft.clone())
            .unwrap_or_default();
        Self {
            id: channel.map(|channel| channel.id.clone()),
            instructions: text_editor::Content::with_text(&draft.instructions),
            draft,
            error: None,
        }
    }

    pub fn to_draft(&self) -> ChannelDraft {
        ChannelDraft {
            instructions: self.instructions.text(),
            ..self.draft.clone()
        }
    }
}

pub struct ChannelMemberOption {
    pub member: ChannelMember,
    pub label: String,
}

pub struct ChannelActions<'a, M> {
    pub editor: Option<&'a ChannelEditor>,
    pub members: Vec<ChannelMemberOption>,
    pub on_action: fn(ChannelAction) -> M,
}

#[derive(Clone, PartialEq)]
struct Choice {
    id: ChannelId,
    label: String,
}

impl From<&Channel> for Choice {
    fn from(channel: &Channel) -> Self {
        Self {
            id: channel.id.clone(),
            label: if channel.archived {
                labels::channel_archived_label(channel.name())
            } else {
                channel.name().to_owned()
            },
        }
    }
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Clone, PartialEq)]
struct Filter(Option<Choice>);

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(choice) => choice.fmt(f),
            None => f.write_str(labels::CHANNELS_ALL),
        }
    }
}

pub(super) fn card<'a, M: Clone + 'a>(
    workspaces: &'a WorkspaceList,
    actions: ChannelActions<'a, M>,
) -> Element<'a, M> {
    let action = actions.on_action;
    let mut channels: Vec<_> = workspaces.context_channels().iter().collect();
    channels.sort_by_cached_key(|channel| (channel.archived, channel.name().to_lowercase()));

    let filters: Vec<_> = std::iter::once(Filter(None))
        .chain(
            channels
                .iter()
                .filter(|channel| !channel.archived)
                .map(|channel| Filter(Some(Choice::from(*channel)))),
        )
        .collect();
    let selected = Filter(workspaces.active_context_channel().map(Choice::from));
    let filter = pick_list(filters, Some(selected), move |filter| {
        action(ChannelAction::Select(filter.0.map(|choice| choice.id)))
    })
    .width(Fill)
    .padding([5, 7])
    .font(theme::FONT_BODY)
    .text_size(theme::SIZE_LABEL);

    let mut content = column![].spacing(6);
    if let Some(editor) = actions.editor {
        content = content.push(heading(labels::CHANNELS_HEADER)).push(form(
            workspaces,
            editor,
            &actions.members,
            action,
        ));
    } else {
        content = content
            .push(
                row![
                    heading(labels::CHANNELS_HEADER).width(Fill),
                    control(labels::CHANNEL_NEW, action(ChannelAction::New))
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .push(filter);
        let choices: Vec<_> = channels
            .iter()
            .map(|channel| Choice::from(*channel))
            .collect();
        content = content.push(channel_picker(
            choices,
            labels::CHANNEL_EDIT_PICKER,
            move |choice| action(ChannelAction::Edit(choice.id)),
        ));
        if let Some(workspace) = workspaces.active() {
            let choices: Vec<_> = channels
                .iter()
                .filter(|channel| !channel.archived && workspace.channel_id() != Some(&channel.id))
                .map(|channel| Choice::from(*channel))
                .collect();
            if !choices.is_empty() {
                content = content.push(tooltip(
                    channel_picker(choices, labels::CHANNEL_MOVE_WORKSPACE, move |choice| {
                        action(ChannelAction::AssignWorkspace(choice.id))
                    }),
                    pane_chrome::tip_bubble(labels::channel_workspace_tooltip(&workspace.name)),
                    tooltip::Position::Bottom,
                ));
            }
        }
    }

    container(container(content).padding(8).width(Fill).style(|_| {
        let p = theme::palette();
        container::Style::default()
            .background(p.panel)
            .border(Border {
                color: p.border,
                width: 1.0,
                radius: theme::RADIUS_CARD.into(),
            })
    }))
    .padding([0, 8])
    .width(Fill)
    .into()
}

fn form<'a, M: Clone + 'a>(
    workspaces: &WorkspaceList,
    editor: &'a ChannelEditor,
    members: &[ChannelMemberOption],
    action: fn(ChannelAction) -> M,
) -> Element<'a, M> {
    let edited = editor.id.as_ref().and_then(|id| workspaces.channel(id));
    let title = match edited {
        Some(channel) if channel.archived => labels::CHANNEL_EDIT_ARCHIVED,
        Some(_) => labels::CHANNEL_EDIT,
        None => labels::CHANNEL_NEW,
    };
    let save = valid_channel_name(&editor.draft.name).then(|| action(ChannelAction::Save));
    let name = text_input(labels::CHANNEL_NAME_PLACEHOLDER, &editor.draft.name)
        .size(theme::SIZE_LABEL)
        .font(theme::FONT_BODY)
        .padding([5, 7])
        .on_input(move |value| action(ChannelAction::Name(value)))
        .on_submit_maybe(save.clone());
    let instructions = text_editor(&editor.instructions)
        .placeholder(labels::CHANNEL_INSTRUCTIONS_PLACEHOLDER)
        .height(theme::SIZE_LABEL * 6.0)
        .size(theme::SIZE_LABEL)
        .font(theme::FONT_BODY)
        .on_action(move |edit| action(ChannelAction::Instructions(edit)));
    let mut form = column![
        heading(title),
        column![heading(labels::CHANNEL_NAME), name].spacing(3),
        folder_field(editor, action),
        column![heading(labels::CHANNEL_INSTRUCTIONS), instructions].spacing(3),
        member_field(editor, members, action),
        reply_field(editor.draft.reply_rule, action),
    ]
    .spacing(8);
    if let Some(error) = &editor.error {
        form = form.push(label(error).color(theme::palette().error));
    }
    let save = button(label(labels::CHANNEL_SAVE))
        .padding([5, 9])
        .on_press_maybe(save);
    form = form.push(
        row![
            control(labels::CHANNEL_CANCEL, action(ChannelAction::Close)),
            container(save).align_right(Fill),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    );
    if let Some(channel) = edited.filter(|channel| channel.can_archive()) {
        let label = if channel.archived {
            labels::CHANNEL_UNARCHIVE
        } else {
            labels::CHANNEL_ARCHIVE
        };
        form = form.push(control(
            label,
            action(ChannelAction::SetArchived(
                channel.id.clone(),
                !channel.archived,
            )),
        ));
    }
    form.into()
}

fn folder_field<'a, M: Clone + 'a>(
    editor: &'a ChannelEditor,
    action: fn(ChannelAction) -> M,
) -> Element<'a, M> {
    let folder = editor.draft.folder.as_deref();
    let field = folder.map_or_else(
        || labels::CHANNEL_NO_FOLDER.to_owned(),
        |path| path.display().to_string(),
    );
    let mut buttons = row![control(
        labels::CHANNEL_PICK_FOLDER,
        action(ChannelAction::PickFolder)
    )];
    if folder.is_some() {
        buttons = buttons.push(control(
            labels::CHANNEL_REMOVE_FOLDER,
            action(ChannelAction::ClearFolder),
        ));
    }
    column![
        heading(labels::CHANNEL_FOLDER),
        label(field).width(Fill),
        buttons.spacing(4)
    ]
    .spacing(3)
    .into()
}

fn member_field<'a, M: Clone + 'a>(
    editor: &ChannelEditor,
    members: &[ChannelMemberOption],
    action: fn(ChannelAction) -> M,
) -> Element<'a, M> {
    let mut field = column![heading(labels::CHANNEL_MEMBERS)].spacing(4);
    if members.is_empty() && editor.draft.members.is_empty() {
        return field.push(label(labels::CHANNEL_NO_SESSIONS)).into();
    }
    for option in members {
        let saved = editor
            .draft
            .members
            .iter()
            .find(|member| member.same_session(&option.member));
        let member = saved.unwrap_or(&option.member);
        field = field.push(member_checkbox(
            labels::channel_member_label(member.label(), &option.label),
            member,
            saved.is_some(),
            action,
        ));
    }
    for member in &editor.draft.members {
        if !members
            .iter()
            .any(|option| member.same_session(&option.member))
        {
            field = field.push(member_checkbox(
                labels::channel_unavailable_member_label(member.label()),
                member,
                true,
                action,
            ));
        }
    }
    field.into()
}

fn member_checkbox<'a, M: Clone + 'a>(
    name: String,
    member: &ChannelMember,
    selected: bool,
    action: fn(ChannelAction) -> M,
) -> Element<'a, M> {
    let member = member.clone();
    checkbox(selected)
        .label(name)
        .text_size(theme::SIZE_LABEL)
        .font(theme::FONT_BODY)
        .width(Fill)
        .size(theme::SIZE_LABEL)
        .on_toggle(move |_| action(ChannelAction::ToggleMember(member.clone())))
        .into()
}

fn reply_field<'a, M: Clone + 'a>(
    rule: ChannelReplyRule,
    action: fn(ChannelAction) -> M,
) -> Element<'a, M> {
    use ChannelReplyRule::{All, Manual, Mention};

    let picker = pick_list(
        [Manual, Mention, All].map(ReplyChoice),
        Some(ReplyChoice(rule)),
        move |choice| action(ChannelAction::ReplyRule(choice.0)),
    )
    .width(Fill)
    .padding([5, 7])
    .font(theme::FONT_BODY)
    .text_size(theme::SIZE_LABEL);
    column![heading(labels::CHANNEL_REPLY_RULE), picker]
        .spacing(3)
        .into()
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ReplyChoice(ChannelReplyRule);

impl fmt::Display for ReplyChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            ChannelReplyRule::Manual => labels::CHANNEL_REPLY_MANUAL,
            ChannelReplyRule::Mention => labels::CHANNEL_REPLY_MENTION,
            ChannelReplyRule::All => labels::CHANNEL_REPLY_ALL,
        })
    }
}

fn channel_picker<'a, M: Clone + 'a>(
    choices: Vec<Choice>,
    prompt: &'static str,
    on_selected: impl Fn(Choice) -> M + 'a,
) -> Element<'a, M> {
    pick_list(choices, None::<Choice>, on_selected)
        .placeholder(prompt)
        .width(Fill)
        .padding([5, 7])
        .font(theme::FONT_BODY)
        .text_size(theme::SIZE_LABEL)
        .into()
}

fn control<'a, M: Clone + 'a>(title: &'static str, action: M) -> Element<'a, M> {
    button(label(title))
        .padding([5, 7])
        .style(theme::ghost_button)
        .on_press(action)
        .into()
}

fn label<'a>(value: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(value).font(theme::FONT_BODY).size(theme::SIZE_LABEL)
}

fn heading<'a>(value: &'static str) -> iced::widget::Text<'a> {
    label(value).font(theme::FONT_BODY_SEMIBOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_editor_defaults_to_manual_and_preserves_multiline_instructions() {
        let mut editor = ChannelEditor::new(None);
        assert_eq!(editor.draft.reply_rule, ChannelReplyRule::Manual);

        let instructions = "Resuma as alterações.\nListe os testes executados.";
        editor.instructions = text_editor::Content::with_text(instructions);
        assert_eq!(editor.to_draft().instructions, instructions);
    }
}
