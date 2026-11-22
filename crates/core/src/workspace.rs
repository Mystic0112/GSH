//! Modelo de workspace do domínio (`Project` e `WorkspaceKind` em
//! `project.dart` do app Flutter).
//!
//! Puro, sem I/O e sem UI: deriva nome e inicial a partir do caminho da pasta
//! e mantém a lista em memória com seleção ativa.

use std::path::{Path, PathBuf};

use crate::tabs::basename;

mod layout;
mod model;

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
    }

    /// Canal de contexto (`realm`) atual do workspace.
    pub fn realm(&self) -> &str {
        &self.realm
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
    active_channel: Option<String>,
}

impl WorkspaceList {
    /// Cria uma lista de workspaces a partir dos itens iniciais, selecionando o primeiro.
    pub fn new(items: Vec<Workspace>) -> Self {
        let next_id = u64::try_from(items.len()).unwrap_or(0).saturating_add(1);
        Self {
            items,
            selected: 0,
            next_id,
            active_channel: None,
        }
    }

    /// Fatia com todos os workspaces registrados (independentemente do canal ativo).
    pub fn items(&self) -> &[Workspace] {
        &self.items
    }

    /// Índice do workspace atualmente selecionado.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Referência ao workspace ativo (com fallback seguro para o primeiro item).
    pub fn active(&self) -> Option<&Workspace> {
        self.items.get(self.selected).or_else(|| self.items.first())
    }

    /// Seleciona o workspace pelo índice, caso esteja dentro dos limites.
    pub fn select(&mut self, index: usize) -> Option<&Workspace> {
        if index < self.items.len() {
            self.selected = index;
            self.items.get(index)
        } else {
            None
        }
    }

    /// Canal de contexto ativo para filtragem (`None` exibe todos os canais).
    pub fn active_channel(&self) -> Option<&str> {
        self.active_channel.as_deref()
    }

    /// Define o filtro de canais, preservando a seleção e os índices originais.
    pub fn set_active_channel(&mut self, channel: Option<String>) {
        self.active_channel = channel
            .filter(|name| valid_channel_name(name))
            .map(|name| normalize_realm(&name));
    }

    /// Índices dos workspaces visíveis no canal ativo.
    ///
    /// Sem filtro, inclui todos os workspaces.
    pub fn visible_indices(&self) -> Vec<usize> {
        match self.active_channel.as_deref() {
            None => (0..self.items.len()).collect(),
            Some(channel) => self
                .items
                .iter()
                .enumerate()
                .filter_map(|(idx, ws)| channels_match(&ws.realm, channel).then_some(idx))
                .collect(),
        }
    }

    /// Canais predefinidos e personalizados em uso, incluindo o canal ativo.
    pub fn channels(&self) -> Vec<String> {
        let mut channels = DEFAULT_CONTEXT_CHANNELS.map(str::to_owned).to_vec();
        for name in self
            .items
            .iter()
            .map(Workspace::realm)
            .chain(self.active_channel())
        {
            let canonical = normalize_realm(name);
            if !channels
                .iter()
                .any(|existing| channels_match(existing, &canonical))
            {
                channels.push(canonical);
            }
        }
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
        if let Some(ws) = self
            .items
            .get_mut(index)
            .filter(|_| valid_channel_name(&realm))
        {
            ws.set_realm(realm);
            true
        } else {
            false
        }
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
            self.selected = existing_idx;
            if let Some(active) = self.active_channel.as_deref() {
                if let Some(realm) = self
                    .items
                    .get(existing_idx)
                    .map(|workspace| workspace.realm().to_owned())
                {
                    if !channels_match(active, &realm) {
                        self.active_channel = Some(realm);
                    }
                }
            }
            return existing_idx;
        }

        let id = format!("ws-{}", self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let mut ws = Workspace::from_path(id, path_buf);
        if let Some(active) = self.active_channel.as_deref() {
            ws.set_realm(active);
        }
        self.items.push(ws);
        let new_idx = self.items.len().saturating_sub(1);
        self.selected = new_idx;
        new_idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canais_predefinidos_existem_sem_workspaces_e_nome_invalido_nao_move_item() {
        let mut list = WorkspaceList::new(vec![Workspace::system("sys", "Terminal", None)]);
        assert!(DEFAULT_CONTEXT_CHANNELS
            .iter()
            .all(|name| list.channels().contains(&name.to_string())));
        assert!(!list.set_workspace_realm(0, " \n "));
        assert!(!list.set_workspace_realm(0, "Nome\nInválido"));
        assert_eq!(
            ContextChannel::parse("Nome\nInválido"),
            ContextChannel::Default
        );
        assert_eq!(list.active().unwrap().realm(), CHANNEL_DEFAULT);
    }

    #[test]
    fn workspace_from_path_extrai_nome_e_inicial() {
        let ws = Workspace::from_path("ws-1", "/home/dev/projetos/meu-app");
        assert_eq!(ws.name, "meu-app");
        assert_eq!(ws.initial(), "M");
        assert_eq!(ws.path, Some(PathBuf::from("/home/dev/projetos/meu-app")));
        assert_eq!(ws.kind, WorkspaceKind::Project);
        assert_eq!(ws.realm, CHANNEL_DEFAULT);
    }

    #[test]
    fn workspace_list_abre_seleciona_e_evita_duplicatas_de_caminho() {
        let initial = vec![Workspace::system("gsh", "GSH", Some("main".into()))];
        let mut list = WorkspaceList::new(initial);
        assert_eq!(list.selected_index(), 0);
        assert_eq!(list.active().map(|w| w.name.as_str()), Some("GSH"));

        let idx = list.open_folder("/tmp/projeto-alfa");
        assert_eq!(idx, 1);
        assert_eq!(list.selected_index(), 1);
        assert_eq!(list.active().map(|w| w.name.as_str()), Some("projeto-alfa"));

        let _ = list.select(0);
        list.set_active_channel(Some(CHANNEL_WORK.to_owned()));
        let reused_idx = list.open_folder("/tmp/projeto-alfa");
        assert_eq!(reused_idx, 1);
        assert_eq!(list.items().len(), 2);
        assert_eq!(list.active_channel(), Some(CHANNEL_DEFAULT));

        assert!(list.select(99).is_none());
        assert_eq!(list.selected_index(), 1);
    }

    #[test]
    fn canais_por_contexto_filtram_sem_clonar_nem_perder_selecao_ou_workspaces() {
        let ws_padrao = Workspace::system("sys", "Terminal", None);
        let ws_trabalho_1 = Workspace::from_path("ws-1", "/work/erp").with_realm(CHANNEL_WORK);
        let ws_pessoal = Workspace::from_path("ws-2", "/home/eu/blog").with_realm(CHANNEL_PERSONAL);
        let ws_projetos =
            Workspace::from_path("ws-3", "/home/eu/oss/gsh").with_realm(CHANNEL_PROJECTS);
        let ws_trabalho_2 = Workspace::from_path("ws-4", "/work/billing").with_realm("trabalho");

        let mut list = WorkspaceList::new(vec![
            ws_padrao,
            ws_trabalho_1,
            ws_pessoal,
            ws_projetos,
            ws_trabalho_2,
        ]);

        let _ = list.select(2);
        assert_eq!(list.selected_index(), 2);
        assert_eq!(list.active_channel(), None);
        assert_eq!(list.visible_indices(), vec![0, 1, 2, 3, 4]);
        assert_eq!(
            list.channels(),
            vec![
                CHANNEL_DEFAULT.to_string(),
                CHANNEL_PERSONAL.to_string(),
                CHANNEL_PROJECTS.to_string(),
                CHANNEL_WORK.to_string(),
            ]
        );

        list.set_active_channel(Some("trabalho".to_string()));
        assert_eq!(list.active_channel(), Some(CHANNEL_WORK));
        assert_eq!(list.visible_indices(), vec![1, 4]);
        assert_eq!(list.items().len(), 5);
        assert_eq!(list.selected_index(), 2);

        list.set_active_channel(Some(CHANNEL_PERSONAL.to_string()));
        assert_eq!(list.visible_indices(), vec![2]);

        list.set_active_channel(Some(CHANNEL_PROJECTS.to_string()));
        assert_eq!(list.visible_indices(), vec![3]);

        assert!(list.set_workspace_realm(0, CHANNEL_PROJECTS));
        assert!(!list.set_workspace_realm(99, CHANNEL_PROJECTS));
        assert_eq!(list.visible_indices(), vec![0, 3]);
        assert_eq!(list.items().len(), 5);

        list.set_active_channel(Some("Projeto-X".to_string()));
        assert!(list.visible_indices().is_empty());
        assert!(list.channels().contains(&"Projeto-X".to_string()));
        assert!(list.set_workspace_realm(0, "projeto-x"));
        assert_eq!(list.visible_indices(), vec![0]);
        let opened = list.open_folder("/tmp/projeto-x");
        assert_eq!(list.visible_indices(), vec![0, opened]);
        assert_eq!(list.items()[opened].realm(), "Projeto-X");

        list.set_active_channel(None);
        assert_eq!(list.active_channel(), None);
        assert_eq!(list.visible_indices(), vec![0, 1, 2, 3, 4, opened]);
    }
}
