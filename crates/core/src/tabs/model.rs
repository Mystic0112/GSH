use crate::panes::TabId;

pub const EMPTY_TAB_TITLE: &str = "New tab";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Empty,
    Terminal,
    Neovim,
    File,
    Diff,
    TaskOutput,
    TelemetryCase,
    Redis,
    Mongo,
    Notebook,
    AgentGraph,
    Farm,
    Harness,
    Chat,
    Browser,
}

impl TabKind {
    pub(crate) fn id_prefix(self) -> &'static str {
        match self {
            Self::Empty => "a",
            Self::Terminal | Self::TaskOutput => "t",
            Self::Neovim => "n",
            Self::TelemetryCase => "y",
            Self::Diff => "d",
            _ => "v",
        }
    }

    /// Valor de `kind` usado por `gsh list-tabs` no Flutter.
    pub fn cli_kind(self) -> &'static str {
        match self {
            Self::Terminal | Self::Neovim => "terminal",
            Self::File => "file",
            Self::TaskOutput => "task",
            Self::Redis => "redis",
            Self::Mongo => "mongo",
            Self::Notebook => "notebook",
            Self::AgentGraph => "agents",
            Self::Farm => "farm",
            Self::Chat => "chat",
            _ => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub kind: TabKind,
    pub title: String,
    pub label: Option<String>,
    pub working_directory: String,
}

impl Tab {
    pub fn display_title(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.title)
    }

    pub fn title_locked(&self) -> bool {
        self.label.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTab {
    pub kind: TabKind,
    pub title: String,
    pub label: Option<String>,
    pub working_directory: String,
}

impl NewTab {
    pub fn new(kind: TabKind, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            label: None,
            working_directory: String::new(),
        }
    }

    pub fn empty() -> Self {
        Self::new(TabKind::Empty, EMPTY_TAB_TITLE)
    }

    pub fn with_working_directory(mut self, dir: impl Into<String>) -> Self {
        self.working_directory = dir.into();
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub(crate) fn into_tab(self, sequential_id: u64) -> Tab {
        Tab {
            id: format!("{}{sequential_id}", self.kind.id_prefix()),
            kind: self.kind,
            title: self.title,
            label: self.label.as_deref().map(str::trim).map(str::to_owned),
            working_directory: self.working_directory,
        }
    }
}
