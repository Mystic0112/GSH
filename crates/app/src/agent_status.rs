//! Ponte entre o `gsh_ipc::StatusServer` + `gsh_core::status::TurnTracker` e a
//! ilha de agentes (`AgentIsland`).
//!
//! Responsabilidades:
//! 1. Subir o `StatusServer` uma vez no startup do `gsh-app` e converter
//!    eventual falha de inicialização em [`IslandServerError`] tipado (sem panic).
//! 2. Montar as variáveis de ambiente (`GSH_PANE_ID`, `GSH_TAB_ID` e o transporte
//!    de `StatusServer::hook_env`) injetadas em cada sessão de terminal.
//! 3. Receber [`Delivery`] do canal do `StatusServer`, roteá-lo ao [`TurnTracker`]
//!    do terminal correspondente (`pane_id`) e produzir a lista de [`IslandAgent`]
//!    ativos para a ilha.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{SystemTime, UNIX_EPOCH};

use gsh_core::file_activity::AgentFileActivity;
use gsh_core::status::{AgentHarness, Applied, HookEvent, StatusUpdate, TurnStatus, TurnTracker};
use gsh_core::wire::{ENV_PANE_ID, ENV_TAB_ID};
use gsh_ipc::{gsh_dir_from_env, Delivery, Endpoint, ServerConfig, ServerError, StatusServer};

use crate::island::{IslandAgent, IslandAgentState, PermissionAsk, PermissionDecision};
use crate::labels;
use crate::pane_model::TermId;

/// Erro tipado de inicialização do `StatusServer`, exibido pela ilha sem panic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IslandServerError {
    CreateDir { path: PathBuf, detail: String },
    StaleSocket { path: PathBuf, detail: String },
    PathOccupied { path: PathBuf },
    Bind { target: String, detail: String },
    Permissions { path: PathBuf, detail: String },
    Token { detail: String },
    Thread { detail: String },
}

impl From<&ServerError> for IslandServerError {
    fn from(err: &ServerError) -> Self {
        match err {
            ServerError::CreateDir { path, source } => Self::CreateDir {
                path: path.clone(),
                detail: source.to_string(),
            },
            ServerError::StaleSocket { path, source } => Self::StaleSocket {
                path: path.clone(),
                detail: source.to_string(),
            },
            ServerError::PathOccupied { path } => Self::PathOccupied { path: path.clone() },
            ServerError::Bind { target, source } => Self::Bind {
                target: target.clone(),
                detail: source.to_string(),
            },
            ServerError::Permissions { path, source } => Self::Permissions {
                path: path.clone(),
                detail: source.to_string(),
            },
            ServerError::Token(source) => Self::Token {
                detail: source.to_string(),
            },
            ServerError::Thread(source) => Self::Thread {
                detail: source.to_string(),
            },
        }
    }
}

impl From<ServerError> for IslandServerError {
    fn from(err: ServerError) -> Self {
        Self::from(&err)
    }
}

impl IslandServerError {
    /// Mensagem formatada para exibição na seção de erro da ilha (usando `labels.rs`).
    pub fn display_message(&self) -> String {
        match self {
            Self::CreateDir { path, detail } => format!(
                "{}{}{}{detail}",
                labels::STATUS_ERR_CREATE_DIR_PREFIX,
                path.display(),
                labels::STATUS_ERR_TARGET_SEP
            ),
            Self::StaleSocket { path, detail } => format!(
                "{}{}{}{detail}",
                labels::STATUS_ERR_STALE_SOCKET_PREFIX,
                path.display(),
                labels::STATUS_ERR_TARGET_SEP
            ),
            Self::PathOccupied { path } => {
                format!(
                    "{}{}",
                    labels::STATUS_ERR_PATH_OCCUPIED_PREFIX,
                    path.display()
                )
            }
            Self::Bind { target, detail } => format!(
                "{}{target}{}{detail}",
                labels::STATUS_ERR_BIND_PREFIX,
                labels::STATUS_ERR_TARGET_SEP
            ),
            Self::Permissions { path, detail } => format!(
                "{}{}{}{detail}",
                labels::STATUS_ERR_PERMISSIONS_PREFIX,
                path.display(),
                labels::STATUS_ERR_TARGET_SEP
            ),
            Self::Token { detail } => format!("{}{detail}", labels::STATUS_ERR_TOKEN_PREFIX),
            Self::Thread { detail } => format!("{}{detail}", labels::STATUS_ERR_THREAD_PREFIX),
        }
    }
}

/// Converte o [`TermId`] numérico de uma aba num `pane_id` estável (`"t1"`, `"t2"`, ...),
/// espelhando `_nid('t')` no Flutter.
pub fn pane_id_for_term(id: TermId) -> String {
    format!("t{id}")
}

/// Extrai o [`TermId`] de um `pane_id` no formato `"t<num>"` (ou numérico puro).
pub fn term_id_from_pane_id(pane_id: &str) -> Option<TermId> {
    let digits = pane_id.strip_prefix('t').unwrap_or(pane_id);
    digits.parse::<TermId>().ok()
}

/// Monta o par de variáveis de ambiente de hook para um terminal específico:
/// `GSH_PANE_ID`, `GSH_TAB_ID` e o transporte retornado por `StatusServer::hook_env()`
/// (`GSH_STATUS_SOCK` em Unix, ou `GSH_STATUS_PORT`/`GSH_STATUS_TOKEN` em TCP).
pub fn build_pane_hook_env(pane_id: &str, server_env: &[(&str, String)]) -> Vec<(String, String)> {
    let mut env = Vec::with_capacity(server_env.len() + 2);
    env.push((ENV_PANE_ID.to_string(), pane_id.to_string()));
    env.push((ENV_TAB_ID.to_string(), pane_id.to_string()));
    for (k, v) in server_env {
        env.push(((*k).to_string(), v.clone()));
    }
    env
}

/// Relógio monotônico/epoch em milissegundos para o [`TurnTracker`].
pub fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// Nome amigável do harness vindo de `labels.rs`.
pub fn harness_label(harness: AgentHarness) -> &'static str {
    match harness {
        AgentHarness::Claude => labels::HARNESS_CLAUDE,
        AgentHarness::Codex => labels::HARNESS_CODEX,
        AgentHarness::Pi => labels::HARNESS_PI,
    }
}

/// Metadados de um terminal aberto no `PaneModel` necessários para montar o [`IslandAgent`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaneDescriptor {
    pub pane_id: String,
    pub tab_title: String,
    pub workspace_name: String,
}

impl PaneDescriptor {
    pub fn new(
        pane_id: impl Into<String>,
        tab_title: impl Into<String>,
        workspace_name: impl Into<String>,
    ) -> Self {
        Self {
            pane_id: pane_id.into(),
            tab_title: tab_title.into(),
            workspace_name: workspace_name.into(),
        }
    }
}

/// Estado de turno por terminal (`pane_id`), combinando [`TurnTracker`] do `gsh-core`
/// com os metadados do último [`StatusUpdate`].
#[derive(Clone, Debug, Default)]
pub struct PaneAgentRecord {
    tracker: TurnTracker,
    observed: bool,
    last_event: Option<HookEvent>,
    harness: Option<AgentHarness>,
    session_id: Option<String>,
    transcript_path: Option<String>,
    files: Vec<String>,
    unseen_finish: bool,
    pending_ask: Option<PermissionAsk>,
}

impl PaneAgentRecord {
    pub fn observed(&self) -> bool {
        self.observed
    }

    pub fn status(&self) -> TurnStatus {
        self.tracker.status()
    }

    pub fn last_event(&self) -> Option<&HookEvent> {
        self.last_event.as_ref()
    }

    pub fn harness(&self) -> Option<AgentHarness> {
        self.harness
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn transcript_path(&self) -> Option<&str> {
        self.transcript_path.as_deref()
    }

    pub fn files(&self) -> &[String] {
        &self.files
    }

    pub fn unseen_finish(&self) -> bool {
        self.unseen_finish
    }

    pub fn pending_ask(&self) -> Option<&PermissionAsk> {
        self.pending_ask.as_ref()
    }

    /// Converte o estado atual deste terminal no [`IslandAgentState`] da ilha.
    pub fn island_state(&self) -> Option<IslandAgentState> {
        IslandAgentState::from_turn_status(
            self.tracker.status(),
            self.last_event.as_ref(),
            self.unseen_finish,
        )
    }

    /// Aplica um [`StatusUpdate`] ao [`TurnTracker`] deste terminal.
    /// Retorna `true` se o estado observável pela ilha mudou.
    pub fn apply_update(&mut self, update: &StatusUpdate, now_ms: u64) -> bool {
        let first_observation = !self.observed;
        let prev_state = self.island_state();
        let prev_harness = self.harness;

        let applied = self.tracker.apply(update, now_ms);
        if applied == Applied::Stale {
            return false;
        }

        self.observed = true;
        if let Some(h) = update.harness {
            self.harness = Some(h);
        }
        if update.event.is_some() {
            self.last_event = update.event.clone();
        }
        if let Some(sid) = &update.session_id {
            self.session_id = Some(sid.clone());
        }
        if let Some(tp) = &update.transcript_path {
            self.transcript_path = Some(tp.clone());
        }
        if !update.files.is_empty() {
            self.files = update.files.clone();
        }

        match self.tracker.status() {
            TurnStatus::Working => {
                self.unseen_finish = false;
                self.pending_ask = None;
            }
            TurnStatus::Waiting => {
                self.unseen_finish = false;
                if !matches!(self.last_event, Some(HookEvent::PermissionRequest)) {
                    self.pending_ask = None;
                }
            }
            TurnStatus::Idle => {
                self.pending_ask = None;
                if matches!(
                    update.event,
                    Some(HookEvent::SessionStart | HookEvent::SessionEnd)
                ) {
                    self.unseen_finish = false;
                } else if matches!(
                    applied,
                    Applied::Changed {
                        from: TurnStatus::Working | TurnStatus::Waiting
                    }
                ) || matches!(update.event, Some(HookEvent::Stop))
                {
                    self.unseen_finish = true;
                }
            }
        }

        first_observation || self.island_state() != prev_state || self.harness != prev_harness
    }
}

/// Gerenciador de runtime do servidor de status e dos [`TurnTracker`]s por pane.
pub struct StatusRuntime {
    server: Option<StatusServer>,
    receiver: Option<Receiver<Delivery>>,
    server_error: Option<IslandServerError>,
    panes: BTreeMap<String, PaneAgentRecord>,
    /// Arquivos tocados por aba, acumulados entre lotes do hook (`agentFiles` do Flutter).
    activity: AgentFileActivity,
    malformed_count: u64,
    commands: Vec<gsh_ipc::PendingCommand>,
}

impl StatusRuntime {
    /// Inicia o `StatusServer` uma vez usando o diretório padrão (`gsh_dir_from_env()`).
    /// Se falhar, guarda o [`IslandServerError`] tipado e segue sem socket.
    pub fn start_default() -> Self {
        let cfg = ServerConfig::new(gsh_dir_from_env());
        Self::start_with_config(&cfg)
    }

    /// Inicia o `StatusServer` com uma [`ServerConfig`] explícita (útil em testes).
    pub fn start_with_config(cfg: &ServerConfig) -> Self {
        match StatusServer::start(cfg) {
            Ok((server, receiver)) => Self {
                server: Some(server),
                receiver: Some(receiver),
                server_error: None,
                panes: BTreeMap::new(),
                activity: AgentFileActivity::default(),
                malformed_count: 0,
                commands: Vec::new(),
            },
            Err(err) => Self {
                server: None,
                receiver: None,
                server_error: Some(IslandServerError::from(&err)),
                panes: BTreeMap::new(),
                activity: AgentFileActivity::default(),
                malformed_count: 0,
                commands: Vec::new(),
            },
        }
    }

    /// Cria um runtime sem socket ativo (para testes unitários ou estado de erro injetado).
    pub fn without_server(server_error: Option<IslandServerError>) -> Self {
        Self {
            server: None,
            receiver: None,
            server_error,
            panes: BTreeMap::new(),
            activity: AgentFileActivity::default(),
            malformed_count: 0,
            commands: Vec::new(),
        }
    }

    /// Erro tipado caso o `StatusServer` tenha falhado no startup.
    pub fn server_error(&self) -> Option<&IslandServerError> {
        self.server_error.as_ref()
    }

    /// Endpoint ativo do `StatusServer` (quando iniciado com sucesso).
    pub fn endpoint(&self) -> Option<&Endpoint> {
        self.server.as_ref().map(StatusServer::endpoint)
    }

    /// Quantidade de linhas malformadas descartadas pelo servidor.
    pub fn malformed_count(&self) -> u64 {
        self.malformed_count
    }

    pub fn take_commands(&mut self) -> Vec<gsh_ipc::PendingCommand> {
        std::mem::take(&mut self.commands)
    }

    /// Variáveis de ambiente (`GSH_PANE_ID`, `GSH_TAB_ID`, `GSH_STATUS_SOCK` / TCP)
    /// para injetar ao criar ou reiniciar o processo filho do terminal `pane_id`.
    pub fn hook_env_for_pane(&self, pane_id: &str) -> Vec<(String, String)> {
        let server_env = self
            .server
            .as_ref()
            .map(StatusServer::hook_env)
            .unwrap_or_default();
        build_pane_hook_env(pane_id, &server_env)
    }

    /// Registra um terminal recém-aberto com um [`TurnTracker`] em `Idle`.
    pub fn register_pane(&mut self, pane_id: impl Into<String>) {
        self.panes.entry(pane_id.into()).or_default();
    }

    /// Remove o registro de status de um terminal fechado.
    pub fn remove_pane(&mut self, pane_id: &str) -> bool {
        self.activity.forget_tab(pane_id);
        self.panes.remove(pane_id).is_some()
    }

    /// Arquivos tocados pelos agentes de cada terminal (`agentFiles` do Flutter).
    pub fn file_activity(&self) -> &AgentFileActivity {
        &self.activity
    }

    /// Consulta o estado acumulado de um terminal pelo `pane_id`.
    pub fn pane_record(&self, pane_id: &str) -> Option<&PaneAgentRecord> {
        self.panes.get(pane_id)
    }

    /// Drena todas as mensagens pendentes do `Receiver<Delivery>` sem bloquear
    /// e avança os prazos (`poll_finished`) dos [`TurnTracker`]s.
    /// Retorna `true` se algum estado mudou.
    pub fn drain_deliveries(&mut self, now_ms: u64) -> bool {
        let mut deliveries = Vec::new();
        let mut disconnected = false;
        if let Some(rx) = &self.receiver {
            loop {
                match rx.try_recv() {
                    Ok(delivery) => deliveries.push(delivery),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        if disconnected {
            self.receiver = None;
        }

        let mut changed = false;
        for delivery in deliveries {
            if self.apply_delivery(delivery, now_ms) {
                changed = true;
            }
        }

        for record in self.panes.values_mut() {
            let _ = record.tracker.poll_finished(now_ms);
        }

        changed
    }

    /// Processa uma única [`Delivery`] vinda do `StatusServer`.
    pub fn apply_delivery(&mut self, delivery: Delivery, now_ms: u64) -> bool {
        match delivery {
            Delivery::Status(update) => self.apply_update(update, now_ms),
            Delivery::Command(command) => {
                self.commands.push(command);
                false
            }
            Delivery::Malformed(_) => {
                self.malformed_count = self.malformed_count.saturating_add(1);
                false
            }
        }
    }

    /// Aplica um [`StatusUpdate`] ao terminal correspondente (`update.pane_id`).
    /// Se o `pane_id` pertencer a um terminal registrado (ou for criado sob demanda),
    /// atualiza o [`TurnTracker`].
    pub fn apply_update(&mut self, update: StatusUpdate, now_ms: u64) -> bool {
        let Some(record) = self.panes.get_mut(&update.pane_id) else {
            return false;
        };
        // Como o Flutter, registra os arquivos de todo lote com `files`, antes de o rastreador
        // de turno decidir se o status é obsoleto.
        self.activity
            .record(&update.pane_id, &update.files, false, now_ms);
        record.apply_update(&update, now_ms)
    }

    /// Trata `ESC` cru enviado pelo usuário ao terminal enquanto o agente está `Working`.
    pub fn interrupt_pane(&mut self, pane_id: &str, now_ms: u64) -> bool {
        let Some(record) = self.panes.get_mut(pane_id) else {
            return false;
        };
        if record.tracker.interrupt(now_ms) {
            record.unseen_finish = false;
            record.pending_ask = None;
            true
        } else {
            false
        }
    }

    /// Consome o estado "terminou e ainda não foi visto" quando o usuário visita a aba.
    pub fn mark_pane_seen(&mut self, pane_id: &str) -> bool {
        let Some(record) = self.panes.get_mut(pane_id) else {
            return false;
        };
        if record.unseen_finish {
            record.unseen_finish = false;
            true
        } else {
            false
        }
    }

    /// Associa (ou limpa) os detalhes do card de permissão de um terminal.
    pub fn set_pending_ask(&mut self, pane_id: &str, ask: Option<PermissionAsk>) -> bool {
        let Some(record) = self.panes.get_mut(pane_id) else {
            return false;
        };
        record.pending_ask = ask;
        true
    }

    /// Aplica a decisão tomada pelo usuário num card de permissão da ilha.
    pub fn apply_permission_decision(
        &mut self,
        pane_id: &str,
        decision: PermissionDecision,
        now_ms: u64,
    ) -> bool {
        let Some(record) = self.panes.get_mut(pane_id) else {
            return false;
        };
        record.pending_ask = None;
        match decision {
            PermissionDecision::Allow | PermissionDecision::AllowAlways => {
                let _ = record
                    .tracker
                    .apply_status(TurnStatus::Working, false, now_ms);
                record.last_event = Some(HookEvent::PreToolUse);
                record.unseen_finish = false;
            }
            PermissionDecision::Deny => {
                let _ = record.tracker.apply_status(TurnStatus::Idle, false, now_ms);
                record.last_event = Some(HookEvent::Stop);
                record.unseen_finish = true;
            }
            PermissionDecision::Pass => {
                let _ = record
                    .tracker
                    .apply_status(TurnStatus::Waiting, false, now_ms);
                record.last_event = Some(HookEvent::Notification);
                record.unseen_finish = false;
            }
        }
        true
    }

    /// Constrói a lista de [`IslandAgent`] ativos a partir dos terminais abertos.
    pub fn island_agents(&self, descriptors: &[PaneDescriptor]) -> Vec<IslandAgent> {
        let mut out = Vec::new();
        for desc in descriptors {
            let Some(record) = self.panes.get(&desc.pane_id) else {
                continue;
            };
            let Some(state) = record.island_state() else {
                continue;
            };
            let h_label = record.harness.map(harness_label);
            let name = labels::format_agent_tab_name(h_label, &desc.tab_title);
            let mut agent = IslandAgent::new(
                desc.pane_id.clone(),
                name,
                desc.workspace_name.clone(),
                state,
            );
            if let Some(h) = record.harness {
                agent = agent.with_harness(h);
            }
            if state == IslandAgentState::AskingPermission {
                if let Some(ask) = &record.pending_ask {
                    agent = agent.with_ask(ask.clone());
                }
            }
            out.push(agent);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    use super::*;

    static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempStatusDir {
        path: PathBuf,
    }

    impl TempStatusDir {
        fn new() -> Self {
            let id = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gsh-agent-status-test-{}-{}",
                std::process::id(),
                id
            ));
            fs::create_dir_all(&path).expect("create temp status dir");
            Self { path }
        }
    }

    impl Drop for TempStatusDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn injeta_envs_de_hook_com_pane_id_estavel() {
        let tmp = TempStatusDir::new();
        let runtime = StatusRuntime::start_with_config(&ServerConfig::new(&tmp.path));
        assert!(runtime.server_error().is_none());

        let pane_id = pane_id_for_term(1);
        assert_eq!(pane_id, "t1");
        assert_eq!(term_id_from_pane_id(&pane_id), Some(1));

        let env = runtime.hook_env_for_pane(&pane_id);
        assert!(env.iter().any(|(k, v)| k == "GSH_PANE_ID" && v == "t1"));
        assert!(env.iter().any(|(k, v)| k == "GSH_TAB_ID" && v == "t1"));
        assert!(env
            .iter()
            .any(|(k, _)| k == "GSH_STATUS_SOCK" || k == "GSH_STATUS_PORT"));
    }

    #[test]
    fn falha_no_startup_do_status_server_gera_erro_tipado_sem_panic() {
        let tmp = TempStatusDir::new();
        // Ocupa o caminho onde o diretório do socket deveria ser criado com um arquivo regular.
        let blocked_dir = tmp.path.join("arquivo-bloqueante");
        fs::write(&blocked_dir, b"nao eh diretorio").unwrap();

        let runtime = StatusRuntime::start_with_config(&ServerConfig::new(&blocked_dir));
        let err = runtime.server_error().expect("deve registrar erro tipado");
        assert!(!err.display_message().is_empty());

        // Mesmo sem servidor, hook_env_for_pane continua seguro e devolve os IDs do pane.
        let env = runtime.hook_env_for_pane("t1");
        assert!(env.iter().any(|(k, v)| k == "GSH_PANE_ID" && v == "t1"));
    }

    #[test]
    fn ciclo_completo_de_turno_por_pane_id_ate_lista_da_ilha() {
        let mut runtime = StatusRuntime::without_server(None);
        runtime.register_pane("t1");
        runtime.register_pane("t2");

        let descriptors = vec![
            PaneDescriptor::new("t1", "Terminal", "GSH"),
            PaneDescriptor::new("t2", "Terminal", "Projeto de exemplo"),
        ];

        // Inicialmente ambos ociosos: ilha vazia.
        assert!(runtime.island_agents(&descriptors).is_empty());

        // t1 inicia turno (UserPromptSubmit -> Working).
        let u1 = StatusUpdate {
            pane_id: "t1".into(),
            status: TurnStatus::Working,
            event: Some(HookEvent::UserPromptSubmit),
            session_id: None,
            transcript_path: None,
            harness: Some(AgentHarness::Claude),
            files: Vec::new(),
        };
        assert!(runtime.apply_update(u1, 1_000));

        let agents = runtime.island_agents(&descriptors);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].tab_id, "t1");
        assert_eq!(agents[0].name, "claude — Terminal");
        assert_eq!(agents[0].state, IslandAgentState::Working);

        // t2 pede permissão (PermissionRequest -> AskingPermission).
        let u2 = StatusUpdate {
            pane_id: "t2".into(),
            status: TurnStatus::Waiting,
            event: Some(HookEvent::PermissionRequest),
            session_id: None,
            transcript_path: None,
            harness: Some(AgentHarness::Codex),
            files: Vec::new(),
        };
        assert!(runtime.apply_update(u2, 1_100));

        let agents = runtime.island_agents(&descriptors);
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[1].tab_id, "t2");
        assert_eq!(agents[1].state, IslandAgentState::AskingPermission);

        // t1 termina o turno (Stop -> Idle com unseen_finish = true -> Finished).
        let u1_stop = StatusUpdate {
            pane_id: "t1".into(),
            status: TurnStatus::Idle,
            event: Some(HookEvent::Stop),
            session_id: None,
            transcript_path: None,
            harness: Some(AgentHarness::Claude),
            files: Vec::new(),
        };
        assert!(runtime.apply_update(u1_stop, 2_000));

        let agents = runtime.island_agents(&descriptors);
        assert_eq!(agents[0].state, IslandAgentState::Finished);

        // Um PreToolUse atrasado (stale < 5s) após o Stop em t1 é ignorado pelo TurnTracker.
        let u1_stale = StatusUpdate {
            pane_id: "t1".into(),
            status: TurnStatus::Working,
            event: Some(HookEvent::PreToolUse),
            session_id: None,
            transcript_path: None,
            harness: None,
            files: Vec::new(),
        };
        assert!(!runtime.apply_update(u1_stale, 2_500));
        assert_eq!(
            runtime.island_agents(&descriptors)[0].state,
            IslandAgentState::Finished
        );

        // Usuário visita a aba t1: consome o aviso "Terminaram" e t1 volta a ficar oculto (Idle).
        assert!(runtime.mark_pane_seen("t1"));
        let agents = runtime.island_agents(&descriptors);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].tab_id, "t2");
    }

    fn files_update(pane_id: &str, event: HookEvent, files: &[&str]) -> StatusUpdate {
        StatusUpdate {
            pane_id: pane_id.into(),
            status: TurnStatus::Working,
            event: Some(event),
            session_id: None,
            transcript_path: None,
            harness: Some(AgentHarness::Claude),
            files: files.iter().map(|f| (*f).to_string()).collect(),
        }
    }

    #[test]
    fn arquivos_tocados_acumulam_entre_lotes_e_somem_com_a_aba() {
        let mut runtime = StatusRuntime::without_server(None);
        runtime.register_pane("t1");
        runtime.register_pane("t2");

        runtime.apply_update(
            files_update("t1", HookEvent::PostToolUse, &["/p/a.rs"]),
            1_000,
        );
        runtime.apply_update(
            files_update("t1", HookEvent::PostToolUse, &["/p/b.rs"]),
            2_000,
        );
        runtime.apply_update(
            files_update("t2", HookEvent::PostToolUse, &["/p/a.rs"]),
            3_000,
        );
        // Lote sem arquivos não apaga o que já foi registrado.
        runtime.apply_update(files_update("t1", HookEvent::PreToolUse, &[]), 4_000);

        let activity = runtime.file_activity();
        assert_eq!(activity.touched_count("t1"), 2);
        let quem: Vec<&str> = activity
            .touchers_of("/p/a.rs")
            .iter()
            .map(|t| t.tab_id)
            .collect();
        assert_eq!(quem, ["t2", "t1"]);
        assert!(activity
            .touchers_of("/p/a.rs")
            .iter()
            .all(|t| !t.touch.inferred));

        // Pane não registrado não grava nada.
        runtime.apply_update(
            files_update("t9", HookEvent::PostToolUse, &["/p/z.rs"]),
            5_000,
        );
        assert!(runtime.file_activity().touchers_of("/p/z.rs").is_empty());

        assert!(runtime.remove_pane("t1"));
        assert_eq!(runtime.file_activity().touched_count("t1"), 0);
        assert_eq!(runtime.file_activity().touchers_of("/p/a.rs").len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn recebe_delivery_real_pelo_socket_unix_e_atualiza_ilha() {
        use std::os::unix::net::UnixStream;

        let tmp = TempStatusDir::new();
        let mut runtime = StatusRuntime::start_with_config(&ServerConfig::new(&tmp.path));
        assert!(runtime.server_error().is_none());
        runtime.register_pane("t1");

        let Endpoint::Unix(sock_path) = runtime.endpoint().unwrap().clone() else {
            panic!("esperava endpoint Unix");
        };

        let mut stream = UnixStream::connect(&sock_path).unwrap();
        writeln!(
            stream,
            r#"{{"paneId":"t1","st":"working","hn":"claude","ev":"UserPromptSubmit"}}"#
        )
        .unwrap();
        stream.flush().unwrap();
        drop(stream);

        // Aguarda a entrega chegar pelo canal do StatusServer.
        let mut updated = false;
        for step in 0..40 {
            if runtime.drain_deliveries(1_000 + step * 10) {
                updated = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(updated, "deveria ter recebido o Delivery pelo StatusServer");

        let descriptors = vec![PaneDescriptor::new("t1", "Terminal", "GSH")];
        let agents = runtime.island_agents(&descriptors);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].tab_id, "t1");
        assert_eq!(agents[0].state, IslandAgentState::Working);
        assert_eq!(agents[0].harness, Some(AgentHarness::Claude));
    }
}
