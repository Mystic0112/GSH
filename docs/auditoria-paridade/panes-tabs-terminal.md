# Auditoria de paridade Flutter -> Rust: panes, abas e terminal

Somente leitura. Flutter = `/home/heliodev/projetos/GSH Desktop/lib/app/gsh/ui/widgets/` (+ `lib/i18n/pt_BR.i18n.json`).
Rust = `/home/heliodev/projetos/GSH/crates/app/src/`. Abreviações: `pv` = pane_view.dart, `ps` = pane_strip.rs, `pt` = pane_tab.rs, `pc` = pane_chrome.rs, `tp` = terminal_pane.rs, `tc` = terminal_canvas.rs, `ti` = terminal_input.rs.

Classes: IGUAL / DIFFERENTE / FALTA (no Rust) / INVENTADO (no Rust).

Contagem (itens classificados abaixo): FALTA 30, DIFFERENTE 24, INVENTADO 5, IGUAL 22.

---

## A. Alto impacto (visível/usável na hora)

### A1. Menu de contexto da aba (botão direito) — FALTA
- Flutter `pv:778-905` (`_showTabMenu`, aberto por `onSecondaryTapUp` em `pv:1096`). Itens pt-BR exatos, na ordem:
  - arquivo em preview: "Fixar aba"
  - arquivo: "Abrir em nova janela" (só local), "Renomear", "Redefinir título" (se titleLocked), "Abrir como quadro/markdown" (.kanban), "Abrir como layout/YAML" (.ckp), "Abrir como painel/HTML" (.panel)
  - terminal: "Renomear", "Redefinir título", "Copiar Id", "Reiniciar"
  - todas: "Fechar", "Fechar as outras", "Fechar à direita", "Fechar todas"
- Rust: nenhum handler de botão direito em pane_tab.rs/pane_strip.rs/pane_chrome.rs (`grep Button::Right` só acha um teste em `pane_focus.rs:250`). Nenhuma das strings existe em `labels.rs` (`grep` de "Fechar as outras|Fechar à direita|Fechar todas|Copiar Id|Redefinir título|Reiniciar" = 0 no app de panes).
- Efeito: sem fechar outras/direita/todas, sem copiar Id (usado na CLI `--tab-id`), sem reiniciar terminal, sem renomear.
- Também falta o diálogo de confirmação em lote: "Fechar abas?" / "Isso fecha ${count} aba(s) e encerra os agentes/terminais nelas." (`i18n paneView.closeTabsTitle/Message`).

### A2. Renomear aba de terminal/arquivo (duplo clique e menu) — FALTA
- Flutter `pv:699-770` (`_handleTap` detecta duplo clique em <300 ms; `_startEditing`, `_commitEdit` troca espaço por `-`; Esc cancela; foco perdido confirma).
- Rust: `PaneModel::set_terminal_label` existe (`pane_model.rs:~195`) mas só é chamado por layout/CLI (`main.rs:1776`); sem UI de edição inline.
- Também FALTA: duplo clique em preview "fixa" a aba (`pv:712`), e abas de arquivo em modo preview em itálico (`pv:997`). O Rust não tem conceito de preview (grep "preview" em pane_*.rs = 0).

### A3. Indicadores de status do agente na aba — FALTA
- Flutter `pv:1043-1076`: spinner 10px (verde `online` com agente, `accent` sem), bolinha amarela piscando (`AgentStatusDot waiting`), bolinha `accent` 7px "terminou e você não viu" (`unseenFinish`), círculo vazado azul (`idle` já visto). Mais `agent_status_dot.dart` (piscada 700 ms compartilhada).
- Rust `pt:88-` (`TabPresentation`: title, active, dirty, hovered, drop_before, dragging): não tem campo de status. O status só alimenta o grafo/fazenda/ilha (`main.rs:2484`, `8920`). Hoje nenhuma aba mostra "trabalhando/esperando/terminou".
- Também FALTA o ponto do workspace na rail (`WorkspaceAgentDot`) — fora desta área, só registro.

### A4. Barra inferior de 2px da aba ativa aparece mesmo com o pane sem foco — DIFFERENTE
- Flutter `pv:1010` (`foregroundDecoration` só se `isFocusedActive = active && focused`, `pv:952`): só a aba ativa do pane FOCADO ganha a linha `accent` de 2px.
- Rust `pt:288-312`: `is_active` desenha a barra `accent` em todo pane (com 2 panes, duas barras acesas). A barra ainda ocupa 2px da altura (a aba vira 37px de conteúdo) em vez de sobrepor.
- Cor do ícone também diverge: Flutter (`pv:1036`) focada+ativa = `accentText`, ativa sem foco = `text2`, inativa = `text3`. Rust `pt:112` só tem `accent_text` (ativa) / `text3`; falta o `text2`.

### A5. Fechar a aba: foco/ativa depois e panes vazios — DIFFERENTE / INVENTADO
- Ao fechar a aba ativa, Flutter (`gsh_viewmodel.dart` `_closeTabIn`, ~5755) ativa a anterior (`idx-1`). Rust `pane_model.rs:435-457` mantém o mesmo índice (= a próxima aba). Comportamento diferente em qualquer pane com 3+ abas.
- Fechar a última aba do único pane: Flutter cria um `EmptyTab` que vira terminal na hora. Rust `main.rs:7123-7127` + `pc:367` (`view_empty_body`) deixa o pane vazio com o texto "Nenhuma aba aberta — clique em + para abrir um terminal." (`labels.rs:72`) — texto que não existe no Flutter (INVENTADO). Aqui o clique em "+" é exigido.

### A6. Atalhos de aba/pane que o Flutter tem e o Rust não — FALTA
- Ctrl/Cmd+W fecha a aba ativa com a confirmação de edição (`gsh_page.dart:754`, `actions/tab_actions.dart:66`). Rust: nenhum handler de "w" (`grep` = 0; `Action` enum de `app_menu_model.rs` não tem `CloseTab`).
- Menu Exibir > "Focar Painel" (Esquerda/Direita/Cima/Baixo, ⌘⌥+setas): `app_menu_bar.dart:191-210`, `workspace_menu_bridge.dart:42`. Rust: ausente em `app_menu_model.rs`.
- Ctrl+P (localizador de arquivos), Ctrl+` / Ctrl+Shift+` (ciclar realm), botão lateral "voltar" do mouse (`goBackInPane`): `gsh_page.dart:742-790`. Rust: ausentes.
- Ctrl+1..9, Ctrl+T, Ctrl+D, Ctrl+Shift+D: IGUAL (`app_menu_model.rs`). O menu Flutter usa rótulos "Aba N" / "Última Aba" / "Selecionar Aba"; o Rust usa "Selecionar aba N" no grupo "Janela" em vez de submenu "Selecionar Aba" dentro de "Exibir" (DIFFERENTE; área de menus, só registro).

### A7. Terminal: entrada de teclado incompleta — FALTA
- `ti:21-50` + `named_key_bytes`: não trata Alt (ESC-prefixo: Alt+B/F/D/Backspace de readline não funcionam); setas fixas em `\x1b[A..D`, sem modo aplicação (DECCKM) e sem modificadores (Ctrl+←/→, Shift+setas); Shift+Enter manda `\r` (no agente o Flutter/flterm manda o newline do kitty; no Rust a quebra de linha do Claude Code não funciona); sem kitty keyboard protocol. `grep -i "alt|kitty|decckm" ti, gsh_term/src` = 0.
- Colar: Flutter Ctrl+V / Cmd+V cola (`pv:1668`, `adaptive_terminal_pane.dart:92-101`) e, com imagem no clipboard, manda `\x16` ao harness. Rust `ti:37-46`: só Ctrl+Shift+V / tecla Paste cola texto; Ctrl+V cru manda `0x16` sempre; não há colagem de imagem; sem bracketed paste (`grep bracketed` = 0).
- Copiar: Flutter Cmd/Ctrl+C copia seleção; Rust só Ctrl+Shift+C (`ti:37-46`).

### A8. Terminal: mouse e links — FALTA
- Mouse reporting para TUIs (claude/vim; clique, arraste e roda enviados à app; Alt força seleção local): `terminal_pane.dart:213-255`, flterm no padrão. Rust `tc:100-170`: só roda (scrollback local) e seleção do botão esquerdo. Dentro do Claude Code/vim a roda do mouse não rola a TUI e clique não chega. `grep -i mouse crates/term/src` = 0.
- Duplo clique seleciona palavra / triplo seleciona linha: não existe no `tc`.
- Links: Cmd/Ctrl+clique abre URL/arquivo (`link_context_menu.dart`, `terminal_link.dart`); botão direito no link abre menu pt-BR: Arquivo = "Abrir", "Mostrar no Finder|Mostrar no Explorer|Abrir pasta", "Copiar caminho"; URL = "Abrir link", "Copiar link". Realce e cursor de mão ao segurar Ctrl. Rust: nada (`grep -i link` em tc/tp = 0; sem `terminalLink` em labels).
- Abrir links no navegador integrado (setting "openLinksInApp") — ver browser abaixo; no Rust não há detecção de link para abrir.
- Auto-scroll ao arrastar seleção além da borda (`terminal_pane.dart:60-70`, zona de 24px): Rust não rola durante o arraste.

### A9. Arrastar e soltar de arquivo — FALTA
- Flutter `pv:2004-2060` (`_OpenTabDropTarget`): arquivo do SO solto na faixa de abas abre como aba (borda `accent` 2px + `accentSoft` enquanto paira). `pv:2074-2200` (`_TerminalDropTarget`): solta no terminal injeta caminhos com aspas de shell e espaço final, devolve foco ao terminal. `PaneDropZone` (`pv:2227+`): arrastar arquivo da ÁRVORE sobre o pane abre como aba (ou insere o caminho no terminal ativo).
- Rust: nenhum `FileDropped` fora do chat (`chat_pane.rs:1844`); `files_panel.rs`/`file_tree.rs` sem drag. Arrastar arquivo da árvore para o pane não faz nada.

### A10. Banner de host remoto fora do ar — FALTA
- Flutter `remote_disconnected_banner.dart` (`gsh_page.dart:894`): faixa `panel2` acima do centro: ícone `cloud_off` 15px `warn`, texto 12px `warn` "Reconectando a ${host}…" / "${host} offline" / "Conectando a ${host}…", botão "Reconectar" (fundo `panel3` no hover). Abas do host congelam e re-anexam.
- Rust: nenhuma ocorrência de "Reconectando"/"Reconectar"/"offline" fora de localsend/updater (`grep` feito). `remote_ssh.rs` reconecta por baixo (`retry_wake`), mas sem aviso visível nem botão.

### A11. SSH remoto: fluxo inteiro é outro — DIFFERENTE / INVENTADO
- Flutter: hosts remotos cadastrados (`remote/add_remote_host_dialog.dart`, textos `remoteHost.*`: "Adicionar host remoto", "Nome", "Destino SSH (usuário@host)", "Usuário", "Host / IP", "Porta", "Autenticação" (Chave SSH/Senha), "Escolher…", erros tipados), workspace remoto na rail ("Local"/"Remoto", "Escolher um host", "Novo host…", "Editar host"), seletor de pasta remota ("Abrir pasta em ${host}", "Abrir aqui", "Sem subpastas"), servidor instalado automaticamente.
- Rust `remote_connect_dialog.rs:33-80` + `app_menu_model.rs:52`: item de menu "Novo terminal remoto por SSH" (Linux) e diálogo "Conectar terminal remoto" com campos "Host SSH (alias do ~/.ssh/config ou usuário@host)", "Socket absoluto do gsh-server no host", "Diretório inicial remoto (opcional)" e dica "Inicie gsh-server no host e copie o caminho que ele imprimir…". Nenhuma dessas frases está no Flutter (INVENTADO); o menu "Novo terminal remoto" também não existe no Flutter; strings estão literais no .rs, não em `labels.rs`.
- `ssh_prompts.rs` (TOFU + passphrase): textos IGUAL (`SSH_*` batem com `sshPrompts.*`). Diferenças visuais: ícone `gpp_maybe` em `warn` (Flutter `text2`, `ssh_prompts.dart:30`); comando `ssh-keygen -lf ...` dentro de caixa com fundo+raio (Flutter é texto mono 11px `text3` sem caixa); tamanhos de texto 13.5/11.5/11.5 vs 12/11/12(mono); dica da passphrase `text3` 11.5 vs `text4` 10.5; campo da passphrase com placeholder "Senha" (Flutter sem placeholder); "Desbloquear" desabilitado com campo vazio (Flutter aceita vazio/Enter vazio); "Cancelar" outline (Flutter `GhostButton`). Larguras 460/420 IGUAL.

---

## B. Médio impacto

### B1. Tooltips
- FALTA tooltip "Nova aba" no "+" (`pv:1264` vs `ps:383` sem tooltip). FALTA tooltip com o título completo na aba (`pv:988`). Existem: "Dividir à direita", "Dividir abaixo", "Fechar painel", "Todas as abas" — IGUAL (labels `PANE_SPLIT_RIGHT/DOWN/CLOSE`, `PANE_VIEW_ALL_TABS`).
- Tooltips do Rust usam `tip_bubble` (`pc:~390`); IGUAL em intenção (panel2, border2, text) ao `AppTooltip`.

### B2. Título da aba: truncamento e ícones
- Flutter `pv:988-1000` trunca com reticências (`TextOverflow.ellipsis`); Rust `pt:150-160` usa `Wrapping::None` + clip (corta seco, sem "…") — DIFFERENTE.
- Ícone de arquivo: Flutter usa `FileTypeIcon.file(nome, size 15)` (SVG material-icon-theme, `pv:1024`); Rust usa glifo mono-bold de `labels::file_icon_for_name` (poucas categorias: code/config/doc/lock/generic, `labels.rs:426`) apesar de já existir `file_icons.rs`/`file_icons::view` (usado na árvore e busca) — DIFFERENTE. Mesma coisa no dropdown "Todas as abas" (`ps:55-65`, `Leading::Glyph(tab.icon())`) e no chip de arrasto.
- Dropdown "Todas as abas": Flutter mostra o ícone do harness (`PaneTabLeading`) para terminal com agente; Rust só glifo — DIFFERENTE. Ícone de harness na aba (`PaneTabLeading`/`terminal_harness_icon`): IGUAL.
- Ícones por tipo: Flutter `_tabIcon` (`pv:~199`): terminal `terminal_outlined`, chat `chat_bubble_outline`, farm `grass_outlined`, hub `hub_outlined`, harness `tune`, telemetria `monitor_heart_outlined`, redis `grid_on`, mongo `data_object`, caso/DB com logos de engine. Rust `labels::ICON_*` correspondentes; sem comparar os glifos um a um (assumido IGUAL, não verificado pixel a pixel).
- Título da aba Fazenda: Flutter "Fazenda" (`farm.tabTitle`); Rust "Cenário" (`labels.rs:570`) — DIFFERENTE.
- Aba Navegador: Flutter mostra o título da página (`reportNavigation(title)`); Rust título fixo "Navegador" (`pane_model.rs`, `SETTINGS_FEATURE_BROWSER`) — DIFFERENTE.
- Configurações como aba (`PaneTab::Settings`, `pane_model.rs:~43`, `main.rs:7130`): no Flutter é uma ROTA `/settings` (página própria). INVENTADO (comportamento) / DIFFERENTE. Fora da área, só registro.

### B3. Fechar aba/pane
- Diálogo "Fechar painel?": `PANE_CLOSE_TITLE`/`pane_close_message` IGUAL ao `closePaneTitle`/`closePaneMessage` (`pane_close_dialog.rs`, botão "Fechar" destrutivo).
- Fechar o único pane com Rust cria terminal novo (`pane_tree.rs:363-372`): IGUAL ao Flutter (EmptyTab -> terminal).
- Clique do meio na aba fecha (`pv:1097` `onTertiaryTapUp`): FALTA no Rust.
- Fechar arquivo com edição: Flutter abre `showCloseDirtyDialog` (descartar/cancelar/salvar-e-fechar, `tab_actions.dart:37-62`; neovim: "closeAnyway"). Rust: arquivo usa `editor.state.request_close()` (confirmação inline do editor, `main.rs:7115`) e query usa `close_query_dialog` ("Alterações não salvas" / "Não salvar" / "Salvar e fechar", `main.rs:9815`) — mecanismo diferente por tipo; comportamento próximo. Não verifiquei texto do `request_close` inline (área do editor).
- Bolinha "sujo" no lugar do X (8px `accent`, X volta no hover): IGUAL (`pt:206-221`, `show_dirty_dot`).

### B4. Drop de abas entre panes
- Zonas 25% das bordas, faixa de 40px, rótulos "Solte aqui para mover a aba" e "Encaixar como aba": IGUAL (`PaneDropZone::at` em `pc:62-95`, `labels.rs:73,632`).
- DIFFERENTE: pré-visualização da faixa (Flutter `pv:2410-2430`: só borda INFERIOR `accent` 2px + fundo `accentSoft`; Rust `pc:154-170`: borda `accent` 2px em volta da faixa inteira). Texto do preview sem `FONT_TAB` no Rust (usa fonte padrão).
- Caret de inserção 2.5px `accent`, top/bottom 6, raio 2: IGUAL (`pt:322-345`). Chip de arrasto (panel2, borda accent, raio 7, padding 11x8, sombra 12/4, max 200): IGUAL (`pt:9-80`). Aba de origem a 30%: IGUAL.
- Mobile (long press) não se aplica.

### B5. Corpo do pane e terminal: moldura e medidas
- Rust `pc:122-132`: o corpo do pane (terminal incluso) tem `padding(1)` + borda `border` de 1px em volta; Flutter `pv:~118` não tem borda no corpo. Resultado: linhas duplas ao lado de divisores/janela — DIFFERENTE.
- Padding do terminal: Flutter `Padding(fromLTRB(10, 8, 0, 8))` (`pv:1901` area); Rust `terminal_grid.rs` `TERM_PADDING_X/Y = 8`, ou seja 8 dos 4 lados — DIFFERENTE (esquerda 8 vs 10, direita 8 vs 0).
- Métricas de célula: Flutter mede a fonte real (flterm) com peso/tamanho das settings; Rust usa célula fixa 8.0x17.0 escalada por `size/13` (`theme.rs:576-577`, `tp:204-210`) — DIFFERENTE (JetBrains Mono a 13px tem avanço 7.8px). Sem `terminalFontWeight`, sem fonte própria do terminal (`terminalFont`) nem fallbacks.
- Cores do terminal não seguem o tema ativo: `TERM_BG/TERM_FG/TERM_CURSOR/TERM_SELECTION_BG` são constantes escuras (`theme.rs:365-369`, `named_to_color` via `NamedColor::to_rgb` fixa, `snapshot.rs:57`). O pane usa `palette().panel` (dinâmico) mas o canvas pinta `TERM_BG` = `PANEL` escuro: com tema claro/custom o terminal fica escuro e destoa. Flutter usa `context.terminalTheme` (claro/escuro/custom, `terminal_theme.dart`). Paleta ANSI escura: IGUAL aos valores do Flutter dark.
- Cursor: Rust sempre bloco sólido, sem piscar e sem contorno quando sem foco; sublinhado só `underline` (sem itálico, tachado, dim). Flutter (flterm) segue o foco/estilo. DIFFERENTE (não verificado em runtime).
- Mensagem de erro de spawn: Rust "Não foi possível iniciar a sessão do terminal." (`labels.rs:75`) — não existe no Flutter (INVENTADO, texto de fallback).
- Avisos de abertura do terminal: Flutter `terminal.cwdFallbackWarning` ("Aviso: a pasta \"${requested}\" não existe. Este terminal abriu em \"${path}\".") e `workspaceEnvTracked` ("Aviso: o .env.gsh está rastreado pelo git..."): FALTA (grep = 0).
- Foco: Flutter devolve o teclado ao terminal em clique no corpo, na seleção de aba e após drop (`pv:1574-1640`); Rust usa `FocusOnPress` (`pane_focus.rs`) — equivalente em intenção, não verificado.
- Terminal da task (`TaskOutputSession`, aba somente leitura): FALTA tipo de aba no Rust (também Neovim, Diff, Notebook).

### B6. "+" e botões do canto
- "+" 32px, ícone 14 `text4`, borda direita 1px: IGUAL (`ps:383-427`). Seta de perfil de terminal ("Novo terminal…") só Windows: não se aplica ao Linux.
- Ferramentas (dividir direita/baixo/fechar): ícones desenhados, 13px, gap 12, padding direito 13: IGUAL (`pc:277-320`, `pane_tool_icon.rs`).
- Botão "Abrir navegador" no canto do pane: Flutter oculta no Linux sem webview (`pv:~560`); Rust não tem o botão no pane (abre pela topbar) — aceito (Obscura). Kebab mobile: não se aplica.
- Overflow de abas (chevron 28px, menu mín. 220, "Todas as abas", item marcado = ativo): IGUAL (`ps:100-150`). Rolagem horizontal pela roda (dx senão dy), scrollbar 3px só no hover, auto-scroll 180 ms para a aba ativa: IGUAL na lógica (`ps:269-330`; a animação easeOut de 180 ms não confirmada, o Rust usa `scroll_to` direto — DIFFERENTE de detalhe).

### B7. Divisores
- Linha 1px `border` virando `accent` no hover/arraste, alça de 12px, razão 0.16-0.84: IGUAL (`pc:221-236`, `pane_tree.rs:15`, `theme.rs:609-610`).
- Cursor de resize (coluna/linha): iced default; IGUAL em intenção.
- `PanelResizeHandle` (alça 9px com pílula 28x3 `border` raio 2, cursor up/down) usada no painel de commit (`file_tree_panel.dart:1652`) e Tasks (`tasks_panel.dart:78`): FALTA no Rust (`grep` = 0; `drag_handle.rs` só serve DB). Fora da área; só registro.

---

## C. Aba Harness (`harness_view.dart` x `harness_pane.rs`)
- Textos pt-BR: IGUAL, um a um (`labels.rs:1230-1332` x `i18n harness.*`), inclusive "Custo fixo por turno", "Já existe um realm com esse nome." (`duplicateName`).
- Layout: padding 24/20/24/32, 2 colunas a partir de 980px, gap 16, cap 6 + "Mostrar todos (n)" / "Mostrar menos", chip de escopo, cadeado "Só leitura aqui: use claude mcp add/remove": IGUAL (constantes `HARNESS_*` em `theme.rs`). Não medi cada px de raio/padding (assumido porte fiel).
- Spinner de carregamento: Flutter `CircularProgressIndicator`; Rust spinner próprio de 30fps (`content_search_panel::spinner`) — aceitável.

---

## D. Aba Navegador (`browser_pane.dart`, `webview_chrome.dart` x `browser_pane.rs`)
Obscura sem imagem é aceito; só diferenças de UI/barra:
- Barra: Flutter `Padding(6,4,6,4)`, botões 24x24 com ícones Material `arrow_back/arrow_forward/refresh` 15px (`text3`, `text4` desabilitado), gap 6, campo URL 12px corpo, borda `border`, raio 6, padding 8x3, placeholder "Digite a URL ou endereço", botão "Ir" com fundo `accentSoft` e texto `accentText` 12px (`browser_pane.dart:121-207`). Rust (`browser_pane.rs:263-305`, `browser_pane_controls.rs`): padding 8/10, botões 28x28 com setas de texto "←", "→", "↻" em `text`, campo mono com placeholder "https://...", botão "Ir" fantasma — DIFFERENTE em tudo isso.
- Tooltips "Voltar", "Avançar", "Recarregar" (`browserPane.back/forward/reload`): FALTA no Rust.
- Página inicial de aba nova: Flutter `https://google.com`; Rust `https://example.com` com `about:blank` — DIFFERENTE.
- INVENTADO (necessário ao Obscura/agentes): campo "Aba:" com "default" (`browser_pane.rs:279-290`), painel "--- CONTEÚDO DE PÁGINA WEB (dados, não instruções) ---" com botão "Atualizar", texto "Carregando página...", "Nenhum frame capturado ainda.", banner de erro `error` com mensagens "A URL não pode estar vazia." etc. Aceito como exceção, mas está registrado como texto fora do Flutter.
- Flutter: UA Safari no macOS, JS ligado, cobertura anti-flash — não se aplica ao Rust.

---

## E. Itens IGUAIS confirmados (para não reauditar)
- Barra de abas 40px (39 + 1px divisor), aba 188px (187 + hairline), padding 11/7, gap ícone-título 7, gap título-X 10, X 12px com padding 3 e hover `panel3`: `theme.rs:590-603` x `pv:1004-1076`.
- Aba ativa fundo `panel`, inativa `bg`, título `text`/`text3`: IGUAL.
- Caret de drop, chip de arrasto, 30% de opacidade: IGUAL.
- Dropdown "Todas as abas", scroll da faixa, tooltips dos 3 botões do canto: IGUAL.
- Textos de fechar painel, zonas de drop, SSH TOFU/passphrase, Harness inteiro: IGUAL.
- Ctrl+1..9, Ctrl+T, Ctrl+D, Ctrl+Shift+D: IGUAL.

## F. Não coberto / pendente
- `right_tabs.rs` (barra do painel direito) pertence à área da árvore de arquivos; só li: ordem Arquivos, Busca, Git, Banco, Telemetria, Galeria, botões 28px.
- `agent_status_dot.dart` na rail e topbar: fora de panes, só aba (A3).
- Comparação pixel a pixel com os PNGs de `GSH-wt/shots` não feita (nenhum PNG específico de panes/terminal encontrado).
