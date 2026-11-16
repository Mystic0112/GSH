//! Estado e operações do navegador de coleções MongoDB (`MongoTabState` + lógica de
//! `db_mongo_view.dart`).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use gsh_db::mongo_browse;
use gsh_db::mongo_store::MongoDatabaseStore;
use gsh_db::{store, DbError, SecretStore};
use iced::widget::text_editor;
use iced::Task;
use serde_json::{Map, Value};

use crate::db_state::{needs_ssh_prompt, run_on_with_ssh};
use crate::labels;

#[derive(Debug, Clone)]
pub enum MongoMsg {
    FilterChanged(String),
    ClearFilter,
    ApplyFilter,
    Refresh,
    LoadMore,
    Loaded(u64, bool, Result<Vec<Map<String, Value>>, DbError>),
    DocSelectAction(usize, text_editor::Action),
    StartEdit(usize),
    EditEditor(text_editor::Action),
    CommitEdit,
    CloseEditors,
    OpenDraft,
    DraftEditor(text_editor::Action),
    CommitDraft,
    AskDelete(usize),
    CancelDelete,
    ConfirmDelete,
    Saved(usize, Result<Map<String, Value>, DbError>),
    Inserted(Result<(), DbError>),
    Deleted(usize, Result<(), DbError>),
    DismissError,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MongoEffect {
    SshPromptNeeded(DbError),
}

pub struct MongoDoc {
    pub doc: Map<String, Value>,
    pub content: text_editor::Content,
    pub json: Arc<str>,
}

impl MongoDoc {
    pub fn from_map(doc: Map<String, Value>) -> Self {
        let pretty = serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string());
        let content = text_editor::Content::with_text(&pretty);
        Self {
            doc,
            content,
            json: Arc::from(pretty),
        }
    }
}

pub struct MongoTab {
    pub conn_name: String,
    pub collection: String,
    root: Option<PathBuf>,
    secrets: SecretStore,
    mongo_store: MongoDatabaseStore,
    ssh_passphrases: BTreeMap<(String, String), String>,
    generation: u64,
    pub filter: String,
    pub filter_input: String,
    pub docs: Vec<MongoDoc>,
    pub has_more: bool,
    pub loaded: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub editing_ix: Option<usize>,
    pub editing_content: text_editor::Content,
    pub editing_error: Option<String>,
    pub editing_busy: bool,
    pub draft_open: bool,
    pub draft_content: text_editor::Content,
    pub draft_error: Option<String>,
    pub draft_busy: bool,
    pub confirm_delete: Option<usize>,
}

impl MongoTab {
    pub fn new(
        conn_name: String,
        collection: String,
        root: Option<PathBuf>,
        secrets: SecretStore,
        mongo_store: MongoDatabaseStore,
        ssh_passphrases: BTreeMap<(String, String), String>,
    ) -> Self {
        Self {
            conn_name,
            collection,
            root,
            secrets,
            mongo_store,
            ssh_passphrases,
            generation: 0,
            filter: String::new(),
            filter_input: String::new(),
            docs: Vec::new(),
            has_more: false,
            loaded: false,
            loading: false,
            error: None,
            editing_ix: None,
            editing_content: text_editor::Content::new(),
            editing_error: None,
            editing_busy: false,
            draft_open: false,
            draft_content: text_editor::Content::new(),
            draft_error: None,
            draft_busy: false,
            confirm_delete: None,
        }
    }

    pub fn sync_context(
        &mut self,
        root: Option<PathBuf>,
        secrets: &SecretStore,
        mongo_store: &MongoDatabaseStore,
        ssh_passphrases: BTreeMap<(String, String), String>,
    ) {
        self.root = root;
        self.secrets = secrets.clone();
        self.mongo_store = mongo_store.clone();
        self.ssh_passphrases = ssh_passphrases;
    }

    pub fn start_initial_load(&mut self) -> Task<MongoMsg> {
        if self.loaded || self.loading {
            return Task::none();
        }
        self.fetch(0, false)
    }

    pub fn retry_refresh(&mut self) -> Task<MongoMsg> {
        self.error = None;
        self.fetch(0, false)
    }

    pub fn id_label(doc: &Map<String, Value>) -> String {
        let Some(id) = doc.get("_id") else {
            return "null".to_string();
        };
        if let Value::Object(obj) = id {
            if obj.len() == 1 {
                if let Some(first) = obj.values().next() {
                    return match first {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                }
            }
        }
        id.to_string()
    }

    pub fn confirm_delete_id(&self) -> Option<String> {
        self.confirm_delete
            .and_then(|ix| self.docs.get(ix))
            .map(|d| Self::id_label(&d.doc))
    }

    fn close_editors(&mut self) {
        self.editing_ix = None;
        self.editing_content = text_editor::Content::new();
        self.editing_error = None;
        self.editing_busy = false;
        self.draft_open = false;
        self.draft_content = text_editor::Content::new();
        self.draft_error = None;
        self.draft_busy = false;
    }

    fn fetch(&mut self, skip: usize, append: bool) -> Task<MongoMsg> {
        if !append {
            self.generation = self.generation.wrapping_add(1);
            self.docs.clear();
            self.close_editors();
        }
        let gen = self.generation;
        self.loading = true;
        self.error = None;
        let filter = self.filter.clone();
        let collection = self.collection.clone();
        self.job(move |ep, db| {
            mongo_browse::find(
                &ep,
                &collection,
                &filter,
                skip,
                mongo_browse::PAGE_SIZE,
                db.as_deref(),
            )
        })
        .map(move |res| MongoMsg::Loaded(gen, append, res))
    }

    fn job<T: Send + 'static>(
        &self,
        work: impl FnOnce(gsh_db::Endpoint, Option<String>) -> Result<T, DbError> + Send + 'static,
    ) -> Task<Result<T, DbError>> {
        let Some(root) = self.root.clone() else {
            return Task::none();
        };
        let conn = self.conn_name.clone();
        let secrets = self.secrets.clone();
        let mongo_store = self.mongo_store.clone();
        let passphrases = self.ssh_passphrases.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let db = resolve_selected_db(&root, &conn, &mongo_store);
                    run_on_with_ssh(&root, &secrets, &conn, &passphrases, move |ep| work(ep, db))
                })
                .await
                .unwrap_or_else(|e| Err(DbError::QueryFailed(e.to_string())))
            },
            |res| res,
        )
    }

    pub fn update(&mut self, msg: MongoMsg) -> (Task<MongoMsg>, Option<MongoEffect>) {
        match msg {
            MongoMsg::FilterChanged(value) => {
                let was_non_empty = !self.filter_input.is_empty();
                self.filter_input = value;
                if self.filter_input.is_empty() && was_non_empty && !self.filter.is_empty() {
                    self.filter.clear();
                    return (self.fetch(0, false), None);
                }
            }
            MongoMsg::ClearFilter => {
                self.filter_input.clear();
                if !self.filter.is_empty() {
                    self.filter.clear();
                    return (self.fetch(0, false), None);
                }
            }
            MongoMsg::ApplyFilter => {
                self.filter = self.filter_input.trim().to_string();
                return (self.fetch(0, false), None);
            }
            MongoMsg::Refresh => {
                return (self.fetch(0, false), None);
            }
            MongoMsg::LoadMore => {
                if !self.loading && self.has_more {
                    return (self.fetch(self.docs.len(), true), None);
                }
            }
            MongoMsg::Loaded(gen, append, result) => {
                if gen != self.generation {
                    return (Task::none(), None);
                }
                self.loading = false;
                match result {
                    Ok(batch) => {
                        self.loaded = true;
                        self.has_more = batch.len() >= mongo_browse::PAGE_SIZE;
                        let mapped = batch.into_iter().map(MongoDoc::from_map);
                        if append {
                            self.docs.extend(mapped);
                        } else {
                            self.docs = mapped.collect();
                        }
                    }
                    Err(err) => {
                        let effect = needs_ssh_prompt(&err)
                            .then(|| MongoEffect::SshPromptNeeded(err.clone()));
                        self.error = Some(labels::db_error_message(&err));
                        return (Task::none(), effect);
                    }
                }
            }
            MongoMsg::DocSelectAction(ix, action) => {
                if !action.is_edit() {
                    if let Some(doc) = self.docs.get_mut(ix) {
                        doc.content.perform(action);
                    }
                }
            }
            MongoMsg::StartEdit(ix) => {
                let Some(doc) = self.docs.get(ix) else {
                    return (Task::none(), None);
                };
                let json = doc.json.to_string();
                self.close_editors();
                self.editing_ix = Some(ix);
                self.editing_content = text_editor::Content::with_text(&json);
            }
            MongoMsg::EditEditor(action) => {
                self.editing_content.perform(action);
                if self.editing_error.is_some() {
                    self.editing_error = None;
                }
            }
            MongoMsg::CommitEdit => {
                let Some(ix) = self.editing_ix else {
                    return (Task::none(), None);
                };
                if self.editing_busy {
                    return (Task::none(), None);
                }
                let text = self.editing_content.text();
                if let Err(err) = mongo_browse::parse_document(&text) {
                    self.editing_error = Some(labels::db_error_message(&err));
                    return (Task::none(), None);
                }
                self.editing_busy = true;
                let collection = self.collection.clone();
                let root = self.root.clone();
                let conn = self.conn_name.clone();
                let secrets = self.secrets.clone();
                let mongo_store = self.mongo_store.clone();
                let passphrases = self.ssh_passphrases.clone();
                let task = match root {
                    Some(root) => Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                let db = resolve_selected_db(&root, &conn, &mongo_store);
                                run_on_with_ssh(&root, &secrets, &conn, &passphrases, |ep| {
                                    mongo_browse::replace_one(
                                        &ep,
                                        &collection,
                                        &text,
                                        db.as_deref(),
                                    )
                                })
                            })
                            .await
                            .unwrap_or_else(|e| Err(DbError::QueryFailed(e.to_string())))
                        },
                        move |res| MongoMsg::Saved(ix, res),
                    ),
                    None => Task::none(),
                };
                return (task, None);
            }
            MongoMsg::Saved(ix, res) => {
                self.editing_busy = false;
                match res {
                    Ok(doc) => {
                        if let Some(slot) = self.docs.get_mut(ix) {
                            *slot = MongoDoc::from_map(doc);
                        }
                        self.close_editors();
                    }
                    Err(err) => {
                        let effect = needs_ssh_prompt(&err)
                            .then(|| MongoEffect::SshPromptNeeded(err.clone()));
                        self.editing_error = Some(labels::db_error_message(&err));
                        return (Task::none(), effect);
                    }
                }
            }
            MongoMsg::CloseEditors => self.close_editors(),
            MongoMsg::OpenDraft => {
                self.close_editors();
                self.draft_open = true;
                self.draft_content = text_editor::Content::with_text("{\n  \n}");
            }
            MongoMsg::DraftEditor(action) => {
                self.draft_content.perform(action);
                if self.draft_error.is_some() {
                    self.draft_error = None;
                }
            }
            MongoMsg::CommitDraft => {
                if !self.draft_open || self.draft_busy {
                    return (Task::none(), None);
                }
                let text = self.draft_content.text();
                if let Err(err) = mongo_browse::parse_document(&text) {
                    self.draft_error = Some(labels::db_error_message(&err));
                    return (Task::none(), None);
                }
                self.draft_busy = true;
                let collection = self.collection.clone();
                let Some(root) = self.root.clone() else {
                    self.draft_busy = false;
                    return (Task::none(), None);
                };
                let conn = self.conn_name.clone();
                let secrets = self.secrets.clone();
                let mongo_store = self.mongo_store.clone();
                let passphrases = self.ssh_passphrases.clone();
                let task = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let db = resolve_selected_db(&root, &conn, &mongo_store);
                            run_on_with_ssh(&root, &secrets, &conn, &passphrases, |ep| {
                                mongo_browse::insert_one(&ep, &collection, &text, db.as_deref())
                            })
                        })
                        .await
                        .unwrap_or_else(|e| Err(DbError::QueryFailed(e.to_string())))
                    },
                    MongoMsg::Inserted,
                );
                return (task, None);
            }
            MongoMsg::Inserted(res) => {
                self.draft_busy = false;
                match res {
                    Ok(()) => {
                        self.close_editors();
                        return (self.fetch(0, false), None);
                    }
                    Err(err) => {
                        let effect = needs_ssh_prompt(&err)
                            .then(|| MongoEffect::SshPromptNeeded(err.clone()));
                        self.draft_error = Some(labels::db_error_message(&err));
                        return (Task::none(), effect);
                    }
                }
            }
            MongoMsg::AskDelete(ix) => {
                if ix < self.docs.len() {
                    self.confirm_delete = Some(ix);
                }
            }
            MongoMsg::CancelDelete => self.confirm_delete = None,
            MongoMsg::ConfirmDelete => {
                let Some(ix) = self.confirm_delete.take() else {
                    return (Task::none(), None);
                };
                let Some(id) = self.docs.get(ix).and_then(|d| d.doc.get("_id")).cloned() else {
                    return (Task::none(), None);
                };
                let collection = self.collection.clone();
                let Some(root) = self.root.clone() else {
                    return (Task::none(), None);
                };
                let conn = self.conn_name.clone();
                let secrets = self.secrets.clone();
                let mongo_store = self.mongo_store.clone();
                let passphrases = self.ssh_passphrases.clone();
                let task = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let db = resolve_selected_db(&root, &conn, &mongo_store);
                            run_on_with_ssh(&root, &secrets, &conn, &passphrases, |ep| {
                                mongo_browse::delete_one(&ep, &collection, id, db.as_deref())
                            })
                        })
                        .await
                        .unwrap_or_else(|e| Err(DbError::QueryFailed(e.to_string())))
                    },
                    move |res| MongoMsg::Deleted(ix, res),
                );
                return (task, None);
            }
            MongoMsg::Deleted(ix, res) => match res {
                Ok(()) => {
                    if ix < self.docs.len() {
                        self.docs.remove(ix);
                    }
                    if self.editing_ix == Some(ix) {
                        self.close_editors();
                    }
                }
                Err(err) => {
                    let effect =
                        needs_ssh_prompt(&err).then(|| MongoEffect::SshPromptNeeded(err.clone()));
                    self.error = Some(labels::db_error_message(&err));
                    return (Task::none(), effect);
                }
            },
            MongoMsg::DismissError => self.error = None,
            MongoMsg::Noop => {}
        }
        (Task::none(), None)
    }
}

fn resolve_selected_db(
    root: &std::path::Path,
    conn_name: &str,
    mongo_store: &MongoDatabaseStore,
) -> Option<String> {
    let all = store::load(root);
    let conn = all.iter().find(|c| c.name == conn_name)?;
    if !conn.database().is_empty() {
        return None;
    }
    mongo_store.selected(&root.to_string_lossy(), conn_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_label_unwraps_oid_wrapper_and_serializes_scalars() {
        let mut doc = Map::new();
        doc.insert(
            "_id".into(),
            serde_json::json!({"$oid": "507f1f77bcf86cd799439011"}),
        );
        assert_eq!(MongoTab::id_label(&doc), "507f1f77bcf86cd799439011");

        let mut int_doc = Map::new();
        int_doc.insert("_id".into(), serde_json::json!(42));
        assert_eq!(MongoTab::id_label(&int_doc), "42");
    }

    #[test]
    fn filter_clear_resets_filter_and_draft_validates_json() {
        let mut tab = MongoTab::new(
            "atlas".into(),
            "users".into(),
            None,
            SecretStore::new(PathBuf::from("/nonexistent")),
            MongoDatabaseStore::new(PathBuf::from("/nonexistent/mongo.json")),
            BTreeMap::new(),
        );
        tab.filter = "{\"a\": 1}".into();
        tab.filter_input = "{\"a\": 1}".into();
        let _ = tab.update(MongoMsg::ClearFilter);
        assert!(tab.filter.is_empty());

        let _ = tab.update(MongoMsg::OpenDraft);
        assert!(tab.draft_open);
        tab.draft_content = text_editor::Content::with_text("not json");
        let _ = tab.update(MongoMsg::CommitDraft);
        assert!(tab.draft_error.is_some());
    }
}
