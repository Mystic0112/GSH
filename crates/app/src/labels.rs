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
pub const WORKSPACE_SETTINGS_TITLE: &str = "Configurações do workspace";
pub const WORKSPACE_NAME: &str = "Nome";
pub const WORKSPACE_ADD_PHOTO: &str = "Adicionar foto";
pub const WORKSPACE_CHANGE_PHOTO: &str = "Trocar foto";
pub const WORKSPACE_REMOVE_PHOTO: &str = "Remover";
pub const WORKSPACE_COLOR: &str = "Cor";
pub const WORKSPACE_FOLDER: &str = "Pasta";
pub const WORKSPACE_SAVE: &str = "Salvar";
pub const WORKSPACE_CANCEL: &str = "Cancelar";
pub const WORKSPACE_CLOSE_TITLE: &str = "Fechar workspace?";
pub const WORKSPACE_CLOSE_MESSAGE: &str =
    "O workspace será removido da lista do GSH. A pasta e os arquivos no disco não serão apagados.";
pub const WORKSPACE_CLOSE_CONFIRM: &str = "Fechar";
pub const NEW_WORKSPACE_LOCAL: &str = "Local";
pub const NEW_WORKSPACE_REMOTE: &str = "Remoto";
pub const WORKSPACE_MENU_CONFIGURE: &str = "Configurações";
pub const WORKSPACE_MENU_COPY_ID: &str = "Copiar ID do workspace";
pub const WORKSPACE_MENU_MOVE_TO: &str = "Mover para";
pub const WORKSPACE_GSH: &str = "GSH Desktop";
pub const RAIL_NO_WORKSPACES: &str = "Nenhum workspace ainda.";
pub const DEFAULT_REALM: &str = "Padrão";
pub const RAIL_KEEP_AWAKE_ON: &str =
    "Mantendo este computador acordado para acesso remoto. Clique para deixá-lo dormir de novo.";
pub const RAIL_KEEP_AWAKE_OFF: &str = "Manter este computador acordado para acesso remoto. Desliga sozinho quando o GSH Desktop reinicia.";
pub const RAIL_KEEP_AWAKE_BATTERY: &str = "Acordado na bateria. Isso consome a bateria.";
pub const RAIL_KEEP_AWAKE_LID: &str =
    "Fechar a tampa do notebook continua colocando-o para dormir.";
pub const RAIL_SETTINGS_TOOLTIP: &str = "Configurações";
pub const UPDATE_CARD_TITLE: &str = "Update available";
pub const UPDATE_CARD_CLICK_TO_DOWNLOAD: &str = "click to download";
pub const UPDATE_CARD_DISMISS_TOOLTIP: &str = "Dispensar";
pub const EDITOR_APPLY_CKP_LAYOUT: &str = "Aplicar layout";
pub const EDITOR_FIND_PLACEHOLDER: &str = "Buscar";
pub const EDITOR_FIND_CASE: &str = "Diferenciar maiúsculas";
pub const EDITOR_FIND_WORD: &str = "Palavra inteira";
pub const EDITOR_FIND_REGEX: &str = "Usar expressão regular";
pub const EDITOR_FIND_PREVIOUS: &str = "Anterior (⇧⏎)";
pub const EDITOR_FIND_NEXT: &str = "Próximo (⏎)";
pub const EDITOR_FIND_CLOSE: &str = "Fechar (Esc)";
pub const EDITOR_FIND_BAD_PATTERN: &str = "Padrão inválido";
pub const EDITOR_FIND_NO_RESULTS: &str = "Nenhum resultado";

pub const CHANNELS_HEADER: &str = "Canais";
pub const CHANNELS_ALL: &str = "Todos os canais";
pub const CHANNEL_NEW: &str = "Novo canal";
pub const CHANNEL_EDIT: &str = "Editar canal";
pub const CHANNEL_EDIT_ARCHIVED: &str = "Editar canal arquivado";
pub const CHANNEL_MOVE_WORKSPACE_SECTION: &str = "Mover workspace ativo para";
pub const CHANNEL_NAME: &str = "Nome";
pub const CHANNEL_NAME_PLACEHOLDER: &str = "Nome do canal";
pub const CHANNEL_INSTRUCTIONS: &str = "Instruções";
pub const CHANNEL_INSTRUCTIONS_PLACEHOLDER: &str = "Contexto para as conversas do canal";
pub const CHANNEL_SAVE: &str = "Salvar";
pub const CHANNEL_CANCEL: &str = "Cancelar";
pub const CHANNEL_ARCHIVE: &str = "Arquivar";
pub const CHANNEL_UNARCHIVE: &str = "Desarquivar";
pub const CHANNEL_FOLDER: &str = "Pasta";
pub const CHANNEL_NO_FOLDER: &str = "Sem pasta própria";
pub const CHANNEL_PICK_FOLDER: &str = "Escolher pasta…";
pub const CHANNEL_REMOVE_FOLDER: &str = "Remover";
pub const CHANNEL_MEMBERS: &str = "Membros";
pub const CHANNEL_NO_SESSIONS: &str = "Nenhuma sessão disponível";
pub const CHANNEL_REPLY_RULE: &str = "Regra de resposta";
pub const CHANNEL_REPLY_MANUAL: &str = "Manual";
pub const CHANNEL_REPLY_MENTION: &str = "Só ao mencionar";
pub const CHANNEL_REPLY_ALL: &str = "Todos";

pub fn channel_member_label(mention: &str, session: &str) -> String {
    format!("@{mention} · {session}")
}

pub fn channel_unavailable_member_label(mention: &str) -> String {
    format!("@{mention} · Sessão indisponível")
}

pub const DEFAULT_TAB_TITLE: &str = "Terminal";
pub const CHAT_TAB_TITLE: &str = "Chat";
pub const PANES_PLACEHOLDER: &str = "Terminal (spike de PTY ainda não ligado)";
pub const PANE_VIEW_DROP_HERE_TO_MOVE: &str = "Solte aqui para mover a aba";
pub const PANE_VIEW_ALL_TABS: &str = "Todas as abas";
pub const PANE_VIEW_NEW_TAB: &str = "Nova aba";
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
pub const CHAT_ALLOWED: &str = "Permitido";
pub const CHAT_DENIED: &str = "Negado";
pub const CHAT_ALLOWED_ALWAYS_PREFIX: &str = "Sempre permitido: ";
pub const CHAT_DENY: &str = "Negar";
pub const CHAT_ATTACH: &str = "Anexar arquivos";
pub const CHAT_MODE_PLAN: &str = "Planejar";
pub const CHAT_MODE_EDITS: &str = "Aceitar edições";
pub const CHAT_MODE_IGNORE: &str = "Ignorar permissões";
pub const CHAT_MODE_ASK: &str = "Pedir permissão";
pub const CHAT_EFFORT_TOOLTIP: &str = "Esforço: ";
pub const CHAT_EFFORT_AUTO: &str = "Padrão";
pub const CHAT_EFFORT_LOW: &str = "Baixo";
pub const CHAT_EFFORT_MEDIUM: &str = "Médio";
pub const CHAT_EFFORT_HIGH: &str = "Alto";
pub const CHAT_EFFORT_XHIGH: &str = "Muito alto";
pub const CHAT_EFFORT_MAX: &str = "Máximo";
pub const CHAT_EFFORT_HINT: &str = "Quanto o modelo pensa antes de responder. Mais esforço = melhor em tarefa difícil, mais lento e mais tokens.";
pub const CHAT_MODEL_EFFORT: &str = "Esforço";
pub const CHAT_MODEL_MORE: &str = "Mais modelos";
pub const CHAT_MODEL_REQUIRES_CREDITS: &str = "Requer créditos de uso";
pub const CHAT_WAITING_PERMISSION: &str = "esperando você";
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
pub const CHAT_THINKING: &str = "Raciocínio";
pub const CHAT_THINKING_LIVE: &str = "Pensando…";
pub const CHAT_COPY: &str = "Copiar";
pub const CHAT_COPIED: &str = "Copiado";
pub const CHAT_STATE_COMPLETED: &str = "Concluída";
pub const CHAT_STATE_REVIEW_READY: &str = "Pronta para revisão";
pub const CHAT_STATE_BLOCKED: &str = "Bloqueada";
pub const CHAT_STATE_FAILED: &str = "Falhou";
pub fn chat_edited_files(count: usize) -> String {
    if count == 1 {
        "Editou 1 arquivo".to_owned()
    } else {
        format!("Editou {count} arquivos")
    }
}
pub fn chat_session_state(state: gsh_chat::SessionState) -> &'static str {
    match state {
        gsh_chat::SessionState::Completed => CHAT_STATE_COMPLETED,
        gsh_chat::SessionState::ReviewReady => CHAT_STATE_REVIEW_READY,
        gsh_chat::SessionState::Blocked => CHAT_STATE_BLOCKED,
        gsh_chat::SessionState::Failed => CHAT_STATE_FAILED,
    }
}
pub fn chat_relative_time(at_ms: u64, now_ms: u64) -> String {
    let elapsed_minutes = now_ms.saturating_sub(at_ms) / 60_000;
    match elapsed_minutes {
        0 => TELEMETRY_JUST_NOW.to_owned(),
        1..=59 => telemetry_minutes_ago(elapsed_minutes as i64),
        _ if elapsed_minutes < 1_440 => telemetry_hours_ago((elapsed_minutes / 60) as i64),
        _ => telemetry_days_ago((elapsed_minutes / 1_440) as i64),
    }
}
pub const CHAT_ASK_LIST_SESSIONS: &str = "Liste as sessões para as quais posso mandar mensagem, com ListAgents, e espere eu escolher uma. ";
pub const CHAT_NO_PEER_SESSIONS: &str = "Nenhuma outra sessão do Claude Code rodando agora.";
pub const CHAT_SEND_TO_SESSION_PREFIX: &str = "Mande esta mensagem para a sessão \"";
pub const CHAT_SEND_TO_SESSION_SUFFIX: &str = "\" com SendMessage e espere a resposta: ";
pub const CHAT_LIST_ALL_SESSIONS: &str = "Listar todas as sessões (pergunta ao agente)…";
pub const CHAT_RENAME: &str = "Renomear conversa";
pub const CHAT_PLAN_APPROVE: &str = "Aprovar e executar";
pub const CHAT_PLAN_REFINE: &str = "Continuar planejando";
pub const CHAT_PLAN_SEND_REFINE: &str = "Enviar ajuste";
pub const CHAT_PLAN_APPROVED: &str = "Plano aprovado";
pub const CHAT_PLAN_REFINED: &str = "Pediu para continuar planejando";
pub const CHAT_PLAN_REFINED_PREFIX: &str = "Ajuste: ";
pub const CHAT_QUESTION_SEND: &str = "Responder";
pub const CHAT_RESUME_LIVE_TITLE: &str = "Esta sessão ainda pode estar ativa. Retomar mesmo assim?";
pub const CHAT_RESUME_CONFIRM: &str = "Retomar mesmo assim";
pub const CHAT_RESUME_CANCEL: &str = "Cancelar";
pub const CHAT_QUESTION_TITLE: &str = "O agente está perguntando";
pub const CHAT_QUESTION_FREE_TEXT: &str = "Outro…";
pub const CHAT_PLAN_TITLE: &str = "Plano do agente";
pub const CHAT_PLAN_FEEDBACK: &str = "O que mudar no plano?";
pub const CHAT_PROMPT_ASK_PREFIX: &str = "Permitir ";
pub const CHAT_PROMPT_ASK_SUFFIX: &str = "?";

pub fn chat_allow_always(rule: &str) -> String {
    format!("{CHAT_ALWAYS} {rule}")
}

pub fn chat_allowed_always(rule: &str) -> String {
    format!("{CHAT_ALLOWED_ALWAYS_PREFIX}{rule}")
}

pub fn chat_plan_refined(feedback: &str) -> String {
    if feedback.is_empty() {
        CHAT_PLAN_REFINED.to_owned()
    } else {
        format!("{CHAT_PLAN_REFINED_PREFIX}{feedback}")
    }
}
pub const CHAT_CMD_PREFIX: &str = "$ ";
pub const CHAT_TOOL_NAME_PAD: &str = "  ";

// Menu do app: submenus de Exibir (`core.menu` do Flutter)
pub const MENU_FOCUS_PANE: &str = "Focar Painel";
pub const MENU_FOCUS_LEFT: &str = "Esquerda  (⌘⌥←)";
pub const MENU_FOCUS_RIGHT: &str = "Direita  (⌘⌥→)";
pub const MENU_FOCUS_UP: &str = "Acima  (⌘⌥↑)";
pub const MENU_FOCUS_DOWN: &str = "Abaixo  (⌘⌥↓)";
pub const MENU_SELECT_TAB: &str = "Selecionar Aba";
pub const MENU_TAB_LABELS: [&str; 8] = [
    "Aba 1", "Aba 2", "Aba 3", "Aba 4", "Aba 5", "Aba 6", "Aba 7", "Aba 8",
];
pub const MENU_LAST_TAB: &str = "Última Aba";

// Viewer de arquivo (estados tipados de limite, binário e erro de leitura)
pub const VIEWER_TOO_LARGE_PREFIX: &str = "Arquivo acima do limite de edição (";
pub const EDITOR_EXTERNAL_CHANGE: &str = "O arquivo mudou no disco. Confirme para sobrescrever.";
pub const EDITOR_OVERWRITE: &str = "Sobrescrever arquivo";
#[cfg(target_os = "linux")]
pub const EDITOR_FORMAT: &str = "Formatar";
#[cfg(target_os = "linux")]
pub const EDITOR_FORMATTING: &str = "Formatando…";
#[cfg(target_os = "linux")]
pub const EDITOR_DIAGNOSTIC_ERROR: &str = "Erro";
#[cfg(target_os = "linux")]
pub const EDITOR_DIAGNOSTIC_WARNING: &str = "Aviso";
#[cfg(target_os = "linux")]
pub const EDITOR_DIAGNOSTIC_INFO: &str = "Informação";
#[cfg(target_os = "linux")]
pub const EDITOR_DIAGNOSTIC_HINT: &str = "Sugestão";
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
pub const FILE_VIEWER_PREVIEW: &str = "Pré-visualização";
pub const FILE_VIEWER_SOURCE: &str = "Código-fonte";

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
pub const FILES_MENU_OPEN: &str = "Abrir";
pub const FILES_MENU_OPEN_WITH: &str = "Abrir com";
pub const FILES_MENU_OPEN_NEW_WINDOW: &str = "Abrir em nova janela";
pub const FILES_MENU_OPEN_AS_MARKDOWN: &str = "Abrir como markdown";
pub const FILES_MENU_OPEN_AS_HTML: &str = "Abrir como HTML";
pub const FILES_MENU_OPEN_LAYOUT: &str = "Abrir layout";
pub const FILES_MENU_SHOW_GIT_DIFF: &str = "Mostrar diff do git";
pub const FILES_MENU_NEW_FILE: &str = "Novo arquivo";
pub const FILES_MENU_NEW_FOLDER: &str = "Nova pasta";
pub const FILES_MENU_CREATE_TERMINAL: &str = "Abrir no terminal";
#[cfg(target_os = "macos")]
pub const FILES_MENU_OPEN_FILE_MANAGER: &str = "Abrir no Finder";
#[cfg(target_os = "windows")]
pub const FILES_MENU_OPEN_FILE_MANAGER: &str = "Abrir no Explorer";
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
pub const FILES_MENU_OPEN_FILE_MANAGER: &str = "Abrir no gerenciador de arquivos";
pub const FILES_MENU_RENAME: &str = "Renomear";
pub const FILES_MENU_DELETE: &str = "Excluir";
pub const FILES_MENU_COPY: &str = "Copiar";
pub const FILES_MENU_CUT: &str = "Recortar";
pub const FILES_MENU_PASTE: &str = "Colar";
pub const FILES_MENU_COPY_RELATIVE_PATH: &str = "Copiar caminho relativo";
pub const FILES_MENU_COPY_ABSOLUTE_PATH: &str = "Copiar caminho absoluto";
pub const FILES_DELETE_QUESTION_TITLE: &str = "Excluir?";
pub const FILES_DELETE_PERMANENT_PREFIX: &str = "Excluir “";
pub const FILES_DELETE_PERMANENT_SUFFIX: &str = " permanentemente? Isso não pode ser desfeito.";
pub const FILES_DELETE_TO_TRASH_PREFIX: &str = "Mover “";
pub const FILES_DELETE_TO_TRASH_SUFFIX: &str = "” para a Lixeira?";
pub const FILES_COULD_NOT_DELETE_TITLE: &str = "Não foi possível excluir";
pub const FILES_COULD_NOT_MOVE_TITLE: &str = "Não foi possível mover";
pub const FILES_COULD_NOT_PASTE_TITLE: &str = "Não foi possível colar";
pub const FILE_OP_ALREADY_EXISTS_PREFIX: &str = "Já existe: “";
pub const FILE_OP_ALREADY_EXISTS_SUFFIX: &str = "”.";
pub const FILE_OP_NOT_FOUND_PREFIX: &str = "Não encontrado: “";
pub const FILE_OP_NOT_FOUND_SUFFIX: &str = "”.";
pub const FILE_OP_INVALID_PATH: &str = "Caminho inválido.";
pub const FILE_OP_EMPTY_NAME: &str = "O nome não pode ficar vazio.";
pub const FILE_OP_NAME_HAS_SLASH: &str = "O nome não pode conter “/”.";
pub const FILE_OP_INVALID_NAME: &str = "Nome inválido.";
pub const FILE_OP_CANNOT_MOVE_INTO_ITSELF: &str =
    "Não é possível mover uma pasta para dentro dela mesma.";
pub const FILE_OP_CLIPBOARD_EMPTY: &str = "A área de transferência está vazia.";
pub const FILE_OP_OS_FAILURE: &str = "Não foi possível executar o formatador.";
pub const NO_LSP_AVAILABLE: &str = "Nenhum LSP disponível";

// Ícones tipográficos dos controles de chrome
pub const ICON_SIDEBAR_LEFT: &str = "\u{f114}";
pub const ICON_SIDEBAR_RIGHT: &str = "\u{f114}";
pub const ICON_LAYERS: &str = "\u{e53b}";
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
pub const ICON_CLOUD: &str = "\u{e2bd}";
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
pub const ICON_HUB_OUTLINED: &str = "\u{e9f4}";
pub const ICON_INFO_OUTLINE: &str = "\u{e88e}";
pub const TOPBAR_COLLAPSE_SIDEBAR: &str = "Recolher barra lateral";
pub const TOPBAR_TOGGLE_FILES: &str = "Mostrar/ocultar arquivos";
pub const TOPBAR_OPEN_CHAT: &str = "Chat com o Claude (beta)";
pub const TOPBAR_OPEN_FARM: &str = "Fazenda de agentes";
pub const TOPBAR_VISUALIZE_AGENTS: &str = "Visualizar agentes";
pub const TOPBAR_LOCALSEND_SEND: &str = "Enviar por LocalSend";
pub const WINDOW_CONTROL_MINIMIZE: &str = "Minimizar";
pub const WINDOW_CONTROL_MAXIMIZE: &str = "Maximizar";
pub const WINDOW_CONTROL_CLOSE: &str = "Fechar";
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
pub const GIT_TAB_BRANCHES: &str = "Branches";
pub const GIT_NO_CHANGES: &str = "Nenhuma alteração.";
pub const GIT_NO_REPO: &str = "Nenhum repositório Git disponível.";
pub const GIT_COMMIT_PLACEHOLDER: &str = "Mensagem do commit";
pub const GIT_COMMIT_BUTTON: &str = "Commit";
pub const GIT_COMMITTING_BUTTON: &str = "Comitando…";
pub const GIT_SUGGEST_COMMIT: &str = "Sugerir mensagem";
pub const GIT_SUGGESTING_COMMIT: &str = "Gerando sugestão…";
pub const GIT_SEND_PR_FEEDBACK: &str = "Corrigir com agente";
pub const GIT_FETCHING_PR_FEEDBACK: &str = "Consultando PR no GitHub…";
pub const GIT_BRANCHES_EMPTY: &str = "Nenhuma branch local.";
pub const GIT_BRANCH_DIRTY_NOTICE: &str =
    "Salve ou faça stash das alterações antes de trocar de branch.";
pub const GIT_BRANCH_CURRENT: &str = "atual";
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
pub const GIT_DIFF_ADD_NOTE: &str = "Anotar";
pub const GIT_DIFF_NOTE_PLACEHOLDER: &str = "Observação sobre esta linha para o agente…";
pub const GIT_DIFF_NOTE_SAVE: &str = "Salvar nota";
pub const GIT_DIFF_NOTE_REMOVE: &str = "Remover";
pub fn format_git_send_notes_button(count: usize) -> String {
    format!("Enviar {count} nota(s) ao agente")
}
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
        gsh_git::GitError::NoStagedChanges => {
            "Adicione alterações ao stage antes de pedir uma sugestão.".to_string()
        }
        gsh_git::GitError::StagedDiffTooLarge { max_bytes } => {
            format!("O diff staged excede o limite de {max_bytes} bytes.")
        }
        gsh_git::GitError::StagedDiffTimeout => {
            "O Git excedeu o tempo limite ao obter o diff staged.".to_string()
        }
        gsh_git::GitError::DirtyWorkingTree => GIT_BRANCH_DIRTY_NOTICE.to_string(),
        gsh_git::GitError::BranchNotFound { name } => {
            format!("Branch local não encontrada: {name}")
        }
        gsh_git::GitError::CommitMessageGenerationFailed(failure) => {
            use gsh_git::error::CommitMessageFailure;
            match failure {
                CommitMessageFailure::Disabled => {
                    "Selecione um harness em Configurações → Automações para gerar sugestões."
                        .to_string()
                }
                _ => format!("Falha ao gerar sugestão: {failure}"),
            }
        }
        gsh_git::GitError::CommandFailed { stderr, .. } => {
            format!("{GIT_ERR_CMD_PREFIX}{stderr}")
        }
        gsh_git::GitError::Io { detail } => format!("{GIT_ERR_IO_PREFIX}{detail}"),
    }
}

pub const FARM_TAB_TITLE: &str = "Cenário";
pub const AGENT_GRAPH_TAB_TITLE: &str = "Agentes";
pub const AGENT_GRAPH_EMPTY_TITLE: &str = "Nenhum agente neste workspace";
pub const AGENT_GRAPH_EMPTY_HINT: &str = "Inicie um agente num terminal ou crie um aqui. Arraste da porta de um agente até outro para conectá-los.";
pub const AGENT_GRAPH_NEW_AGENT: &str = "Novo agente";
pub const AGENT_GRAPH_SHELL: &str = "Shell";
pub const AGENT_GRAPH_DESKTOP_APPS: &str = "Apps desktop";
pub const AGENT_GRAPH_AGENT_NAME: &str = "Nome";
pub const AGENT_GRAPH_INTEGRATION: &str = "Integração";
pub const AGENT_GRAPH_INTEGRATION_HINT: &str = "O GSH Desktop adiciona um servidor MCP chamado \"gsh\" a este arquivo e salva um backup ao lado. Os outros servidores não são tocados.";
pub fn agent_graph_integration_consent(app: &str) -> String {
    format!("Adicionar o servidor MCP do GSH Desktop ao config do {app}")
}
pub fn agent_graph_restart_hint(app: &str) -> String {
    format!(
        "Reinicie o {app} se ele já estava aberto, para carregar o servidor MCP do GSH Desktop; depois peça a ele para chamar wait_messages."
    )
}
pub fn agent_graph_integration_invalid(path: &str) -> String {
    format!(
        "{path} não é um JSON/TOML simples que o GSH Desktop consiga editar com segurança (comentários ou outro formato). Nada foi alterado."
    )
}
pub fn agent_graph_cli_missing(path: &str) -> String {
    format!(
        "A CLI do GSH Desktop ainda não foi instalada ({path}). Reinicie o GSH Desktop e tente de novo."
    )
}
pub fn agent_graph_write_failed(detail: &str) -> String {
    format!("Não foi possível atualizar o config: {detail}")
}
pub const AGENT_GRAPH_QUICK_START: &str = "INÍCIO RÁPIDO";
pub const AGENT_GRAPH_DETAILS: &str = "Detalhes";
pub const AGENT_GRAPH_APPEARANCE: &str = "Aparência";
pub const AGENT_GRAPH_AGENT: &str = "Agente";
pub const AGENT_GRAPH_NAME: &str = "Nome do terminal";
pub const AGENT_GRAPH_NAME_PLACEHOLDER: &str = "ex.: Revisor";
pub const AGENT_GRAPH_NAME_TAKEN: &str =
    "Outra aba já usa este nome. Os nomes precisam ser únicos para o `gsh msg --to` achar a aba.";
pub const AGENT_GRAPH_COMMAND: &str = "Comando";
pub const AGENT_GRAPH_COMMAND_PLACEHOLDER: &str = "ex.: claude --model opus (vazio = shell)";
pub const AGENT_GRAPH_CWD: &str = "Diretório de trabalho";
pub const AGENT_GRAPH_BROWSE: &str = "Procurar…";
pub const AGENT_GRAPH_CWD_MISSING: &str = "Esta pasta não existe.";
pub const AGENT_GRAPH_MONITOR: &str = "Monitorar atividade";
pub const AGENT_GRAPH_MONITOR_HINT: &str = "Badge, som e notificação quando um turno termina.";
pub const AGENT_GRAPH_MAESTRO: &str = "Maestro";
pub const AGENT_GRAPH_MAESTRO_TOOLTIP: &str =
    "Promove este agente a maestro: ele fica conectado a todos os agentes do canvas e os coordena.";
pub const AGENT_GRAPH_COLOR: &str = "Cor";
pub const AGENT_GRAPH_COLOR_AUTO: &str = "Automática";
pub const AGENT_GRAPH_ROLE: &str = "Papel / instruções";
pub const AGENT_GRAPH_ROLE_PLACEHOLDER: &str =
    "Enviado como primeira mensagem assim que o agente estiver pronto.";
pub const AGENT_GRAPH_TASK: &str = "Tarefa inicial";
pub const AGENT_GRAPH_TASK_PLACEHOLDER: &str = "Enviada logo depois do papel.";
pub const AGENT_GRAPH_STATUS_WORKING: &str = "Trabalhando";
pub const AGENT_GRAPH_STATUS_WAITING: &str = "Aguardando";
pub const AGENT_GRAPH_STATUS_IDLE: &str = "Ocioso";
pub const AGENT_GRAPH_STATUS_OFFLINE: &str = "Offline";
pub const AGENT_GRAPH_STATUS_ONLINE: &str = "Online";
pub const AGENT_GRAPH_DESKTOP_BADGE: &str = "desktop";
pub const PANE_VIEW_DOCK_AS_TAB: &str = "Encaixar como aba";
pub const NOTIFICATION_AGENT_FINISHED: &str = "Agente terminou";
pub const NOTIFICATION_AGENT_NEEDS_YOU: &str = "Agente precisa de você";
pub const AGENT_GRAPH_LINKED: &str = "Conectado";
pub const AGENT_GRAPH_REMOVE_LINK_HINT: &str = "Clique direito para remover a conexão";
pub fn agent_graph_message_count(count: usize) -> String {
    format!("Mensagens: {count}")
}
pub fn agent_graph_queued(count: usize) -> String {
    format!("{count} na fila")
}
pub fn agent_graph_last_message(preview: &str) -> String {
    format!("Última: {preview}")
}
pub const AGENT_GRAPH_CONNECT_HINT: &str = "Arraste até outro agente para conectar";
pub const AGENT_GRAPH_REMOVE_LINK: &str = "Remover conexão";
pub const AGENT_GRAPH_FOCUS: &str = "Focar";
pub const AGENT_GRAPH_RENAME: &str = "Renomear";
pub const AGENT_GRAPH_RENAME_TITLE: &str = "Renomear agente";
pub const AGENT_GRAPH_MAKE_MAESTRO: &str = "Tornar maestro";
pub const AGENT_GRAPH_REMOVE_MAESTRO: &str = "Remover maestro";
pub const AGENT_GRAPH_CLOSE_TAB: &str = "Fechar aba";
pub const AGENT_GRAPH_REMOVE_FROM_CANVAS: &str = "Remover do canvas";
pub const AGENT_GRAPH_IMPORT_TEAM: &str = "Importar time (.md)…";
pub const AGENT_GRAPH_EXPORT_TEAM: &str = "Exportar time (.md)…";
pub const AGENT_GRAPH_IMPORT_REVIEW_TITLE: &str = "Revisar time importado (.md)";
pub const AGENT_GRAPH_IMPORT_SECURITY_NOTICE: &str = "Revisão de segurança: nenhum comando foi executado na leitura. Apenas harnesses oficiais e pastas dentro do workspace serão abertos se você confirmar.";
pub const AGENT_GRAPH_IMPORT_CONFIRM: &str = "Criar time";
pub fn agent_graph_maestro_replaces(name: &str) -> String {
    format!("{name} é o maestro atual e deixará de ser.")
}
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
/// Menu de contexto da aba (`paneView.*` do Flutter, `_showTabMenu`).
pub const PANE_TAB_RENAME: &str = "Renomear";
pub const PANE_TAB_RESET_TITLE: &str = "Redefinir título";
pub const PANE_TAB_COPY_ID: &str = "Copiar Id";
pub const PANE_TAB_RESTART: &str = "Reiniciar";
pub const PANE_TAB_CLOSE: &str = "Fechar";
pub const PANE_TAB_CLOSE_OTHERS: &str = "Fechar as outras";
pub const PANE_TAB_CLOSE_TO_THE_RIGHT: &str = "Fechar à direita";
pub const PANE_TAB_CLOSE_ALL: &str = "Fechar todas";
pub const PANE_CLOSE_TABS_TITLE: &str = "Fechar abas?";
/// Ícones `Symbols.tab_close_*` e `Icons.restart_alt` do `_showTabMenu`.
pub const ICON_RESTART_ALT: &str = "\u{f053}";
pub const ICON_TAB_CLOSE_INACTIVE: &str = "\u{f3d0}";
pub const ICON_TAB_CLOSE_RIGHT: &str = "\u{f746}";
pub const ICON_TAB_CLOSE: &str = "\u{f745}";

/// Aviso do diálogo de fechar abas em lote (`closeTabsMessage` do Flutter).
pub fn pane_close_tabs_message(count: usize) -> String {
    format!("Isso fecha {count} aba(s) e encerra os agentes/terminais nelas.")
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
pub const SETTINGS_AUTOMATIONS: &str = "Automações";
pub const SETTINGS_LANGUAGE_SERVERS: &str = "Servidores de linguagem";
pub const SETTINGS_FEATURES: &str = "Recursos do GSH";
pub const SETTINGS_AUTOMATION_HARNESS: &str = "Harness padrão";
pub const SETTINGS_AUTOMATION_MODEL: &str = "Modelo padrão";
pub const SETTINGS_AUTOMATION_DISABLED: &str = "Desativado";
pub const SETTINGS_ROUTINES_TITLE: &str = "Rotinas agendadas";
pub const SETTINGS_ROUTINES_DESC: &str = "Escolha os dias da semana e o horário local. A rotina abre o agente no repositório e salva a saída no histórico. O agendamento funciona enquanto o GSH estiver aberto.";
pub const SETTINGS_ROUTINE_NAME_HINT: &str = "Nome (ex.: Revisão matinal)";
pub const SETTINGS_ROUTINE_TIME_HINT: &str = "HH:MM (ex.: 09:00)";
pub const SETTINGS_ROUTINE_WORKSPACE_HINT: &str = "Caminho do repositório";
pub const SETTINGS_ROUTINE_PROMPT_HINT: &str = "Prompt para o agente rodar neste repositório…";
pub const SETTINGS_ROUTINE_ADD: &str = "Agendar rotina";
pub const SETTINGS_ROUTINE_RUN_NOW: &str = "Testar agora";
pub const SETTINGS_ROUTINE_REMOVE: &str = "Remover";
pub const SETTINGS_ROUTINES_EMPTY: &str = "Nenhuma rotina agendada.";
pub const SETTINGS_LSP_COMMANDS: &str = "Servidores de linguagem";
pub const SETTINGS_LSP_FORMATTERS: &str = "Formatadores";
pub const SETTINGS_LSP_HELP: &str =
    "Uma entrada por linha: linguagem = comando. Exemplo: rust = rust-analyzer";
pub const SETTINGS_LSP_INVALID: &str =
    "Confira o formato: linguagem = comando, uma entrada por linha.";
pub const SETTINGS_AUTOMATION_INVALID: &str = "O nome do modelo deve ter até 128 caracteres.";
pub const SETTINGS_FEATURE_CHAT: &str = "Chat";
pub const SETTINGS_FEATURE_FARM: &str = "Fazenda";
pub const SETTINGS_FEATURE_GRAPH: &str = "Grafo de agentes";
pub const SETTINGS_FEATURE_BROWSER: &str = "Navegador";
pub const SETTINGS_FEATURE_HARNESS: &str = "Harness";
pub const SETTINGS_FEATURE_ISLAND: &str = "Ilha de agentes";
pub const SETTINGS_TEAM: &str = "Fazenda do time";
pub const SETTINGS_TEAM_ENABLE: &str = "Ver a fazenda dos colegas";
pub const SETTINGS_TEAM_ENABLE_DESC: &str = "Na mesma rede (ex.: no escritório), cada GSH anuncia a própria fazenda e mostra a dos colegas ao lado da sua — diminua o zoom para ver. Sem servidor: nada sai da rede local. Desligado, nada sai do seu GSH.";
pub const SETTINGS_TEAM_NAME: &str = "Time";
pub const SETTINGS_TEAM_NAME_DESC: &str = "Só quem usar o mesmo nome de time se enxerga.";
pub const SETTINGS_TEAM_NAME_HINT: &str = "ex.: gorila";
pub const SETTINGS_TEAM_ME: &str = "Seu nome";
pub const SETTINGS_TEAM_ME_DESC: &str = "Aparece em cima da sua fazenda para os colegas.";
pub const SETTINGS_TEAM_SHOW_TITLES: &str = "Mostrar títulos das sessões";
pub const SETTINGS_TEAM_SHOW_TITLES_DESC: &str = "Os títulos podem ter trechos dos seus prompts. Desligado, os colegas veem só a pasta, o status e os tokens de cada agente.";
pub const SETTINGS_TEAM_SECRET: &str = "Senha do time";
pub const SETTINGS_TEAM_SECRET_DESC: &str = "Liga a conversa entre agentes: o seu agente pode mandar mensagens e arquivos para os agentes dos colegas, e os deles para o seu. Todo mundo do time usa a mesma senha; sem ela, ninguém bate na sua porta (e a fazenda de quem não tem a senha não aparece).";
pub const SETTINGS_TEAM_SECRET_HINT: &str = "combinem uma senha";
pub const SETTINGS_TEAM_DOOR_ALLOW: &str = "Quem pode bater na porta";
pub const SETTINGS_TEAM_DOOR_ALLOW_DESC: &str =
    "Nomes dos colegas, separados por vírgula. Vazio = todo o time.";
pub const SETTINGS_TEAM_DOOR_ALLOW_HINT: &str = "todo o time";
pub const SETTINGS_TEAM_DOOR_ASK: &str = "Perguntar antes de abrir a porta";
pub const SETTINGS_TEAM_DOOR_ASK_DESC: &str = "Quando um colega mandar algo para um agente seu, o GSH pergunta a você antes de entregar. Desligado, o seu agente recebe direto, marcado como informação de colega (não como ordem sua).";
pub const SETTINGS_TEAM_NETWORK: &str = "Rede do trabalho";
pub const SETTINGS_TEAM_NETWORK_DESC: &str = "Faixa de IP da rede do escritório (ex.: 192.168.10.0/24). Preenchida, o GSH só anuncia e escuta quando você está nela.";
pub const SETTINGS_TEAM_NETWORK_HINT: &str = "qualquer rede local";

// LocalSend
pub const LOCALSEND_TITLE: &str = "LocalSend";
pub const LOCALSEND_POPUP_RECEIVE: &str = "Receber";
pub const LOCALSEND_POPUP_SEND: &str = "Enviar";
pub const LOCALSEND_POPUP_SETTINGS: &str = "Configurações";
pub const LOCALSEND_SELECTION: &str = "Seleção";
pub const LOCALSEND_FILE: &str = "Arquivo";
pub const LOCALSEND_FOLDER: &str = "Pasta";
pub const LOCALSEND_TEXT: &str = "Texto";
pub const LOCALSEND_PASTE: &str = "Colar";
pub const LOCALSEND_NEARBY: &str = "Dispositivos por perto";
pub const LOCALSEND_WIFI_HELP: &str =
    "Certifique-se de que ambos os dispositivos estão na mesma rede Wi-Fi.";
pub const LOCALSEND_CHOOSE_RECEIVE_DIR: &str = "Escolher pasta…";
pub const LOCALSEND_RESTART_SERVER: &str = "Reiniciar servidor";
pub const LOCALSEND_STOP_SERVER: &str = "Parar servidor";
pub const LOCALSEND_START_SERVER: &str = "Iniciar servidor";
pub const LOCALSEND_AWAITING_ACCEPTANCE: &str = "Aguardando o outro lado aceitar…";
pub const LOCALSEND_SENDING: &str = "Enviando";
pub const LOCALSEND_SEND_COMPLETED: &str = "Concluído";
pub const LOCALSEND_SEND_REJECTED: &str = "Recusado";
pub const LOCALSEND_SEND_CANCELLED: &str = "Envio cancelado";
pub const LOCALSEND_SEND_IN_PROGRESS: &str = "Já existe um envio em andamento.";
pub const LOCALSEND_EMPTY_SELECTION: &str = "Selecione arquivos antes de escolher um dispositivo.";
pub const LOCALSEND_STOPPED: &str = "Parado";
pub const LOCALSEND_STATE: &str = "Estado do servidor";
pub const LOCALSEND_SETTINGS_RECEIVE: &str = "Recepção";
pub const LOCALSEND_SETTINGS_NETWORK: &str = "Rede";
pub const LOCALSEND_TEXT_HINT: &str = "Digite a mensagem para enviar como arquivo .txt";
pub const LOCALSEND_ON: &str = "Ativo";
pub const LOCALSEND_OFF: &str = "Desativado";
pub const LOCALSEND_CLOSE: &str = "Fechar";
pub const LOCALSEND_EMPTY_ALIAS: &str = "Meu dispositivo";
pub const LOCALSEND_SELECTED_FILES: &str = "Arquivos selecionados";
pub const LOCALSEND_PORT: &str = "Porta";
pub const LOCALSEND_MULTICAST: &str = "Endereço de multicast";
pub const LOCALSEND_ALWAYS_ON: &str = "Ficar disponível para receber arquivos o tempo todo";
pub const LOCALSEND_ALIAS: &str = "Nome deste dispositivo";
pub const LOCALSEND_ALIAS_HINT: &str = "vazio usa o nome do computador";
pub const LOCALSEND_RECEIVE_DIR: &str = "Pasta para receber";
pub const LOCALSEND_CHOOSE_DIR: &str = "Escolher pasta…";
pub const LOCALSEND_LIMITS: &str = "Limites de recebimento";
pub const LOCALSEND_MAX_FILE_MB_HINT: &str = "por arquivo";
pub const LOCALSEND_MAX_FILES_HINT: &str = "quantidade";
pub const LOCALSEND_MB: &str = "MB por arquivo";
pub const LOCALSEND_FILES: &str = "arquivos";
pub const LOCALSEND_SERVER: &str = "Servidor";
pub const LOCALSEND_STARTING: &str = "Iniciando…";
pub const LOCALSEND_DEVICES: &str = "Dispositivos encontrados";
pub const LOCALSEND_NO_DEVICES: &str = "Nenhum dispositivo LocalSend encontrado na rede.";
pub const LOCALSEND_SEND_FILE: &str = "Enviar arquivo…";
pub const LOCALSEND_CANCEL: &str = "Cancelar envio";
pub const LOCALSEND_SEND_FAILED: &str = "Não foi possível enviar os arquivos.";
pub const SETTINGS_INVALID_LOCALSEND_LIMIT: &str =
    "Use um limite de 1 a 2.048 MB e de 1 a 100 arquivos.";
pub const LOCALSEND_RECEIVE_TITLE: &str = "Receber arquivos?";
pub const LOCALSEND_RECEIVE_FROM: &str = " quer enviar arquivos";
pub const LOCALSEND_ACCEPT: &str = "Aceitar";
pub const LOCALSEND_REJECT: &str = "Recusar";
pub const LOCALSEND_RECEIVE_TO: &str = "Pasta de destino";
pub const LOCALSEND_TOTAL: &str = "Total";
pub const LOCALSEND_RECEIVED: &str = "Recebido";
pub const LOCALSEND_OPEN_FOLDER: &str = "Abrir pasta";
pub const LOCALSEND_SHOW: &str = "Mostrar";
pub const LOCALSEND_PROGRESS: &str = "Recebendo";
pub const LOCALSEND_PORT_BUSY: &str =
    "A porta 53317 já está em uso. Feche o outro LocalSend e tente novamente.";
pub const LOCALSEND_SERVER_ERROR: &str = "Não foi possível iniciar o servidor LocalSend.";
pub fn localsend_server_status(port: u16, fingerprint: &str) -> String {
    format!("Ativo na porta {port} · certificado {fingerprint}")
}
pub fn localsend_server_start_error(detail: &str) -> String {
    format!("{} {detail}", LOCALSEND_SERVER_ERROR)
}
pub const FARM_TEAM_DOOR_TITLE: &str = "Batendo na porta";
pub const FARM_TEAM_DOOR_OPEN: &str = "Abrir a porta";
pub const FARM_TEAM_DOOR_REFUSE: &str = "Recusar";
pub fn farm_team_door_message(name: &str, agent: &str, target: &str) -> String {
    format!("{name} (agente {agent}) quer falar com o seu agente {target}")
}
pub fn farm_team_door_file(name: &str, agent: &str, file: &str, target: &str) -> String {
    format!("{name} (agente {agent}) quer entregar {file} ao seu agente {target}")
}

pub fn farm_peer_banner(name: &str) -> String {
    format!("TIME · {name}")
}
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
pub const SETTINGS_AGENT_NOTIFICATIONS: &str = "Conclusão de respostas e permissões do Claude Code";
pub const SETTINGS_INVALID_SHELL: &str = "Informe um shell não vazio.";
pub const SETTINGS_INVALID_SCROLLBACK: &str = "Informe um histórico entre 1000 e 100000.";
pub const SETTINGS_INVALID_TERMINAL_SIZE: &str = "Informe um tamanho de fonte entre 8 e 32.";
pub const SETTINGS_INVALID_TAB_SIZE: &str = "Informe 2 ou 4 para a tabulação.";
pub const SETTINGS_TERMINAL_RESTART: &str = "As alterações valem para novos terminais.";

// Banco de dados (`pt_BR.i18n.json` -> `gsh.dbPanel`, `gsh.dbConnectionDialog`, `gsh.dbQueryView`)
pub const DB_TOOLTIP: &str = "Banco de dados";
pub const ICON_DATABASE: &str = "\u{f20e}";
pub const ICON_STORAGE: &str = "\u{e1db}";
pub const ICON_MORE_HORIZ: &str = "\u{e5d3}";
pub const ICON_VISIBILITY: &str = "\u{e8f4}";
pub const ICON_TABLE_CHART: &str = "\u{e265}";
pub const ICON_CONTENT_COPY: &str = "\u{e14d}";
pub const ICON_CONTENT_CUT: &str = "\u{e14e}";
pub const ICON_CONTENT_PASTE: &str = "\u{e14f}";
pub const ICON_DELETE: &str = "\u{e92e}";
pub const ICON_PLAY_ARROW: &str = "\u{e037}";
pub const ICON_HOURGLASS_TOP: &str = "\u{ea5b}";
pub const ICON_CHECK_CIRCLE: &str = "\u{f0be}";
pub const ICON_FOLDER_OPEN: &str = "\u{e2c8}";
pub const ICON_GRID_ON: &str = "\u{e3ec}";
pub const ICON_DATA_OBJECT: &str = "\u{ead3}";
pub const ICON_GPP_MAYBE: &str = "\u{f014}";

pub const DB_SECTION: &str = "BANCO DE DADOS";
pub const DB_EDIT: &str = "Editar…";
pub const DB_COPY_NAME: &str = "Copiar nome";
pub const DB_NEW_QUERY: &str = "Nova query";
pub const DB_BROWSE_KEYS: &str = "Ver chaves";
pub const DB_DELETE_CONNECTION_TITLE: &str = "Excluir conexão";
pub const DB_NO_CONNECTIONS: &str = "Nenhuma conexão ainda.";
pub const DB_FOOTER_ONE: &str = ".gsh/databases.json · 1 conexão";
pub const DB_LOADING: &str = "Loading…";
pub const DB_NO_TABLES: &str = "No tables.";
pub const DB_NO_DATABASES: &str = "No databases.";
pub const DB_NO_COLLECTIONS: &str = "No collections.";
pub const DB_DEFAULT_SCHEMA: &str = "(default)";
pub const DB_PRIMARY_KEY: &str = "PK";
pub const DB_PASSWORD_REQUIRED_ON_HOST: &str = "Senha não encontrada no host. Abra esta conexão e digite-a de novo — ela fica salva na máquina que executa o banco, não nesta.";
// Selos da linha: texto fixo em inglês no Flutter
pub const DB_CHIP_DETECTED: &str = "detected";
pub const DB_CHIP_LOCAL: &str = "local";
pub const DB_CHIP_READWRITE: &str = "rw";
pub const DB_CHIP_NO_AGENTS: &str = "no agents";

pub fn db_delete_connection_message(name: &str) -> String {
    format!(
        "Remover \"{name}\" deste workspace? Qualquer senha salva será descartada. Arquivos .dbq que fazem referência a ela não são afetados."
    )
}

pub fn db_footer(count: usize) -> String {
    format!(".gsh/databases.json · {count} conexões")
}

pub const DB_DIALOG_CHOOSE_FILE_TITLE: &str = "Escolher banco SQLite";
pub const DB_DIALOG_FILE: &str = "Arquivo";
pub const DB_DIALOG_CHOOSE_FILE_PLACEHOLDER: &str = "Escolha um arquivo SQLite…";
pub const DB_DIALOG_NAME: &str = "Nome";
pub const DB_DIALOG_NAME_HINT: &str = "dev-local";
pub const DB_DIALOG_PASSWORD: &str = "Senha";
pub const DB_DIALOG_SAVE_PASSWORD: &str = "Salvar senha";
pub const DB_DIALOG_ALLOW_WRITES: &str = "Permitir escrita (agentes)";
pub const DB_DIALOG_ALLOW_WRITES_HINT: &str = "desligado = agentes só leem via CLI";
pub const DB_DIALOG_VISIBLE_TO_AGENTS: &str = "Visível para agentes";
pub const DB_DIALOG_VISIBLE_TO_AGENTS_HINT: &str = "desligado = oculto da CLI, só na GUI";
pub const DB_DIALOG_TESTING: &str = "Testando conexão…";
pub const DB_DIALOG_CONNECTION_OK: &str = "Conexão OK";
pub const DB_DIALOG_CONNECTION_FAILED: &str = "Falha na conexão";
pub const DB_DIALOG_EDIT_TITLE: &str = "Editar conexão";
pub const DB_DIALOG_NEW_TITLE: &str = "Nova conexão";
pub const DB_DIALOG_CONNECTION_STRING: &str = "Connection string";
pub const DB_DIALOG_INVALID_URL: &str = "URL de conexão inválida.";
pub const DB_DIALOG_SSH_TUNNEL: &str = "Túnel SSH";
pub const DB_DIALOG_SSH_HOST: &str = "Host SSH";
pub const DB_DIALOG_SSH_HOST_HINT: &str = "bastion.example.com";
pub const DB_DIALOG_SSH_PORT: &str = "Porta SSH";
pub const DB_DIALOG_SSH_PORT_HINT: &str = "22";
pub const DB_DIALOG_SSH_USER: &str = "Usuário SSH";
pub const DB_DIALOG_SSH_USER_HINT: &str = "ubuntu";
pub const DB_DIALOG_SSH_KEY: &str = "Chave privada";
pub const DB_DIALOG_SSH_KEY_PLACEHOLDER: &str = "Escolha uma chave privada…";
pub const DB_DIALOG_SSH_KEY_DIALOG_TITLE: &str = "Escolher chave privada SSH";
pub const DB_DIALOG_SSH_PASSPHRASE: &str = "Senha da chave";
pub const DB_DIALOG_SSH_SAVE_PASSPHRASE: &str = "Salvar senha da chave";
pub const DB_DIALOG_PASSWORD_ON_HOST: &str = "A senha fica salva no host, não nesta máquina.";
pub const DB_DIALOG_SAVE: &str = "Salvar";
pub const DB_DIALOG_ADD: &str = "Adicionar";
pub const DB_DIALOG_DELETE: &str = "Excluir";
pub const DB_DIALOG_TEST: &str = "Testar";
pub const DB_DEFAULT_CONNECTION_NAME: &str = "new-connection";

// Prompts de SSH (`sshPrompts`)
pub const SSH_UNKNOWN_HOST_TITLE: &str = "Host SSH desconhecido";
pub const SSH_TRUST_HINT: &str = "Confie apenas se esta fingerprint corresponder ao servidor. Você pode verificar no servidor com:";
pub const SSH_COMMAND_PREVIEW: &str = "ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub";
pub const SSH_TRUST: &str = "Confiar";
pub const SSH_KEY_PASSPHRASE_TITLE: &str = "Senha da chave SSH";
pub const SSH_KEPT_IN_MEMORY_HINT: &str = "Mantida em memória até o GSH Desktop fechar. Para permitir que agentes usem esta conexão, ative \"Salvar senha da chave\" na conexão.";
pub const SSH_UNLOCK: &str = "Desbloquear";

pub fn ssh_never_connected(endpoint: &str) -> String {
    format!("O GSH Desktop nunca se conectou a {endpoint} antes.")
}

pub fn ssh_unlock_message(key_path: &str, connection_name: &str) -> String {
    format!("Desbloqueie {key_path} para conectar \"{connection_name}\".")
}

// Navegador Redis (`dbRedisTable`)
pub const REDIS_DELETE_KEY_TITLE: &str = "Excluir chave";
pub const REDIS_REFRESH: &str = "Atualizar";
pub const REDIS_NEW_KEY: &str = "Nova chave";
pub const REDIS_COLUMN_KEY: &str = "CHAVE";
pub const REDIS_COLUMN_VALUE: &str = "VALOR";
pub const REDIS_COLUMN_TYPE: &str = "TIPO";
pub const REDIS_COLUMN_TTL: &str = "TTL";
pub const REDIS_NO_KEYS: &str = "Nenhuma chave neste banco de dados.";
pub const REDIS_LOAD_MORE: &str = "Carregar mais";
pub const REDIS_LOADING_FULL_VALUE: &str = "Carregando valor completo…";
pub const REDIS_TTL_MUST_BE_NUMBER: &str = "TTL deve ser um número de segundos.";
pub const REDIS_ADD_KEY: &str = "Adicionar chave";
pub const REDIS_KEY_FIELD_HINT: &str = "chave";
pub const REDIS_TTL_FIELD_HINT: &str = "ttl (s, opcional)";
pub const REDIS_VALUE_FIELD_HINT: &str = "valor";
pub const REDIS_SEARCH_HINT: &str = "Buscar — padrão, ex.: user:*";
pub const REDIS_NEW_KEY_DRAFT_TITLE: &str = "Nova chave";

pub fn redis_delete_key_message(key: &str) -> String {
    format!("Excluir \"{key}\" deste banco Redis?")
}

pub fn redis_key_count(n: usize) -> String {
    if n == 1 {
        "1 chave".to_string()
    } else {
        format!("{n} chaves")
    }
}

pub fn redis_no_keys_match(pattern: &str) -> String {
    format!("Nenhuma chave corresponde a \"{pattern}\".")
}

// Navegador MongoDB (`dbMongoView`)
pub const MONGO_DELETE_DOC_TITLE: &str = "Excluir documento";
pub const MONGO_FILTER_HINT: &str = "Filtro — JSON, ex.: {\"status\": \"active\"}";
pub const MONGO_REFRESH: &str = "Atualizar";
pub const MONGO_INSERT_DOC: &str = "Inserir documento";
pub const MONGO_NO_DOCUMENTS: &str = "Nenhum documento nesta coleção.";
pub const MONGO_NO_DOCUMENTS_MATCH: &str = "Nenhum documento corresponde a este filtro.";
pub const MONGO_LOAD_MORE: &str = "Carregar mais";
pub const MONGO_EDIT: &str = "Editar";
pub const MONGO_INSERT: &str = "Inserir";

pub fn mongo_delete_doc_message(id: &str, collection: &str) -> String {
    format!("Excluir o documento com _id {id} de \"{collection}\"?")
}

pub fn mongo_doc_count(n: usize) -> String {
    if n == 1 {
        "1 doc".to_string()
    } else {
        format!("{n} docs")
    }
}

pub const UNSAVED_CHANGES_TITLE: &str = "Alterações não salvas";
pub const UNSAVED_DONT_SAVE: &str = "Não salvar";
pub const UNSAVED_SAVE_AND_CLOSE: &str = "Salvar e fechar";

pub fn unsaved_changes_message(file_name: &str) -> String {
    format!("“{file_name}” tem alterações não salvas. Salvar antes de fechar?")
}

pub const DB_QUERY_SAVE_AS: &str = "Salvar query como";
pub const DB_QUERY_COULD_NOT_SAVE: &str = "Não foi possível salvar";
pub const DB_QUERY_SELECT_DATABASE: &str = "Selecionar banco de dados";
pub const DB_QUERY_NO_SQL_CONNECTIONS: &str = "Nenhuma conexão SQL";
pub const DB_QUERY_RUNNING: &str = "Executando…";
pub const DB_QUERY_RUN_SELECTION: &str = "Executar seleção";
pub const DB_QUERY_RUN: &str = "Executar";
pub const DB_QUERY_RUN_SHORTCUT: &str = "⌘↵";
pub const DB_QUERY_PICK_DATABASE_HINT: &str =
    "Escolha um banco de dados acima e depois Executar (⌘↵).";
pub const DB_QUERY_RUN_HINT: &str = "Execute a query (⌘↵) para ver os resultados aqui.";
pub const DB_QUERY_NO_ROWS: &str = "Nenhuma linha.";
pub const DB_QUERY_TRUNCATED_SUFFIX: &str = " · truncado (aumente -- limit)";
pub const DB_QUERY_TABLE: &str = "Tabela";
pub const DB_QUERY_JSON: &str = "JSON";
pub const DB_QUERY_UNSAVED: &str = "não salvo";
pub const DB_QUERY_SAVED: &str = "salvo";
pub const DB_QUERY_COPIED: &str = "Copiado";
pub const DB_QUERY_COPY: &str = "Copiar";
pub const DB_QUERY_DEFAULT_FILE_NAME: &str = "query.dbq";
pub const DB_NULL_CELL: &str = "NULL";

pub fn db_rows_affected(count: u64) -> String {
    if count == 1 {
        "1 linha afetada".to_string()
    } else {
        format!("{count} linhas afetadas")
    }
}

pub fn db_rows_footer(count: usize) -> String {
    format!("{count} linhas")
}

/// Aviso do modo JSON (texto fixo em inglês no Flutter).
pub fn db_json_preview_limited(shown: usize, total: usize) -> String {
    format!("Preview limited to the first {shown} of {total} rows — use Copy for the full JSON.")
}

pub const DB_ERR_SAVE_EMPTY_NAME: &str = "O nome não pode ficar vazio.";
pub const DB_ERR_SAVE_NAME_HAS_SLASH: &str = "O nome não pode conter “/”.";
pub const DB_ERR_SAVE_INVALID_NAME: &str = "Nome inválido.";
pub const DB_ERR_SAVE_NO_WORKSPACE: &str = "Nenhum workspace selecionado.";
pub const DB_ERR_SAVE_WRITE_FAILED: &str = "Não foi possível gravar o arquivo.";

pub fn db_err_save_already_exists(name: &str) -> String {
    format!("Já existe: “{name}”.")
}

/// Frase de um erro do banco. O texto do driver sai como veio (não se traduz); as frases que o
/// próprio GSH inventa são as do Flutter, em inglês.
pub fn db_error_message(error: &gsh_db::DbError) -> String {
    use gsh_db::DbError;
    match error {
        DbError::ConnectionFailed(detail)
        | DbError::QueryFailed(detail)
        | DbError::SshKeyMissing(detail)
        | DbError::SshCredentialRequired(detail)
        | DbError::SshAuthFailed(detail)
        | DbError::SshConnectFailed(detail) => detail.clone(),
        DbError::Timeout(secs) => format!("Query exceeded {secs}s"),
        DbError::UnsupportedEngine(engine) => {
            format!("{} support is not available yet.", engine.label())
        }
        DbError::UnknownConnection { name, available } => {
            let list = if available.is_empty() {
                "(none)".to_string()
            } else {
                available.join(", ")
            };
            format!("No connection named \"{name}\" in this workspace. Available: {list}")
        }
        DbError::NothingToRun => "Nothing to run.".to_string(),
        DbError::InvalidIdentifier(name) => format!("Invalid table name: \"{name}\""),
        DbError::SshHostKeyUnknown { message, .. } => message.clone(),
        DbError::SshHostKeyChanged { .. } => error.to_string(),
    }
}

pub const SETTINGS_NAV_SEARCH: &str = "Buscar configurações";
pub const SETTINGS_NAV_NO_RESULTS: &str = "Nenhum resultado";
pub const SETTINGS_NAV_CLAUDE: &str = "Claude";

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
pub const GALLERY_INTRO: &str = "Documentos especiais do GSH Desktop para que você tenha o visual do que o agente de IA esteja fazendo.";
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
        GalleryTemplate::DbQuery => {
            "Editor SQL com grid de resultado, usando uma das conexões registradas."
        }
        GalleryTemplate::Kanban => {
            "Colunas e cards gravados como markdown puro. Arraste, edite e comente."
        }
        GalleryTemplate::Notebook => {
            "Uma pasta de notas curtas com tags. O agente escreve, você lê e edita. Abre no Obsidian também."
        }
        GalleryTemplate::Html => {
            "Uma página que o agente escreve e o GSH Desktop renderiza direto: mapa mental, diagrama, gráfico, o que precisar."
        }
        GalleryTemplate::Panel => {
            "Uma página HTML com ponte para o app: os botões rodam comandos da CLI do GSH Desktop e do shell nesta máquina. Um playground para dashboards rápidos."
        }
        GalleryTemplate::HttpRequest => {
            "Escreva requests num arquivo e execute com a resposta ao lado."
        }
        GalleryTemplate::Tasks => {
            "Comandos para rodar pelo painel de Tasks, como dev server, testes e build. Fica em .gsh/tasks.json."
        }
        GalleryTemplate::Layout => {
            "Terminais e splits para abrir de uma vez, estilo tmuxinator. Pode rodar sozinho em worktrees novas."
        }
        GalleryTemplate::WorkspaceEnv => {
            "Variáveis injetadas em todo terminal deste workspace. Coloque tokens ou logins de API aqui em vez de colar no prompt do agente. Fica fora do git."
        }
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
pub const ICON_PICTURE_AS_PDF: &str = "\u{e415}";
pub const ICON_CHECK_BOX: &str = "\u{e834}";
pub const ICON_CHECK_BOX_OUTLINE_BLANK: &str = "\u{e835}";
pub const ICON_RADIO_BUTTON_UNCHECKED: &str = "\u{e836}";
pub const ICON_RADIO_BUTTON_CHECKED: &str = "\u{e837}";
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
        Kind::Memory => {
            "CLAUDE.md lidos numa sessão aqui (da pasta para cima), o seu do usuário e o AGENTS.md (Codex)."
        }
        Kind::Skills => {
            "Capacidades que o agente aciona sozinho quando a descrição bate com a tarefa."
        }
        Kind::Agents => {
            "Especialistas para quem o agente delega (com ferramentas e modelo próprios)."
        }
        Kind::Commands => "Atalhos /nome que você digita na sessão.",
        Kind::Hooks => {
            "Comandos que rodam em eventos da sessão (antes de uma ferramenta, ao parar...)."
        }
        Kind::Permissions => "Regras de ferramenta: liberar, perguntar ou bloquear.",
        Kind::Mcp => {
            "Servidores de ferramentas extras. Os do usuário e os locais vêm do ~/.claude.json (mude com claude mcp)."
        }
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

// Aba Claude das Configurações (`pt_BR.i18n.json` -> `settings.page.claudeCode`, `account`, `privacy`,
// `generalClaude`, `claude`, `nav`)
pub const SETTINGS_CC_SECTION_GENERAL: &str = "Geral";
pub const SETTINGS_CC_CLASSIFY_TITLE: &str = "Classificar estados das sessões";
pub const SETTINGS_CC_CLASSIFY_DESC: &str = "Mostra no cabeçalho do chat o estado que o Claude Code atribui ao fim de cada turno: concluída, pronta para revisão ou bloqueada. O Claude Code classifica de qualquer jeito; desligar só esconde o selo.";
pub const SETTINGS_CC_SWITCH_MODELS_TITLE: &str =
    "Alternar modelos quando uma mensagem for sinalizada";
pub const SETTINGS_CC_SWITCH_MODELS_DESC: &str = "Quando as salvaguardas sinalizarem uma mensagem, alternar automaticamente para um modelo diferente para continuar a conversa. Quando desativado, sua sessão será pausada. Fica no ~/.claude/settings.json (switchModelsOnFlag).";
pub const SETTINGS_CC_SECTION_CODE_APPEARANCE: &str = "Aparência do código";
pub const SETTINGS_CC_FOLLOW_THEME: &str = "Do tema ativo";
pub const SETTINGS_CC_CODE_THEME_LIGHT_HINT: &str = "Cores do código no modo claro";
pub const SETTINGS_CC_CODE_THEME_DARK_HINT: &str = "Cores do código no modo escuro";
pub const SETTINGS_CC_CODE_FONT_TITLE: &str = "Fonte de código";
pub const SETTINGS_CC_CODE_FONT_DESC: &str = "Defina uma fonte monoespaçada personalizada para código: viewer, editor, diffs e blocos de código do chat. O terminal tem fonte própria em Aparência.";
pub const SETTINGS_CC_CODE_FONT_PLACEHOLDER: &str = "ex.: JetBrains Mono";
pub const SETTINGS_CC_SECTION_APPEARANCE: &str = "Aparência";
pub const SETTINGS_CC_TRANSCRIPT_TITLE: &str = "Visualização padrão da transcrição";
pub const SETTINGS_CC_TRANSCRIPT_DESC: &str =
    "A visualização com que novas conversas do chat abrem. Conversas já abertas mantêm a delas.";
pub const SETTINGS_CC_VIEW_NORMAL: &str = "Normal";
pub const SETTINGS_CC_VIEW_THINKING: &str = "Raciocínio";
pub const SETTINGS_CC_VIEW_VERBOSE: &str = "Detalhado";
pub const SETTINGS_CC_SECTION_REMOTE_CONTROL: &str = "Remote Control";
pub const SETTINGS_CC_CHAT_REMOTE_CONTROL_TITLE: &str = "Chat com Remote Control (experimental)";
pub const SETTINGS_CC_CHAT_REMOTE_CONTROL_DESC: &str = "O chat do GSH roda o claude interativo num terminal escondido, conectado ao Remote Control: a conversa aparece no celular e em claude.ai/code, e sessões de outras máquinas enxergam ela. A resposta chega por mensagem (sem aparecer letra a letra). Vale para chats abertos depois da troca; desligue para voltar ao motor rápido.";
pub const SETTINGS_CC_REMOTE_CONTROL_TITLE: &str = "Conectar novas sessões ao Remote Control";
pub const SETTINGS_CC_REMOTE_CONTROL_DESC: &str = "Toda sessão do claude que você iniciar num terminal se conecta ao Remote Control, para continuar no celular ou em claude.ai/code. Fica no ~/.claude/settings.json (remoteControlAtStartup). Exige login claude.ai (Pro, Max, Team ou Enterprise).";
pub const SETTINGS_CC_SECTION_SESSIONS: &str = "Sessões";
pub const SETTINGS_CC_PROMPT_SUGGESTIONS_TITLE: &str = "Sugestões de prompts";
pub const SETTINGS_CC_PROMPT_SUGGESTIONS_DESC: &str = "Mostrar uma sugestão de próximo prompt depois que o Claude responder. As sugestões consomem um pouco do seu uso.";
pub const SETTINGS_CC_OUTPUT_STYLE_TITLE: &str = "Estilo de saída";
pub const SETTINGS_CC_OUTPUT_STYLE_DESC: &str =
    "Como o Claude estrutura suas respostas nas sessões do Code. Aplica-se a novas sessões.";
pub const SETTINGS_CC_OUTPUT_STYLE_DEFAULT: &str = "Padrão";
pub const SETTINGS_CC_CROSS_SESSION_TITLE: &str = "Mensagens de outras sessões";
pub const SETTINGS_CC_CROSS_SESSION_DESC: &str = "O que fazer quando outra sessão do Claude Code manda mensagem para esta (SendMessage). O GSH deixa Aceitar para toda sessão da máquina; Segurar guarda para você aprovar, Recusar descarta. Fica em crossSessionInbound.";
pub const SETTINGS_CC_CROSS_SESSION_ACCEPT: &str = "Aceitar";
pub const SETTINGS_CC_CROSS_SESSION_HOLD: &str = "Segurar para aprovar";
pub const SETTINGS_CC_CROSS_SESSION_REFUSE: &str = "Recusar";
pub const SETTINGS_CC_SANDBOX_TITLE: &str = "Sandbox local";
pub const SETTINGS_CC_SANDBOX_DESC: &str =
    "Executa os comandos bash do Claude Code num sandbox isolado. Aplica-se a novas sessões.";
pub const SETTINGS_CC_SANDBOX_STRICT_TITLE: &str = "Modo sandbox estrito";
pub const SETTINGS_CC_SANDBOX_STRICT_DESC: &str = "Bloqueia comandos que não podem ser executados dentro do sandbox em vez de executá-los fora dele.";
pub const SETTINGS_CC_SECTION_SECURITY: &str = "Segurança";
pub const SETTINGS_CC_PROTECTED_HOSTS_TITLE: &str = "Hosts protegidos";
pub const SETTINGS_CC_PROTECTED_HOSTS_DESC: &str = "Hosts que nenhum agente pode tocar num comando: ssh, scp, workspace remoto, qualquer coisa que cite o nome. O GSH bloqueia o comando antes de rodar, em toda sessão do Claude Code e do Codex nesta máquina, e manda o agente pedir a você. Separe por vírgula; vale a palavra inteira (prod bloqueia \"ssh prod\" e \"prod.x.com\", não \"production\"). Fica em ~/.gsh/protected-hosts.json.";
pub const SETTINGS_CC_SECTION_BROWSER: &str = "Navegador";
pub const SETTINGS_CC_LINKS_IN_APP_TITLE: &str = "Abrir links no navegador integrado";
pub const SETTINGS_CC_LINKS_IN_APP_DESC: &str = "Links clicados nos terminais abrem numa aba de navegador do GSH em vez do seu navegador padrão.";
pub const SETTINGS_ACCOUNT_TITLE: &str = "Conta";
pub const SETTINGS_ACCOUNT_PROFILE: &str = "Perfil";
pub const SETTINGS_ACCOUNT_INSTRUCTIONS: &str = "Instruções para o Claude";
pub const SETTINGS_ACCOUNT_INSTRUCTIONS_DESC: &str =
    "Gravado em ~/.claude/CLAUDE.md: o Claude Code carrega em toda sessão deste computador.";
pub const SETTINGS_ACCOUNT_INSTRUCTIONS_HINT: &str = "ex.: manter explicações breves e diretas";
pub const SETTINGS_ACCOUNT_LOGGED_IN: &str = "Conectado";
pub const SETTINGS_ACCOUNT_LOGGED_OUT: &str = "Desconectado";
pub const SETTINGS_ACCOUNT_LOGGED_IN_DESC: &str = "O Claude Code deste computador usa esta conta.";
pub const SETTINGS_ACCOUNT_LOGGED_OUT_DESC: &str =
    "Entre para usar sua assinatura do Claude no Claude Code.";
pub const SETTINGS_ACCOUNT_LOGIN: &str = "Entrar";
pub const SETTINGS_ACCOUNT_LOGOUT: &str = "Sair";
pub const SETTINGS_ACCOUNT_SWITCH: &str = "Trocar de conta";
pub const SETTINGS_ACCOUNT_LOGOUT_CONFIRM_TITLE: &str = "Sair do Claude Code?";
pub const SETTINGS_ACCOUNT_LOGOUT_CONFIRM_MESSAGE: &str = "Roda `claude auth logout`. As sessões deste computador deixam de usar sua assinatura até você entrar de novo.";
pub const SETTINGS_ACCOUNT_SWITCH_CONFIRM_TITLE: &str = "Trocar de conta do Claude Code?";
pub const SETTINGS_ACCOUNT_SWITCH_CONFIRM_MESSAGE: &str = "Encerra a sessão atual (`claude auth logout`) e abre um terminal com `claude auth login` para autenticar outra conta.";
pub const SETTINGS_ACCOUNT_EMAIL: &str = "E-mail";
pub const SETTINGS_ACCOUNT_ORGANIZATION: &str = "Organização";
pub const SETTINGS_ACCOUNT_ORG_ID: &str = "ID da organização";
pub const SETTINGS_ACCOUNT_PLAN: &str = "Plano";
pub const SETTINGS_ACCOUNT_METHOD: &str = "Método de login";
pub const SETTINGS_PRIVACY_TITLE: &str = "Dados";
pub const SETTINGS_PRIVACY_GSH_TELEMETRY: &str = "Telemetria do GSH";
pub const SETTINGS_PRIVACY_GSH_TELEMETRY_DESC: &str =
    "Só local: erros e logs ficam num SQLite deste computador e nunca são enviados.";
pub const SETTINGS_PRIVACY_CC_TELEMETRY: &str = "Desligar a telemetria do Claude Code";
pub const SETTINGS_PRIVACY_CC_TELEMETRY_DESC: &str = "Define DISABLE_TELEMETRY no env do ~/.claude/settings.json (métricas de uso não são enviadas).";
pub const SETTINGS_PRIVACY_CC_ERRORS: &str = "Desligar relatórios de erro do Claude Code";
pub const SETTINGS_PRIVACY_CC_ERRORS_DESC: &str =
    "Define DISABLE_ERROR_REPORTING no env do ~/.claude/settings.json.";
pub const SETTINGS_GC_SECTION_APPEARANCE: &str = "Aparência";
pub const SETTINGS_GC_THEME: &str = "Tema";
pub const SETTINGS_GC_THEME_SYSTEM: &str = "Sistema";
pub const SETTINGS_GC_THEME_LIGHT: &str = "Claro";
pub const SETTINGS_GC_THEME_DARK: &str = "Escuro";
pub const SETTINGS_GC_TEXT_SIZE: &str = "Tamanho do texto da conversa";
pub const SETTINGS_GC_TEXT_SIZE_DESC: &str =
    "Tamanho base do texto do app (o mesmo ajuste de Aparência).";
pub const SETTINGS_GC_SIZE_SMALL: &str = "Pequeno";
pub const SETTINGS_GC_SIZE_MEDIUM: &str = "Médio";
pub const SETTINGS_GC_SIZE_LARGE: &str = "Grande";
pub const SETTINGS_GC_TRANSCRIPT_WIDTH: &str = "Largura da transcrição";
pub const SETTINGS_GC_TRANSCRIPT_WIDTH_DESC: &str =
    "Largura máxima das colunas da transcrição e do campo de mensagem.";
pub const SETTINGS_GC_WIDTH_NARROW: &str = "Estreito";
pub const SETTINGS_GC_WIDTH_MEDIUM: &str = "Médio";
pub const SETTINGS_GC_WIDTH_WIDE: &str = "Largo";
pub const SETTINGS_GC_REDUCED_MOTION: &str = "Movimento reduzido";
pub const SETTINGS_GC_REDUCED_MOTION_DESC: &str = "Menos animações no Claude Code (brilho do spinner, piscadas). Gravado como prefersReducedMotion no ~/.claude/settings.json.";
pub const SETTINGS_GC_SECTION_NOTIFICATIONS: &str = "Notificações";
pub const SETTINGS_GC_NOTIFY_TURN_DONE: &str = "Conclusão de respostas";
pub const SETTINGS_GC_NOTIFY_TURN_DONE_DESC: &str =
    "Receba uma notificação quando um agente terminar uma resposta e a janela não estiver em foco.";
pub const SETTINGS_GC_NOTIFY_PERMISSION: &str = "Solicitações de permissão do Code";
pub const SETTINGS_GC_NOTIFY_PERMISSION_DESC: &str =
    "Toca um som quando um agente estiver esperando sua aprovação ou resposta.";
pub const SETTINGS_GC_PUSH_INPUT_NEEDED: &str = "Push no celular quando precisar de você";
pub const SETTINGS_GC_PUSH_INPUT_NEEDED_DESC: &str = "O Claude Code manda um push quando uma permissão ou pergunta estiver esperando (inputNeededNotifEnabled).";
pub const SETTINGS_GC_PUSH_AGENT: &str = "Notificações proativas no celular";
pub const SETTINGS_GC_PUSH_AGENT_DESC: &str = "Deixa o Claude avisar no celular sobre atualizações importantes de uma sessão (agentPushNotifEnabled).";
pub const SETTINGS_CLAUDE_GROUP: &str = "Configurações";
pub const SETTINGS_CLAUDE_GROUP_COMPUTER: &str = "Este computador";
pub const SETTINGS_CLAUDE_GROUP_PERSONALIZATION: &str = "Personalização";
pub const SETTINGS_CLAUDE_GROUP_PLATFORM: &str = "Plataforma";
pub const SETTINGS_NAV_ACCOUNT: &str = "Conta";
pub const SETTINGS_NAV_PRIVACY: &str = "Privacidade";
pub const SETTINGS_NAV_USAGE: &str = "Uso";
pub const SETTINGS_NAV_RESOURCES: &str = "Recursos";
pub const SETTINGS_NAV_MEMORY: &str = "Memória";
pub const SETTINGS_NAV_DESIGN_SYSTEMS: &str = "Design systems";
pub const SETTINGS_NAV_CLAUDE_CODE: &str = "Claude Code";
pub const SETTINGS_NAV_COWORK: &str = "Cowork";
pub const SETTINGS_NAV_SYSTEM: &str = "Sistema";
pub const SETTINGS_NAV_EXTENSIONS: &str = "Extensões";
pub const SETTINGS_NAV_DEVELOPER: &str = "Desenvolvedor";
pub const SETTINGS_NAV_SKILLS: &str = "Skills";
pub const SETTINGS_NAV_CONNECTORS: &str = "Conectores";
pub const SETTINGS_NAV_PLUGINS: &str = "Plugins";
pub const SETTINGS_NAV_API_KEYS: &str = "Chaves de API";
pub const SETTINGS_CLAUDE_REFRESH: &str = "Atualizar";
pub const SETTINGS_CLAUDE_LOADING: &str = "Carregando…";
pub const SETTINGS_CLAUDE_TERMINAL_TITLE: &str = "Abrir um terminal";
pub const SETTINGS_CLAUDE_NO_WORKSPACE: &str = "Abra um workspace local antes: isto roda a CLI `claude` num terminal dele, para você acompanhar.";
pub const SETTINGS_COMMON_OPEN: &str = "Abrir";
pub const SETTINGS_COMMON_ADD: &str = "Adicionar";
pub const ICON_ACCOUNT_CIRCLE: &str = "\u{f20b}";
pub const ICON_CANCEL: &str = "\u{e888}";
pub const ICON_CODE: &str = "\u{e86f}";
pub const ICON_DIFFERENCE: &str = "\u{eb7d}";
pub const ICON_LIGHTBULB: &str = "\u{e90f}";
pub const ICON_DARK_MODE: &str = "\u{e51c}";
pub const ICON_DESKTOP_WINDOWS: &str = "\u{e30c}";
pub const ICON_LIGHT_MODE: &str = "\u{e518}";
pub const ICON_DATA_USAGE: &str = "\u{eff2}";
pub const ICON_WORK_OUTLINE: &str = "\u{e943}";
pub const ICON_PSYCHOLOGY: &str = "\u{ea4a}";
pub const ICON_CHECKLIST: &str = "\u{e6b1}";
pub const ICON_BUILD: &str = "\u{f8cd}";
pub const ICON_HISTORY_EDU: &str = "\u{ea3e}";
pub const ICON_WIDGETS: &str = "\u{e1bd}";
pub const ICON_POWER: &str = "\u{e63c}";
pub const ICON_VPN_KEY: &str = "\u{e0da}";
pub const ICON_NORTH_EAST: &str = "\u{f1e1}";
pub const ICON_STOREFRONT: &str = "\u{ea12}";
pub const ICON_LANGUAGE: &str = "\u{ea07}";
pub const ICON_KEY: &str = "\u{e73c}";
pub const ICON_PAUSE: &str = "\u{e034}";
pub const ICON_HELP_OUTLINE: &str = "\u{e8fd}";
pub const ICON_DELETE_OUTLINE: &str = "\u{e92e}";
pub const ICON_MONITOR_HEART: &str = "\u{eaa2}";
pub const ICON_VISIBILITY_OFF: &str = "\u{e8f5}";
pub const ICON_OPEN_IN_NEW: &str = "\u{e89e}";

// Fontes (`generalClaude` + `appearance.fontPicker*`)
pub const SETTINGS_GC_UI_FONT: &str = "Fonte da interface";
pub const SETTINGS_GC_UI_FONT_DESC: &str =
    "Fonte para toda a interface — menus, barras laterais e painéis.";
pub const SETTINGS_GC_UI_FONT_HINT: &str = "Montserrat · Hanken";
pub const SETTINGS_GC_CHAT_FONT: &str = "Fonte do chat";
pub const SETTINGS_GC_CHAT_FONT_DESC: &str = "Texto da conversa no chat com o Claude.";
pub const SETTINGS_GC_CHAT_FONT_DEFAULT: &str = "Mesma da interface";
pub const SETTINGS_FONT_PICKER_TITLE: &str = "Escolher uma fonte";
pub const SETTINGS_FONT_PICKER_SEARCH: &str = "Buscar fontes";
pub const SETTINGS_FONT_PICKER_EMPTY: &str = "Nenhuma fonte correspondente nesta máquina.";
pub const SETTINGS_FONT_PICKER_BUNDLED: &str = "inclusa";
pub const SETTINGS_FONT_PICKER_CUSTOM: &str = "Não está na lista? Digite o nome exato da família.";
pub const SETTINGS_FONT_PICKER_CUSTOM_HINT: &str = "Nome da família";
pub const SETTINGS_FONT_PICKER_USE: &str = "Usar";
pub const SETTINGS_FONT_MISSING: &str = "Não encontrada nesta máquina — usando o fallback.";

// Aba Uso (`usage`)
pub const SETTINGS_USAGE_TITLE: &str = "Seus limites de uso";
pub const SETTINGS_USAGE_WEEKLY_TITLE: &str = "Limites semanais";
pub const SETTINGS_USAGE_UNAVAILABLE: &str =
    "Limites de plano não se aplicam a este login (chave de API ou provedor terceiro).";
pub const SETTINGS_USAGE_WINDOW_SESSION: &str = "Sessão atual";
pub const SETTINGS_USAGE_WINDOW_WEEKLY: &str = "Todos os modelos";
pub const SETTINGS_USAGE_JUST_NOW: &str = "há menos de um minuto";
pub const SETTINGS_USAGE_PLAN_PRO: &str = "Pro";
pub const SETTINGS_USAGE_PLAN_MAX: &str = "Max";
pub const SETTINGS_USAGE_PLAN_TEAM: &str = "Equipe";
pub const SETTINGS_USAGE_PLAN_ENTERPRISE: &str = "Empresa";

pub fn usage_resets_in(when: &str) -> String {
    format!("Reinicia em {when}")
}
pub fn usage_resets_at(when: &str) -> String {
    format!("Reinicia {when}")
}
pub fn usage_percent_used(pct: i64) -> String {
    format!("{pct}% usado")
}
pub fn usage_updated(when: &str) -> String {
    format!("Última atualização: {when}")
}
pub fn usage_minutes_ago(n: i64) -> String {
    format!("há {n} min")
}
pub fn usage_minutes(m: i64) -> String {
    format!("{m} min")
}
pub fn usage_hours_minutes(h: i64, m: i64) -> String {
    format!("{h} h {m} min")
}
pub fn usage_plan_label(plan: &str) -> String {
    match plan.to_lowercase().as_str() {
        "pro" => String::from(SETTINGS_USAGE_PLAN_PRO),
        "max" => String::from(SETTINGS_USAGE_PLAN_MAX),
        "team" => String::from(SETTINGS_USAGE_PLAN_TEAM),
        "enterprise" => String::from(SETTINGS_USAGE_PLAN_ENTERPRISE),
        _ => plan.to_owned(),
    }
}

// Aba Recursos (`resources`)
pub const SETTINGS_RESOURCES_SKILLS_MOVED: &str = "As Skills foram movidas para ";

// Aba Memória (`memory`)
pub const SETTINGS_MEMORY_INTRO: &str = "O que uma sessão do Claude Code no workspace ativo carrega: suas instruções e a memória que o próprio Claude grava. Clique para abrir o arquivo.";
pub const SETTINGS_MEMORY_INSTRUCTIONS_TITLE: &str = "Instruções";
pub const SETTINGS_MEMORY_AUTO_TITLE: &str = "Memória automática";
pub const SETTINGS_MEMORY_EMPTY: &str = "Nenhum CLAUDE.md encontrado.";
pub const SETTINGS_MEMORY_AUTO_EMPTY: &str =
    "O Claude ainda não gravou memórias para este workspace.";
pub const SETTINGS_MEMORY_SCOPE_USER: &str = "Usuário (todos os projetos)";
pub const SETTINGS_MEMORY_SCOPE_PROJECT: &str = "Projeto";

pub fn memory_updated(when: &str) -> String {
    format!("Atualizado {when}")
}

// Aba Design Systems (`designSystems` + `appearance.themeImported`)
pub const SETTINGS_DS_TITLE: &str = "Design systems";
pub const SETTINGS_DS_DESC: &str =
    "No GSH, são os temas: cores da interface, do terminal e do código num pacote só.";
pub const SETTINGS_DS_ADD: &str = "Novo design system";
pub const SETTINGS_DS_SEARCH: &str = "Pesquisar por nome";
pub const SETTINGS_DS_COL_NAME: &str = "Nome";
pub const SETTINGS_DS_COL_ORIGIN: &str = "Origem";
pub const SETTINGS_DS_COL_AUTHOR: &str = "Autor";
pub const SETTINGS_DS_ACTIVE: &str = "Ativo";
pub const SETTINGS_DS_IMPORTED: &str = "Importado";
pub const SETTINGS_DS_BUILT_IN: &str = "Embutido";
pub const SETTINGS_DS_ACTIVATE: &str = "Usar este tema";
pub const SETTINGS_DS_REMOVE: &str = "Remover";
pub const SETTINGS_DS_IMPORT_DIALOG: &str = "Escolha um arquivo de tema";

pub fn ds_showing(n: usize, total: usize) -> String {
    format!("Exibindo {n} de {total}")
}
pub fn ds_theme_imported(name: &str) -> String {
    format!("Tema \"{name}\" importado.")
}

// Placeholders (`placeholder`)
pub const SETTINGS_PH_COWORK_1: &str = "O Cowork é um recurso do Claude Desktop (tarefas que rodam neste computador com arquivos locais, navegador e MCPs).";
pub const SETTINGS_PH_COWORK_2: &str = "No GSH, o equivalente é o chat com o Claude num workspace e a Fazenda, onde os loops de agente rodam em terminais.";
pub const SETTINGS_PH_EXTENSIONS_DESC: &str = "Permitir que o Claude interaja diretamente com aplicativos, dados e ferramentas no seu computador.";
pub const SETTINGS_PH_EXTENSIONS_EMPTY: &str = "Nenhuma extensão instalada";
pub const SETTINGS_PH_EXTENSIONS: &str = "Extensões .MCPB/.DXT são do Claude Desktop. No GSH, os servidores MCP ficam em Conectores e Desenvolvedor.";

// Aba Sistema + Armazenamento (`system` + `general.launchAtStartup*` + `storage`)
pub const SETTINGS_SYSTEM_TITLE: &str = "Sistema";
pub const SETTINGS_SYSTEM_VERSION: &str = "Versão do aplicativo";
pub const SETTINGS_SYSTEM_LAUNCH_TITLE: &str = "Iniciar ao ligar";
pub const SETTINGS_SYSTEM_LAUNCH_DESC: &str =
    "Inicia o GSH Desktop automaticamente quando você faz login no computador.";
pub const SETTINGS_SYSTEM_KEEP_AWAKE_TITLE: &str = "Manter computador ativo";
pub const SETTINGS_SYSTEM_KEEP_AWAKE_DESC: &str = "Impede a suspensão por inatividade enquanto ligado (o mesmo botão de raio do rail). Desliga ao fechar o app; fechar a tampa ainda suspende.";
pub const SETTINGS_SYSTEM_LID_SESSION_TITLE: &str = "Sessão com a tampa fechada";
pub const SETTINGS_SYSTEM_LID_SESSION_DESC: &str = "Mantém o notebook acordado no Linux mesmo com a tampa fechada por tempo definido (systemd-inhibit handle-lid-switch). Aviso: deixe o notebook em superfície ventilada — nunca dentro de mochila fechada. Encerra sozinho se a bateria cair a 20% ou menos ou se a temperatura atingir 85 °C.";
pub const SETTINGS_SYSTEM_LID_START: &str = "Iniciar sessão";
pub const SETTINGS_SYSTEM_LID_STOP: &str = "Encerrar agora";
pub fn settings_lid_active_remaining(mins: u64) -> String {
    format!("Ativa · restam {mins} min")
}
pub fn settings_lid_stopped_battery(pct: u8) -> String {
    format!("Sessão encerrada por bateria baixa ({pct}%).")
}
pub fn settings_lid_stopped_temp(temp_c: i32) -> String {
    format!("Sessão encerrada por temperatura alta ({temp_c} °C).")
}
pub const SETTINGS_STORAGE_SECTION_TITLE: &str = "Armazenamento";
pub const SETTINGS_STORAGE_LOCATION_TITLE: &str = "Local de armazenamento";
pub const SETTINGS_STORAGE_USE_DEFAULT: &str = "Usar padrão";
pub const SETTINGS_STORAGE_WORKING: &str = "Trabalhando…";
pub const SETTINGS_STORAGE_CHANGE: &str = "Alterar…";
pub const SETTINGS_STORAGE_RESET_TITLE: &str = "Redefinir o GSH Desktop";
pub const SETTINGS_STORAGE_RESET_DESC: &str = "Exclui todos os dados locais — projetos, layouts, configurações e histórico do terminal — e volta ao local padrão.";
pub const SETTINGS_STORAGE_RESET_BUTTON: &str = "Redefinir…";
pub const SETTINGS_STORAGE_RESET_CONFIRM: &str = "Redefinir";
pub const SETTINGS_STORAGE_RESET_DIALOG_TITLE: &str = "Redefinir o GSH Desktop?";
pub const SETTINGS_STORAGE_RESET_DIALOG_CONTENT: &str = "Isso exclui permanentemente todos os dados locais do GSH Desktop — projetos, layouts, configurações e histórico do terminal. Isso não pode ser desfeito. O GSH Desktop será fechado para você começar do zero.";
pub const SETTINGS_STORAGE_RESTART_TITLE: &str = "Reinicialização necessária";
pub const SETTINGS_STORAGE_RESTART_DEFAULT_MSG: &str = "O GSH Desktop usará o local padrão do sistema a partir da próxima abertura. Seus dados na pasta personalizada permanecem intactos.";
pub const SETTINGS_STORAGE_RESTART_RESET_MSG: &str =
    "Todos os dados do GSH Desktop foram apagados. Reinicie para começar do zero.";
pub const SETTINGS_STORAGE_LATER: &str = "Mais tarde";
pub const SETTINGS_STORAGE_QUIT: &str = "Sair do GSH Desktop";
pub const SETTINGS_STORAGE_CHOOSE_DIALOG: &str = "Escolha uma pasta para os dados do GSH Desktop";

pub fn storage_location_desc(root: &str) -> String {
    format!(
        "O GSH Desktop guarda seus projetos, layouts e configurações aqui. Aponte para uma pasta sincronizada para fazer backup.\n{root}"
    )
}
pub fn storage_restart_change_msg(path: &str) -> String {
    format!("O GSH Desktop usará esta pasta a partir da próxima abertura:\n{path}")
}

// Aba Desenvolvedor (`developer` + `general.developerMode*`)
pub const SETTINGS_DEV_TITLE: &str = "Desenvolvedor";
pub const SETTINGS_DEV_MCP_TITLE: &str = "Servidores MCP locais";
pub const SETTINGS_DEV_MCP_DESC: &str =
    "Servidores MCP stdio que o Claude Code sobe neste computador. Eles ficam no ~/.claude.json.";
pub const SETTINGS_DEV_MCP_EMPTY: &str = "Nenhum servidor MCP local.";
pub const SETTINGS_DEV_EDIT_CONFIG: &str = "Editar configuração";
pub const SETTINGS_DEV_COMMAND: &str = "Comando";
pub const SETTINGS_DEV_ARGUMENTS: &str = "Argumentos";
pub const SETTINGS_DEV_ERROR: &str = "Erro";
pub const SETTINGS_DEV_MODE_TITLE: &str = "Modo desenvolvedor";
pub const SETTINGS_DEV_MODE_DESC: &str = "Registra os erros e avisos do próprio GSH Desktop numa base separada da Telemetria (chip “GSH Desktop”) e liga as métricas de performance. Só serve a quem mantém o app.";
pub const SETTINGS_DEV_TELEMETRY_TITLE: &str = "Telemetria";
pub const SETTINGS_DEV_TELEMETRY_DESC: &str = "Erros e logs dos processos do workspace ficam no painel Telemetria do shell; com o modo desenvolvedor ele mostra também os do próprio GSH Desktop.";
pub const SETTINGS_DEV_CLAUDE_SETTINGS_TITLE: &str = "Arquivo de configurações do Claude Code";
pub const SETTINGS_DEV_CLAUDE_SETTINGS_DESC: &str =
    "~/.claude/settings.json — permissões, hooks, ambiente e modelo.";
pub const SETTINGS_DEV_API_KEY_TITLE: &str = "Quer uma chave de API?";
pub const SETTINGS_DEV_API_KEY_DESC: &str = "Crie chaves e veja o uso no Claude Console. Isso requer outra conta, e o uso da API é cobrado separadamente.";
pub const SETTINGS_DEV_API_KEY_BUTTON: &str = "Ir para o Console";
pub const API_KEYS_URL: &str = "https://console.anthropic.com/settings/keys";
pub const MCP_DOCS_URL: &str = "https://docs.anthropic.com/en/docs/claude-code/mcp";

// Abas-catálogo: Skills, Conectores, Plugins (`catalog`, `skills`, `connectors`, `plugins`)
pub const SETTINGS_CAT_MINE: &str = "Meus";
pub const SETTINGS_CAT_DISCOVER: &str = "Descobrir";
pub const SETTINGS_CAT_ADD: &str = "Adicionar";
pub const SETTINGS_CAT_ALL: &str = "Todos";
pub const SETTINGS_CAT_NEW_SKILL: &str = "Nova skill…";
pub const SETTINGS_CAT_ADD_MARKETPLACE: &str = "Adicionar marketplace…";
pub const SETTINGS_SKILLS_GROUP_USER: &str = "Criado por você";
pub const SETTINGS_SKILLS_GROUP_PROJECT: &str = "Do projeto";
pub const SETTINGS_SKILLS_GROUP_PLUGINS: &str = "Dos plugins";
pub const SETTINGS_SKILLS_BY_YOU: &str = "por você";
pub const SETTINGS_SKILLS_BY_PROJECT: &str = "do projeto";
pub const SETTINGS_SKILLS_EMPTY: &str = "Nenhuma skill instalada.";
pub const SETTINGS_CONN_CHECKING: &str = "Verificando cada servidor… pode levar alguns segundos.";
pub const SETTINGS_CONN_EMPTY: &str = "Nenhum servidor MCP configurado.";
pub const SETTINGS_CONN_ADD_SERVER: &str = "Adicionar servidor MCP";
pub const SETTINGS_CONN_COL_CONNECTOR: &str = "Conector";
pub const SETTINGS_CONN_COL_TYPE: &str = "Tipo";
pub const SETTINGS_CONN_COL_STATUS: &str = "Status";
pub const SETTINGS_CONN_TYPE_WEB: &str = "Web";
pub const SETTINGS_CONN_TYPE_LOCAL: &str = "Local";
pub const SETTINGS_CONN_ORIGIN_PLUGIN: &str = "Plugin";
pub const SETTINGS_CONN_LINK: &str = "Vincular";
pub const SETTINGS_CONN_DISCOVER_INTRO: &str = "O diretório de conectores do claude.ai não tem API pública, então o GSH não o lista. Adicione qualquer servidor MCP aqui: o GSH roda `claude mcp add` num terminal.";
pub const SETTINGS_CONN_FORM_NAME: &str = "Nome";
pub const SETTINGS_CONN_FORM_TYPE: &str = "Tipo";
pub const SETTINGS_CONN_FORM_URL: &str = "URL";
pub const SETTINGS_CONN_FORM_COMMAND: &str = "Comando";
pub const SETTINGS_CONN_FORM_SCOPE: &str = "Escopo";
pub const SETTINGS_CONN_FORM_SCOPE_DESC: &str =
    "Usuário = todos os projetos; Local = só o projeto do terminal.";
pub const SETTINGS_CONN_SCOPE_USER: &str = "Usuário";
pub const SETTINGS_CONN_SCOPE_LOCAL: &str = "Local";
pub const SETTINGS_CONN_DOCS: &str = "Docs de MCP";
pub const SETTINGS_CONN_ADD_IN_TERMINAL: &str = "Adicionar no terminal";
pub const SETTINGS_CONN_STATUS_CONNECTED: &str = "Conectado";
pub const SETTINGS_CONN_STATUS_NEEDS_AUTH: &str = "Precisa autenticar";
pub const SETTINGS_CONN_STATUS_FAILED: &str = "Falhou";
pub const SETTINGS_CONN_STATUS_PENDING: &str = "Aprovação pendente";
pub const SETTINGS_CONN_STATUS_UNKNOWN: &str = "Desconhecido";
pub const SETTINGS_PLUGINS_SEARCH: &str = "Pesquisar skills e plugins";
pub const SETTINGS_PLUGINS_GROUP_PROJECT: &str = "Neste projeto";
pub const SETTINGS_PLUGINS_GROUP_MARKETPLACES: &str = "Dos marketplaces que você adicionou";
pub const SETTINGS_PLUGINS_NO_WORKSPACE: &str =
    "Abra um workspace para ver os plugins habilitados nele.";
pub const SETTINGS_PLUGINS_PROJECT_EMPTY: &str =
    "Nenhum plugin habilitado no .claude/settings.json deste projeto.";
pub const SETTINGS_PLUGINS_EMPTY: &str = "Nenhum plugin instalado.";
pub const SETTINGS_PLUGINS_DISABLED: &str = "Desabilitado";

pub fn catalog_by(author: &str) -> String {
    format!("por {author}")
}
pub fn connectors_from_plugin(plugin: &str) -> String {
    format!("Fornecido pelo plugin {plugin}")
}

/// Frase do erro da CLI (o stderr entra cru como detalhe).
pub fn claude_info_error(error: &crate::settings::claude::cli::InfoError) -> String {
    use crate::settings::claude::cli::InfoError;
    match error {
        InfoError::CliMissing => {
            String::from("CLI do Claude Code (`claude`) não encontrada neste computador.")
        }
        InfoError::Timeout => String::from("A CLI `claude` demorou demais para responder."),
        InfoError::Failed(detail) => format!(
            "A CLI `claude` falhou: {}",
            detail.as_deref().unwrap_or("—")
        ),
    }
}

pub fn settings_save_failed(detail: &str) -> String {
    format!("Não consegui salvar: {detail}")
}

// Telemetria (`pt_BR.i18n.json` -> `gsh.telemetry`)
pub const TELEMETRY_TOOLTIP: &str = "Telemetria";
pub const TELEMETRY_TITLE: &str = "Telemetria";
pub fn telemetry_live_runs(n: usize) -> String {
    format!("{n} ativos")
}
pub const TELEMETRY_NO_WORKSPACE: &str = "Abra um workspace para ver a telemetria dele.";
pub const TELEMETRY_EMPTY: &str = "Nada aqui ainda.";
pub const TELEMETRY_EMPTY_FILTERED: &str = "Nada aqui ainda.";
pub const TELEMETRY_SEARCH_HINT: &str = "Filtrar casos";
pub const TELEMETRY_CHIP_OPEN: &str = "Abertos";
pub const TELEMETRY_CHIP_NEW: &str = "Novos";
pub const TELEMETRY_CHIP_RESOLVED: &str = "Resolvidos";
pub const TELEMETRY_CHIP_IGNORED: &str = "Ignorados";
pub const TELEMETRY_CHIP_WARNINGS: &str = "Avisos";
pub const TELEMETRY_CHIP_WARNINGS_WITH_ICON: &str = "⚠ Avisos";
pub const TELEMETRY_CHIP_APP: &str = "GSH Desktop";
pub const TELEMETRY_TAG_NEW: &str = "novo";
pub const TELEMETRY_TAG_REGRESSION: &str = "regressão";
pub const TELEMETRY_TAG_RESOLVED: &str = "resolvido";
pub const TELEMETRY_TAG_IGNORED: &str = "ignorado";
pub const TELEMETRY_SRC_TASK: &str = "task";
pub const TELEMETRY_SRC_WRAPPER: &str = "gsh telemetry";
pub const TELEMETRY_SRC_APP: &str = "App";
pub fn telemetry_run(id: &str) -> String {
    format!("run {id}")
}
pub const TELEMETRY_RESOLVE: &str = "Marcar resolvido";
pub const TELEMETRY_IGNORE: &str = "Ignorar (some para os agentes também)";
pub const TELEMETRY_REOPEN: &str = "Reabrir";
pub const TELEMETRY_CLEAR: &str = "Limpar ocorrências";
pub const TELEMETRY_CLEAR_RUN: &str = "Limpar este run";
pub const TELEMETRY_CLEAR_PROJECT: &str = "Limpar projeto";
pub fn telemetry_open_file(location: &str) -> String {
    format!("Abrir {location}")
}
pub const TELEMETRY_SHOW_IN_TERMINAL: &str = "Ver no terminal";
pub const TELEMETRY_COPY_COMMAND: &str = "Copiar comando da CLI";
pub const TELEMETRY_COPIED: &str = "Copiado";
pub const TELEMETRY_STATUS_OPEN: &str = "Aberto";
pub const TELEMETRY_STATUS_RESOLVED: &str = "Resolvido";
pub const TELEMETRY_STATUS_IGNORED: &str = "Ignorado";
pub const TELEMETRY_OCCURRENCES: &str = "Ocorrências";
pub fn telemetry_in_runs(n: usize) -> String {
    format!("em {n} runs")
}
pub const TELEMETRY_FIRST: &str = "Primeira";
pub const TELEMETRY_LAST: &str = "Última";
pub const TELEMETRY_ORIGIN: &str = "Origem";
pub const TELEMETRY_SECTION_STACK: &str = "Stack";
pub const TELEMETRY_STACK_HINT: &str = "frames do projeto em destaque · clique abre o arquivo";
pub const TELEMETRY_SECTION_CORRELATED: &str = "Log correlacionado";
pub const TELEMETRY_CORRELATED_HINT: &str = "o JSON mais próximo antes do erro, no mesmo run";
pub const TELEMETRY_NONE: &str = "nenhum";
pub const TELEMETRY_SECTION_RUNS: &str = "Ocorrências por run";
pub const TELEMETRY_RUNS_HINT: &str =
    "mesma chave (cwd, comando): é assim que novo e regressão são calculados";
pub const TELEMETRY_SECTION_CONTEXT: &str = "Contexto cru";
pub const TELEMETRY_CONTEXT_HINT: &str = "linhas do terminal ao redor da última ocorrência";
pub const TELEMETRY_CURRENT: &str = "atual";
pub fn telemetry_fingerprint_hint(location: &str) -> String {
    format!("fingerprint = tipo + mensagem normalizada + {location}")
}
pub const TELEMETRY_BLAME_UNCOMMITTED: &str = "alterado no working tree";
pub fn telemetry_blame_commit(ago: &str, sha: &str) -> String {
    format!("alterado {ago} ({sha})")
}
pub const TELEMETRY_JUST_NOW: &str = "agora";
pub fn telemetry_minutes_ago(n: i64) -> String {
    format!("há {n} min")
}
pub fn telemetry_hours_ago(n: i64) -> String {
    format!("há {n} h")
}
pub fn telemetry_days_ago(n: i64) -> String {
    format!("há {n} d")
}
pub fn telemetry_case_tab_title(err_type: &str, file: &str) -> String {
    format!("{err_type} · {file}")
}
pub const TELEMETRY_PTY: &str = "PTY";
pub const TELEMETRY_STDOUT: &str = "stdout";
pub const TELEMETRY_DASH: &str = "—";
pub const TELEMETRY_EMPTY_SET: &str = "∅";
pub const TELEMETRY_CRUMB_SEP: &str = " › ";
pub const TELEMETRY_CRUMB_DOT: &str = "  ·  ";
