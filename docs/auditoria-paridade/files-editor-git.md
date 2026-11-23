# Auditoria de paridade Flutter -> Rust: arquivos, editor, viewers, busca e git

Somente leitura. Flutter (verdade): `/home/heliodev/projetos/GSH Desktop/lib/app/gsh/ui/widgets/` e `lib/i18n/pt_BR.i18n.json`. Rust: `/home/heliodev/projetos/GSH/crates/app/src/`.
Abreviações: `FTP` = `file_tree_panel.dart`; `FV` = `file_viewer.dart`. Todo FALTA abaixo foi conferido por grep em `crates/` (kanban só aparece em gallery/labels/file_icons; zero hits para notebook_view, panel_view, layout_preview, diff_viewer, MediaView, breadcrumb, find bar, quick open, overview ruler, scm gutter, go-to-definition, semantic tokens, comment toggle).

Legenda: IGUAL / DIFERENTE / FALTA (no Rust) / INVENTADO (no Rust, inexistente no Flutter).

---

## A. Ordenado por impacto visível (do mais grave ao menos)

### A1. FALTA: viewers `.kanban` e `.notebook` (a Galeria cria arquivos que o app não sabe abrir)
- Flutter: `kanban_board_view.dart` (2742 linhas: quadro/lista, colunas, cards, marcadores, dependências, filtro, comentários, arrastar) e `notebook_view.dart` (1887 linhas: lista de notas, tags, backlinks `[[…]]`, barra de formatação, busca). Textos: `gsh.kanbanView.*` (ex.: "Novo card", "Nova coluna", "Apagar coluna", "Sem cards", "Este arquivo ainda não tem colunas ## — ele abre como markdown.", "Começar um quadro") e `gsh.notebook.*` ("Nova nota", "Buscar notas", "Nenhuma nota ainda. Crie uma, ou peça pro agente escrever aqui.", "Selecione uma nota").
- Rust: nada. `gallery_template.rs:11-28` e `labels.rs:1186-1205` criam "Quadro kanban" e "Caderno", mas `open_in_editor` (`main.rs:3884-3907`) abre tudo como texto no editor cru. Confirma o checklist (kanban e notebook).
- Também afeta "Abrir como markdown" (menu de contexto de `.kanban`/`.panel`, FTP:2318).

### A2. FALTA: menu de contexto e operações da árvore de arquivos
- Flutter: `_showNodeMenu` (FTP:2295-2440), minWidth 220. Arquivo: Abrir, Abrir com, Abrir em nova janela, Abrir como markdown/HTML (só `.kanban`/`.panel`), Abrir layout (só `.ckp`), Mostrar diff do git (desabilitado sem mudança). Pasta: Novo arquivo, Nova pasta, Abrir no terminal, "Abrir no <agente>" por harness instalado, Abrir no gerenciador de arquivos ("Abrir no Finder/Explorer/gerenciador de arquivos"). Comum: Renomear, Excluir, Copiar, Recortar, Colar, Copiar caminho relativo, Copiar caminho absoluto. Também há menu na área vazia da árvore (alvo = raiz).
- Rust: `files_panel.rs:21-37` (`FilesMsg`) não tem nenhum item de contexto; grep de `on_right|context_menu` em `app/src` não acha nada na árvore.
- FALTA junto: renomear inline (F2 no Linux, FTP:1046-1090; campo pré-seleciona o nome sem extensão), excluir (diálogos "Excluir?", "Mover “x” para a Lixeira?", "Excluir “x” permanentemente? Isso não pode ser desfeito.", "Não foi possível excluir"), copiar/recortar/colar (Ctrl+C/X/V; "Não foi possível colar"), arrastar para mover ("Mover?", "Mover “x” para “y”?", "Não foi possível mover"), drop nativo do SO ("Copiar aqui?", "Copiar N itens para “x”?"), clique em área vazia desseleciona.

### A3. DIFERENTE: criar arquivo/pasta e tooltips do cabeçalho
- Flutter: botões do cabeçalho criam na pasta/arquivo selecionado (senão raiz), com linha-input inline dentro da árvore, indentada na profundidade certa (`_InlineEntry`, FTP:2660). Tooltips "Novo arquivo", "Nova pasta", "Recolher todas as pastas", "Atualizar" (FTP:1260-1290).
- Rust `files_panel.rs:88-142`: sempre na raiz, `text_input` solto acima da árvore (placeholder "Nome da pasta"/"Nome do arquivo", `labels.rs:196-197`), mensagens de erro próprias (`labels.rs:198-201`). `header_action` (`files_panel.rs:94-115`) não usa `tooltip`: os quatro botões do cabeçalho ficam sem tooltip (só o filtro de agente tem). Ordem dos botões IGUAL à do Flutter.

### A4. FALTA: barra de rodapé do viewer (breadcrumb + switch Pré-visualização/Código-fonte + ponto de sujo + recarregar)
- Flutter `_Toolbar` (FV:1014-1098): 34px, fundo `bg`, borda-topo, `FilePathBreadcrumb` à esquerda (chevron_right 14, ícone do arquivo 13 no último segmento, 12px, último `text2` e demais `text4`), ponto `accent` 7x7 quando sujo, botão recarregar 24x24 (só `.html` com webview, não aparece no Linux), segmento "Pré-visualização | Código-fonte" para md/svg.
- Rust: `editor_view.rs:94-225` e `file_viewer_view.rs`: nenhuma barra inferior, sem breadcrumb; markdown e SVG não têm modo fonte (somente leitura, `file_viewer.rs:219-231`) nem edição. O Flutter edita md/svg quando o switch está em "Código-fonte".
- A mesma barra (breadcrumb 34px) existe no Flutter em `DiffViewer._DiffFooter`: FALTA também.

### A5. FALTA: busca no arquivo (Ctrl+F), quick-open (Ctrl+P), régua de visão geral
- `FileFindBar` (file_find_bar.dart): flutuante top 8/right 16, campo 200px, padding `8,6,6,6`, radius 8, borda `border2`, sombra; toggles "Aa" / "ab" (sublinhado) / ".*" com tooltips "Diferenciar maiúsculas", "Palavra inteira", "Usar expressão regular"; contador 66px (literal "N of M" em inglês no Flutter; "Nenhum resultado", "Padrão inválido"); botões "Anterior (⇧⏎)", "Próximo (⏎)", "Fechar (Esc)". Ctrl+F no `FV:build`. Rust: inexistente (`editor_keys.rs` só tem Ctrl+Z/Y, Tab, PageUp/Down).
- `showFileFinderPalette` (Ctrl+P, `gsh_page.dart:723`): 560px, maxHeight 420, alinhamento (0,-0.55), radius 10, placeholder literal "Go to file…", vazio "No files.", linha com nome 13px + diretório 11px `text4`, ícone 16, debounce 120ms. Rust: inexistente (nenhum atalho Ctrl+P).
- `editor_overview_ruler.dart` (495 linhas): régua com marcas ao lado da scrollbar. Rust: inexistente. O checklist do Codex lista estes três: CONFIRMADO.

### A6. DIFERENTE: composer de commit
- Flutter `_CommitComposer` (FTP:1531-1805): fixo no rodapé do Controle de Versão, altura 170-520 redimensionável (`PanelResizeHandle`), toolbar 34px com checkbox "Amend" + seletor "último commit" (menu dos 5 últimos), botão `auto_awesome` com tooltips "Gerar mensagem de commit" / "Gerar com <harness>" / "Cancelar geração" / "Indisponível durante o amend de um commit", campo multilinha mono 12 com placeholder "Mensagem do commit", botão primário "Commit" ou "Amend do commit" à direita, spinner durante o envio.
- Rust `git_changes_view.rs:55-124`: `text_input` de uma linha no topo da lista, botão largo "Sugerir mensagem"/"Gerando sugestão…" (`labels.rs:273-274`, textos que não existem no Flutter) e botão largo "Commit"/"Comitando…" (`labels.rs:271-272`). Sem Amend, sem multilinha, sem resize.
- FALTA: diálogo de commit por arquivo ("Commit" / "Stage e Commit", `gsh.commitMessageDialog.*`: nota "Commit apenas de "x".", placeholder "fix: resumo curto da mudança", validações de assunto: vazio, curto, longo, ponto final, caracteres de controle, segunda linha em branco).

### A7. DIFERENTE: lista de alterações do Git
- Flutter `_ChangedTree`/`_ChangedRow` (FTP:2824-3110, 3415-3600): seções "ALTERAÇÕES EM STAGE (N)" / "ALTERAÇÕES (N)" com ações no cabeçalho (alternar "Ver como lista"/"Ver como árvore"; "Descartar todas as alterações"; "Tirar tudo do stage"/"Colocar tudo no stage"), linha 26px com ícone de arquivo 16, nome colorido pelo status (sem letra; deletado = riscado), diretório esmaecido 11px, ações no hover (Abrir, "Descartar alterações", "Tirar do stage"/"Colocar no stage"), duplo clique abre diff, menu de contexto por arquivo ("Ver Diff", stage/unstage, "Descartar alterações"), modo árvore por pasta com ações por pasta ("Descartar alterações da pasta", "Tirar pasta do stage", "Colocar pasta no stage"), multi-root em seções recolhíveis (nome + branch), vazio "Nenhuma alteração." centralizado.
- Rust `git_file_row.rs`: linha 26px mas com letra de status mono (`badge_code`), nome truncado em 18 chars e dir em 12 (`git_presenter::shorten`), botão `+`/`-` (28px) sempre visível, sem descartar, sem árvore, sem multi-root, sem menu. Os cabeçalhos `format_staged_header`/`format_changes_header` (`labels.rs:502-509`) têm o mesmo texto, mas sem as ações.
- FALTA: descartar e todos os diálogos ("Descartar alterações?", "Excluir arquivo novo?", "Excluir todos os arquivos novos?", "Erro do Git"). O Rust mostra erro como texto inline (`git_panel.rs:43-54`).

### A8. DIFERENTE: cabeçalho e abas do Controle de Versão
- Flutter: título "CONTROLE DE VERSÃO" (10px, letterSpacing 1.1) com dois ícones (`edit_note_outlined` = Alterações, `history` = Histórico; tooltips "Alteracoes"/"Historico", sem acento no pt-BR do Flutter). A aba só existe se o workspace é repo git.
- Rust `git_header.rs:31-61`: título + botão atualizar + abas de texto "Alterações | Histórico | Branches". Botão atualizar e aba Branches são INVENTADOS (seção B). Fora de repo o Rust mostra "Nenhum repositório Git disponível." (`labels.rs:269`); no Flutter a aba some.
- Acentos: o pt-BR do Flutter em `fileTreePanel.history*`/`diff*` está sem acento ("Historico", "Repositorio", "Nenhum repositorio Git disponivel.", "Nao foi possivel carregar o historico Git.", "ha 5 min", "Arvore vazia", "Diretorio de trabalho", "Arquivo binario - sem diff de texto.", "Sem alteracoes."). O Rust usa acentos. Decisão para o dono: literal ou correção.

### A9. DIFERENTE: Histórico do Git
- Flutter `GitHistoryPanel` (git_history_panel.dart): dropdown "Repositorio" com 2+ roots, linha de branch com ícone `account_tree_outlined` 14, timeline com marcador (HEAD em negrito), chips de refs, metadados mono 10px "há 5 min · autor · hash", clique expande "Arquivos alterados" por commit e abre o diff do arquivo na revisão; estados "Nenhum commit encontrado.", "Nao foi possivel carregar o historico Git.", "Nenhum arquivo alterado.".
- Rust `git_history_view.rs` + `git_presenter::commit`: lista plana título + meta, sem timeline, sem refs, sem expandir arquivos, sem abrir diff de commit, sem seletor de repo. Data vem do git (`date_relative`), não do formato "há N min".

### A10. DIFERENTE: diff
- Flutter `DiffViewer` abre como aba no pane central: split lado a lado, cabeçalho de revisão ("Original <ref>" / "Modificado <ref>" / "Diretorio de trabalho"), cabeçalho de hunk (fundo `panel3`, mono 11.5), linhas mono 12.5, add/remove com `gitStaged/gitDeleted` alpha 0.14, syntax highlight por linguagem, seleção contínua, rodapé com breadcrumb, mensagens "Arvore vazia", "Arquivo binario - sem diff de texto.", "Sem alteracoes.", "Não foi possível ler o diff: …".
- Rust `git_diff_view.rs`: unified inline dentro do painel direito (largura `FILES_PANEL_WIDTH`), badge "STAGED"/"WORKING TREE", título "Diff: <arquivo>", botão fechar, limite 2000 linhas ("Diff limitado a 2000 linhas."), sem highlight, sem split. Não existe aba de diff (`pane_model.rs:28-61` não tem `PaneTab::Diff`).
- INVENTADO dentro dele: "Anotar" por linha, "Observação sobre esta linha para o agente…", "Salvar nota", "Enviar N nota(s) ao agente", seletor de agente (`git_diff_view.rs:100-190`, `git_diff_notes.rs`).

### A11. DIFERENTE: editor de código
- Flutter `CodeEditor` (code_editor.dart, 1042 linhas): gutter com número + ícone de severidade LSP + decorações SCM (barras de mudanças git), sublinhado e tooltip de diagnóstico por linha, Cmd/Ctrl+clique = ir para definição, tokens semânticos, comentar linha (`editor_comment.dart`), indentação inteligente (`editor_indent.dart`), clonar linhas Alt+Shift+↑/↓, destaque de matches da busca, `revealLine`.
- Rust (`editor_gutter.rs`, `editor_view.rs`, `editor_keys.rs`): gutter só com número (linha ativa em `text`, demais `EDITOR_GUTTER`), `GUTTER_ICON=16` reservado sem ícone, highlight léxico. Sem comentar linha, indent, clonar, go-to-definition, SCM, semantic tokens, hover.
- INVENTADO: botão "Formatar"/"Formatando…" na tela (`editor_view.rs:113-129`, `labels.rs:166-168`) e lista de diagnósticos em botões ("Erro · L12  mensagem", `editor_view.rs:134-163`). No Flutter, Formatar é item do menu Arquivo e diagnósticos aparecem no gutter/hover. O menu "Arquivo" do Rust (`app_menu_model.rs:99-103`) só tem "Salvar"; faltam "Descartar" e "Formatar" (`core.menu.discard/format`).

### A12. DIFERENTE: fechar arquivo com alterações
- Flutter: modal "Alterações não salvas" / "“x” tem alterações não salvas. Salvar antes de fechar?" com "Não salvar" / Cancelar / "Salvar e fechar" (`confirm_dialog.dart:47-80`).
- Rust: faixa inline "Alterações não salvas. Clique novamente em fechar para descartá-las." (`labels.rs:157-158`, `editor_view.rs:166-189`) e, no pane, "Descartar e fechar". Fluxo e texto diferentes. Também "O arquivo mudou no disco. Confirme para sobrescrever." + botão "Sobrescrever arquivo" (`labels.rs:163-164`): sem equivalente achado no Flutter; tratar como INVENTADO até o dono confirmar.

### A13. DIFERENTE: markdown, imagem, SVG
- Flutter: markdown via `AgentMarkdown` com paleta do app (headings 22/18/16/14.5), frontmatter YAML renderizado como bloco, padding `20,16,20,24`, selecionável, links, tabelas GFM, código com highlight, rolagem.
- Rust `file_viewer_view.rs:15-26`: `iced::markdown` com `Style::from(Theme::Dark)` (ignora tema claro/paleta importada), links ignorados (`FileViewerMsg::Ignore`), frontmatter removido (`file_viewer_assets.rs:46`), sem padding e sem scrollable (o `view` do markdown é devolvido direto em `pane_content`, `main.rs:8803-8815`): markdown longo provavelmente não rola. Candidato a bug.
- Imagem/SVG: Flutter usa `InteractiveViewer` (zoom até 8x, padding 16, `BoxFit.contain`, erro "Não foi possível carregar a imagem."). Rust: `image`/`svg` com `Fill`, sem zoom/pan, converte imagem para PNG de preview.
- Extensões: Flutter imagem = png jpg jpeg gif webp bmp ico; vídeo = mp4 mov avi mkv webm m4v wmv flv; áudio = mp3 wav aac m4a flac ogg opus; markdown = md mdx markdown (`file_reader_impl.dart:23-52`). Rust: imagem só png/jpg/jpeg (`file_viewer_assets.rs:26-33`), mídia sem wmv/flv/opus, markdown sem mdx.

### A14. DIFERENTE: áudio e vídeo
- Flutter `MediaView` (media_view.dart): player embutido (media_kit), play/pause, slider de posição, volume, não autoplay, pausa ao perder foco, ícone `audiotrack` 40 para áudio.
- Rust `file_viewer_view.rs:73-92`: mostra o caminho e um botão "Abrir no reprodutor padrão do sistema" (texto inventado; fora do Linux, "A reprodução pelo sistema está disponível no GSH para Linux.").

### A15. DIFERENTE: leitura de arquivo (limites e erros)
- Flutter: >2MB ou não-arquivo = "Não é possível abrir este arquivo."; não-UTF-8 cai para Latin-1 (abre como texto); binário abre como texto; sqlite não abre.
- Rust: >2MiB mostra prévia de 64 KiB somente leitura ("Arquivo acima do limite de edição (X MiB; limite: 2 MiB). Somente leitura: exibindo uma prévia do início do arquivo.") — INVENTADO; bytes nulos / UTF-8 inválido = "Arquivo binário…" (`labels.rs:184-187`) — DIFERENTE; erros `VIEWER_ERR_*` (`labels.rs:188-191`) sem equivalente (o Flutter só tem `cantOpen`). Risco: arquivo Latin-1 não abre no Rust.

### A16. DIFERENTE: rodapé LSP
- Flutter `_LspStatusBar` (gsh_page.dart:1700-1790): 34px, em toda aba do painel direito (`?widget.footer`, FTP:1510): ponto 8x8 online/`text4`, "<Linguagem> LSP · rodando/parado", botão reiniciar 26x26 com tooltip, vazio "Nenhum LSP disponível" quando não há servidor.
- Rust `files_panel.rs:279-294`: texto estático "Nenhum LSP disponível" (34px, medida IGUAL) só na aba Arquivos; sem status do servidor focado, sem reiniciar, ausente nas outras abas.

### A17. FALTA: aplicar layout `.ckp`
- Flutter `LayoutPreviewTab`/`LayoutPreviewView`: prévia do layout (comandos, pastas, "divide à direita"/"divide abaixo", "sem comando, abre um shell"), botão primário "Aplicar em <workspace>" + alternativas ("Abrir como novo workspace"), diálogo "Substituir o layout de <ws>?" ("N abas abertas deste workspace serão fechadas…"), nota de autorun worktree, falha "Não foi possível aplicar o layout".
- Rust: `.ckp` abre no editor de texto com um botão "Aplicar layout" (`labels.rs:33`, `editor_view.rs:104-122`), sem prévia nem confirmação. Também falta "Abrir layout" no menu de contexto (A2).

### A18. FALTA: `.panel` (e prévia web)
- Flutter: `PanelView` com ponte `window.gsh`. No Linux mostra "Painéis precisam da web view embutida, que ainda não existe no Linux." com "Abrir no navegador" / "Abrir como HTML" (`panelView.*`, `panel_view.dart:307-441`). `WebMarkdownPreview`/`WebHtmlPreview` só rodam com webview (`BrowserCapability.isInline`), então não são lacuna no Linux.
- Rust: sem `.panel` (abre como texto cru). Confirma o checklist.

### A19. FALTA: revelar arquivo ativo na árvore e atualização automática
- Flutter: `revealPath` expande ancestrais e rola até o arquivo da aba ativa (`_RevealTarget`, FTP:2065); `refreshToken` recarrega em mutações. Rust: sem reveal (grep `reveal` só acha abas de pane/terminal); a árvore só recarrega pelo botão atualizar (`FileTree::refresh`); não achei watcher da árvore.

### A20. FALTA: worktrees e janela de processo git
- Flutter: `showGitProcessDialog` (git_process_dialog.dart: título, spinner "Running…", "Success"/"Failed" literais em inglês, saída mono selecionável, botão único Fechar), `worktree_create_dialog.dart` ("Fork da worktree", "Criar worktree", "feat/minha-feature", "Configurações Avançadas", "Branch base", etc., `worktreeCreateDialog.*`). Rust: nada de worktree (grep só acha `--git-path` e labels de gallery).

---

## B. INVENTADO no Rust (o Flutter não tem)
1. Aba "Branches" no Controle de Versão (`git_header.rs:48-52`, `git_branch_view.rs`, `labels.rs:267,277-281`: "Nenhuma branch local.", "Salve ou faça stash das alterações antes de trocar de branch.", "atual"). O Flutter não troca branch por este painel (branch aparece em `projects_rail`/worktrees).
2. Painel de PR/checks via `gh` (`git_pr.rs`, `git_changes_view.rs:125-300`): "Consultar PR da branch", "Atualizar PR", "Checks", "Passou/Falhou/Em andamento", "Carregar log da falha", "Comentários de revisão abertos", "Corrigir com agente", "Consultando PR no GitHub…".
3. Notas por linha do diff e "Enviar N nota(s) ao agente" (`git_diff_notes.rs`, `git_diff_view.rs`).
4. Botões "Sugerir mensagem"/"Gerando sugestão…" (no Flutter é o botão `auto_awesome` com tooltips).
5. Botão "Formatar" e lista de diagnósticos sob o editor.
6. Botão "Atualizar" no cabeçalho do Controle de Versão.
7. `team_markdown.rs` / `team_markdown_view.rs` (importar/exportar "times e bots" em Markdown com revisão): sem equivalente no Flutter (nem em `agentGraph.*`).
8. Prévia de 64 KiB para arquivo grande e mensagens de binário/erro de leitura específicas (A15).
9. Botão "Abrir no reprodutor padrão do sistema" (A14).
10. Item "Buscar em arquivos" no menu Exibir (`app_menu_model.rs:120-125`): o Flutter só tem o atalho Ctrl/Cmd+Shift+F (`gsh_page.dart:735`), sem item em `core.menu.*`. Menor.
11. Faixa "Sobrescrever arquivo" (A12), a confirmar.

## C. IGUAL (conferido)
- Medidas da árvore: linha 26, indent 14, padding 6, scroll vertical 8, chevron 14/15 gap 2, ícone 16 gap 7, nome 13, raio 5 (`theme.rs:621-631` = FTP:2563-2640). Cabeçalho "ARQUIVOS" (10px; padding 12/10/6/4; ações 14px padding 3, `theme.rs:632-638` = `_PanelHeader`), vazio "Nenhuma pasta — abra um workspace.", ordem de abas (Arquivos, Busca, Git, Banco, Telemetria, Galeria), regras de filtro/ordem da árvore (`.git/.hg/.svn`, `.DS_Store`, pastas primeiro), cor do nome por status git, rodapé 34px.
- Busca no conteúdo (`content_search_panel.rs`, `content_search_state.rs`, `labels.rs:1156-1172`): todos os textos (BUSCA, "Buscar nos arquivos", "Diferenciar maiúsculas", "Palavra inteira", "Usar expressão regular", "Expressão regular inválida.", "Digite para buscar em todos os arquivos.", "Buscando…", "Nenhum resultado."), pílula "N in M", toggles Aa/ab/.*, cabeçalho 34px, grupos por arquivo, highlight de match, Ctrl/Cmd+Shift+F. Porte fiel.
- `agent_file_marks.rs` (filtro por agente, bolinhas, "Editado por X · hh:mm", "(inferido)", "Nenhum arquivo tocado por este agente ainda"): estrutura e textos iguais ao `agent_file_marks.dart`; validar visualmente.
- Ícones de arquivo (`file_icons*.rs`) incluindo `kanban`, `http`, `gsh-notebook`: presentes.

## D. Contagem
- IGUAL: 4 blocos (medidas/cabeçalho/regras da árvore, busca no conteúdo, marcas de agente, ícones).
- DIFERENTE: 12 (A3, A6, A7, A8, A9, A10, A11, A12, A13, A14, A15, A16).
- FALTA: 8 grupos (A1, A2, A4, A5, A17, A18, A19, A20).
- INVENTADO: 11 itens (seção B).
