//! Constantes de rótulo da UI do GSH (português, espelhando `pt_BR.i18n.json`
//! do app Flutter).
//!
//! Nenhuma string literal de interface fica nos widgets: concentrar aqui
//! permite que o futuro módulo de i18n substitua os textos sem varrer a árvore
//! de componentes.

pub const APP_TITLE: &str = "GSH";

// Shell (colunas laterais, topbar, abas e rodapés)
pub const WORKSPACES_HEADER: &str = "Workspaces";
pub const WORKSPACE_GSH: &str = "GSH";
pub const WORKSPACE_GSH_INITIAL: &str = "G";
pub const WORKSPACE_GSH_BRANCH: &str = "main";
pub const WORKSPACE_EXAMPLE: &str = "Projeto de exemplo";
pub const WORKSPACE_EXAMPLE_INITIAL: &str = "P";
pub const WORKSPACE_EXAMPLE_BRANCH: &str = "feat/ui";
pub const DEFAULT_REALM: &str = "Padrão";

pub const DEFAULT_TAB_TITLE: &str = "Terminal";
pub const CHAT_TAB_TITLE: &str = "Chat";
pub const PANES_PLACEHOLDER: &str = "Terminal (spike de PTY ainda não ligado)";
pub const PANE_EMPTY_STATE: &str = "Nenhuma aba aberta — clique em + para abrir um terminal.";
pub const TERMINAL_SPAWN_ERROR: &str = "Não foi possível iniciar a sessão do terminal.";

// Chat com o agente (`pt_BR.i18n.json` -> `gsh.chat`)
pub const CHAT_NEW_CONVERSATION: &str = "Nova conversa";
pub const CHAT_EMPTY_TITLE: &str = "Nova conversa";
pub const CHAT_EMPTY_SUBTITLE_PREFIX: &str = "Claude Code em ";
pub const CHAT_EMPTY_SUBTITLE_SUFFIX: &str = ", com o seu login.";
pub const CHAT_EMPTY_HINT_SEND: &str = "Enter envia; Shift+Enter quebra linha.";
pub const CHAT_EMPTY_HINT_PASTE: &str = "Cole um print com Ctrl+V ou solte arquivos no chat.";
pub const CHAT_EMPTY_HINT_MODE: &str =
    "Escolha o modo de permissão embaixo do campo: pedir, aceitar edições, planejar ou ignorar.";
pub const CHAT_PLACEHOLDER: &str =
    "Mensagem para o Claude… (Enter envia, Shift+Enter quebra linha)";
pub const CHAT_SEND: &str = "Enviar";
pub const CHAT_STOP: &str = "Parar";
pub const CHAT_WORKING: &str = "trabalhando…";
pub const CHAT_MODE_DEFAULT: &str = "Pedir permissão";
pub const CHAT_STREAM_CURSOR: &str = " ▍";
pub const CHAT_START_FAILED_PREFIX: &str = "Não consegui iniciar o claude: ";
pub const CHAT_ENGINE_EXITED_PREFIX: &str = "O claude encerrou (código ";
pub const CHAT_ENGINE_EXITED_SUFFIX: &str = ").";
pub const CHAT_TURN_FAILED_PREFIX: &str = "O turno terminou com erro: ";
pub const CHAT_COST_PREFIX: &str = "custo US$ ";
pub const CHAT_CMD_PREFIX: &str = "$ ";
pub const CHAT_TOOL_NAME_PAD: &str = "  ";

// Viewer de arquivo (estados tipados de limite, binário e erro de leitura)
pub const VIEWER_TOO_LARGE_PREFIX: &str = "Arquivo muito grande para exibir no viewer (";
pub const VIEWER_TOO_LARGE_LIMIT_SEP: &str = " MiB; limite: ";
pub const VIEWER_TOO_LARGE_SUFFIX: &str = " MiB).";
pub const VIEWER_BINARY_NULL_BYTES: &str =
    "Arquivo binário detectado (contém bytes nulos) — visualização de texto indisponível.";
pub const VIEWER_BINARY_INVALID_UTF8: &str =
    "Arquivo binário ou codificação não UTF-8 — visualização de texto indisponível.";
pub const VIEWER_ERR_NOT_FOUND: &str = "Arquivo não encontrado.";
pub const VIEWER_ERR_PERMISSION: &str = "Permissão negada ao ler o arquivo.";
pub const VIEWER_ERR_NOT_FILE: &str = "O caminho selecionado não é um arquivo regular.";
pub const VIEWER_ERR_IO_PREFIX: &str = "Erro ao ler arquivo: ";

pub const FILES_HEADER: &str = "Arquivos";
pub const FILES_SECTION_UPPER: &str = "ARQUIVOS";
pub const FILES_PLACEHOLDER: &str = "(árvore entra depois)";
pub const FILES_NO_FOLDER: &str = "Nenhuma pasta — abra um workspace.";
pub const FILES_EMPTY_FOLDER: &str = "Pasta vazia.";
pub const FILES_ERR_NOT_FOUND: &str = "Pasta não encontrada.";
pub const FILES_ERR_PERMISSION: &str = "Permissão negada ao ler a pasta.";
pub const FILES_ERR_IO_PREFIX: &str = "Erro ao ler pasta: ";
pub const NO_LSP_AVAILABLE: &str = "Nenhum LSP disponível";

// Ícones tipográficos dos controles de chrome
pub const ICON_SIDEBAR_LEFT: &str = "◧";
pub const ICON_SIDEBAR_RIGHT: &str = "◨";
pub const ICON_LAYERS: &str = "◈";
pub const ICON_PLUS: &str = "+";
pub const ICON_CLOSE: &str = "×";
pub const ICON_TERMINAL: &str = ">_";
pub const ICON_SPLIT_H: &str = "◫";
pub const ICON_SPLIT_V: &str = "⬒";
pub const ICON_FOLDER: &str = "◰";
pub const ICON_REFRESH: &str = "↻";
pub const ICON_COLLAPSE: &str = "▴";
pub const ICON_SETTINGS: &str = "⚙";
pub const ICON_CHEVRON_DOWN: &str = "▾";
pub const ICON_CHEVRON_RIGHT: &str = "▸";
pub const ICON_FILE_ITEM: &str = "◦";
pub const ICON_FILE_CODE: &str = "‹›";
pub const ICON_FILE_CONFIG: &str = "⚙";
pub const ICON_FILE_DOC: &str = "¶";
pub const ICON_FILE_LOCK: &str = "⚲";
pub const ICON_FILE_GENERIC: &str = "≡";
pub const ICON_CHAT: &str = "✦";
pub const ICON_SEND: &str = "↵";
pub const ICON_STOP: &str = "■";
pub const ICON_TOOL_RUNNING: &str = "◌";
pub const ICON_GIT_BRANCH: &str = "⎇";
pub const ICON_DIFF: &str = "±";
pub const ICON_MINUS: &str = "−";

// Controle de versão (Git) — painel lateral, histórico e diff (`pt_BR.i18n.json` -> `gsh.fileTreePanel`)
pub const SOURCE_CONTROL_HEADER: &str = "Controle de versão";
pub const SOURCE_CONTROL_SECTION_UPPER: &str = "CONTROLE DE VERSÃO";
pub const GIT_TAB_CHANGES: &str = "Alterações";
pub const GIT_TAB_HISTORY: &str = "Histórico";
pub const GIT_NO_CHANGES: &str = "Nenhuma alteração.";
pub const GIT_NO_REPO: &str = "Nenhum repositório Git disponível.";
pub const GIT_COMMIT_PLACEHOLDER: &str = "Mensagem do commit";
pub const GIT_COMMIT_BUTTON: &str = "Commit";
pub const GIT_COMMITTING_BUTTON: &str = "Comitando…";
pub const GIT_HISTORY_EMPTY: &str = "Nenhum commit encontrado.";
pub const GIT_HISTORY_UNTITLED: &str = "Commit sem título";
pub const GIT_DIFF_TAB_PREFIX: &str = "Diff: ";
pub const GIT_DIFF_STAGED_BADGE: &str = "STAGED";
pub const GIT_DIFF_WORKTREE_BADGE: &str = "WORKING TREE";
pub const GIT_DIFF_BINARY: &str = "Arquivo binário — sem diff de texto.";
pub const GIT_RENAMED_FROM: &str = "Renomeado de: ";
pub const GIT_ELLIPSIS: &str = "…";
pub const GIT_DIFF_TRUNCATED: &str = "Diff limitado a 2000 linhas.";
pub const GIT_DIFF_UNCHANGED: &str = "Sem alterações.";
pub const GIT_DIFF_PREFIX_ADD: &str = "+";
pub const GIT_DIFF_PREFIX_DEL: &str = "-";
pub const GIT_DIFF_PREFIX_CTX: &str = " ";
pub const GIT_ERR_NOT_INSTALLED_PREFIX: &str = "Git não instalado: ";
pub const GIT_ERR_NOT_A_REPO: &str = "Esta pasta não é um repositório Git.";
pub const GIT_ERR_EMPTY_MESSAGE: &str = "Digite uma mensagem de commit.";
pub const GIT_ERR_EMPTY_PATHS: &str = "Nenhum arquivo selecionado para a operação Git.";
pub const GIT_ERR_CMD_PREFIX: &str = "Erro do Git: ";
pub const GIT_ERR_IO_PREFIX: &str = "Falha de I/O no Git: ";

// Ilha de agentes — seções do painel (na ordem exata de `agent_island_panel.dart`)
pub const SECTION_STATUS_ERROR: &str = "Erro no servidor de status";
pub const SECTION_ASKS: &str = "Pedem permissão";
pub const SECTION_WAITING: &str = "Esperando você";
pub const SECTION_FINISHED: &str = "Terminaram";
pub const SECTION_WORKING: &str = "Trabalhando";

// Ilha de agentes — erros tipados do servidor de status (`gsh_ipc::ServerError`)
pub const STATUS_ERR_CREATE_DIR_PREFIX: &str = "Não foi possível criar o diretório de status (";
pub const STATUS_ERR_STALE_SOCKET_PREFIX: &str = "Não foi possível remover o socket antigo (";
pub const STATUS_ERR_PATH_OCCUPIED_PREFIX: &str =
    "O caminho do socket está ocupado por outro arquivo: ";
pub const STATUS_ERR_BIND_PREFIX: &str = "Não foi possível escutar no socket de status (";
pub const STATUS_ERR_PERMISSIONS_PREFIX: &str =
    "Não foi possível ajustar as permissões do socket (";
pub const STATUS_ERR_TOKEN_PREFIX: &str = "Falha ao gerar token de autenticação do status: ";
pub const STATUS_ERR_THREAD_PREFIX: &str = "Falha ao iniciar a thread do servidor de status: ";
pub const STATUS_ERR_TARGET_SEP: &str = "): ";

// Ilha de agentes — nomes de harness e formatação de aba
pub const HARNESS_CLAUDE: &str = "claude";
pub const HARNESS_CODEX: &str = "codex";
pub const HARNESS_PI: &str = "pi";
pub const AGENT_TAB_SEPARATOR: &str = " — ";

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

/// Rótulo da aba do agente exibido na ilha (ex.: `"claude — Terminal"` ou apenas `"Terminal"`).
pub fn format_agent_tab_name(harness_label: Option<&str>, tab_title: &str) -> String {
    match harness_label.filter(|h| !h.is_empty()) {
        Some(h) if !tab_title.starts_with(h) => {
            format!("{h}{AGENT_TAB_SEPARATOR}{tab_title}")
        }
        _ => tab_title.to_string(),
    }
}

/// Aviso formatado quando o arquivo excede o limite do viewer (`max_bytes`).
pub fn format_viewer_too_large(size_bytes: u64, max_bytes: u64) -> String {
    const MIB: f64 = (1024 * 1024) as f64;
    let size_mib = (size_bytes as f64) / MIB;
    let max_mib = (max_bytes as f64) / MIB;
    format!(
        "{VIEWER_TOO_LARGE_PREFIX}{size_mib:.2}{VIEWER_TOO_LARGE_LIMIT_SEP}{max_mib:.0}{VIEWER_TOO_LARGE_SUFFIX}"
    )
}

/// Resolve o ícone tipográfico de uma aba de arquivo a partir do nome do arquivo
/// (`FileTypeIcon.file(name)` no Flutter).
pub fn file_icon_for_name(file_name: &str) -> &'static str {
    let lower = file_name.to_ascii_lowercase();
    let ext = lower.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    match ext {
        "rs" | "dart" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "c" | "cpp" | "h" | "sh"
        | "html" | "css" | "sql" => ICON_FILE_CODE,
        "toml" | "json" | "yaml" | "yml" | "ini" | "conf" | "env" => ICON_FILE_CONFIG,
        "md" | "mdx" | "txt" | "rst" => ICON_FILE_DOC,
        "lock" => ICON_FILE_LOCK,
        _ if lower.starts_with(".env") => ICON_FILE_CONFIG,
        _ => ICON_FILE_GENERIC,
    }
}

/// Subtítulo do estado vazio do chat (`tr.emptySubtitle(folder)` do Flutter).
pub fn format_chat_empty_subtitle(folder: &str) -> String {
    format!("{CHAT_EMPTY_SUBTITLE_PREFIX}{folder}{CHAT_EMPTY_SUBTITLE_SUFFIX}")
}

/// Aviso de falha ao iniciar o processo do `claude` (`tr.startFailed`).
pub fn format_chat_start_failed(detail: &str) -> String {
    format!("{CHAT_START_FAILED_PREFIX}{detail}")
}

/// Aviso de encerramento inesperado do `claude` (`tr.engineExited`).
pub fn format_chat_engine_exited(exit_code: Option<i32>, stderr: &str) -> String {
    let code = exit_code.unwrap_or(-1);
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        format!("{CHAT_ENGINE_EXITED_PREFIX}{code}{CHAT_ENGINE_EXITED_SUFFIX}")
    } else {
        format!("{CHAT_ENGINE_EXITED_PREFIX}{code}{CHAT_ENGINE_EXITED_SUFFIX}\n{trimmed}")
    }
}

/// Aviso de erro de turno retornado pelo `claude` (`tr.turnFailed`).
pub fn format_chat_turn_failed(detail: &str) -> String {
    format!("{CHAT_TURN_FAILED_PREFIX}{detail}")
}

/// Rótulo de custo acumulado do turno/conversa (`tr.cost`).
pub fn format_chat_cost(cost_usd: f64) -> String {
    format!("{CHAT_COST_PREFIX}{cost_usd:.2}")
}

/// Formata a linha de comando ou resumo exibida no bloco expandido da ferramenta.
pub fn format_chat_tool_detail(name: &str, summary: &str, command: Option<&str>) -> String {
    match command.filter(|c| !c.trim().is_empty()) {
        Some(cmd) => format!("{CHAT_CMD_PREFIX}{cmd}"),
        None if summary.is_empty() => name.to_string(),
        None => format!("{name} {summary}"),
    }
}

/// Cabeçalho da seção de alterações em stage (`ALTERAÇÕES EM STAGE (N)`).
pub fn format_staged_header(count: usize) -> String {
    format!("ALTERAÇÕES EM STAGE ({count})")
}

/// Cabeçalho da seção de alterações no working tree (`ALTERAÇÕES (N)`).
pub fn format_changes_header(count: usize) -> String {
    format!("ALTERAÇÕES ({count})")
}

/// Título de uma aba de diff no pane central (`Diff: <nome>`).
pub fn format_diff_tab_title(file_name: &str) -> String {
    format!("{GIT_DIFF_TAB_PREFIX}{file_name}")
}

/// Linha de metadados de um commit no histórico (`data · autor · hash`).
pub fn format_commit_meta(short_hash: &str, author: &str, date_label: &str) -> String {
    let mut parts = Vec::with_capacity(3);
    if !date_label.is_empty() {
        parts.push(date_label);
    }
    if !author.is_empty() {
        parts.push(author);
    }
    if !short_hash.is_empty() {
        parts.push(short_hash);
    }
    parts.join(SUMMARY_SEPARATOR)
}

/// Converte um [`gsh_git::GitError`] em mensagem amigável para exibição na UI.
pub fn format_git_error(err: &gsh_git::GitError) -> String {
    match err {
        gsh_git::GitError::GitNotInstalled { binary } => {
            format!("{GIT_ERR_NOT_INSTALLED_PREFIX}{binary}")
        }
        gsh_git::GitError::NotARepository { .. } => GIT_ERR_NOT_A_REPO.to_string(),
        gsh_git::GitError::EmptyCommitMessage => GIT_ERR_EMPTY_MESSAGE.to_string(),
        gsh_git::GitError::EmptyPaths => GIT_ERR_EMPTY_PATHS.to_string(),
        gsh_git::GitError::CommandFailed { stderr, .. } => {
            format!("{GIT_ERR_CMD_PREFIX}{stderr}")
        }
        gsh_git::GitError::Io { detail } => format!("{GIT_ERR_IO_PREFIX}{detail}"),
    }
}
