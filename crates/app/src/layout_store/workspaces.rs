use gsh_core::workspace::{ChannelId, Workspace, WorkspaceList, WorkspaceRef};
use serde::{Deserialize, Serialize};

use super::LayoutDoc;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<ChannelId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct SystemWorkspace {
    id: String,
    name: String,
    git_branch: Option<String>,
    realm: String,
    index: usize,
    active: bool,
}

impl LayoutDoc {
    pub fn save_workspaces(&mut self, list: &WorkspaceList) {
        self.workspaces.clear();
        self.active_workspace = 0;
        self.workspace_names.clear();
        self.workspace_realms.clear();
        self.system_workspaces.clear();
        for (index, workspace) in list.items().iter().enumerate() {
            let active = index == list.selected_index();
            if let Some(path) = &workspace.path {
                if active {
                    self.active_workspace = self.workspaces.len();
                }
                self.workspaces.push(path.clone());
                self.workspace_names
                    .insert(path.clone(), workspace.name.clone());
                self.workspace_realms
                    .insert(path.clone(), workspace.realm().to_owned());
            } else {
                self.system_workspaces.push(SystemWorkspace {
                    id: workspace.id.clone(),
                    name: workspace.name.clone(),
                    git_branch: workspace.git_branch.clone(),
                    realm: workspace.realm().to_owned(),
                    index,
                    active,
                });
            }
        }
        self.active_channel = list.active_channel().map(str::to_owned);
        self.channel_state = Some(list.channel_state());
    }

    pub fn restore_workspaces(&self) -> WorkspaceList {
        let mut workspaces: Vec<_> = self
            .workspaces
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let mut workspace = Workspace::from_path(format!("ws-{}", index + 1), path);
                if let Some(name) = self.workspace_names.get(path) {
                    workspace.name.clone_from(name);
                }
                if let Some(realm) = self.workspace_realms.get(path) {
                    workspace.set_realm(realm);
                }
                workspace
            })
            .collect();
        let mut active_id = workspaces
            .get(self.active_workspace)
            .map(|workspace| workspace.id.clone());
        for system in &self.system_workspaces {
            // The workspace list reserves "ws-" IDs for local projects.
            if system.id.is_empty()
                || system.id.starts_with("ws-")
                || workspaces.iter().any(|workspace| workspace.id == system.id)
            {
                continue;
            }
            let index = system.index.min(workspaces.len());
            workspaces.insert(
                index,
                Workspace::system(&system.id, &system.name, system.git_branch.clone())
                    .with_realm(&system.realm),
            );
            if system.active {
                active_id = Some(system.id.clone());
            }
        }
        let active = active_id
            .and_then(|id| workspaces.iter().position(|workspace| workspace.id == id))
            .unwrap_or(0);
        let mut list = WorkspaceList::new(workspaces);
        let _ = list.select(active);
        if let Some(state) = &self.channel_state {
            list.restore_channel_state(state.clone());
        } else {
            list.set_active_channel(self.active_channel.clone());
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use gsh_core::workspace::{ChannelDraft, ChannelMember, ChannelReplyRule};

    use super::*;

    #[test]
    fn channel_metadata_and_shared_session_reference_survive_a_layout_roundtrip() {
        let first = Workspace::from_path("a", "/tmp/channel-a");
        let second = Workspace::system("system", "Terminal", None);
        let member = ChannelMember::Session {
            workspace: first.reference(),
            session_id: "existing-session".into(),
            label: "Ferro".into(),
        };
        let mut list = WorkspaceList::new(vec![first, second]);
        let draft = ChannelDraft {
            name: "Contexto A".into(),
            folder: Some("/tmp/channel-a".into()),
            instructions: "Instruções do canal".into(),
            reply_rule: ChannelReplyRule::Mention,
            members: vec![member.clone()],
        };
        let a = list.create_channel(draft.clone()).unwrap();
        let b = list
            .create_channel(ChannelDraft {
                name: "Contexto B".into(),
                ..draft.clone()
            })
            .unwrap();
        list.assign_workspace_channel(0, &a).unwrap();
        list.assign_workspace_channel(1, &b).unwrap();
        list.set_channel_archived(&b, true).unwrap();
        list.select_context_channel(Some(&a)).unwrap();
        list.update_channel(
            &a,
            ChannelDraft {
                name: "Renomeado".into(),
                ..draft
            },
        )
        .unwrap();

        let mut doc = crate::pane_tree::PaneTree::default().snapshot();
        doc.save_workspaces(&list);
        let chat_context = SessionContext {
            channel: Some(a.clone()),
            workspace: Some(list.items()[0].reference()),
        };
        let terminal_context = SessionContext {
            channel: Some(b.clone()),
            workspace: Some(list.items()[1].reference()),
        };
        doc.chat_contexts.insert(7, chat_context.clone());
        doc.terminal_contexts.insert(1, terminal_context.clone());
        assert_eq!(doc.active_channel.as_deref(), Some("Renomeado"));
        let encoded = serde_json::to_vec(&doc).unwrap();
        let decoded: LayoutDoc = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.chat_contexts.get(&7), Some(&chat_context));
        assert_eq!(decoded.terminal_contexts.get(&1), Some(&terminal_context));
        let restored = decoded.restore_workspaces();
        assert_eq!(restored.active_context_channel().unwrap().id, a);
        assert_eq!(restored.active().unwrap().channel_id(), Some(&a));
        assert_eq!(restored.items()[1].channel_id(), Some(&b));
        assert!(restored.channel(&b).unwrap().archived);
        assert_eq!(
            restored.channel(&a).unwrap().draft.members,
            vec![member.clone()]
        );
        assert_eq!(restored.channel(&b).unwrap().draft.members, vec![member]);
        assert_eq!(
            restored.channel(&a).unwrap().draft.reply_rule,
            ChannelReplyRule::Mention
        );

        let mut legacy = serde_json::to_value(&doc).unwrap();
        legacy.as_object_mut().unwrap().remove("channel_state");
        legacy.as_object_mut().unwrap().remove("chat_contexts");
        legacy.as_object_mut().unwrap().remove("terminal_contexts");
        let legacy: LayoutDoc = serde_json::from_value(legacy).unwrap();
        assert!(legacy.chat_contexts.is_empty());
        assert!(legacy.terminal_contexts.is_empty());
        let restored = legacy.restore_workspaces();
        assert_eq!(restored.active_channel(), Some("Renomeado"));
        assert_eq!(restored.items()[1].realm(), "Contexto B");

        let empty = list
            .create_channel(ChannelDraft {
                name: "Vazio".into(),
                ..ChannelDraft::default()
            })
            .unwrap();
        list.select_context_channel(Some(&empty)).unwrap();
        doc.save_workspaces(&list);
        let restored = doc.restore_workspaces();
        assert_eq!(restored.active_context_channel().unwrap().id, empty);
        assert!(restored.active().is_none());
    }
}
