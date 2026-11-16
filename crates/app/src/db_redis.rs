//! Estado e operações do navegador de chaves Redis (`RedisTabState` + lógica de
//! `db_redis_table.dart`).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use gsh_db::redis_browse::{self, RedisKeyEntry, RedisScanPage, RedisValueKind};
use gsh_db::{DbError, SecretStore};
use iced::widget::text_editor;
use iced::Task;

use crate::db_state::{needs_ssh_prompt, run_on_with_ssh};
use crate::{labels, theme};

#[derive(Debug, Clone)]
pub enum RedisMsg {
    PatternChanged(String),
    ClearPattern,
    ApplyPattern,
    Refresh,
    LoadMore,
    Scanned(u64, bool, Result<RedisScanPage, DbError>),
    StartKeyEdit(String),
    KeyInput(String),
    CommitKeyEdit,
    StartStringEdit(String),
    StringInput(String),
    CommitStringEdit,
    StartTtlEdit(String),
    TtlInput(String),
    CommitTtlEdit,
    ToggleExpanded(String),
    FullLoaded(u64, FullTarget, String, Result<String, DbError>),
    ExpandedEditor(text_editor::Action),
    CommitExpanded(String),
    CloseEditors,
    OpenDraft,
    CloseDraft,
    ToggleDraftKindMenu,
    CloseDraftKindMenu,
    DraftKey(String),
    DraftKind(RedisValueKind),
    DraftTtl(String),
    DraftValue(text_editor::Action),
    CommitDraft,
    AskDelete(String),
    CancelDelete,
    ConfirmDelete,
    Mutated(Result<MutateOutcome, DbError>),
    FlashClear(u64),
    DismissError,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullTarget {
    StringInline,
    CompositeExpanded,
}

#[derive(Debug, Clone)]
pub enum MutateOutcome {
    Renamed {
        old_key: String,
        new_key: String,
        fresh: Option<RedisKeyEntry>,
    },
    Updated {
        key: String,
        fresh: Option<RedisKeyEntry>,
    },
    Created {
        key: String,
        fresh: Option<RedisKeyEntry>,
    },
    Deleted {
        key: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedisEffect {
    SshPromptNeeded(DbError),
}

pub struct NewKeyDraft {
    pub key: String,
    pub kind: RedisValueKind,
    pub kind_menu_open: bool,
    pub ttl: String,
    pub value: text_editor::Content,
    pub error: Option<String>,
    pub busy: bool,
}

impl Default for NewKeyDraft {
    fn default() -> Self {
        Self {
            key: String::new(),
            kind: RedisValueKind::String,
            kind_menu_open: false,
            ttl: String::new(),
            value: text_editor::Content::new(),
            error: None,
            busy: false,
        }
    }
}

pub struct RedisTab {
    pub conn_name: String,
    root: Option<PathBuf>,
    secrets: SecretStore,
    ssh_passphrases: BTreeMap<(String, String), String>,
    generation: u64,
    flash_generation: u64,
    pub pattern: String,
    pub pattern_input: String,
    pub cursor: String,
    pub entries: Vec<RedisKeyEntry>,
    pub loaded: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub editing_key: Option<String>,
    pub editing_value: String,
    pub editing_busy: bool,
    pub ttl_key: Option<String>,
    pub ttl_input: String,
    pub rename_key: Option<String>,
    pub rename_input: String,
    pub expanded_key: Option<String>,
    pub expanded_content: text_editor::Content,
    pub expanded_error: Option<String>,
    pub expanded_loading: bool,
    pub draft: Option<NewKeyDraft>,
    pub flash_key: Option<String>,
    pub confirm_delete: Option<String>,
}

impl RedisTab {
    pub fn new(
        conn_name: String,
        root: Option<PathBuf>,
        secrets: SecretStore,
        ssh_passphrases: BTreeMap<(String, String), String>,
    ) -> Self {
        Self {
            conn_name,
            root,
            secrets,
            ssh_passphrases,
            generation: 0,
            flash_generation: 0,
            pattern: String::new(),
            pattern_input: String::new(),
            cursor: "0".to_string(),
            entries: Vec::new(),
            loaded: false,
            loading: false,
            error: None,
            editing_key: None,
            editing_value: String::new(),
            editing_busy: false,
            ttl_key: None,
            ttl_input: String::new(),
            rename_key: None,
            rename_input: String::new(),
            expanded_key: None,
            expanded_content: text_editor::Content::new(),
            expanded_error: None,
            expanded_loading: false,
            draft: None,
            flash_key: None,
            confirm_delete: None,
        }
    }

    pub fn sync_context(
        &mut self,
        root: Option<PathBuf>,
        secrets: &SecretStore,
        ssh_passphrases: BTreeMap<(String, String), String>,
    ) {
        self.root = root;
        self.secrets = secrets.clone();
        self.ssh_passphrases = ssh_passphrases;
    }

    pub fn start_initial_scan(&mut self) -> Task<RedisMsg> {
        if self.loaded || self.loading {
            return Task::none();
        }
        self.scan("0".to_string(), false)
    }

    pub fn retry_refresh(&mut self) -> Task<RedisMsg> {
        self.error = None;
        self.scan("0".to_string(), false)
    }

    fn close_editors(&mut self) {
        self.editing_key = None;
        self.editing_value.clear();
        self.editing_busy = false;
        self.ttl_key = None;
        self.ttl_input.clear();
        self.rename_key = None;
        self.rename_input.clear();
        self.expanded_key = None;
        self.expanded_content = text_editor::Content::new();
        self.expanded_error = None;
        self.expanded_loading = false;
    }

    fn scan(&mut self, cursor: String, append: bool) -> Task<RedisMsg> {
        let Some(root) = self.root.clone() else {
            return Task::none();
        };
        if !append {
            self.generation = self.generation.wrapping_add(1);
            self.close_editors();
        }
        let gen = self.generation;
        self.loading = true;
        self.error = None;
        let conn = self.conn_name.clone();
        let secrets = self.secrets.clone();
        let passphrases = self.ssh_passphrases.clone();
        let pattern = self.pattern.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    run_on_with_ssh(&root, &secrets, &conn, &passphrases, |ep| {
                        redis_browse::scan(&ep, &pattern, &cursor)
                    })
                })
                .await
                .unwrap_or_else(|e| Err(DbError::QueryFailed(e.to_string())))
            },
            move |res| RedisMsg::Scanned(gen, append, res),
        )
    }

    fn job<T: Send + 'static>(
        &self,
        work: impl FnOnce(gsh_db::Endpoint) -> Result<T, DbError> + Send + 'static,
    ) -> Task<Result<T, DbError>> {
        let Some(root) = self.root.clone() else {
            return Task::none();
        };
        let conn = self.conn_name.clone();
        let secrets = self.secrets.clone();
        let passphrases = self.ssh_passphrases.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    run_on_with_ssh(&root, &secrets, &conn, &passphrases, work)
                })
                .await
                .unwrap_or_else(|e| Err(DbError::QueryFailed(e.to_string())))
            },
            |res| res,
        )
    }

    fn entry(&self, key: &str) -> Option<&RedisKeyEntry> {
        self.entries.iter().find(|e| e.key == key)
    }

    fn trigger_flash(&mut self, key: String) -> Task<RedisMsg> {
        self.flash_key = Some(key);
        self.flash_generation = self.flash_generation.wrapping_add(1);
        let gen = self.flash_generation;
        Task::perform(
            async move {
                tokio::time::sleep(Duration::from_millis(theme::REDIS_FLASH_MILLIS)).await;
            },
            move |_| RedisMsg::FlashClear(gen),
        )
    }

    pub fn update(&mut self, msg: RedisMsg) -> (Task<RedisMsg>, Option<RedisEffect>) {
        match msg {
            RedisMsg::PatternChanged(value) => {
                let was_empty = self.pattern_input.is_empty();
                self.pattern_input = value;
                if self.pattern_input.is_empty() && !was_empty && !self.pattern.is_empty() {
                    self.pattern.clear();
                    return (self.scan("0".to_string(), false), None);
                }
            }
            RedisMsg::ClearPattern => {
                self.pattern_input.clear();
                if !self.pattern.is_empty() {
                    self.pattern.clear();
                    return (self.scan("0".to_string(), false), None);
                }
            }
            RedisMsg::ApplyPattern => {
                self.pattern = self.pattern_input.trim().to_string();
                return (self.scan("0".to_string(), false), None);
            }
            RedisMsg::Refresh => {
                return (self.scan("0".to_string(), false), None);
            }
            RedisMsg::LoadMore => {
                if !self.loading && self.cursor != "0" {
                    let cursor = self.cursor.clone();
                    return (self.scan(cursor, true), None);
                }
            }
            RedisMsg::Scanned(gen, append, result) => {
                if gen != self.generation {
                    return (Task::none(), None);
                }
                self.loading = false;
                match result {
                    Ok(page) => {
                        self.loaded = true;
                        self.cursor = page.cursor;
                        if append {
                            self.entries.extend(page.entries);
                        } else {
                            self.entries = page.entries;
                        }
                    }
                    Err(err) => {
                        let effect = needs_ssh_prompt(&err)
                            .then(|| RedisEffect::SshPromptNeeded(err.clone()));
                        self.error = Some(labels::db_error_message(&err));
                        return (Task::none(), effect);
                    }
                }
            }
            RedisMsg::StartKeyEdit(key) => {
                self.close_editors();
                self.rename_input = key.clone();
                self.rename_key = Some(key);
            }
            RedisMsg::KeyInput(value) => self.rename_input = value,
            RedisMsg::CommitKeyEdit => {
                let Some(old_key) = self.rename_key.take() else {
                    return (Task::none(), None);
                };
                let new_key = self.rename_input.trim().to_string();
                self.rename_input.clear();
                if new_key.is_empty() || new_key == old_key {
                    return (Task::none(), None);
                }
                let task = self
                    .job(move |ep| {
                        redis_browse::rename(&ep, &old_key, &new_key)?;
                        let fresh = redis_browse::refresh_entry(&ep, &new_key)?;
                        Ok(MutateOutcome::Renamed {
                            old_key,
                            new_key,
                            fresh,
                        })
                    })
                    .map(RedisMsg::Mutated);
                return (task, None);
            }
            RedisMsg::StartStringEdit(key) => {
                let Some(entry) = self.entry(&key).cloned() else {
                    return (Task::none(), None);
                };
                if entry.kind != RedisValueKind::String {
                    return (Task::none(), None);
                }
                self.close_editors();
                self.editing_key = Some(key.clone());
                self.editing_busy = true;
                let gen = self.generation;
                let key_load = key.clone();
                let task = self
                    .job(move |ep| redis_browse::read_full(&ep, &key_load, RedisValueKind::String))
                    .map(move |res| {
                        RedisMsg::FullLoaded(gen, FullTarget::StringInline, key.clone(), res)
                    });
                return (task, None);
            }
            RedisMsg::StringInput(value) => self.editing_value = value,
            RedisMsg::CommitStringEdit => {
                let Some(key) = self.editing_key.take() else {
                    return (Task::none(), None);
                };
                if self.editing_busy {
                    return (Task::none(), None);
                }
                let val = std::mem::take(&mut self.editing_value);
                let task = self
                    .job(move |ep| {
                        redis_browse::write_string(&ep, &key, &val)?;
                        let fresh = redis_browse::refresh_entry(&ep, &key)?;
                        Ok(MutateOutcome::Updated { key, fresh })
                    })
                    .map(RedisMsg::Mutated);
                return (task, None);
            }
            RedisMsg::StartTtlEdit(key) => {
                let Some(entry) = self.entry(&key).cloned() else {
                    return (Task::none(), None);
                };
                self.close_editors();
                self.ttl_input = if entry.ttl < 0 {
                    String::new()
                } else {
                    entry.ttl.to_string()
                };
                self.ttl_key = Some(key);
            }
            RedisMsg::TtlInput(value) => self.ttl_input = value,
            RedisMsg::CommitTtlEdit => {
                let Some(key) = self.ttl_key.take() else {
                    return (Task::none(), None);
                };
                let raw = self.ttl_input.trim().to_string();
                self.ttl_input.clear();
                let parsed: Option<i64> = if raw.is_empty() {
                    None
                } else {
                    match raw.parse::<i64>() {
                        Ok(n) if n > 0 => Some(n),
                        Ok(_) => None,
                        Err(_) => {
                            self.error = Some(labels::REDIS_TTL_MUST_BE_NUMBER.to_string());
                            return (Task::none(), None);
                        }
                    }
                };
                let task = self
                    .job(move |ep| {
                        redis_browse::set_ttl(&ep, &key, parsed)?;
                        let fresh = redis_browse::refresh_entry(&ep, &key)?;
                        Ok(MutateOutcome::Updated { key, fresh })
                    })
                    .map(RedisMsg::Mutated);
                return (task, None);
            }
            RedisMsg::ToggleExpanded(key) => {
                if self.expanded_key.as_deref() == Some(key.as_str()) {
                    self.close_editors();
                    return (Task::none(), None);
                }
                let Some(entry) = self.entry(&key).cloned() else {
                    return (Task::none(), None);
                };
                if !entry.kind.editable() || entry.kind == RedisValueKind::String {
                    return (Task::none(), None);
                }
                self.close_editors();
                self.expanded_key = Some(key.clone());
                self.expanded_loading = true;
                let gen = self.generation;
                let kind = entry.kind;
                let key_load = key.clone();
                let task = self
                    .job(move |ep| redis_browse::read_full(&ep, &key_load, kind))
                    .map(move |res| {
                        RedisMsg::FullLoaded(gen, FullTarget::CompositeExpanded, key.clone(), res)
                    });
                return (task, None);
            }
            RedisMsg::FullLoaded(gen, target, key, result) => {
                if gen != self.generation {
                    return (Task::none(), None);
                }
                match target {
                    FullTarget::StringInline => {
                        if self.editing_key.as_deref() != Some(key.as_str()) {
                            return (Task::none(), None);
                        }
                        self.editing_busy = false;
                        match result {
                            Ok(val) => self.editing_value = val,
                            Err(err) => {
                                self.editing_key = None;
                                self.error = Some(labels::db_error_message(&err));
                            }
                        }
                    }
                    FullTarget::CompositeExpanded => {
                        if self.expanded_key.as_deref() != Some(key.as_str()) {
                            return (Task::none(), None);
                        }
                        self.expanded_loading = false;
                        match result {
                            Ok(val) => {
                                self.expanded_content = text_editor::Content::with_text(&val);
                                self.expanded_error = None;
                            }
                            Err(err) => {
                                self.expanded_key = None;
                                self.error = Some(labels::db_error_message(&err));
                            }
                        }
                    }
                }
            }
            RedisMsg::ExpandedEditor(action) => {
                self.expanded_content.perform(action);
                if self.expanded_error.is_some() {
                    self.expanded_error = None;
                }
            }
            RedisMsg::CommitExpanded(key) => {
                let Some(entry) = self.entry(&key).cloned() else {
                    return (Task::none(), None);
                };
                let json_text = self.expanded_content.text();
                if let Err(err) =
                    redis_browse::build_composite_command(&key, entry.kind, &json_text)
                {
                    self.expanded_error = Some(labels::db_error_message(&err));
                    return (Task::none(), None);
                }
                let kind = entry.kind;
                let task = self
                    .job(move |ep| {
                        redis_browse::write_composite(&ep, &key, kind, &json_text)?;
                        let fresh = redis_browse::refresh_entry(&ep, &key)?;
                        Ok(MutateOutcome::Updated { key, fresh })
                    })
                    .map(RedisMsg::Mutated);
                return (task, None);
            }
            RedisMsg::CloseEditors => self.close_editors(),
            RedisMsg::OpenDraft => {
                self.close_editors();
                self.draft = Some(NewKeyDraft::default());
            }
            RedisMsg::CloseDraft => self.draft = None,
            RedisMsg::ToggleDraftKindMenu => {
                if let Some(d) = self.draft.as_mut() {
                    d.kind_menu_open = !d.kind_menu_open;
                }
            }
            RedisMsg::CloseDraftKindMenu => {
                if let Some(d) = self.draft.as_mut() {
                    d.kind_menu_open = false;
                }
            }
            RedisMsg::DraftKey(value) => {
                if let Some(d) = self.draft.as_mut() {
                    d.key = value;
                    d.error = None;
                }
            }
            RedisMsg::DraftKind(kind) => {
                if let Some(d) = self.draft.as_mut() {
                    d.kind = kind;
                    d.kind_menu_open = false;
                    d.error = None;
                }
            }
            RedisMsg::DraftTtl(value) => {
                if let Some(d) = self.draft.as_mut() {
                    d.ttl = value;
                    d.error = None;
                }
            }
            RedisMsg::DraftValue(action) => {
                if let Some(d) = self.draft.as_mut() {
                    d.value.perform(action);
                    d.error = None;
                }
            }
            RedisMsg::CommitDraft => {
                let Some(d) = self.draft.as_mut() else {
                    return (Task::none(), None);
                };
                if d.busy {
                    return (Task::none(), None);
                }
                let key = d.key.trim().to_string();
                if key.is_empty() {
                    d.error = Some("Key must not be empty.".into());
                    return (Task::none(), None);
                }
                let ttl = match d.ttl.trim() {
                    "" => None,
                    raw => match raw.parse::<i64>() {
                        Ok(n) if n > 0 => Some(n),
                        _ => {
                            d.error = Some(labels::REDIS_TTL_MUST_BE_NUMBER.to_string());
                            return (Task::none(), None);
                        }
                    },
                };
                let kind = d.kind;
                let value = d.value.text();
                if kind != RedisValueKind::String {
                    if let Err(err) = redis_browse::build_composite_command(&key, kind, &value) {
                        d.error = Some(labels::db_error_message(&err));
                        return (Task::none(), None);
                    }
                }
                d.busy = true;
                let task = self
                    .job(move |ep| {
                        redis_browse::create(&ep, &key, kind, &value, ttl)?;
                        let fresh = redis_browse::refresh_entry(&ep, &key)?;
                        Ok(MutateOutcome::Created { key, fresh })
                    })
                    .map(RedisMsg::Mutated);
                return (task, None);
            }
            RedisMsg::AskDelete(key) => {
                self.confirm_delete = Some(key);
            }
            RedisMsg::CancelDelete => {
                self.confirm_delete = None;
            }
            RedisMsg::ConfirmDelete => {
                let Some(key) = self.confirm_delete.take() else {
                    return (Task::none(), None);
                };
                let task = self
                    .job(move |ep| {
                        redis_browse::delete(&ep, &key)?;
                        Ok(MutateOutcome::Deleted { key })
                    })
                    .map(RedisMsg::Mutated);
                return (task, None);
            }
            RedisMsg::Mutated(result) => match result {
                Ok(MutateOutcome::Renamed {
                    old_key,
                    new_key,
                    fresh,
                }) => {
                    if let Some(pos) = self.entries.iter().position(|e| e.key == old_key) {
                        match fresh {
                            Some(entry) => {
                                if let Some(slot) = self.entries.get_mut(pos) {
                                    *slot = entry;
                                }
                            }
                            None => {
                                self.entries.remove(pos);
                            }
                        }
                    }
                    return (self.trigger_flash(new_key), None);
                }
                Ok(MutateOutcome::Updated { key, fresh }) => {
                    self.close_editors();
                    if let Some(pos) = self.entries.iter().position(|e| e.key == key) {
                        match fresh {
                            Some(entry) => {
                                if let Some(slot) = self.entries.get_mut(pos) {
                                    *slot = entry;
                                }
                            }
                            None => {
                                self.entries.remove(pos);
                            }
                        }
                    }
                    return (self.trigger_flash(key), None);
                }
                Ok(MutateOutcome::Created { key, fresh }) => {
                    self.draft = None;
                    self.entries.retain(|e| e.key != key);
                    if let Some(entry) = fresh {
                        self.entries.insert(0, entry);
                    }
                    return (self.trigger_flash(key), None);
                }
                Ok(MutateOutcome::Deleted { key }) => {
                    if self.expanded_key.as_deref() == Some(key.as_str()) {
                        self.close_editors();
                    }
                    self.entries.retain(|e| e.key != key);
                }
                Err(err) => {
                    if let Some(d) = self.draft.as_mut() {
                        if d.busy {
                            d.busy = false;
                            d.error = Some(labels::db_error_message(&err));
                            return (Task::none(), None);
                        }
                    }
                    let effect =
                        needs_ssh_prompt(&err).then(|| RedisEffect::SshPromptNeeded(err.clone()));
                    self.error = Some(labels::db_error_message(&err));
                    return (Task::none(), effect);
                }
            },
            RedisMsg::FlashClear(gen) => {
                if gen == self.flash_generation {
                    self.flash_key = None;
                }
            }
            RedisMsg::DismissError => self.error = None,
            RedisMsg::Noop => {}
        }
        (Task::none(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_clear_resets_filter_without_root() {
        let mut tab = RedisTab::new(
            "cache".into(),
            None,
            SecretStore::new(PathBuf::from("/nonexistent")),
            BTreeMap::new(),
        );
        tab.pattern = "user:*".into();
        tab.pattern_input = "user:*".into();
        let _ = tab.update(RedisMsg::ClearPattern);
        assert!(tab.pattern.is_empty());
        assert!(tab.pattern_input.is_empty());
    }

    #[test]
    fn ttl_validation_rejects_non_numeric_input() {
        let mut tab = RedisTab::new(
            "cache".into(),
            None,
            SecretStore::new(PathBuf::from("/nonexistent")),
            BTreeMap::new(),
        );
        tab.ttl_key = Some("k1".into());
        tab.ttl_input = "abc".into();
        let _ = tab.update(RedisMsg::CommitTtlEdit);
        assert_eq!(tab.error.as_deref(), Some(labels::REDIS_TTL_MUST_BE_NUMBER));
    }

    #[test]
    fn draft_opens_switches_kind_and_closes() {
        let mut tab = RedisTab::new(
            "cache".into(),
            None,
            SecretStore::new(PathBuf::from("/nonexistent")),
            BTreeMap::new(),
        );
        let _ = tab.update(RedisMsg::OpenDraft);
        assert!(tab.draft.is_some());
        let _ = tab.update(RedisMsg::DraftKind(RedisValueKind::Hash));
        assert_eq!(tab.draft.as_ref().unwrap().kind, RedisValueKind::Hash);
        let _ = tab.update(RedisMsg::CloseDraft);
        assert!(tab.draft.is_none());
    }
}
