use std::{collections::BTreeSet, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    channels_match, valid_channel_name, CHANNEL_DEFAULT, CHANNEL_PERSONAL, CHANNEL_PROJECTS,
    CHANNEL_WORK,
};

mod list;

const DEFAULT_CHANNEL_ID: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelId(String);

impl ChannelId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(super) fn default_channel() -> Self {
        Self(DEFAULT_CHANNEL_ID.to_owned())
    }

    fn is_valid(&self) -> bool {
        !self.0.is_empty()
            && self.0.len() <= 80
            && self
                .0
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    }
}

/// A saved workspace is located by its folder, or by its system ID when it has no folder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRef {
    Project(PathBuf),
    System(String),
}

/// Membership references an existing harness session, including its workspace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ChannelMember {
    Session {
        workspace: WorkspaceRef,
        session_id: String,
        label: String,
    },
}

impl ChannelMember {
    pub fn label(&self) -> &str {
        match self {
            Self::Session { label, .. } => label,
        }
    }

    pub fn same_session(&self, other: &Self) -> bool {
        self.session_identity() == other.session_identity()
    }

    fn session_identity(&self) -> (&WorkspaceRef, &str) {
        match self {
            Self::Session {
                workspace,
                session_id,
                ..
            } => (workspace, session_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChannelReplyRule {
    #[default]
    Manual,
    Mention,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ChannelDraft {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub instructions: String,
    #[serde(default)]
    pub reply_rule: ChannelReplyRule,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<ChannelMember>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    pub id: ChannelId,
    #[serde(flatten)]
    pub draft: ChannelDraft,
    #[serde(default)]
    pub archived: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) last_workspace: Option<WorkspaceRef>,
}

impl Channel {
    pub fn name(&self) -> &str {
        &self.draft.name
    }

    pub fn can_archive(&self) -> bool {
        self.id.as_str() != DEFAULT_CHANNEL_ID
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelState {
    #[serde(default)]
    pub(super) channels: Vec<Channel>,
    #[serde(default)]
    pub(super) active: Option<ChannelId>,
    #[serde(default)]
    pub(super) assignments: Vec<(WorkspaceRef, ChannelId)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ChannelError {
    #[error("Nome de canal inválido")]
    InvalidName,
    #[error("Já existe um canal com esse nome")]
    DuplicateName,
    #[error("A pasta do canal deve ser um caminho absoluto válido")]
    InvalidFolder,
    #[error("Instruções do canal inválidas")]
    InvalidInstructions,
    #[error("Lista de membros do canal inválida")]
    InvalidMembers,
    #[error("Nome de menção inválido")]
    InvalidMemberLabel,
    #[error("Os nomes de menção precisam ser diferentes")]
    DuplicateMemberLabel,
    #[error("Canal desconhecido")]
    UnknownChannel,
    #[error("Canal arquivado")]
    Archived,
    #[error("O canal padrão não pode ser arquivado")]
    DefaultArchive,
    #[error("Workspace desconhecido")]
    UnknownWorkspace,
}

pub(super) fn validate_draft(mut draft: ChannelDraft) -> Result<ChannelDraft, ChannelError> {
    if !valid_channel_name(&draft.name) {
        return Err(ChannelError::InvalidName);
    }
    draft.name = draft.name.trim().to_owned();
    if draft
        .folder
        .as_ref()
        .is_some_and(|path| !path.is_absolute() || path.as_os_str().as_encoded_bytes().contains(&0))
    {
        return Err(ChannelError::InvalidFolder);
    }
    if draft.instructions.len() > 32 * 1024 || draft.instructions.contains('\0') {
        return Err(ChannelError::InvalidInstructions);
    }
    if draft.members.len() > 256 {
        return Err(ChannelError::InvalidMembers);
    }
    for member in &mut draft.members {
        let ChannelMember::Session {
            workspace,
            session_id,
            label,
        } = member;
        let valid_workspace = match workspace {
            WorkspaceRef::Project(path) => {
                path.is_absolute() && !path.as_os_str().as_encoded_bytes().contains(&0)
            }
            WorkspaceRef::System(id) => valid_id(id),
        };
        if !valid_workspace || !valid_id(session_id) {
            return Err(ChannelError::InvalidMembers);
        }
        if !valid_channel_name(label) || label.contains('@') {
            return Err(ChannelError::InvalidMemberLabel);
        }
        *label = label.trim().to_owned();
    }
    let mut sessions = BTreeSet::new();
    draft.members.retain(|member| {
        let (workspace, session_id) = member.session_identity();
        sessions.insert((workspace.clone(), session_id.to_owned()))
    });
    let mut labels = BTreeSet::new();
    if !draft
        .members
        .iter()
        .all(|member| labels.insert(member.label().to_lowercase()))
    {
        return Err(ChannelError::DuplicateMemberLabel);
    }
    Ok(draft)
}

fn valid_id(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= 256 && !id.chars().any(char::is_control)
}

pub(super) fn preset_channels() -> Vec<Channel> {
    [
        (DEFAULT_CHANNEL_ID, CHANNEL_DEFAULT),
        ("work", CHANNEL_WORK),
        ("personal", CHANNEL_PERSONAL),
        ("projects", CHANNEL_PROJECTS),
    ]
    .into_iter()
    .map(|(id, name)| {
        new_channel(
            ChannelId(id.to_owned()),
            ChannelDraft {
                name: name.to_owned(),
                ..ChannelDraft::default()
            },
        )
    })
    .collect()
}

fn create_channel(channels: &[Channel], draft: ChannelDraft) -> Channel {
    let mut next = channels.len().saturating_add(1);
    loop {
        let id = ChannelId(format!("channel-{next}"));
        if channels.iter().all(|channel| channel.id != id) {
            return new_channel(id, draft);
        }
        next += 1;
    }
}

fn new_channel(id: ChannelId, draft: ChannelDraft) -> Channel {
    Channel {
        id,
        draft,
        archived: false,
        last_workspace: None,
    }
}

fn restore_channels(saved: Vec<Channel>) -> Vec<Channel> {
    let mut channels: Vec<Channel> = Vec::new();
    for mut channel in saved {
        if !channel.id.is_valid() || channels.iter().any(|item| item.id == channel.id) {
            continue;
        }
        let Ok(draft) = validate_draft(channel.draft) else {
            continue;
        };
        if channels
            .iter()
            .any(|item| channels_match(item.name(), &draft.name))
        {
            continue;
        }
        channel.draft = draft;
        if !channel.can_archive() {
            channel.archived = false;
        }
        channels.push(channel);
    }
    for preset in preset_channels() {
        if channels.iter().any(|item| item.id == preset.id) {
            continue;
        }
        // A conflicting record cannot replace the stable ID of a missing preset.
        if !channels
            .iter()
            .any(|item| channels_match(item.name(), preset.name()))
        {
            channels.push(preset);
        } else if !preset.can_archive() {
            channels.retain(|item| !channels_match(item.name(), preset.name()));
            channels.push(preset);
        }
    }
    channels
}
