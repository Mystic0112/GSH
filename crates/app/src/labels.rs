//! Constantes de rótulo da UI do GSH (português, espelhando `pt_BR.i18n.json`
//! do app Flutter).
//!
//! Nenhuma string literal de interface fica nos widgets: concentrar aqui
//! permite que o futuro módulo de i18n substitua os textos sem varrer a árvore
//! de componentes.

pub const APP_TITLE: &str = "GSH";

// Shell (colunas laterais e área central)
pub const WORKSPACES_HEADER: &str = "Workspaces";
pub const WORKSPACE_GSH: &str = "GSH";
pub const WORKSPACE_EXAMPLE: &str = "Projeto de exemplo";
pub const PANES_PLACEHOLDER: &str = "Terminal (spike de PTY ainda não ligado)";
pub const FILES_HEADER: &str = "Arquivos";
pub const FILES_PLACEHOLDER: &str = "(árvore entra depois)";

// Ilha de agentes — seções do painel (na ordem exata de `agent_island_panel.dart`)
pub const SECTION_ASKS: &str = "Pedem permissão";
pub const SECTION_WAITING: &str = "Esperando você";
pub const SECTION_FINISHED: &str = "Terminaram";
pub const SECTION_WORKING: &str = "Trabalhando";

// Ilha de agentes — botões e ações do painel
pub const ACTION_ALLOW: &str = "Permitir";
pub const ACTION_ALWAYS: &str = "Sempre";
pub const ACTION_DENY: &str = "Negar";
pub const ACTION_IN_TERMINAL: &str = "No terminal";
pub const ACTION_IN_TERMINAL_TOOLTIP: &str = "Responder no terminal do agente";
pub const ACTION_GO_TO_TAB: &str = "Ir para a aba";
pub const ALWAYS_TOOLTIP_PREFIX: &str = "Sempre permitir ";

// Ilha de agentes — estado vazio e separadores
pub const ISLAND_EMPTY: &str = "Nenhum agente rodando agora.";
pub const SUMMARY_WORKING: &str = "trabalhando";
pub const SUMMARY_WAITING: &str = "esperando você";
pub const SUMMARY_FINISHED: &str = "terminaram";
pub const SUMMARY_SEPARATOR: &str = " · ";
pub const TOOL_SUMMARY_SEPARATOR: &str = " · ";

// Agentes iniciais do shell (enquanto a ligação com o socket de status é outro card)
pub const DEMO_TAB_1_ID: &str = "tab-1";
pub const DEMO_TAB_1_NAME: &str = "claude — feat/island";
pub const DEMO_TAB_1_WORKSPACE: &str = "GSH";
pub const DEMO_ASK_1_ID: &str = "ask-1";
pub const DEMO_ASK_1_TOOL: &str = "Bash";
pub const DEMO_ASK_1_INPUT: &str = "cargo test -p gsh-app";
pub const DEMO_ASK_1_RULE: &str = "Bash(cargo test:*)";

pub const DEMO_TAB_2_ID: &str = "tab-2";
pub const DEMO_TAB_2_NAME: &str = "codex — status-socket";
pub const DEMO_TAB_2_WORKSPACE: &str = "GSH";

pub const DEMO_TAB_3_ID: &str = "tab-3";
pub const DEMO_TAB_3_NAME: &str = "claude — layout";
pub const DEMO_TAB_3_WORKSPACE: &str = "Projeto de exemplo";

/// Resumo textual de acessibilidade da ilha (`tr.summary` do Flutter).
pub fn format_summary(working: usize, waiting: usize, finished: usize) -> String {
    format!(
        "{working} {SUMMARY_WORKING}{SUMMARY_SEPARATOR}{waiting} {SUMMARY_WAITING}{SUMMARY_SEPARATOR}{finished} {SUMMARY_FINISHED}"
    )
}

/// Tooltip do botão "Sempre" (`tr.alwaysTooltip` do Flutter).
pub fn format_always_tooltip(rule: &str) -> String {
    format!("{ALWAYS_TOOLTIP_PREFIX}{rule}")
}

/// Linha de resumo da ferramenta pedindo permissão (`toolName · summary`).
pub fn format_tool_line(tool_name: &str, input_summary: &str) -> String {
    if input_summary.is_empty() {
        tool_name.to_string()
    } else {
        format!("{tool_name}{TOOL_SUMMARY_SEPARATOR}{input_summary}")
    }
}
