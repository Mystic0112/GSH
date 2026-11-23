use std::collections::BTreeMap;

use crate::workspace::{channels_match, normalize_realm, Workspace, WorkspaceList};

use super::{
    create_channel, restore_channels, validate_draft, Channel, ChannelDraft, ChannelError,
    ChannelId, ChannelState, WorkspaceRef,
};

impl WorkspaceList {
    pub fn context_channels(&self) -> &[Channel] {
        &self.channels
    }

    pub fn channel(&self, id: &ChannelId) -> Option<&Channel> {
        self.channels.iter().find(|item| item.id == *id)
    }

    pub fn active_context_channel(&self) -> Option<&Channel> {
        self.active_channel.as_ref().and_then(|id| self.channel(id))
    }

    pub fn select_context_channel(&mut self, id: Option<&ChannelId>) -> Result<(), ChannelError> {
        if let Some(id) = id {
            let channel = self.channel(id).ok_or(ChannelError::UnknownChannel)?;
            if channel.archived {
                return Err(ChannelError::Archived);
            }
        }
        self.active_channel = id.cloned();
        self.select_channel_workspace();
        Ok(())
    }

    pub fn create_channel(&mut self, draft: ChannelDraft) -> Result<ChannelId, ChannelError> {
        let draft = self.check_draft(draft, None)?;
        let channel = create_channel(&self.channels, draft);
        let id = channel.id.clone();
        self.channels.push(channel);
        Ok(id)
    }

    pub fn update_channel(
        &mut self,
        id: &ChannelId,
        draft: ChannelDraft,
    ) -> Result<(), ChannelError> {
        let draft = self.check_draft(draft, Some(id))?;
        let channel = self
            .channels
            .iter_mut()
            .find(|item| item.id == *id)
            .ok_or(ChannelError::UnknownChannel)?;
        for workspace in &mut self.items {
            if workspace.channel_id.as_ref() == Some(id) {
                workspace.realm.clone_from(&draft.name);
            }
        }
        channel.draft = draft;
        Ok(())
    }

    pub fn set_channel_archived(
        &mut self,
        id: &ChannelId,
        archived: bool,
    ) -> Result<(), ChannelError> {
        let channel = self
            .channels
            .iter_mut()
            .find(|item| item.id == *id)
            .ok_or(ChannelError::UnknownChannel)?;
        if archived && !channel.can_archive() {
            return Err(ChannelError::DefaultArchive);
        }
        channel.archived = archived;
        if archived && self.active_channel.as_ref() == Some(id) {
            self.select_context_channel(None)?;
        }
        Ok(())
    }

    pub fn assign_workspace_channel(
        &mut self,
        index: usize,
        id: &ChannelId,
    ) -> Result<(), ChannelError> {
        let channel = self
            .channels
            .iter_mut()
            .find(|item| item.id == *id)
            .ok_or(ChannelError::UnknownChannel)?;
        if channel.archived {
            return Err(ChannelError::Archived);
        }
        let workspace = self
            .items
            .get_mut(index)
            .ok_or(ChannelError::UnknownWorkspace)?;
        if workspace.channel_id.as_ref() == Some(id) {
            return Ok(());
        }
        workspace.realm.clone_from(&channel.draft.name);
        workspace.channel_id = Some(id.clone());
        if self.selected == index {
            channel.last_workspace = Some(workspace.reference());
            self.select_channel_workspace();
        }
        Ok(())
    }

    pub fn channel_state(&self) -> ChannelState {
        ChannelState {
            channels: self.channels.clone(),
            active: self.active_channel.clone(),
            assignments: self
                .items
                .iter()
                .filter_map(|workspace| {
                    Some((workspace.reference(), workspace.channel_id.clone()?))
                })
                .collect(),
        }
    }

    pub fn restore_channel_state(&mut self, state: ChannelState) {
        self.channels = restore_channels(state.channels);
        let assignments: BTreeMap<_, _> = state.assignments.into_iter().collect();
        self.restore_assignments(&assignments);
        if self.select_context_channel(state.active.as_ref()).is_err() {
            let _ = self.select_context_channel(None);
        }
    }

    fn check_draft(
        &self,
        draft: ChannelDraft,
        current: Option<&ChannelId>,
    ) -> Result<ChannelDraft, ChannelError> {
        let draft = validate_draft(draft)?;
        if self
            .channels
            .iter()
            .any(|item| Some(&item.id) != current && channels_match(item.name(), &draft.name))
        {
            return Err(ChannelError::DuplicateName);
        }
        Ok(draft)
    }

    pub(in crate::workspace) fn workspace_visible(&self, workspace: &Workspace) -> bool {
        self.active_channel
            .as_ref()
            .is_none_or(|id| workspace.channel_id.as_ref() == Some(id))
    }

    fn select_channel_workspace(&mut self) {
        let preferred = self
            .active_context_channel()
            .and_then(|channel| channel.last_workspace.as_ref());
        let target = preferred
            .and_then(|workspace| {
                self.items
                    .iter()
                    .position(|item| self.workspace_visible(item) && item.reference() == *workspace)
            })
            .or_else(|| self.active().map(|_| self.selected))
            .or_else(|| {
                self.items
                    .iter()
                    .position(|item| self.workspace_visible(item))
            });
        if let Some(index) = target {
            let _ = self.select(index);
        }
    }

    pub(in crate::workspace) fn ensure_legacy_channel(&mut self, name: &str) -> ChannelId {
        let name = normalize_realm(name);
        if let Some(channel) = self
            .channels
            .iter()
            .find(|item| channels_match(item.name(), &name))
        {
            return channel.id.clone();
        }
        let channel = create_channel(
            &self.channels,
            ChannelDraft {
                name,
                ..ChannelDraft::default()
            },
        );
        let id = channel.id.clone();
        self.channels.push(channel);
        id
    }

    pub(in crate::workspace) fn restore_legacy_assignments(&mut self) {
        self.restore_assignments(&BTreeMap::new());
    }

    fn restore_assignments(&mut self, assignments: &BTreeMap<WorkspaceRef, ChannelId>) {
        for index in 0..self.items.len() {
            let Some(workspace) = self.items.get(index) else {
                continue;
            };
            let realm = workspace.realm.clone();
            let id = assignments
                .get(&workspace.reference())
                .filter(|id| self.channel(id).is_some())
                .cloned()
                .unwrap_or_else(|| self.ensure_legacy_channel(&realm));
            if let Some(name) = self.channel(&id).map(|channel| channel.name().to_owned()) {
                if let Some(workspace) = self.items.get_mut(index) {
                    workspace.channel_id = Some(id);
                    workspace.realm = name;
                }
            }
        }
    }
}
