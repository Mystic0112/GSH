//! Constantes de rótulo da UI do GSH (português, espelhando `pt_BR.i18n.json`
//! do app Flutter).
//!
//! Nenhuma string literal de interface fica nos widgets: concentrar aqui
//! permite que o futuro módulo de i18n substitua os textos sem varrer a árvore
//! de componentes.

use gsh_harness::{Issue, Kind, Scope};

use crate::gallery_template::GalleryTemplate;

pub const APP_TITLE: &str = "GSH Desktop";

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
pub const CHAT_ALLOW: &str = "Permitir";
pub const CHAT_ALWAYS: &str = "Sempre permitir";
pub const CHAT_DENY: &str = "Negar";
pub const CHAT_ATTACH: &str = "Anexar arquivos";
pub const CHAT_MODE_PLAN: &str = "Planejar";
pub const CHAT_MODE_EDITS: &str = "Aceitar edições";
pub const CHAT_MODE_IGNORE: &str = "Ignorar permissões";
pub const CHAT_MODE_ASK: &str = "Pedir permissão";
pub const CHAT_RESUME: &str = "Retomar conversa";
pub const CHAT_RESUME_EMPTY: &str = "Retomar uma conversa";
pub const CHAT_REMOTE_CONTROL: &str = "Remote Control";
pub const CHAT_RECENT_SESSIONS: &str = "Conversas recentes";
pub const CHAT_RESUME_NONE: &str = "Nenhuma conversa recente.";
pub const CHAT_USAGE: &str = "Uso da conta";
pub const CHAT_USAGE_UNAVAILABLE: &str = "Uso da conta indisponível";
pub const CHAT_USAGE_SESSION: &str = "Sessão (5 horas)";
pub const CHAT_USAGE_WEEKLY: &str = "Semana";
pub const CHAT_USAGE_RESET_TODAY: &str = "reinicia hoje às ";
pub const CHAT_USAGE_RESET_ON: &str = "reinicia em ";
pub const CHAT_CONTEXT_UNAVAILABLE: &str = "Contexto indisponível";
pub const CHAT_PEER_PICKER: &str = "Enviar para sessão…";
pub const CHAT_HANDOFF: &str = "Continuar no terminal";
pub const CHAT_HANDOFF_NO_SESSION: &str = "Ainda não há uma sessão para continuar no terminal.";
pub const CHAT_HANDOFF_BUSY: &str = "Aguarde o turno terminar antes de continuar no terminal.";
pub const CHAT_HANDOFF_ACTIVE: &str = "Esta conversa já está no Remote Control: ";
pub const CHAT_HANDOFF_NOTICE: &str = "A conversa continuará no terminal.";
pub const CHAT_EDITED_FILES: &str = "arquivos editados";
pub const CHAT_ASK_LIST_SESSIONS: &str = "Liste as sessões para as quais posso mandar mensagem, com ListAgents, e espere eu escolher uma. ";
pub const CHAT_NO_PEER_SESSIONS: &str = "Nenhuma outra sessão do Claude Code rodando agora.";
pub const CHAT_SEND_TO_SESSION_PREFIX: &str = "Mande esta mensagem para a sessão \"";
pub const CHAT_SEND_TO_SESSION_SUFFIX: &str = "\" com SendMessage e espere a resposta: ";
pub const CHAT_LIST_ALL_SESSIONS: &str = "Listar todas as sessões (pergunta ao agente)…";
pub const CHAT_RENAME: &str = "Renomear conversa";
pub const CHAT_PLAN_APPROVE: &str = "Aprovar plano";
pub const CHAT_PLAN_REFINE: &str = "Refinar plano";
pub const CHAT_QUESTION_SEND: &str = "Enviar resposta";
pub const CHAT_RESUME_LIVE_TITLE: &str = "Esta sessão ainda pode estar ativa. Retomar mesmo assim?";
pub const CHAT_RESUME_CONFIRM: &str = "Retomar mesmo assim";
pub const CHAT_RESUME_CANCEL: &str = "Cancelar";
pub const CHAT_QUESTION_TITLE: &str = "O Claude precisa de algumas respostas";
pub const CHAT_QUESTION_FREE_TEXT: &str = "Outra resposta…";
pub const CHAT_PLAN_TITLE: &str = "Plano proposto";
pub const CHAT_PLAN_FEEDBACK: &str = "O que você gostaria de ajustar?";
pub const CHAT_CMD_PREFIX: &str = "$ ";
pub const CHAT_TOOL_NAME_PAD: &str = "  ";

// Viewer de arquivo (estados tipados de limite, binário e erro de leitura)
pub const VIEWER_TOO_LARGE_PREFIX: &str = "Arquivo acima do limite de edição (";
pub const EDITOR_CONFIRM_CLOSE: &str =
    "Alterações não salvas. Clique novamente em fechar para descartá-las.";
pub const EDITOR_CONFIRM_WINDOW: &str = "Há alterações não salvas. Descartar e fechar a janela?";
pub const EDITOR_DISCARD: &str = "Descartar e fechar";
pub const EDITOR_CANCEL: &str = "Cancelar";
pub const EDITOR_EXTERNAL_CHANGE: &str = "O arquivo mudou no disco. Confirme para sobrescrever.";
pub const EDITOR_OVERWRITE: &str = "Sobrescrever arquivo";
pub const EDITOR_SAVE_ERROR: &str = "Não foi possível salvar: ";
pub const EDITOR_INVALID_POSITION: &str = "Posição de edição inválida.";
pub const EDITOR_READ_ONLY: &str = "O arquivo é somente leitura.";
pub const EDITOR_CLEAN: &str = "";
pub const VIEWER_LARGE_PREVIEW: &str = "Somente leitura: exibindo uma prévia do início do arquivo.";
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
pub const FILES_NEW_FOLDER_HINT: &str = "Nome da pasta";
pub const FILES_NEW_FILE_HINT: &str = "Nome do arquivo";
pub const FILES_INVALID_NAME: &str = "Informe um nome válido, sem caminhos.";
pub const FILES_ALREADY_EXISTS: &str = "Já existe um item com este nome.";
pub const FILES_CREATE_PERMISSION: &str = "Permissão negada para criar o item.";
pub const FILES_CREATE_FAILED: &str = "Não foi possível criar o item.";
pub const FILES_NO_FOLDER: &str = "Nenhuma pasta — abra um workspace.";
pub const FILES_EMPTY_FOLDER: &str = "Pasta vazia.";
pub const FILES_ERR_NOT_FOUND: &str = "Pasta não encontrada.";
pub const FILES_ERR_PERMISSION: &str = "Permissão negada ao ler a pasta.";
pub const FILES_ERR_IO_PREFIX: &str = "Erro ao ler pasta: ";
pub const NO_LSP_AVAILABLE: &str = "Nenhum LSP disponível";

// Ícones tipográficos dos controles de chrome
pub const ICON_SIDEBAR_LEFT: &str = "\u{f114}";
pub const ICON_SIDEBAR_RIGHT: &str = "\u{f114}";
pub const ICON_LAYERS: &str = "\u{e9f4}";
pub const ICON_BACK: &str = "\u{e5c4}";
pub const ICON_NEW_FOLDER: &str = "\u{e2cc}";
pub const ICON_NEW_FILE: &str = "\u{e89c}";
pub const ICON_DOT: &str = "\u{ef4a}";
pub const ICON_MORE_VERT: &str = "\u{e5d4}";
pub const ICON_PLUS: &str = "\u{e145}";
pub const ICON_CLOSE: &str = "\u{e5cd}";
pub const ICON_TERMINAL: &str = "\u{eb8e}";
pub const ICON_SPLIT_H: &str = "\u{e949}";
pub const ICON_SPLIT_V: &str = "\u{e947}";
pub const ICON_FOLDER: &str = "\u{e2c7}";
pub const ICON_REFRESH: &str = "\u{e5d5}";
pub const ICON_COLLAPSE: &str = "\u{e5d6}";
pub const ICON_SETTINGS: &str = "\u{e8b8}";
pub const ICON_SEARCH: char = '\u{ef7a}';
pub const ICON_APPEARANCE: &str = "\u{e3ab}";
pub const ICON_NOTIFICATIONS: &str = "\u{e7f5}";
pub const ICON_CHEVRON_DOWN: &str = "\u{e313}";
pub const ICON_CHEVRON_RIGHT: &str = "\u{e5cc}";
pub const ICON_FILE_ITEM: &str = "\u{e873}";
pub const ICON_FILE_CODE: &str = "‹›";
pub const ICON_FILE_CONFIG: &str = "⚙";
pub const ICON_FILE_DOC: &str = "¶";
pub const ICON_FILE_LOCK: &str = "⚲";
pub const ICON_FILE_GENERIC: &str = "≡";
pub const ICON_WINDOW_MAXIMIZE: &str = "\u{e3c6}";
pub const ICON_WINDOW_MINIMIZE: &str = "\u{e15b}";
pub const ICON_FARM: &str = "\u{f205}";
pub const ICON_CHAT: &str = "\u{e0cb}";
pub const ICON_SEND: &str = "\u{e163}";
pub const ICON_STOP: &str = "\u{e047}";
pub const ICON_TOOL_RUNNING: &str = "◌";
pub const ICON_GIT_BRANCH: &str = "\u{e0b6}";
pub const ICON_ACCOUNT_TREE: &str = "\u{e97a}";
pub const ICON_DIFF: &str = "\u{eb7d}";
pub const ICON_MINUS: &str = "\u{e15b}";
pub const ICON_SPEED: &str = "\u{e9e4}";
pub const ICON_HISTORY: &str = "\u{e8b3}";
pub const ICON_FORUM: &str = "\u{e8af}";
pub const ICON_PHONE_IPHONE: &str = "\u{f2da}";
pub const ICON_LAPTOP: &str = "\u{e31e}";
pub const ICON_KEYBOARD_RETURN: &str = "\u{e31b}";
pub const ICON_IMAGE: &str = "\u{e3f4}";
pub const ICON_EXPAND_MORE: &str = "\u{e5cf}";

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
pub const SUMMARY_IDLE: &str = "parados";
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

/// Tooltip dos contadores da topbar (`FarmStrip` do Flutter).
pub fn format_farm_summary(working: usize, waiting: usize, idle: usize) -> String {
    format!(
        "{working} {SUMMARY_WORKING}{SUMMARY_SEPARATOR}{}{SUMMARY_SEPARATOR}{idle} {SUMMARY_IDLE}",
        farm_need_you(waiting)
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
pub fn format_viewer_large_preview(size_bytes: u64, max_bytes: u64) -> String {
    format!(
        "{} {VIEWER_LARGE_PREVIEW}",
        format_viewer_too_large(size_bytes, max_bytes)
    )
}

pub fn format_editor_error(error: &gsh_editor::EditorError) -> String {
    match error {
        gsh_editor::EditorError::InvalidPosition => EDITOR_INVALID_POSITION.to_owned(),
        gsh_editor::EditorError::ReadOnly => EDITOR_READ_ONLY.to_owned(),
        gsh_editor::EditorError::ModifiedOnDisk => EDITOR_EXTERNAL_CHANGE.to_owned(),
        gsh_editor::EditorError::Save(detail) => format!("{EDITOR_SAVE_ERROR}{detail}"),
    }
}

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
pub fn format_chat_clipboard_failed(detail: &str) -> String {
    format!("Não foi possível ler a imagem da área de transferência: {detail}")
}
pub fn format_chat_title_failed(detail: &str) -> String {
    format!("Não foi possível renomear a conversa: {detail}")
}
pub fn format_chat_account_failed(detail: &str) -> String {
    format!("Não foi possível consultar a conta Claude: {detail}")
}
pub fn format_chat_context(tokens: u64, window: u64) -> String {
    fn compact(n: u64) -> String {
        if n >= 1_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else if n >= 1_000 {
            format!("{:.1}k", n as f64 / 1_000.0)
        } else {
            n.to_string()
        }
    }
    format!("Contexto {}/{} tokens", compact(tokens), compact(window))
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

pub const FARM_TAB_TITLE: &str = "Cenário";
pub const FARM_OFFICE: &str = "Escritório";
pub const FARM_FARM: &str = "Fazenda";

pub const FARM_EMPTY_TITLE: &str = "Nenhum agente rodando";
pub const FARM_EMPTY_HINT: &str = "Todo loop do Claude Code nesta máquina aparece aqui enquanto trabalha, inclusive subagentes e sessões fora do GSH Desktop.";

pub fn farm_spent_hud(tokens: &str) -> String {
    format!("{tokens} tokens gastos")
}
pub fn farm_agents(count: usize) -> String {
    format!("{count} agentes")
}
pub fn farm_subagents(count: usize) -> String {
    format!("{count} subagentes")
}
pub fn farm_working(count: usize) -> String {
    format!("{count} trabalhando")
}
pub fn farm_idle(count: usize) -> String {
    format!("{count} parados")
}
pub const FARM_ALERT: &str = "!";
pub const PANE_SPLIT_RIGHT: &str = "Dividir à direita";
pub const PANE_SPLIT_DOWN: &str = "Dividir abaixo";
pub const PANE_CLOSE: &str = "Fechar painel";
pub const PANE_CLOSE_TITLE: &str = "Fechar painel?";
pub const PANE_CLOSE_CONFIRM: &str = "Fechar";
pub const DIALOG_CANCEL: &str = "Cancelar";

/// Aviso do diálogo de fechar painel (`closePaneMessage` do Flutter).
pub fn pane_close_message(count: usize) -> String {
    format!("Isso fecha todas as {count} aba(s) deste painel e encerra os agentes/terminais nele.")
}
pub const FARM_NEED_YOU_ONE: &str = "1 precisa de você";
pub const FARM_NEED_YOU_MANY: &str = "precisam de você";

pub fn farm_need_you(count: usize) -> String {
    if count == 1 {
        FARM_NEED_YOU_ONE.to_string()
    } else {
        format!("{count} {FARM_NEED_YOU_MANY}")
    }
}
// Configurações
pub const SETTINGS_TITLE: &str = "Configurações";
pub const SETTINGS_GSH_DESKTOP: &str = "GSH Desktop";
pub const SETTINGS_GENERAL: &str = "Geral";
pub const SETTINGS_APPEARANCE: &str = "Aparência";
pub const SETTINGS_TERMINAL: &str = "Terminal";
pub const SETTINGS_EDITOR: &str = "Editor";
pub const SETTINGS_NOTIFICATIONS: &str = "Notificações";
pub const SETTINGS_COMING_SOON: &str = "Em breve";
pub const SETTINGS_LOCALE: &str = "Idioma";
pub const SETTINGS_LOCALE_HINT: &str = "Ex.: pt-BR ou en; vazio acompanha o sistema";
pub const SETTINGS_SAVE: &str = "Salvar";
pub const SETTINGS_INVALID_LOCALE: &str = "Idioma inválido. Use letras e hífens, como pt-BR.";
pub const SETTINGS_CONFIG_ERROR: &str = "Não foi possível ler ou salvar as configurações.";
pub const SETTINGS_SAVED: &str = "Configuração salva.";

pub const SETTINGS_INTERFACE_SIZE: &str = "Tamanho da interface";
pub const SETTINGS_INTERFACE_SIZE_HINT: &str = "De 10 a 20";
pub const SETTINGS_INVALID_INTERFACE_SIZE: &str = "Informe um tamanho entre 10 e 20.";

pub const SETTINGS_TRANSCRIPT_WIDTH: &str = "Largura da transcrição";
pub const SETTINGS_WIDTH_NARROW: &str = "Estreito";
pub const SETTINGS_WIDTH_MEDIUM: &str = "Médio";
pub const SETTINGS_WIDTH_WIDE: &str = "Largo";
pub const SETTINGS_NOTIFY_TURN_DONE: &str = "Conclusão de respostas";
pub const SETTINGS_NOTIFY_PERMISSION: &str = "Solicitações de permissão do Code";
pub const SETTINGS_THEME: &str = "Tema";
pub const SETTINGS_DARK: &str = "Escuro";
pub const SETTINGS_LIGHT: &str = "Claro";
pub const SETTINGS_INVALID_THEME: &str = "Tema inválido. Escolha escuro ou claro.";
pub const SETTINGS_SHELL: &str = "Shell padrão";
pub const SETTINGS_SCROLLBACK: &str = "Histórico do terminal (1000 a 100000 linhas)";
pub const SETTINGS_TERMINAL_SIZE: &str = "Fonte do terminal (8 a 32)";
pub const SETTINGS_TAB_SIZE: &str = "Tabulação (2 ou 4)";
pub const SETTINGS_HIGHLIGHT: &str = "Realce de sintaxe";
pub const SETTINGS_WRAP: &str = "Quebra de linha";
pub const SETTINGS_AGENT_NOTIFICATIONS: &str = "Notificações de agente";
pub const SETTINGS_NOTIFICATIONS_PENDING: &str =
    "A entrega de notificações ainda não está disponível neste aplicativo.";
pub const SETTINGS_INVALID_SHELL: &str = "Informe um shell não vazio.";
pub const SETTINGS_INVALID_SCROLLBACK: &str = "Informe um histórico entre 1000 e 100000.";
pub const SETTINGS_INVALID_TERMINAL_SIZE: &str = "Informe um tamanho de fonte entre 8 e 32.";
pub const SETTINGS_INVALID_TAB_SIZE: &str = "Informe 2 ou 4 para a tabulação.";
pub const SETTINGS_TERMINAL_RESTART: &str = "As alterações valem para novos terminais.";

// Banco de dados (pane de consulta PostgreSQL)
pub const DB_TAB_TITLE: &str = "Banco";
pub const ICON_DATABASE: &str = "\u{f20e}";
pub const DB_CONN_PLACEHOLDER: &str = "postgres://usuario@host:5432/banco";
pub const DB_SQL_PLACEHOLDER: &str = "select * from tabela limit 100;";
pub const DB_RUN: &str = "Executar";
pub const DB_IDLE: &str = "Informe a conexão e escreva uma consulta.";
pub const DB_RUNNING: &str = "Executando…";
pub const DB_NO_ROWS: &str = "Nenhuma linha.";
pub const DB_ROWS_AFFECTED: &str = "Linhas afetadas:";
pub const DB_NULL: &str = "∅";
pub const DB_TRUNCATED: &str = "Resultado limitado às primeiras linhas.";
pub const DB_ERR_INVALID: &str = "Conexão inválida. Use postgres://usuario@host/banco.";
pub const DB_ERR_CONNECT: &str = "Não foi possível conectar ao banco.";
pub const DB_ERR_QUERY: &str = "Erro na consulta";
pub const DB_ERR_TIMEOUT: &str = "A consulta demorou demais e foi cancelada.";
pub const DB_ERR_RUNTIME: &str = "Falha interna ao executar a consulta.";

pub const SETTINGS_NAV_SEARCH: &str = "Buscar configurações";
pub const SETTINGS_NAV_NO_RESULTS: &str = "Nenhum resultado";
pub const SETTINGS_NAV_CLAUDE: &str = "Claude";
pub const SETTINGS_CLAUDE_UNAVAILABLE: &str =
    "As configurações do Claude ainda não estão disponíveis neste app.";

pub const SETTINGS_LOCALE_SYSTEM: &str = "Sistema";
pub const SETTINGS_LOCALE_EN: &str = "English";
pub const SETTINGS_LOCALE_ES: &str = "Español";
pub const SETTINGS_LOCALE_PT: &str = "Português (Brasil)";
pub const SETTINGS_LOCALES: [&str; 4] = [
    SETTINGS_LOCALE_SYSTEM,
    SETTINGS_LOCALE_EN,
    SETTINGS_LOCALE_ES,
    SETTINGS_LOCALE_PT,
];

// Abas do painel direito e Galeria (`pt_BR.i18n.json` -> `gsh.fileTreePanel`, `gsh.gallery`, `fileOperation.error`)
pub const FILES_TOOLTIP: &str = "Arquivos";
pub const SOURCE_CONTROL_TOOLTIP: &str = "Controle de versão";
pub const GALLERY_TOOLTIP: &str = "Galeria";
pub const SEARCH_TOOLTIP: &str = "Buscar";
pub const ICON_SEARCH_TAB: &str = "\u{ef7a}";

// Busca por conteúdo (`pt_BR.i18n.json` -> `gsh.contentSearch`)
pub const CONTENT_SEARCH_SECTION: &str = "BUSCA";
pub const CONTENT_SEARCH_PLACEHOLDER: &str = "Buscar nos arquivos";
pub const CONTENT_SEARCH_MATCH_CASE: &str = "Diferenciar maiúsculas";
pub const CONTENT_SEARCH_WHOLE_WORD: &str = "Palavra inteira";
pub const CONTENT_SEARCH_USE_REGEX: &str = "Usar expressão regular";
pub const CONTENT_SEARCH_INVALID_REGEX: &str = "Expressão regular inválida.";
pub const CONTENT_SEARCH_TYPE_TO_SEARCH: &str = "Digite para buscar em todos os arquivos.";
pub const CONTENT_SEARCH_SEARCHING: &str = "Buscando…";
pub const CONTENT_SEARCH_NO_RESULTS: &str = "Nenhum resultado.";
pub const CONTENT_SEARCH_ELLIPSIS: &str = "…";
// Rótulos dos toggles (fixos no Flutter, não traduzidos)
pub const CONTENT_SEARCH_TOGGLE_CASE: &str = "Aa";
pub const CONTENT_SEARCH_TOGGLE_WORD: &str = "ab";
pub const CONTENT_SEARCH_TOGGLE_REGEX: &str = ".*";

/// Pílula do cabeçalho: texto fixo em inglês no Flutter (`'$total in $files'`).
pub fn content_search_summary(total: usize, files: usize) -> String {
    format!("{total} in {files}")
}
pub const GALLERY_SECTION_UPPER: &str = "GALERIA";
pub const ICON_GALLERY: &str = "\u{e660}";
pub const GALLERY_INTRO: &str =
    "Documentos especiais do GSH Desktop para que você tenha o visual do que o agente de IA esteja fazendo.";
pub const GALLERY_CREATE_ERROR_TITLE: &str = "Não foi possível criar o arquivo";
pub const GALLERY_ERR_NO_WORKSPACE: &str = "Nenhum workspace selecionado.";
pub const GALLERY_ERR_WRITE_FAILED: &str = "Não foi possível gravar o arquivo.";
pub const DIALOG_OK: &str = "OK";

pub fn gallery_title(template: GalleryTemplate) -> &'static str {
    match template {
        GalleryTemplate::DbQuery => "Consulta de banco",
        GalleryTemplate::Kanban => "Quadro kanban",
        GalleryTemplate::Notebook => "Caderno",
        GalleryTemplate::Html => "Visual HTML",
        GalleryTemplate::Panel => "Painel",
        GalleryTemplate::HttpRequest => "Requests HTTP",
        GalleryTemplate::Tasks => "Tarefas",
        GalleryTemplate::Layout => "Layout de panes",
        GalleryTemplate::WorkspaceEnv => "Env do workspace",
    }
}

pub fn gallery_description(template: GalleryTemplate) -> &'static str {
    match template {
        GalleryTemplate::DbQuery => "Editor SQL com grid de resultado, usando uma das conexões registradas.",
        GalleryTemplate::Kanban => "Colunas e cards gravados como markdown puro. Arraste, edite e comente.",
        GalleryTemplate::Notebook => "Uma pasta de notas curtas com tags. O agente escreve, você lê e edita. Abre no Obsidian também.",
        GalleryTemplate::Html => "Uma página que o agente escreve e o GSH Desktop renderiza direto: mapa mental, diagrama, gráfico, o que precisar.",
        GalleryTemplate::Panel => "Uma página HTML com ponte para o app: os botões rodam comandos da CLI do GSH Desktop e do shell nesta máquina. Um playground para dashboards rápidos.",
        GalleryTemplate::HttpRequest => "Escreva requests num arquivo e execute com a resposta ao lado.",
        GalleryTemplate::Tasks => "Comandos para rodar pelo painel de Tasks, como dev server, testes e build. Fica em .gsh/tasks.json.",
        GalleryTemplate::Layout => "Terminais e splits para abrir de uma vez, estilo tmuxinator. Pode rodar sozinho em worktrees novas.",
        GalleryTemplate::WorkspaceEnv => "Variáveis injetadas em todo terminal deste workspace. Coloque tokens ou logins de API aqui em vez de colar no prompt do agente. Fica fora do git.",
    }
}

// Aba Harness (`pt_BR.i18n.json` -> `gsh.harness`, `gsh.topbar.openHarness`, `gsh.realmDialogs.duplicateName`)
pub const TOPBAR_OPEN_HARNESS: &str = "Harness do workspace";
pub const HARNESS_TAB_TITLE: &str = "Harness";
pub const HARNESS_REFRESH: &str = "Ler de novo";
pub const HARNESS_EDIT_FILE: &str = "Editar arquivo";
pub const HARNESS_NEW_ITEM: &str = "Novo";
pub const HARNESS_MEMORY_NEW: &str = "Criar CLAUDE.md";
pub const HARNESS_CREATE: &str = "Criar";
pub const HARNESS_NAME_PLACEHOLDER: &str = "nome (vira o nome do arquivo)";
pub const HARNESS_EMPTY: &str = "Nada aqui ainda.";
pub const HARNESS_READ_ONLY: &str = "Só leitura aqui: use claude mcp add/remove";
pub const HARNESS_COST_TITLE: &str = "Custo fixo por turno";
pub const HARNESS_COST_HINT: &str = "O que vai para o contexto em toda mensagem, antes de qualquer trabalho: as instruções inteiras e o nome + descrição de cada skill, subagente e comando. Ferramentas MCP não entram na conta.";
pub const HARNESS_COST_TIP: &str = "Para enxugar: o que só vale às vezes sai do CLAUDE.md e vira skill; skills e comandos que ninguém usa, apague.";
pub const HARNESS_SHOW_LESS: &str = "Mostrar menos";
pub const HARNESS_DUPLICATE_NAME: &str = "Já existe um realm com esse nome.";
pub const HARNESS_ELLIPSIS: &str = "…";
pub const ICON_TUNE: &str = "\u{e429}";
pub const ICON_WARNING_AMBER: &str = "\u{f083}";
pub const ICON_DESCRIPTION: &str = "\u{e873}";
pub const ICON_AUTO_AWESOME: &str = "\u{e65f}";
pub const ICON_SMART_TOY: &str = "\u{f06c}";
pub const ICON_BOLT: &str = "\u{ea0b}";
pub const ICON_SHIELD: &str = "\u{e9e0}";
pub const ICON_EXTENSION: &str = "\u{e87b}";
pub const ICON_EDIT: &str = "\u{f097}";
pub const ICON_ERROR_OUTLINE: &str = "\u{f8b6}";
pub const ICON_LOCK_OUTLINE: &str = "\u{e899}";

pub fn harness_subtitle(path: &str) -> String {
    format!("O que o agente carrega ao rodar em {path}")
}

pub fn harness_cost_value(tokens: &str) -> String {
    format!("≈ {tokens} tokens")
}

pub fn harness_per_turn(tokens: &str) -> String {
    format!("≈ {tokens}/turno")
}

pub fn harness_new_title(section: &str) -> String {
    format!("Novo item em {section}")
}

pub fn harness_invalid_json(file: &str) -> String {
    format!("{file}: JSON inválido — o Claude Code ignora este arquivo inteiro.")
}

pub fn harness_issues(n: usize) -> String {
    format!("Avisos: {n}")
}

pub fn harness_show_all(n: usize) -> String {
    format!("Mostrar todos ({n})")
}

pub fn harness_title(kind: Kind) -> &'static str {
    match kind {
        Kind::Memory => "Instruções",
        Kind::Skills => "Skills",
        Kind::Agents => "Subagentes",
        Kind::Commands => "Comandos",
        Kind::Hooks => "Hooks",
        Kind::Permissions => "Permissões",
        Kind::Mcp => "MCP",
    }
}

pub fn harness_description(kind: Kind) -> &'static str {
    match kind {
        Kind::Memory => "CLAUDE.md lidos numa sessão aqui (da pasta para cima), o seu do usuário e o AGENTS.md (Codex).",
        Kind::Skills => "Capacidades que o agente aciona sozinho quando a descrição bate com a tarefa.",
        Kind::Agents => "Especialistas para quem o agente delega (com ferramentas e modelo próprios).",
        Kind::Commands => "Atalhos /nome que você digita na sessão.",
        Kind::Hooks => "Comandos que rodam em eventos da sessão (antes de uma ferramenta, ao parar...).",
        Kind::Permissions => "Regras de ferramenta: liberar, perguntar ou bloquear.",
        Kind::Mcp => "Servidores de ferramentas extras. Os do usuário e os locais vêm do ~/.claude.json (mude com claude mcp).",
    }
}

pub fn harness_issue(issue: Issue) -> &'static str {
    match issue {
        Issue::NoDescription => "Sem descrição: o agente não sabe quando usar.",
        Issue::MissingScript => "O script que este hook chama não existe.",
        Issue::TooLarge => {
            "Grande demais: pesa em toda mensagem (o Claude Code avisa acima de 40 mil caracteres)."
        }
    }
}

pub fn harness_scope(scope: Scope) -> &'static str {
    match scope {
        Scope::User => "Usuário",
        Scope::Project => "Projeto",
        Scope::Local => "Local",
    }
}

/// Regra de permissão (`allow`/`ask`/`deny`) traduzida; outro valor fica como veio.
pub fn harness_rule(rule: &str) -> &str {
    match rule {
        "allow" => "liberar",
        "ask" => "perguntar",
        "deny" => "bloquear",
        other => other,
    }
}

// Filtro por agente do painel Arquivos (`pt_BR.i18n.json` -> `gsh.fileTreePanel`)
pub const AGENT_FILTER_TOOLTIP: &str = "Filtrar por agente";
pub const ALL_AGENTS: &str = "Todos os agentes";
pub const TRACKING_UNAVAILABLE: &str = "rastreio indisponível";
pub const INFERRED_SUFFIX: &str = "(inferido)";
pub const NO_TOUCHED_FILES: &str = "Nenhum arquivo tocado por este agente ainda";
pub const CONCURRENT_AGENT_EDITS: &str = "Editado por 2+ agentes nos últimos 10 minutos";
pub const ICON_GROUPS: &str = "\u{f233}";
pub const ICON_CHECK: &str = "\u{e668}";

pub fn format_edited_by(label: &str, time: &str) -> String {
    format!("Editado por {label} · {time}")
}
