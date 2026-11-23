# Auditoria de paridade Flutter -> Rust: bancos, HTTP, tasks e telemetria

Somente leitura. Flutter = `/home/heliodev/projetos/GSH Desktop/lib/app/gsh/ui/widgets/*` e `lib/i18n/pt_BR.i18n.json`.
Rust = `/home/heliodev/projetos/GSH/crates/app/src/*`. Textos pt-BR conferidos por script: toda chave `dbPanel`, `dbConnectionDialog`,
`dbQueryView`, `dbMongoView`, `dbRedisTable` e `telemetry` do pt-BR existe literalmente no Rust. Faltam apenas `httpView.*` (cerca de 25 strings),
`tasksPanel.*` (18) e `tasks.*` (5), porque as telas nao existem.

Legenda: IGUAL / DIFERENTE / FALTA / INVENTADO. Ordenado por impacto visivel.

## Resumo de contagem

- FALTA: 8
- DIFERENTE: 14
- INVENTADO: 6
- IGUAL: mais de 45 elementos (listados no fim)

---

## 1. FALTA (maior impacto)

### F1. Painel de Tasks inteiro (confirmado por grep: nao existe)
- Flutter: `tasks_panel.dart:19-587`. Sub-painel na coluna direita, abaixo de Arquivos/Busca/DB/Telemetria/Galeria (`file_tree_panel.dart:1522`, `if (!scMode) ?widget.tasksPanel`; montagem em `gsh_page.dart:1564-1577`).
  - Alca de resize `PanelResizeHandle`, altura persistida (`setTasksPanelHeight`, `gsh_page.dart:951`).
  - Header 34px: icone `play_circle_outline`, "TAREFAS", spinner ou botao "Recarregar tasks".
  - Linha de task: bolinha de estado (idle/starting/stopping/building/running/success/failed/stopped), label com tooltip do comando, botoes dirigidos por dados (teclas interativas primarias, menu "Mais teclas", Reiniciar, Parar, Executar), chip de profile ("Trocar perfil"), spinner "Iniciando…"/"Parando…".
  - Vazio: "Nenhuma tarefa detectada neste projeto." + botao "Criar tasks.json".
  - Botao "Gerar tasks do Compose" quando o arquivo ativo e .yaml/.yml, com dialog "Escolha uma engine Compose" (Docker Compose / Podman Compose) e toasts ("Docker Compose ou Podman Compose nao foi encontrado.", "O arquivo ativo nao e um Compose valido com services.", "tasks.json e invalido. Nenhuma alteracao foi gravada.", "Tasks nao Compose com rotulos conflitantes foram mantidas: …").
  - Funciona local e remoto (descoberta no host, PTY no host).
- Flutter, aba de saida read-only: `TaskOutputSession` em `pane_view.dart:1879+`, `gsh_viewmodel.dart:1160` (`openTaskOutput`).
- Flutter, motor: `data/tasks/` (`task_discovery_impl`, `npm_adapter`, `flutter_adapter`, `compose_tasks`, `tasks_json_loader`, `pty_task_runner`, `task_process_registry`), `data/remote/remote_task_gateway.dart`.
- Strings pt-BR ausentes no Rust: `gsh.tasksPanel.*` (18) e `gsh.tasks.*` ("Hot reload", "Hot restart", "Alternar debug paint", "Alternar plataforma", "Sair").
- Rust: nada. `RightTab` so tem Files/Search/Git/Database/Telemetry/Gallery (`right_tabs.rs:13-20`); nenhum `tasks_panel`, runner ou discovery. O unico rastro e o template da Galeria:
  - `gallery_template.rs:72` cria `.gsh/tasks.json`, que nada le.
  - `labels.rs:1218` descreve "Comandos para rodar pelo painel de Tasks…", e esse painel nao existe.

### F2. HTTP workbench (`.http` / `.rest`) (confirmado por grep: nao existe)
- Flutter: `http_request_view.dart:31-716`, roteado em `pane_view.dart:1750-1763` para `.http`/`.rest`.
  - Barra 36px com icone `swap_horiz`, seletor de request ("Selecionar request", "N requests"), botao Executar/"Executando…" com "⌘↵".
  - Editor + divisor 7px + resposta com toggle JSON/Headers/Text e rodape 26px.
  - Salvar como ("Salvar request como"), erro "Nao foi possivel salvar".
  - Erros tipados: "Falha no request", "Nenhum request encontrado na posicao do cursor.", variavel sem valor, arquivo de corpo ausente, timeout, resposta grande demais, "Corpo da resposta vazio.", "truncado (resposta grande demais)".
  - Parser `http_document` e `http_response_result` no domain.
- Rust: `.http` abre como arquivo de texto comum. O unico tratamento especial de extensao e `.dbq` (`main.rs:3886-3892`). Existem apenas o icone (`file_icons.rs:44`) e o template da Galeria (`gallery_template.rs:56,91`, `labels.rs:1190` "Requests HTTP"), que cria um arquivo sem executor.
- As strings `gsh.httpView.*` (cerca de 25) nao existem no Rust.

### F3. Verbos da CLI interna para db/redis/mongo/http/tasks
- Flutter `cli/src/main.rs:68-76`: `list-tasks`, `read-task`, `run-task`/`stop-task`/`restart-task`/`send-task-key`, `db`, `telemetry`, `http`, `redis`, `mongo`.
- Rust `crates/cli/src/args.rs:352-383`: so `telemetry` entre os de dados. `db`, `redis`, `mongo`, `http` e todos os verbos de task caem em `parse_open` (tratados como caminho de arquivo).
- Impacto: as skills `gsh-db`, `gsh-cli` (tasks) e `gsh-design` (.http) dependem desses verbos. Nao e UI, mas e a mesma area funcional.

### F4. Banco em workspace remoto
- Flutter: `data/remote/remote_db_executor.dart` e `remote_db_writer_impl.dart`. O painel usa `treeRootPath` do host e a senha mora no cofre do host.
- Rust: nao ha executor remoto.
  - `ConnectionDialog::with_remote_workspace` (`db_dialog.rs:223`) nunca e chamado, entao a dica "A senha fica salva no host, nao nesta maquina." (`labels.rs:964`, Flutter `db_connection_dialog.dart:780-791`) nunca aparece.
  - `DB_PASSWORD_REQUIRED_ON_HOST` (`labels.rs:917`) esta sem uso.
  - `db_error_message` (`labels.rs:1105-1131`) nao tem o ramo `password_required` do Flutter (`db_error_message.dart:26`).

### F5. Texto selecionavel na aba de caso da Telemetria
- Flutter usa `SelectableText` em: mensagem (`telemetry_case_view.dart:197`), `shortId` (`:450`), frames sem link (`:774`), valores da tabela de log correlacionado (`:849`), linhas de contexto (`:993`) e erro.
- Rust usa `text(...)` simples em todos (`telemetry_case_view.rs`: `message`, `meta_wrap`, `frame_row`, `correlated_log_section`, `context_section`). Nao da para selecionar nem copiar.

### F6. Tooltip com a chave completa na celula de chave do Redis
- Flutter `db_redis_table.dart:755`: `AppTooltip(message: entry.key)`.
- Rust `db_redis_view.rs:294-309`: botao sem tooltip. A chave e cortada sem reticencias e sem como ver inteira.

### F7. Esc cancela editores (Mongo e Redis) e blur confirma (Redis)
- Flutter: `_escapable` + `KeyDownEvent` Esc em editor de documento, draft, celulas inline e expandido (`db_mongo_view.dart:608-621`, `db_redis_table.dart:995-1008`). Na celula inline, perder o foco confirma (`db_redis_table.dart:979-983`).
- Rust: nenhuma assinatura de Esc para esses editores (so `cancel_create` do arquivo, `main.rs:8618`). No lugar ha os botoes ✓/✗ (ver I2).

### F8. Auto-reaplicar filtro/pattern ao esvaziar o campo
- Flutter: listener `_onFilterCleared` (`db_mongo_view.dart:109-113`) e `_onPatternCleared` (`db_redis_table.dart:140-144`) reaplicam quando o texto fica vazio por qualquer via (X do `InputFeature.clear`, Backspace).
- Rust: so o botao ✕ envia `ClearFilter`/`ClearPattern`. Apagar com o teclado ate esvaziar nao recarrega. Nao verifiquei o corpo de `FilterChanged`, entao fica como risco.

---

## 2. DIFERENTE

### D1. Indicador de carregamento: spinner no Flutter, texto "Loading…" no Rust
- Mongo: Flutter `db_mongo_view.dart:392-403` (`CircularProgressIndicator` 16x16, padding 14). Rust `db_mongo_view.rs:197-208` (texto `Loading…` 11.5).
- Redis: Flutter `db_redis_table.dart:634-645`. Rust `db_redis_view.rs:236-247` (mesmo problema).
- Aba de caso da Telemetria enquanto carrega: Flutter `telemetry_case_view.dart:152-156` (spinner 18x18 strokeWidth 2). Rust (`telemetry_case_view.rs:30-47`) renderiza o caso parcial sem detalhe ou o vazio.
- Run vivo no painel de Telemetria: Flutter `telemetry_panel.dart:431-438` (spinner 9x9, strokeWidth 1.5). Rust `telemetry_panel.rs:515-528` desenha bolinha solida 7px cor `online`.
- Arvore de schema / colunas / collections: ambos usam texto "Loading…" (IGUAL, em ingles nos dois: `db_panel.dart:444`, `db_panel.rs:911`).

### D2. Bordas: o Flutter desenha so a borda de baixo, o Rust desenha caixa inteira
O `Border` do iced tem uma unica largura para os quatro lados. Cada item abaixo desenha linha em cima, esquerda e direita que o Flutter nao tem:
- Mongo: toolbar (`db_mongo_view.rs:137-145` vs `db_mongo_view.dart:280-282`) e cabecalho do card do documento (`rs:284-294` vs `dart:506-508`).
- Redis: toolbar (`db_redis_view.rs:143-151` vs `dart:485-487`), header de colunas (`rs:215-223` vs `dart:590-593`), linhas (`rs:423-434`, largura 0.5 em caixa inteira, mais `padding [1,0]` em `:437`, vs `dart:739-741` so bottom, `minHeight 27`), editor expandido (`rs:503-511` vs `dart:808-810`) e linha de draft (`db_redis_view.rs` `draft_row`, mesma caixa inteira).
- Telemetria: sem diferenca nesse ponto (usa `hairline_h`).

### D3. Altura dos editores inline
- Mongo editar: Rust fixo 220px (`db_mongo_view.rs:302`). Draft: fixo 180px (`:367`). Flutter cresce de 3 a 20 linhas (`db_mongo_view.dart:446-447, 574-575`).
- Redis expandido: Rust fixo 180px (`db_redis_view.rs:460`). Flutter 3 a 12 linhas (`dart:827-828`).
- Redis draft: Rust 34px string / 110px composto (`db_redis_view.rs` `value_height`). Flutter 1 linha string / 6 linhas composto (`dart:936`).

### D4. Flash de confirmacao do Redis
- Flutter 700 ms (`db_redis_table.dart:205`). Rust 900 ms (`theme.rs:951` usado em `db_redis.rs:277`).

### D5. Constantes de tema mortas que contradizem o Flutter
Nao afetam a tela (o codigo usa literais corretos), mas enganam quem for ajustar:
- `theme.rs:946-950, 953-964`: `REDIS_TOOLBAR_HEIGHT 38`, `REDIS_HEADER_HEIGHT 28`, `REDIS_ROW_HEIGHT 32`, `REDIS_KEY_COL_WIDTH 220`, `REDIS_TTL_COL_WIDTH 78`, `REDIS_ACTIONS_COL_WIDTH 40`, `MONGO_TOOLBAR_HEIGHT 38`, `MONGO_CARD_RADIUS 6`, `MONGO_CARD_PADDING`, `MONGO_EDITOR_MIN/MAX_HEIGHT`. O Flutter usa toolbar 34, header 26, linha min 27, chave 170, ttl 64, acoes 30, card radius 7.
- `theme.rs:915-927, 838`: `DB_DIALOG_SSH_RADIUS 8`, `DB_DIALOG_SSH_HEADER_PADDING`, `DB_DIALOG_SSH_BODY_PADDING`, `DB_DIALOG_SSH_PORT_WIDTH`, `DB_DIALOG_SSH_BADGE_*`, `DB_DIALOG_SSH_SUBTITLE_SIZE`, `DB_DIALOG_SSH_LABEL_SIZE` e `DB_DIALOG_MAX_WIDTH` ficaram sem uso. Descrevem outro layout de SSH (selo, subtitulo, raio 8) que o Flutter nao tem.

### D6. Chip "GSH Desktop" da Telemetria
- Flutter renderiza com contador `0` (`telemetry_panel.dart`, `_Chip(label: tr.chipApp, count: 0 …)` no `_Header`).
- Rust omite o contador (`telemetry_panel.rs:222-229`, `None`). Mostra "GSH Desktop" sem numero.

### D7. Botao "Copiar comando da CLI" (caso)
- Flutter: so copia; rotulo mono `gsh telemetry show <id>` sempre fixo (`telemetry_case_view.dart:246-255, 566-573`). A chave i18n `copied` nao e usada em lugar nenhum do Flutter.
- Rust: troca o icone por ✓ e o rotulo por "Copiado" apos copiar (`telemetry_case_view.rs`, `action_bar`, `copied`). Ver tambem I5.

### D8. Cor do texto do botao primario
- Flutter `Colors.white` fixo (`telemetry_case_view.dart:547`). Rust `theme::on_color(p.accent)` (`telemetry_case_view.rs` `action_btn`). Muda em tema com acento claro.

### D9. Linhas de frame do Stack
- Flutter: padding vertical 0, `height: 1.65` (`telemetry_case_view.dart:767-770, 806`).
- Rust: padding vertical 2 + `TELEMETRY_FRAME_LINE_HEIGHT` (`telemetry_case_view.rs`, `frame_row`). Linhas ficam mais altas.

### D10. Truncamento por estimativa de caracteres
- Painel e caso da Telemetria usam `flutter_ellipsis(texto, n_chars)` com largura estimada em `0.60 * fonte` por caractere (`telemetry_panel.rs:399, 481-494, 606-619`; `telemetry_case_view.rs` `CRUMB_MAX_CHARS 148`, `CONTEXT_BODY_MAX_CHARS 136`). Flutter mede de verdade e quebra por largura (`maxLines: 3` nas linhas de contexto). Resultado diverge quando a fonte ou o painel mudam de largura.
- Painel de DB: nome, alvo, tabela e colunas sao cortados com `clip` sem reticencias (`db_panel.rs` cabecalho do arquivo diz "o iced nao as desenha"). Flutter usa `TextOverflow.ellipsis`.

### D11. letterSpacing nao aplicado
- iced nao suporta. Afeta: "BANCO DE DADOS" (1.1, `db_panel.dart:216`), cabecalhos CHAVE/VALOR/TIPO/TTL (0.6, `db_redis_table.dart:583-587`), pills de origem/tag (0.4), cabecalhos de secao e de stats da Telemetria (0.6 e 0.8), "TAREFAS". Visualmente o texto fica um pouco mais estreito.

### D12. Botao de seletor de conexao `.dbq`
- Flutter `HoverTap(color: panel3)` com hover default (`db_query_view.dart:503-507`). Rust troca o fundo para `panel` ao passar o mouse (`db_query_view.rs:169-186`). Nao confirmei a cor de hover do `HoverTap`; marcar para teste visual.

### D14. "Ver no terminal" na aba de caso
- Flutter: aparece quando `repRun.taskKey != null` e abre a aba read-only da task (`gsh.openTaskOutput`) (`telemetry_case_view.dart:237-245`).
- Rust: aparece quando `rep_run.pane_id` existe e foca o terminal daquele pane (`telemetry_case_view.rs` `action_bar`, `TelemetryMsg::ShowInTerminal` -> `FocusTerminalPane`, `telemetry_state.rs:645`). Sem tasks no Rust a condicao de exibicao e a acao mudaram.

### D15. Switch e campos do dialogo de conexao
- Flutter usa `Switch` e `TextField` do shadcn (`db_connection_dialog.dart:296, 446`). Rust usa `switch_row` e `text_input` proprios com padding `[7,10]` e raio 6 equivalentes. Medidas de papel batem; aparencia do switch nao pude comparar sem renderizar.

### D16. Aba de Telemetria sem workspace
- Rust tem um estado "Abra um workspace para ver a telemetria dele." (`telemetry_panel.rs:78-80`). No Flutter a aba e escondida nesse caso e a chave i18n `noWorkspace` nao e usada na UI. Em pratica so aparece em borda; ver I6.


---

## 3. INVENTADO no Rust (nao existe no Flutter)

### I1. Cor por tipo na coluna TIPO do Redis
- Rust: `kind_color()` com `REDIS_KIND_STRING_COLOR` verde, `HASH` azul, `LIST` roxo, `SET` ambar, `ZSET` esmeralda, `STREAM` rosa (`db_redis_view.rs:400-405, 700-709`; `theme.rs:936-941`).
- Flutter: sempre `colors.text3` (`db_redis_table.dart:775-778`).

### I2. Botoes ✓ e ✗ nas celulas inline do Redis
- Rust `inline_editor` (`db_redis_view.rs:608-640`) mostra ✓ (`online`) e ✗ ao lado do campo.
- Flutter: campo nu. Enter ou blur confirma, Esc cancela (`db_redis_table.dart:968-992`).

### I3. Chevron no botao de tipo do draft Redis e largura minima do menu
- Rust `db_redis_view.rs:517-530` (texto + `ICON_EXPAND_MORE`, menu `120.0`).
- Flutter: `OutlineButton(child: Text(d.kind.label))` sem icone (`dart:915-918`), menu com `minWidth` padrao 200.

### I4. Dicas de placeholder nos campos SSH
- Rust `labels.rs:954` "bastion.example.com" e `:958` "ubuntu" aplicados em `db_dialog.rs:875, 898`.
- Flutter: Host SSH e Usuario SSH sem placeholder (`db_connection_dialog.dart:631, 643-646`); so a porta tem a dica "22" (igual).

### I6. Estado "sem pasta" no painel de DB e "sem workspace" na Telemetria
- Rust `db_panel.rs:36-38` mostra `FILES_NO_FOLDER` ("Nenhuma pasta — abra um workspace.") quando `db.root()` e nulo.
- Flutter: o painel so existe com projeto selecionado e usa o estado "Nenhuma conexao ainda." (`db_panel.dart:240-250`).

### I7. ✕ de limpar no campo de busca da Telemetria
- Rust `telemetry_panel.rs:167-185` adiciona um botao ✕ quando ha texto.
- Flutter: `TextField` simples sem `InputFeature.clear` (`telemetry_panel.dart:244-255`).

---

## 4. IGUAL (verificado arquivo contra arquivo)

Painel Banco de dados (`db_panel.dart` x `db_panel.rs`):
- Header "BANCO DE DADOS" com refresh e "+".
- Menu de motores na ordem SQLite, Postgres, MySQL, SQL Server, Redis, MongoDB com icone de marca, `minWidth 200`.
- Menu da conexao: Editar…, Copiar nome, Nova query (so SQL), Ver chaves (so Redis), divisor, Excluir (danger).
- Excluir conexao: titulo "Excluir conexao" e mensagem completa. So conexoes registradas sao removidas.
- Rodape ".gsh/databases.json · 1 conexao" e "· N conexoes".
- Estado vazio "Nenhuma conexao ainda.".
- Linha da conexao: chevron ou ponto, icone 13px, nome 12.5, "Motor · alvo" mono 10, "…" 14px.
- Chips `detected`, `local`, `rw`, `no agents` (borda `border2`, raio 4, 9.5).
- Arvore SQL: agrupa por schema so se houver mais de um, "(default)", contagem, tabela vs view, "+" com tooltip "Nova query", colunas com tipo e "PK".
- Arvore Mongo: databases (quando a URL nao fixa um) e collections, com os hints "Loading…", "No tables.", "No databases.", "No collections.".
- Nova query de tabela: `SELECT * FROM <schema.tabela> LIMIT 100;`.
- Ordem alfabetica das conexoes.

Dialogo de conexao (`db_connection_dialog.dart` x `db_dialog.rs`):
- Titulo "Nova conexao"/"Editar conexao" com icone 18px.
- Campo Nome com dica `dev-local`.
- SQLite: campo Arquivo com picker e caminho relativo, e nome sugerido.
- Rede: Connection string com placeholder `esquema://user:password@host:porta/database`, altura 96 (min 58, max 260), alca de resize e duplo clique reseta, erro "URL de conexao invalida." em vermelho.
- "Salvar senha" extrai a senha da URL e mostra o campo Senha com dica `*******` na edicao.
- Secao "Tunel SSH": colapsada com cadeado e chevron, expandida com ✕ e campos Host SSH, Porta SSH (dica 22), Usuario SSH, Chave privada via picker, abertura em `~/.ssh` e encurtamento para `~/…`.
- Semeia usuario e chave padrao na primeira abertura.
- Passphrase so aparece se a chave e criptografada, com "Salvar senha da chave".
- Switches "Permitir escrita (agentes)" e "Visivel para agentes" com as dicas.
- Linha de teste "Testando conexao…", "Conexao OK" e o erro.
- Botoes: Excluir (so edicao, a esquerda) | Cancelar | Testar | Salvar/Adicionar. Testar usa a senha guardada pelo nome original quando o campo esta vazio.
- Conferido tambem: largura de conteudo 600.

Aba `.dbq` (`db_query_view.dart` x `db_query_view.rs`):
- Barra 36px, seletor de conexao ("Selecionar banco de dados" em `warn`), chip do motor, botao Executar / Executar selecao / Executando… com "⌘↵" (alfa 0.7, fundo `panel3` sem conexao).
- Divisor 7px e split 0.12-0.85, editor min 90 / resultado min 110.
- Erro com icone, `kind` e mensagem selecionavel.
- Dicas "Escolha um banco de dados acima…" e "Execute a query (⌘↵)…", "Nenhuma linha.", "N linhas afetadas" (singular/plural).
- Rodape 26px "N linhas · truncado (aumente -- limit) · X ms", toggle Tabela/JSON, "nao salvo"/"salvo" (`warn`/`text4`).
- Grade: indice 44, header 34, linha 26, larguras base `7.5*chars+26` clamp 64-340, drag de coluna 60-2000, scrollbar 12, NULL em italico, `⟨blob · N B⟩`, numeros a direita.
- JSON: preview 500 linhas, banner "Preview limited to the first…", botao Copiar/Copiado no canto.
- Dialogs "Salvar query como" (padrao `query.dbq`) e "Nao foi possivel salvar".
- Ctrl+Enter e Ctrl+S.

Mongo (`db_mongo_view.dart` x `db_mongo_view.rs`):
- Toolbar 34 com icone 14, "conexao / colecao", filtro (max 340, dica `Filtro — JSON, ex.: {"status": "active"}`), contador "N docs+", Atualizar, Inserir documento.
- Banner de erro (alfa 0.12), cards (raio 7, cabecalho `_id: …`, Editar e Excluir em 13px).
- Confirmacao "Excluir documento" com a mensagem exata.
- Rascunho `{\n  \n}` com fundo `accentSoft` alfa 0.25 e botao "Inserir".
- "Carregar mais", "Nenhum documento nesta colecao.", "Nenhum documento corresponde a este filtro.".
- Titulo da aba "colecao (conexao)".

Redis (`db_redis_table.dart` x `db_redis_view.rs`):
- Larguras 170 / 78 / 64 / 30 e header 26, toolbar 34, busca 240 com dica `Buscar — padrao, ex.: user:*`.
- Contador "N chaves+", "∞" no TTL, zebra alfa 0.35, flash alfa 0.14.
- Textos: "Nenhuma chave neste banco de dados.", `Nenhuma chave corresponde a "…".`, "Carregando valor completo…", "Adicionar chave", dicas `chave`, `ttl (s, opcional)`, `valor`, `{"field": "value"}`, `[{"value": "member", "score": 1}]`, `["item1", "item2"]`.
- Excluir chave com a mensagem exata.
- Tipos STRING/HASH/LIST/SET/ZSET/OTHER.

Telemetria (`telemetry_panel.dart`, `telemetry_case_view.dart` x `telemetry_panel.rs`, `telemetry_case_view.rs`):
- Header (titulo, "N ativos" com ponto de erro, tamanho em KB/MB, busca "Filtrar casos", chips Abertos/Novos/Resolvidos/Ignorados/"⚠ Avisos", chip "GSH Desktop" so em modo desenvolvedor).
- Arvore projeto -> run -> caso, com modo plano quando ha uma raiz so, indentacoes 10/22 e 24/36, pills TASK / GSH TELEMETRY / APP.
- Caso: bolinha por severidade/status, tags NOVO/REGRESSAO/RESOLVIDO/IGNORADO, local, "agora/ha N min/h/d", "×N", acoes rapidas ao passar o mouse (Marcar resolvido, Ignorar…, Reabrir, Limpar ocorrencias), limpar run.
- Aba de caso: padding (26,22,26,40), max 980, migalhas, pill do tipo, mensagem mono 17, codigo curto, status, `fingerprint = tipo + mensagem normalizada + <loc>`, blame, botoes (Abrir <local>, Copiar comando da CLI, Marcar resolvido, Ignorar…, Reabrir, Limpar ocorrencias), stats em 4 celulas (Ocorrencias "em N runs", Primeira, Ultima, Origem com PTY/stdout), secoes Stack, Log correlacionado, Ocorrencias por run (barra 6px, "atual", "exit N", " →"), Contexto cru, e os hints de cada secao.
- Titulo da aba "tipo · arquivo".

