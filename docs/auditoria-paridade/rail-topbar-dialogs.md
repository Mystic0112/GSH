# Auditoria de paridade Flutter -> Rust: rail, workspaces, topbar, menus, diálogos gerais

Somente leitura. Flutter = `/home/heliodev/projetos/GSH Desktop/lib/app/...` (abreviado `F:`), Rust = `/home/heliodev/projetos/GSH/crates/app/src/...` (abreviado `R:`).
Textos pt-BR do Flutter: `lib/i18n/pt_BR.i18n.json` (`projectsRail` l.627, `topbar` l.680, `core.menu` l.41, `welcomeView` l.227, `realmDialogs` l.401).
Fora do escopo (já sabido): diálogo de configuração do workspace e menu do workspace no Rust (codex implementando). Eles aparecem aqui só como dependência dos achados.

Legenda: IGUAL / DIFERENTE / FALTA (existe no Flutter, não no Rust) / INVENTADO (existe no Rust, não no Flutter).
Pedidos explícitos do dono (LocalSend, navegador/Obscura) não são listados como inventados.

## Resumo

| Classe | Qtde |
|---|---|
| FALTA | 17 |
| DIFERENTE | 24 |
| INVENTADO | 9 |
| IGUAL (conferido) | 14 |

---

## A. O que o usuário nota de cara (rail e topbar)

### A1. Ícone do cabeçalho "Workspaces" é o glifo errado: hub em vez de layers — DIFERENTE
- F: `projects_rail.dart:436` `Icons.layers_outlined` (pilha de camadas), 16px, cor text2.
- R: `rail.rs:95` usa `labels::ICON_LAYERS`, definido em `labels.rs:212` como `\u{e9f4}`, o mesmo codepoint de `ICON_HUB_OUTLINED` (`labels.rs:247`). Na Material Symbols `e9f4` é `hub` (rede de nós). O screenshot `shots/gsh-rail-footer-fechado.png` mostra o hub. O correto é `layers` (`e53b` na fonte Symbols).

### A2. Workspaces de demonstração falsos no primeiro uso — INVENTADO
- R: `rail.rs:42-57` `default_workspace_list()` cria "GSH" (branch fixa `main`) e "Projeto de exemplo" (branch fixa `feat/ui`); textos em `labels.rs:16-21`.
- F: não existe lista de exemplo. Sem workspace, o rail mostra "Nenhum workspace ainda." e o centro mostra a `WelcomeView` (ver A4). A branch dos badges vem do git real.
- Consequência: os badges `main`/`feat/ui` são falsos. `Workspace::with_branch` (`core/src/workspace.rs:195`) nunca é chamado para workspaces reais, então um workspace aberto por pasta nunca mostra badge git (ver A5).

### A3. Trocar de workspace pode ser bloqueado por um aviso — INVENTADO
- R: `main.rs:8482-8495` + `layout_switch_blocker` (`main.rs:1655-1690`) e overlay `main.rs:9642-9652` (`pane_close_dialog::notice("Trocar de workspace", ...)`). Mensagens: "Salve ou feche os documentos com alterações antes de trocar de workspace." e "Conclua as conversas em andamento e limpe os rascunhos antes de trocar de workspace." (e a variante de terminais).
- F: trocar de workspace nunca bloqueia. Os workspaces mantêm as sessões vivas (`_onCardTap` só chama `onSelect`, `projects_rail.dart:235-257`).
- Impacto alto: o usuário clica no workspace e nada acontece, só aparece um modal que o Flutter não tem.

### A4. Tela de boas-vindas (estado sem workspace) — FALTA
- F: `welcome_view.dart:1-125`, usada em `gsh_page.dart:1134`. Logo `assets/branding/gsh_logo.png` 64x64 raio 14; título 20px "Bem-vindo ao GSH Desktop"; subtítulo 13.5px text2, altura 1.4: "Abra uma pasta ou conecte a um host remoto para começar."; botões lado a lado com 10px de espaço: primário "Conectar a um host" (ícone cloud_outlined) e secundário "Abrir pasta local" (ícone folder_outlined); largura máx 460.
- R: não existe (grep "Bem-vindo": 0 resultados). O Rust nunca fica sem workspace: a lista padrão tem 2 itens e não há ação "Fechar workspace" (ver A6).
- Estado vazio do rail: F `_EmptyRail` (`projects_rail.dart:1957-1974`) centraliza "Nenhum workspace ainda." (label, text3, padding 16). R: `rail.rs:119-121` mostra `labels::CHANNELS_EMPTY` = "Nenhum workspace neste canal" (`labels.rs:37`), texto cru no canto, sem centralizar nem cor text3. Texto DIFERENTE e layout DIFERENTE.

### A5. Card do workspace: faltam cor do avatar, badge git real, agente, notificações, worktrees, arrastar — FALTA/DIFERENTE
Medidas que batem (IGUAL): avatar 30x30 raio 7, fonte da inicial 0,43x, nome 13,5 (w500 se selecionado), espaço de 10 entre avatar e texto, badge git com padding (4,1,5,1), raio 4, mono 9,5, ícone 9 (R: `theme.rs:975-976,588`, `rail.rs:196-255`). Fundo selecionado panel2, hover panel3, raio do card 7, espaço vertical 2 entre cards, rail 252px com borda direita.
Diferenças (F `projects_rail.dart:1045-1237`, `workspace_avatar.dart`; R `rail.rs:191-285`):
- Cor do avatar: F usa `colorValue` por workspace (cor sugerida da paleta) e texto `onColor(cor)`, ou a imagem escolhida (`workspace_avatar.dart:20-101`, ícone `broken_image_outlined` se faltar o arquivo). R: sempre `panel` com texto `text` (`rail.rs:206-213`). FALTA cor e imagem do avatar (depende do diálogo de configuração já conhecido, mas o avatar em si também não existe).
- Badge git: F mostra branch real (`call_split`), `↓behind` (warn), `↑ahead` (accentText) e contagem de sujos; sujo vira fundo `editedBg` + texto warn (`_GitBadge`, l.1721-1781). R mostra só `git_branch` estático em text3/panel3 (`rail.rs:225-255`). FALTA ahead/behind/dirty e a cor de sujo.
- Multi-root: F chip "N roots · M" com popup por root (`_MultiRootBadge`, l.1542-1619; `_RootStatusRow` l.1624-1719). R: FALTA.
- Ponto de status de agente (`WorkspaceAgentDot`, l.1188) e contador de notificações (pílula accent com `minWidth 18`, mono 11 w600, l.1189-1213). R: FALTA.
- Chevron de worktrees (l.1214-1218, `_WorktreeChevron` l.675-706, tooltips "Expandir worktrees"/"Recolher worktrees"), resumo "N worktrees" (l.1223-1230), lista de forks com linha de árvore (`_WorktreeItem` l.1245-1372, `_ForkLinePainter` l.1509-1534), sinal de sujo/notificação do fork (`_WorktreeSignal` l.1441-1503) e menu do fork (`_WorktreeMenu` l.1376-1436: "Mesclar no pai", "Atualizar a partir do pai", "Criar worktree derivada", "Copiar branch", "Remover"). R: FALTA tudo. Não há conceito de worktree no Rust (grep em `main.rs`/`labels.rs`: 0).
- Reordenar arrastando (`_WorkspaceReorderable`, l.2103-2214: Draggable + caret accent 2,5px + chip de arrasto `_WorkspaceDragChip` l.2233-2282). R: FALTA.
- Padding do card: F `fromLTRB(9,7,5,7)`; R `[7,9]` (direita 9 em vez de 5). Visualmente irrelevante hoje, mas importa quando entrarem chevron/badges.
- Scrollbar: F `Scrollbar(thumbVisibility: true)` (l.471-474). R: `scrollable` padrão (`rail.rs:171`). DIFERENTE (thumb sempre visível no Flutter).
- Workspace remoto (`_RemoteSlot`, l.804-1043: badge "SSH" 9px accent com fundo accent 15%, chevron). R: FALTA, só existe um diálogo de terminal remoto (ver C2).

### A6. Slot fixo "GSH Desktop" — DIFERENTE
- F: `_GshSlot` (l.728-770), fixo logo abaixo do cabeçalho, fora da lista: caixa 30x30 `panel` com borda e ícone `terminal` 17px text2, texto literal `GSH Desktop` 13,5; sem avatar/imagem, sem menu, não renomeável nem fechável; padding `(8,0,8,4)`.
- R: "GSH" é um item comum da lista, com inicial "G" e badge `main` (`rail.rs:43-47`); recebe `with_path(cwd)`. Nome "GSH" (`labels.rs:16`) em vez de "GSH Desktop". O título da topbar também sai como "GSH" em vez de "GSH Desktop" (`main.rs:9263-9267`, F `gsh_page.dart:863` `shell.title ?? 'GSH Desktop'`).

### A7. Menu "+" do rail (Local vs Remoto) — DIFERENTE
- F: "+" tem tooltip "Novo workspace" e abre `showAppMenu` com "Local" (`folder_outlined`) e "Remoto" (`cloud_outlined`) (`remote_workspace_actions.dart:56-80`). Local abre pasta e depois o diálogo de configuração (nome + cor). Remoto: escolhe host / "Novo host…" e depois pasta no host.
- R: "+" (`rail.rs:105`) sem tooltip, abre direto o seletor de pastas do SO (`main.rs:9385` `Msg::PickWorkspaceFolder`, `main.rs:1276` sem título e sem diretório inicial; F usa títulos "Escolha a pasta do workspace"/"Escolha a pasta do projeto" e começa na pasta do projeto atual, `workspace_actions.dart:28-36`). Não passa pelo diálogo de configuração (já sabido).

### A8. Rodapé do rail — DIFERENTE em detalhes
- IGUAL: altura 34, linha superior, padding horizontal 12, botão 26x26 raio 5, ícone 16 text3, tooltip de "Configurações", textos de keep-awake (`labels.rs:23-29` = `projectsRail.keepAwake*`), cor do bolt (text3 / warn na bateria / accent).
- Seletor: F `_RealmSelector` (l.2009-2097) é um botão do tamanho do conteúdo, alinhado à esquerda: ponto 7px verde com brilho (`boxShadow blur 8` cor `online` cheia), 8px, nome (label text2, elipse), 4px, `expand_more` 14. R `channel_footer.rs:26-78`: botão ocupa a largura toda e empurra o chevron para a extrema direita (`Space::new().width(Fill)`, `channel_footer.rs:42`, visível em `shots/gsh-rail-footer-fechado.png`); gap do ponto 5 (F 8); brilho com blur 5 e alpha 0,48 (`channel_footer.rs:80-97`). Hover do botão: F só no conteúdo; R na largura toda.
- Bolt do keep-awake: F usa `bolt_outlined` desligado e `bolt` preenchido ligado, e treme ~420 ms ao ligar (`_KeepAwakeButton`, l.535-616). R usa o mesmo glifo nos dois estados (`rail.rs:132-144`, `labels.rs:1251`), sem preenchimento nem animação.
- Espaço entre bolt e engrenagem: F 0 (botões encostados); R `spacing(4)` (`rail.rs:162`).
- Botão de keep-awake some quando a plataforma não suporta (F `ctl.isSupported`). R: sempre aparece.

### A9. Concept "realm" vs "canal" — INVENTADO/DIFERENTE (decisão do dono pendente)
O Rust renomeou "realm" para "canal" e acrescentou um editor de canal rico. O Flutter não tem nada disso.
- Realm padrão: F cria só `Default` (`json_realm_repository.dart:29`, `realm_controller.dart:31`, nome literal "Default"). R pré-cria 4 canais: "Padrão", "Trabalho", "Pessoal", "Projetos" (`core/src/workspace.rs:30-47`). INVENTADO (3 canais).
- Menu do seletor. F (`_RealmSelector._open`, l.2024-2057): um item por realm com ícone `public` e check no ativo; divisória; "Novo realm…" (`add`); "Gerenciar realms…" (`tune`); `minWidth 180`. R (`channel_footer.rs:105-170`, screenshot `gsh-rail-footer-menu.png`): sem ícones; "Todos os canais" (INVENTADO, não há "todos" no F); itens; divisória; "Novo canal"; "Editar canal" (INVENTADO); divisória; seção "Mover workspace ativo para" + itens (INVENTADO; no F isso é "Mover para realm" dentro do menu do workspace, submenu).
- Criar: F abre `AlertDialog` "Novo realm" com placeholder "Nome do realm", erro "Já existe um realm com esse nome." e botões Cancelar / Criar (`realm_dialogs.dart:9-134`), e troca para o realm criado. R abre um cartão inline dentro do rail com campos Nome, Instruções, Pasta, Membros, Regra de resposta (Manual / Só ao mencionar / Todos), Arquivar/Desarquivar (`channel_editor.rs`, `labels.rs:35-59`). Tudo isso é INVENTADO.
- "Gerenciar realms…" (diálogo com lista: bolinha ativa/inativa, nome, "N workspaces", editar, excluir; excluir pede confirmação "Excluir realm"/"Excluir "x"? Nenhum workspace é excluído..."; botões "Novo realm" e "Concluído"; `realm_dialogs.dart:136-306`). R: FALTA.
- Se o dono quiser manter "canais", isso é decisão explícita dele; pelo critério "igual ao Flutter", o equivalente seria realm.

### A10. Card de atualização — DIFERENTE (textos IGUAL)
Textos IGUAIS (o Flutter também é em inglês literal): "Update available", "v{versão} — click to download", tooltip de dispensar "Dispensar" (`update_viewmodel.dart:105-122`, `R: labels.rs:30-32`). No Flutter ainda existem "Update ready", "restart to install", "Downloading…", "click to install" (fluxo self-update macOS/Windows; no Linux só "click to download", então IGUAL na prática).
Visual (F `update_card.dart:21-83`; R `rail.rs:333-392`):
- F tem ícone à esquerda: `system_update_alt` 15px accentText (ou `restart_alt` se pronto). R: sem ícone.
- Borda: F `accent` com alpha 0,5, raio 8; R accent cheio, raio 7.
- Padding: F `(10,8,6,8)` único; R `[8,10]`.
- Hover: F `panel2 -> panel3`; R sem hover.
- Botão dispensar: F ícone `close` 14 em área de 4px; R botão 26x26 com ícone 16.
- Textos: F título `label` w600 text, subtítulo label text3, ambos com elipse; R equivalente (semibold). OK.

---

## B. Barra do topo e menu do app

### B1. Botão de menu e menu — DIFERENTE
- Botão: F `WindowMenuBar` (`app_menu_bar.dart:374-455`) é um hambúrguer `Icons.menu` 16px, 28x28, sem tooltip; aberto = fundo accentSoft e ícone accentText; hover `panel`. R `topbar.rs:63-88`: `more_vert` (três pontos verticais) 17px, **com** tooltip "Menu" (INVENTADO o tooltip).
- Posição: F padding esquerdo 8 (`window_controls.dart:124`), menu, 8px, botão da rail, 8px, título. R: `Space 12` + menu + botão da rail encostado (0px) + 8px + título (`topbar.rs:89-100`). DIFERENTE: 12 vs 8 à esquerda e 0 vs 8 entre menu e toggle.
- Conteúdo achatado com divisória entre grupos: IGUAL na estrutura (F `_toggle` l.400-427, R `topbar.rs:56-62`), mas o conteúdo difere (B2). Largura mínima: F 200 (`showAppMenu` default), R 180 (`topbar.rs:80`).
- Atalhos: F não desenha coluna de atalho no menu Windows/Linux (`AppMenuItem` sem accelerator). R desenha o atalho à direita (`app_menu.rs:236-248`). DIFERENTE.
- Itens desabilitados: F desabilita (cinza) sem workspace/sem editor; R usa `on_press None` só para Salvar.

### B2. Conteúdo do menu: textos, ordem, itens — DIFERENTE/FALTA/INVENTADO
Fonte F: `app_menu_bar.dart:40-290` + `core.menu` (`pt_BR.i18n.json:41-70`). Fonte R: `app_menu_model.rs:40-250`.
Os rótulos do Flutter são em Title Case; o Rust usa sentence case. Todos DIFERENTES:

| Flutter | Rust | Classe |
|---|---|---|
| Configurações… | Configurações | DIFERENTE (reticências) |
| Verificar Atualizações… | Verificar atualizações… | DIFERENTE (caixa) |
| (divisória) Sair | Sair, sem divisória antes | DIFERENTE |
| Novo Terminal | Novo terminal | DIFERENTE |
| Novo Chat | Novo chat | DIFERENTE |
| Novo Chat à Direita | Novo chat à direita | DIFERENTE |
| (divisória) Abrir Workspace | Abrir workspace | DIFERENTE |
| (divisória) Salvar / Descartar / Formatar | só "Salvar" | FALTA Descartar, FALTA Formatar |
| Alternar Painel de Workspaces | Alternar painel de workspaces | DIFERENTE |
| Alternar Arquivos | Alternar arquivos | DIFERENTE |
| (divisória) Dividir à Direita / Dividir Abaixo | Dividir à direita / abaixo | DIFERENTE |
| submenu "Focar Painel" (Esquerda/Direita/Acima/Abaixo com hint ⌘⌥ setas) | não existe | FALTA |
| submenu "Selecionar Aba" (Aba 1..Aba 8, Última Aba) em Exibir | 9 itens soltos "Selecionar aba 1..8" + "Última aba" no grupo "Janela" (`app_menu_model.rs:177-235`) | DIFERENTE |
| (divisória) Workspace Anterior / Próximo Workspace | Workspace anterior / Próximo workspace, sem divisórias | DIFERENTE |
| (divisória) Aumentar Zoom / Diminuir Zoom / Tamanho Real | Aumentar zoom / Diminuir zoom / **Tamanho original** | DIFERENTE (texto "Tamanho Real") |
| Janela: Minimizar, Zoom | Minimizar janela, Alternar tamanho da janela | DIFERENTE |
| não existe | "Novo terminal remoto por SSH" (`app_menu_model.rs:49-56`) | INVENTADO |
| não existe | "Buscar em arquivos" Ctrl+Shift+F (`app_menu_model.rs:118-125`) | INVENTADO (no F, ⇧⌘F é "Formatar") |

Notas:
- Ordem F por grupo: GSH Desktop / Arquivo / Exibir / Janela. R: GSH / Arquivo / Exibir / Janela (`main.rs:9159`). O grupo "GSH" no F não tem rótulo de topo visível (achatado), só divisória; mesma coisa no R. IGUAL.
- Atalhos IGUAIS ao Flutter na versão Linux (Ctrl no lugar de ⌘): T, Shift+L, Shift+J, O, `,`, S, B, Shift+B, D, Shift+D, Shift+N, Shift+M, `=`, `-`, `0`, 1..9. Bate.
- Atalhos editáveis (`app_menu_model::binding`/`settings.shortcuts`): não há equivalente no Flutter que eu tenha achado nesta área; checar se é pedido do dono.

### B3. Botões da direita da topbar — DIFERENTE
Ordem F (`gsh_topbar.dart:104-148`): FarmStrip, Chat, Fazenda, Harness, Grafo, Arquivos, controles de janela.
Ordem R (`topbar.rs:107-148`): FarmStrip, Fazenda, Chat, [Navegador, permitido], Harness, Grafo, [LocalSend, permitido], Arquivos. **Chat e Fazenda estão trocados** em relação ao Flutter.
Tooltips:
- "Recolher barra lateral" (toggle da rail): F tem; R não tem (`topbar.rs:89-93`). FALTA.
- "Mostrar/ocultar arquivos" (toggle da direita): F tem; R não tem (`topbar.rs:149-153`). FALTA.
- "Chat com o Claude (beta)": F tem; R sem tooltip (`topbar.rs:110-112`). FALTA.
- "Fazenda de agentes": F tem; R sem tooltip (`topbar.rs:107-109`). FALTA.
- "Harness do workspace" e "Visualizar agentes": IGUAL (`labels.rs:1230`, `labels.rs:249`).
- Toggle de arquivos desabilitado: F deixa o botão inerte e com 35% de alpha + tooltip "Arquivos indisponíveis no GSH Desktop" no workspace GSH (`filesEnabled`, `gsh_topbar.dart:152-161`). R: sempre ativo. FALTA.
- Hover do ícone: F mantém `text3` com fundo `panel3`; R troca o ícone para `text2` ao passar o mouse (`topbar.rs:236-240`). DIFERENTE (sutil).
- Farm strip: R mostra contadores mesmo zerados (decisão já documentada no código, `topbar.rs:~265`). O tooltip do R é um `text` cru (`topbar.rs:291-295`) em vez do balão `tip_bubble`; o F usa `AppTooltip` com "Trabalhando · N precisam de você · Ocioso". A pílula "N precisam de você" no F abre um menu com a fila de agentes que esperam (clicar foca a aba); no R só abre a fazenda (`topbar.rs:313`). DIFERENTE.
- Duplo clique na barra para maximizar: F `_DragToMoveArea` (`window_controls.dart:197-209`, `onDoubleTap: _toggleZoom`). R só tem `on_press(WindowDrag)` (`main.rs:9335`). FALTA.

### B4. Botões de janela (Linux) — DIFERENTE
- F `WindowControlsTrailing` (`window_controls.dart:295-370`): 3 botões 46x46 sem raio, ícones `remove` / `crop_square` / `close` 16px cor text2; hover `panel3`, no Fechar hover vermelho `#E81123` com ícone branco; tooltips "Minimizar", "Maximizar", "Fechar".
- R `main.rs:9420-9437,9870`: botões `ghost` 28x28 com raio 5, `spacing(2)`, largura total 88 (`theme.rs:584`); sem hover vermelho; sem tooltip. DIFERENTE em tamanho (46 vs 28), formato, hover e tooltips.

### B5. Título — IGUAL com exceção
Fonte 14, `title`, text (F `gsh_topbar.dart:93-95`, R `theme.rs:537`). Altura 46 com linha inferior de 1px. Ilha centralizada some abaixo de 160px: IGUAL. Texto do workspace GSH difere (A6).

---

## C. Diálogos gerais

### C1. Base visual dos diálogos de confirmação — IGUAL
`pane_close_dialog.rs` replica `showConfirmDialog` (`confirm_dialog.dart:99-148`): título 15, mensagem 13,5 text2, largura 380, padding 24, raio 12, ações à direita 8px, "Cancelar" outline + confirmar destrutivo/primário, scrim, clique fora cancela (`theme.rs:612-620`). "Fechar painel?" e mensagem batem com `paneView.closePaneTitle/Message`.
- Rótulo do aviso informativo: F `showInfoDialog` usa botão "Entendi" (`common.gotIt`, `confirm_dialog.dart:41`); R `notice()` usa "OK" (`labels.rs:1181`, `pane_close_dialog.rs:42-50`). DIFERENTE (o uso do "OK" do R é para o erro de Galeria, conferir o equivalente no F antes de mudar).

### C2. Conectar a host remoto — DIFERENTE
- F: "Adicionar host remoto" (`remoteHost.*`, `pt_BR.i18n.json` bloco `remoteHost`): Nome, Usuário, Host/IP, Porta, Autenticação (Chave SSH/Senha), seletor de chave "Escolher…", erros tipados; cadastra o host e abre picker de pasta "Abrir pasta em {host}" ("Abrir aqui", "Sem subpastas"); resultado é um workspace remoto com badge SSH.
- R `remote_connect_dialog.rs:36-63`: título "Conectar terminal remoto", campos "Host SSH (alias do ~/.ssh/config ou usuário@host)", "Socket absoluto do gsh-server no host", "Diretório inicial remoto (opcional)". Strings literais no módulo, fora de `labels.rs`. Fluxo e campos DIFERENTES; não gera workspace nem badge SSH. O ponto de entrada também difere (A7, B2).

### C3. Confirmar fechar editor/janela com alterações — DIFERENTE / INVENTADO
- Fechar aba de arquivo com alterações: F abre `showCloseDirtyDialog` ("Alterações não salvas", "“{arquivo}” tem alterações não salvas. Salvar antes de fechar?", botões Não salvar / Cancelar / Salvar e fechar; `confirm_dialog.dart:51-97`, `tab_actions.dart:44`). R usa um aviso inline "Alterações não salvas. Clique novamente em fechar para descartá-las." (`labels.rs:158`, `editor_view.rs:197`). DIFERENTE. O diálogo de 3 botões só existe no R para queries de banco (`main.rs:9826`, `close_query_dialog`).
- Fechar a janela com alterações: R mostra uma faixa crua com `button` padrão do iced, "Há alterações não salvas. Descartar e fechar a janela?" / "Descartar e fechar" / "Cancelar" (`main.rs:9448-9458`, `labels.rs:160-162`), sem estilo de diálogo. INVENTADO: o F não pergunta ao fechar a janela (`bootstrapper.dart:643` só salva bounds e destrói).

### C4. Fechar workspace e diálogos de ação de workspace — FALTA (ligado ao menu do workspace já sabido)
- "Fechar workspace" / `Fechar "{nome}"? Os agentes deste workspace serão encerrados. A pasta no disco é mantida.` / botão "Fechar" (`gshPage.closeWorkspace*`, `workspace_actions.dart:98-110`). R: sem fluxo de fechar workspace.
- Aviso "Workspace renomeado" / `O novo nome "{nome}" só será enviado aos agentes após reiniciar o workspace ou o aplicativo.` (`workspace_actions.dart:80-88`). R: FALTA.
- Diálogos de Sync / Pull / Push (`gshPage.syncTitle/pullTitle/pushTitle`, `git_process_dialog.dart`), "Atualizar a partir do Pai", "Merge para o Pai", "Remover worktree" (e seu aviso de branch não mesclado), "Falha ao remover a worktree", "Substituir o layout atual?" (`gshPage.*`). R: FALTA todos.
- Diálogo "Criar worktree" / "Fork da worktree" (`worktree_create_dialog.dart`, 688 linhas, textos em `worktreeCreateDialog.*`: validação de nome de branch, Branch base, opções "Copiar arquivos ignorados", "Copiar arquivos não rastreados", "Sincronizar branch remota (fetch)", "Configurações Avançadas", "Executando…"). R: FALTA (sem nenhum suporte a worktree).

### C5. Diálogo de mensagem de commit — FALTA (provável fora desta área)
- F `commit_message_dialog.dart`: aberto pelo commit por arquivo no painel de arquivos (`file_tree_panel.dart:660`). Títulos "Commit" / "Stage e Commit", nota `Commit apenas de "{arquivo}".`, placeholder `fix: resumo curto da mudança`, contador n/72, validação do assunto (3 a 72 caracteres, sem ponto final, segunda linha em branco), "Gerar mensagem de commit" / "Gerar com {harness}" / "Gerando…" / "Cancelar geração".
- R: commit inline no painel git (`git_changes_view.rs:~100-115`, "Mensagem do commit", "Sugerir mensagem", "Gerando sugestão…"); nenhuma validação nem diálogo. Textos DIFERENTES. Confirmar com quem audita o painel git.

### C6. Diálogo "Novo agente" — majoritariamente IGUAL
- Textos e tamanhos batem (`agent_graph_dialog.rs:98-260` vs `new_agent_dialog.dart`, `agentGraph.*`): "Novo agente", "INÍCIO RÁPIDO", "Shell", "APPS DESKTOP", abas Detalhes/Aparência/Agente, campos, dicas, "Integração", aceite MCP, Cancelar / Criar.
- DIFERENTE: lista de harnesses. F usa o catálogo (`core/domain/entities/harness.dart:42-91`): Antigravity, Claude Code, Codex, Cursor, **GitHub Copilot**, OpenCode, Pi. R hardcoda 6 (`agent_graph_dialog.rs:127-158`) e **falta GitHub Copilot**.
- DIFERENTE (menor): as abas são `Tabs` do shadcn no F e botões simples no R.
- Rótulo "APPS DESKTOP": no R está literal no módulo (`agent_graph_dialog.rs:~224`), enquanto a constante `AGENT_GRAPH_DESKTOP_APPS` existe em `labels.rs:576` (o F usa `toUpperCase()`). Valor igual.

---

## D. Lista completa por elemento (referência)

| # | Elemento | Flutter | Rust | Classe |
|---|---|---|---|---|
| 1 | Título do cabeçalho "Workspaces" | `projects_rail.dart:439` | `rail.rs:100`, `labels.rs:15` | IGUAL |
| 2 | Padding do cabeçalho (16,16,12,12), ícone 16, espaço 9 | l.433-437 | `rail.rs:109-114,99` | IGUAL |
| 3 | Ícone do cabeçalho | layers_outlined | hub (e9f4) | DIFERENTE (A1) |
| 4 | Botão "+" 26x26 raio 5 | `_SmallIcon` l.1976 | `rail.rs:289-331` | IGUAL (medidas) |
| 5 | Tooltip do "+" "Novo workspace" | l.448 | ausente | FALTA |
| 6 | Menu Local/Remoto do "+" | `remote_workspace_actions.dart:56` | ausente | FALTA |
| 7 | Slot "GSH Desktop" | l.728 | item comum "GSH" | DIFERENTE |
| 8 | Lista de workspaces | `ListView.builder` + scrollbar | `column` + `scrollable` | DIFERENTE (scrollbar) |
| 9 | Avatar colorido / imagem | `workspace_avatar.dart` | `panel` fixo | FALTA |
| 10 | Badge git (dirty/ahead/behind) | `_GitBadge` | só branch estática | FALTA |
| 11 | Badge multi-root | `_MultiRootBadge` | ausente | FALTA |
| 12 | Ponto de agente | `WorkspaceAgentDot` | ausente | FALTA |
| 13 | Contador de notificações | l.1189 | ausente | FALTA |
| 14 | Chevron/lista de worktrees | l.1214, 622 | ausente | FALTA |
| 15 | Drag para reordenar | l.2103 | ausente | FALTA |
| 16 | Menu de contexto do workspace | l.1810 | ausente | FALTA (já sabido) |
| 17 | Menu de contexto do fork | l.1376 | ausente | FALTA |
| 18 | Badge SSH / workspace remoto | l.995 | ausente | FALTA |
| 19 | Estado vazio do rail | l.1957 | `CHANNELS_EMPTY` cru | DIFERENTE |
| 20 | Card de atualização | `update_card.dart` | `rail.rs:333` | DIFERENTE (A10) |
| 21 | Rodapé: altura 34, borda, padding | l.493-499 | `rail.rs:165-168` | IGUAL |
| 22 | Seletor de realm | l.2009 | `channel_footer.rs` | DIFERENTE (A8, A9) |
| 23 | Keep-awake: tooltip e cores | l.571-615 | `rail.rs:23-39,132-144` | IGUAL |
| 24 | Keep-awake: glifo cheio/outline + tremor | l.606-611 | glifo único | DIFERENTE |
| 25 | Engrenagem + tooltip "Configurações" | l.512-516 | `rail.rs:145-149` | IGUAL |
| 26 | Topbar altura 46 + linha | `window_controls.dart:106` | `topbar.rs:158-180` | IGUAL |
| 27 | Botão da rail (ícone sidebar, ativo = accentSoft) | `_IconBtn` | `topbar.rs:219-258` | IGUAL (cores), sem tooltip |
| 28 | Ordem Chat/Fazenda | chat, fazenda | fazenda, chat | DIFERENTE |
| 29 | Tooltips de rail/arquivos/chat/fazenda | `topbar.*` | ausentes | FALTA |
| 30 | Toggle de arquivos inerte no GSH | l.152 | sempre ativo | FALTA |
| 31 | Duplo clique maximiza | `window_controls.dart:197` | ausente | FALTA |
| 32 | Botões de janela 46x46, hover vermelho | `window_controls.dart:327` | 28x28 ghost | DIFERENTE |
| 33 | Hambúrguer do menu | `Icons.menu` | `more_vert` + tooltip "Menu" | DIFERENTE/INVENTADO |
| 34 | Itens do menu | `core.menu.*` | `app_menu_model.rs` | DIFERENTE (B2) |
| 35 | Menu Focar Painel | l.190 | ausente | FALTA |
| 36 | Menu Descartar/Formatar | l.136-153 | ausente | FALTA |
| 37 | Welcome view | `welcome_view.dart` | ausente | FALTA |
| 38 | Diálogo de nome de realm | `realm_dialogs.dart:9` | cartão de canal inline | DIFERENTE/INVENTADO |
| 39 | Gerenciar realms | `realm_dialogs.dart:136` | ausente | FALTA |
| 40 | Fechar workspace (confirm) | `workspace_actions.dart:98` | ausente | FALTA |
| 41 | Criar/Fork worktree | `worktree_create_dialog.dart` | ausente | FALTA |
| 42 | Commit por arquivo | `commit_message_dialog.dart` | commit inline | FALTA/DIFERENTE |
| 43 | Close dirty (3 botões) para arquivo | `confirm_dialog.dart:51` | aviso inline | DIFERENTE |
| 44 | Confirmar fechar janela | não existe | faixa crua | INVENTADO |
| 45 | Bloqueio ao trocar de workspace | não existe | `main.rs:9642` | INVENTADO |
| 46 | Workspaces de exemplo | não existe | `rail.rs:42` | INVENTADO |
| 47 | Canais pré-criados (3 extras), "Todos os canais", editor de canal, regra de resposta, instruções, membros, pasta, arquivar | não existe | `channel_editor.rs`, `core/workspace.rs:30` | INVENTADO |
| 48 | Atalho Ctrl+Shift+F "Buscar em arquivos" no menu | ⇧⌘F é Formatar | `app_menu_model.rs:118` | INVENTADO |
| 49 | "Novo terminal remoto por SSH" no menu | não existe | `app_menu_model.rs:49` | INVENTADO |
| 50 | Harness GitHub Copilot no Novo agente | `harness.dart:75` | ausente | FALTA |
| 51 | Diálogo remoto (host/pasta) | `remoteHost.*` | `remote_connect_dialog.rs` | DIFERENTE |
| 52 | Seletor de pasta do SO (título e diretório inicial) | `workspace_actions.dart:28-36` | `main.rs:1276` | DIFERENTE |
| 53 | Base de diálogos (tamanhos/botões) | `confirm_dialog.dart` | `pane_close_dialog.rs` | IGUAL |
| 54 | "Fechar painel?" | `paneView.closePane*` | `labels.rs:688-695` | IGUAL |
| 55 | Textos de update | `update_viewmodel.dart` | `labels.rs:30-32` | IGUAL |
| 56 | Textos do diálogo Novo agente | `agentGraph.*` | `labels.rs:571-660` | IGUAL |

(Total de linhas D por classe: ver contagem no Resumo, que agrega A-C e D.)
