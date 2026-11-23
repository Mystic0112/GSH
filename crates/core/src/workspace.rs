//! Modelo de workspace do domínio (`Project` e `WorkspaceKind` em
//! `project.dart` do app Flutter).
//!
//! Puro, sem I/O e sem UI: deriva nome e inicial a partir do caminho da pasta
//! e mantém a lista em memória com seleção ativa.

use std::path::{Path, PathBuf};

use crate::tabs::basename;

mod channel;
#[cfg(test)]
mod channel_tests;
mod layout;
mod model;

pub use channel::{
    Channel, ChannelDraft, ChannelError, ChannelId, ChannelMember, ChannelReplyRule, ChannelState,
    WorkspaceRef,
};
pub use layout::{PaneCreated, TabClosed, WorkspaceEditor, WorkspaceState};
pub use model::{WorkspaceError, Workspaces};

/// Nome do canal de contexto padrão.
pub const CHANNEL_DEFAULT: &str = "Padrão";
/// Nome do canal de contexto de trabalho.
pub const CHANNEL_WORK: &str = "Trabalho";
/// Nome do canal de contexto pessoal.
pub const CHANNEL_PERSONAL: &str = "Pessoal";
/// Nome do canal de contexto de projetos.
pub const CHANNEL_PROJECTS: &str = "Projetos";

/// Canais de contexto pré-definidos disponíveis por padrão.
pub const DEFAULT_CONTEXT_CHANNELS: [&str; 4] = [
    CHANNEL_DEFAULT,
    CHANNEL_WORK,
    CHANNEL_PERSONAL,
    CHANNEL_PROJECTS,
];
pub const MAX_CHANNEL_NAME_LENGTH: usize = 48;

pub fn valid_channel_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name.chars().count() <= MAX_CHANNEL_NAME_LENGTH
        && !name.chars().any(char::is_control)
}

/// Canal de contexto (`realm`) para separar workspaces (trabalho, pessoal e cada
/// projeto separados) sem clonar workspaces nem agentes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ContextChannel {
    /// Canal padrão (`"Padrão"`).
    #[default]
    Default,
    /// Canal de trabalho (`"Trabalho"`).
    Work,
    /// Canal pessoal (`"Pessoal"`).
    Personal,
    /// Canal de projetos (`"Projetos"`).
    Projects,
    /// Canal personalizado (ex.: contexto específico de um projeto).
    Custom(String),
}

impl ContextChannel {
    /// Lista com os canais de contexto pré-definidos.
    pub const PRESETS: [Self; 4] = [Self::Default, Self::Work, Self::Personal, Self::Projects];

    /// Interpreta um nome de canal, reconhecendo os canais padrão de forma case-insensitive.
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        if !valid_channel_name(trimmed) || channels_match(trimmed, CHANNEL_DEFAULT) {
            Self::Default
        } else if channels_match(trimmed, CHANNEL_WORK) {
            Self::Work
        } else if channels_match(trimmed, CHANNEL_PERSONAL) {
            Self::Personal
        } else if channels_match(trimmed, CHANNEL_PROJECTS) {
            Self::Projects
        } else {
            Self::Custom(trimmed.to_string())
        }
    }

    /// Nome textual do canal.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Default => CHANNEL_DEFAULT,
            Self::Work => CHANNEL_WORK,
            Self::Personal => CHANNEL_PERSONAL,
            Self::Projects => CHANNEL_PROJECTS,
            Self::Custom(name) => name.as_str(),
        }
    }
}

impl AsRef<str> for ContextChannel {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for ContextChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ContextChannel {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for ContextChannel {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<ContextChannel> for String {
    fn from(channel: ContextChannel) -> Self {
        channel.as_str().to_string()
    }
}

fn channels_match(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

fn normalize_realm(raw: &str) -> String {
    if valid_channel_name(raw) {
        ContextChannel::parse(raw).into()
    } else {
        CHANNEL_DEFAULT.to_owned()
    }
}

/// Natureza de um workspace no rail (`WorkspaceKind` em `project.dart`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceKind {
    /// Workspace ancorado numa pasta local do sistema de arquivos.
    Project,
    /// Workspace sintético sem pasta local obrigatória (ex.: terminal de sistema).
    SystemTerminal,
}

/// Um workspace exibido no rail lateral (`Project` em `project.dart`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: Option<PathBuf>,
    pub git_branch: Option<String>,
    pub kind: WorkspaceKind,
    pub realm: String,
    channel_id: Option<ChannelId>,
}

impl Workspace {
    /// Cria um workspace sintético (sem pasta de projeto associada).
    pub fn system(
        id: impl Into<String>,
        name: impl Into<String>,
        git_branch: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            path: None,
            git_branch,
            kind: WorkspaceKind::SystemTerminal,
            realm: CHANNEL_DEFAULT.to_string(),
            channel_id: None,
        }
    }

    /// Cria um workspace de projeto a partir do caminho da pasta (`nome = basename(pasta)`).
    pub fn from_path(id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        let path_buf = path.into();
        let name = folder_display_name(&path_buf);
        Self {
            id: id.into(),
            name,
            path: Some(path_buf),
            git_branch: None,
            kind: WorkspaceKind::Project,
            realm: CHANNEL_DEFAULT.to_string(),
            channel_id: None,
        }
    }

    /// Define ou atualiza a branch git associada ao workspace.
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Define o caminho da pasta associado ao workspace.
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Define o canal de contexto (`realm`) associado ao workspace.
    pub fn with_realm(mut self, realm: impl Into<String>) -> Self {
        self.set_realm(realm);
        self
    }

    /// Atualiza o canal de contexto (`realm`) do workspace no lugar.
    pub fn set_realm(&mut self, realm: impl Into<String>) {
        self.realm = normalize_realm(&realm.into());
        self.channel_id = None;
    }

    /// Canal de contexto (`realm`) atual do workspace.
    pub fn realm(&self) -> &str {
        &self.realm
    }

    pub fn channel_id(&self) -> Option<&ChannelId> {
        self.channel_id.as_ref()
    }

    pub fn reference(&self) -> WorkspaceRef {
        self.path.as_ref().map_or_else(
            || WorkspaceRef::System(self.id.clone()),
            |path| WorkspaceRef::Project(path.clone()),
        )
    }

    /// Inicial em maiúsculo para o avatar do rail (`Project.initial` no Flutter).
    pub fn initial(&self) -> String {
        self.name
            .trim()
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string())
    }
}

/// Extrai o nome de exibição da pasta (último segmento não vazio do caminho).
pub fn folder_display_name(path: &Path) -> String {
    if let Some(os_name) = path.file_name() {
        let name = os_name.to_string_lossy();
        if !name.is_empty() {
            return name.into_owned();
        }
    }
    let raw = path.to_string_lossy();
    let base = basename(&raw);
    if base.is_empty() {
        raw.into_owned()
    } else {
        base
    }
}

/// Coleção pura de workspaces com controle do item selecionado e filtro por canal de contexto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceList {
    items: Vec<Workspace>,
    selected: usize,
    next_id: u64,
    channels: Vec<Channel>,
    active_channel: Option<ChannelId>,
}

impl WorkspaceList {
    /// Cria uma lista de workspaces a partir dos itens iniciais, selecionando o primeiro.
    pub fn new(items: Vec<Workspace>) -> Self {
        let next_id = u64::try_from(items.len()).unwrap_or(0).saturating_add(1);
        let mut list = Self {
            items,
            selected: 0,
            next_id,
            channels: channel::preset_channels(),
            active_channel: None,
        };
        list.restore_legacy_assignments();
        let _ = list.select(0);
        list
    }

    /// Fatia com todos os workspaces registrados (independentemente do canal ativo).
    pub fn items(&self) -> &[Workspace] {
        &self.items
    }

    /// Índice do workspace atualmente selecionado.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Um canal sem workspaces tem seleção efetiva vazia.
    pub fn active(&self) -> Option<&Workspace> {
        self.items
            .get(self.selected)
            .filter(|workspace| self.workspace_visible(workspace))
    }

    /// Seleciona o workspace pelo índice, caso esteja dentro dos limites.
    pub fn select(&mut self, index: usize) -> Option<&Workspace> {
        let workspace = self.items.get(index)?;
        if !self.workspace_visible(workspace) {
            return None;
        }
        let channel_id = workspace.channel_id.clone();
        if let Some(channel) = self
            .channels
            .iter_mut()
            .find(|item| Some(&item.id) == channel_id.as_ref())
        {
            channel.last_workspace = Some(workspace.reference());
        }
        self.selected = index;
        self.items.get(index)
    }

    /// Canal de contexto ativo para filtragem (`None` exibe todos os canais).
    pub fn active_channel(&self) -> Option<&str> {
        self.active_context_channel().map(Channel::name)
    }

    /// Aplica o filtro legado armazenado pelo nome.
    pub fn set_active_channel(&mut self, channel: Option<String>) {
        let channel = channel
            .filter(|name| valid_channel_name(name))
            .map(|name| self.ensure_legacy_channel(&name));
        let _ = self.select_context_channel(channel.as_ref());
    }

    /// Índices dos workspaces visíveis no canal ativo.
    ///
    /// Sem filtro, inclui todos os workspaces.
    pub fn visible_indices(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| self.workspace_visible(item).then_some(index))
            .collect()
    }

    /// Canais predefinidos e personalizados em uso, incluindo o canal ativo.
    pub fn channels(&self) -> Vec<String> {
        let mut channels: Vec<_> = self
            .channels
            .iter()
            .filter(|channel| !channel.archived)
            .map(|channel| channel.name().to_owned())
            .collect();
        channels.sort_by(|a, b| {
            a.to_lowercase()
                .cmp(&b.to_lowercase())
                .then_with(|| a.cmp(b))
        });
        channels
    }

    /// Altera o canal (`realm`) de um workspace existente no lugar, sem clonar.
    /// Retorna `true` se o índice e o nome forem válidos.
    pub fn set_workspace_realm(&mut self, index: usize, realm: impl Into<String>) -> bool {
        let realm = realm.into();
        if index >= self.items.len() || !valid_channel_name(&realm) {
            return false;
        }
        let id = self.ensure_legacy_channel(&realm);
        self.assign_workspace_channel(index, &id).is_ok()
    }

    /// Abre uma pasta como workspace: se já existir um workspace com o mesmo
    /// caminho, apenas o seleciona; senão cria um novo, adiciona ao final e o
    /// seleciona, retornando o índice selecionado.
    pub fn open_folder(&mut self, path: impl Into<PathBuf>) -> usize {
        let path_buf = path.into();
        if let Some(existing_idx) = self
            .items
            .iter()
            .position(|ws| ws.path.as_ref() == Some(&path_buf))
        {
            if self.active_channel.is_some()
                && self
                    .items
                    .get(existing_idx)
                    .is_some_and(|item| !self.workspace_visible(item))
            {
                let channel = self
                    .items
                    .get(existing_idx)
                    .and_then(|item| item.channel_id.clone());
                let _ = self
                    .select_context_channel(channel.as_ref())
                    .or_else(|_| self.select_context_channel(None));
            }
            let _ = self.select(existing_idx);
            return existing_idx;
        }

        let id = format!("ws-{}", self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let mut ws = Workspace::from_path(id, path_buf);
        let channel_id = self
            .active_channel
            .clone()
            .unwrap_or_else(ChannelId::default_channel);
        if let Some(channel) = self.channel(&channel_id) {
            ws.realm = channel.name().to_owned();
        }
        ws.channel_id = Some(channel_id);
        self.items.push(ws);
        let new_idx = self.items.len().saturating_sub(1);
        let _ = self.select(new_idx);
        new_idx
    }

    /// Atualiza o nome visível de um item do rail. Nomes vazios são recusados.
    pub fn rename(&mut self, index: usize, name: &str) -> bool {
        let name = name.trim();
        let Some(workspace) = self.items.get_mut(index) else {
            return false;
        };
        if name.is_empty() {
            return false;
        }
        workspace.name = name.to_owned();
        true
    }

    /// Remove um item sem tocar nos arquivos da pasta e mantém a seleção válida.
    pub fn remove(&mut self, index: usize) -> Option<Workspace> {
        if index >= self.items.len() {
            return None;
        }
        let removed = self.items.remove(index);
        for channel in &mut self.channels {
            if channel.last_workspace.as_ref() == Some(&removed.reference()) {
                channel.last_workspace = None;
            }
        }
        self.selected = match (self.items.len(), self.selected) {
            (0, _) => 0,
            (_, selected) if selected > index => selected - 1,
            (len, selected) if selected >= len => len - 1,
            (_, selected) => selected,
        };
        Some(removed)
    }
}
