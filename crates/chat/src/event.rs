//! Eventos e tipos de conta do chat (`chat_event.dart` e `chat_usage.dart` do Flutter).
//!
//! Um evento por bloco: texto, chamada de ferramenta, resultado, pedido de permissão, fim de
//! turno. A UI monta a conversa a partir disto.

use serde_json::{Map, Value};

/// Um modelo que a conta pode usar no chat (lista do próprio Claude Code).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelOption {
    /// O que vai no `--model` / `set_model` (`opus`, `sonnet`, um id...).
    pub value: String,
    pub display_name: String,
    pub description: String,
    /// Id real que o alias vira (`claude-opus-5-5`).
    pub resolved: Option<String>,
    /// Níveis de esforço que o modelo aceita (`low`...`max`); vazio = não tem.
    pub efforts: Vec<String>,
    /// O CLI atual não informa requisito de créditos por modelo; fica falso até haver sinal.
    pub requires_credits: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    Session,
    Weekly,
    WeeklyModel,
    Other,
}

/// Um limite do plano: quanto já foi usado e quando zera.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limit {
    pub kind: LimitKind,
    /// 0-100.
    pub percent: u8,
    /// Nome do modelo, para limite por modelo ("Fable").
    pub model: Option<String>,
    /// Quando zera, em epoch ms (UTC).
    pub resets_at_ms: Option<u64>,
    /// O Claude Code marcou como perto do fim.
    pub warning: bool,
}

/// Limites do plano (sessão de 5 h, semana, por modelo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanUsage {
    pub limits: Vec<Limit>,
    /// `team`, `max`, `pro`... `None` se não veio.
    pub plan: Option<String>,
}

/// Um comando `/nome` da sessão: comandos do usuário/projeto, skills e os embutidos do
/// Claude Code (lista do `initialize`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOption {
    pub name: String,
    pub description: String,
    /// O que vem depois do nome (`<arquivo>`, `[tarefa]`...).
    pub argument_hint: String,
    pub aliases: Vec<String>,
}

/// O que dá para saber da conta sem conversa aberta: modelos, comandos e uso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountProbe {
    pub models: Vec<ModelOption>,
    pub usage: Option<PlanUsage>,
    pub commands: Vec<CommandOption>,
}

/// Estado que o Claude Code classifica ao fim do turno (`system/post_turn_summary`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Completed,
    ReviewReady,
    Blocked,
    Failed,
}

/// O que o motor de chat (Claude Code em `stream-json`) avisa.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatEvent {
    /// Sessão no ar (`system/init`).
    Ready {
        session_id: String,
        model: String,
        permission_mode: Option<String>,
    },
    /// O modo de permissão mudou (`plan`, `default`, `acceptEdits`...).
    ModeChanged(String),
    /// Pedaço de texto chegando (`--include-partial-messages`). O [`ChatEvent::Text`] vem depois.
    TextDelta(String),
    /// Pedaço do resumo de raciocínio (`thinking_delta`).
    ThinkingDelta(String),
    /// Resumo de raciocínio do modelo (um bloco `thinking` com texto).
    Thinking(String),
    /// O Claude Code classificou a sessão ao fim do turno (`detail` = `status_detail` cru).
    SessionClassified { state: SessionState, detail: String },
    /// Mensagem de OUTRA sessão do Claude Code (SendMessage entre sessões).
    PeerMessage {
        from_name: String,
        from: String,
        text: String,
    },
    /// Aviso do próprio Claude Code (`system/informational`).
    Info { text: String, warning: bool },
    /// Mensagem do usuário lida do histórico. `at_ms` = quando foi escrita (`None` = agora).
    UserText { text: String, at_ms: Option<u64> },
    /// Nome dado à sessão (`/rename`), lido do histórico.
    Renamed(String),
    /// Texto do assistente (um bloco `text`).
    Text { text: String, at_ms: Option<u64> },
    /// O assistente chamou uma ferramenta.
    ToolCall {
        id: String,
        name: String,
        input: Map<String, Value>,
    },
    /// Uma ferramenta voltou.
    ToolResult {
        tool_use_id: String,
        is_error: bool,
        text: String,
    },
    /// O Claude Code pede permissão para rodar uma ferramenta e espera a resposta.
    PermissionAsk {
        request_id: String,
        tool_name: String,
        input: Map<String, Value>,
        description: Option<String>,
        /// Regra que "sempre permitir" grava (ex.: `Bash(npm test *)`).
        always_rule: Option<String>,
    },
    /// Fim do turno (`result`).
    TurnEnd {
        is_error: bool,
        text: String,
        session_id: String,
        cost_usd: Option<f64>,
    },
    /// O processo do motor saiu sem a gente pedir.
    EngineFailed {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// Modelos que a conta pode usar (resposta do `initialize`).
    Models(Vec<ModelOption>),
    /// O modelo da conversa mudou (pedido do usuário). O valor é o id real.
    ModelChanged(String),
    /// Tamanho do contexto (a cada resposta do modelo) e janela do modelo (ao fim do turno).
    /// 0 = não veio.
    Context { tokens: u64, window: u64 },
    /// Comandos `/nome` da sessão (resposta do `initialize`).
    Commands(Vec<CommandOption>),
    /// Resposta ao `get_usage` (`None` = o motor não soube dizer).
    PlanUsage(Option<PlanUsage>),
}
