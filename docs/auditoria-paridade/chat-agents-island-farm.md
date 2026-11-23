# Auditoria de paridade Flutter -> Rust: chat, agentes, ilha, fazenda, grafo, galeria, time

Somente leitura. Verdade = `GSH Desktop/lib/app/gsh/ui/widgets/` (F) e `lib/i18n/pt_BR.i18n.json`. Rust = `GSH/crates/app/src/` (R).
Caminhos abreviados: F = `/home/heliodev/projetos/GSH Desktop/lib/app/gsh/ui/widgets/`, R = `/home/heliodev/projetos/GSH/crates/app/src/`.
Classes: IGUAL / DIFERENTE / FALTA (no Rust) / INVENTADO (no Rust, sem equivalente no Flutter).
Ordenado por impacto visível ao usuário. Nenhum dos dois apps foi executado: tudo vem de leitura de código (medidas e textos exatos citados onde diferem). "Por leitura" = código conferido, pixel não conferido.

---------------------------------------------------------------------
## A. Achados de maior impacto (ordem de gravidade)
---------------------------------------------------------------------

### A1. Chat: a transcrição não acompanha o fim do stream  [FALTA]
- F `chat/chat_pane.dart:96-121`: `_changed()` rola para `maxScrollExtent` se o usuário já estava a menos de 80px do fim (não puxa de volta quem rolou para cima).
- R `chat_view.rs:385`: `scrollable(centered)` sem id, sem `anchor_bottom` nem `snap_to`. Grep em `chat_*.rs`/`main.rs` por `snap_to|scroll_to|anchor_bottom|stick` = zero. A resposta cresce abaixo da dobra e o usuário precisa rolar à mão.

### A2. Fazenda: fonte de dados e comportamento completamente diferentes  [DIFERENTE/FALTA]
O Flutter (`farm/farm_view.dart`, `FarmViewModel`) mostra "todo loop do Claude Code nesta máquina" (loops principais + subagentes + sessões fora do GSH), com tokens, contexto, atividade e minis. O Rust (`main.rs:5278-5310`, `farm/agents.rs`) só alimenta a cena com abas de terminal observadas por hook e com abas de chat ("Chat {id} · pasta"), e só tem 4 estados (Idle/Working/Waiting/Done).
Lista completa na seção B (tooltip no hover, clique que revela/retoma a sessão, arrastar canteiro + "Redefinir posições", tokens reais, subagentes, bichinhos, passeio/sono, pinch-zoom, persistência do cenário, título da aba, cenário padrão).

### A3. Ilha de agentes: abre só por clique; faltam hover, abertura automática, ícone do harness, chats, ilha flutuante Linux  [FALTA/DIFERENTE]
Ver seção D. F abre em hover (timer de saída 180ms), em clique (fixa até clicar fora) e sozinha enquanto houver pedido de permissão; R só alterna no clique (`island.rs:521-541`, `IslandMsg::TogglePanel`). R também mostra o título com sufixo interno `· t3` e não inclui chats.

### A4. Chat: cartões de permissão/pergunta/plano com outro layout, outros textos e somem ao responder  [DIFERENTE]
- F `chat_prompt_cards.dart:10-62` (`_PromptFrame`: fundo panel2, borda `warn` a 55% enquanto pendente, raio 10, padding 14/12/14/14, ícone + título 14px/600) e `366-440` (`ChatPermissionCard`: título "Permitir ${tool}?", caixa mono 12px com o resumo, diff `-`/`+` para Edit/Write, botões Permitir / "Sempre permitir ${rule}" / Negar; depois do clique fica uma linha de resultado "Permitido" / "Negado" / "Sempre permitido: ${rule}").
- R `chat_view.rs:576-649`: texto simples `"{tool}: {description}"` (peso médio) + três botões ghost (`Permitir`, `Sempre permitir` SEM a regra, `Negar`); sem diff, sem moldura âmbar, sem estado final: `chat_pane.rs:760-770` faz `items.retain(...)` e o cartão desaparece.
- Plano: F `265-360` título "Plano do agente", botões "Aprovar e executar" (primário) / "Continuar planejando" (outline); o refino abre textarea 2-6 linhas com placeholder "O que mudar no plano?", botões "Enviar ajuste"/"Cancelar"; resultado "Plano aprovado" / "Pediu para continuar planejando" / "Ajuste: ${text}". R `chat_view.rs:747-804`: título "Plano proposto", "Aprovar plano"/"Refinar plano", campo de uma linha "O que você gostaria de ajustar?" sempre visível, item some após decidir (`chat_pane.rs:1163-1196`).
- Pergunta: F `77-260` título "O agente está perguntando", ícone rádio/checkbox por opção com descrição embaixo, campo "Outro…", botão primário "Responder" (desabilitado até todas respondidas), resultado "→ resposta" em verde. R `chat_view.rs:650-746`: título "O Claude precisa de algumas respostas", opções como botões de texto com glifos ○ ◉ ☐ ☑, campo "Outra resposta…", botão "Enviar resposta" sempre habilitado.

### A5. Chat: barra de ação (copiar + "há 5 min") embaixo das mensagens e texto selecionável  [FALTA]
F `chat/chat_items.dart:145-229` (`_MessageActions`): ícone copiar (14px, vira check por 1,5s), tooltip "Copiar"/"Copiado", tempo relativo com tooltip de data; mensagem do usuário alinhada à direita, assistente à esquerda; sempre visível na última resposta, no hover nas outras (`chatRowActions`, `118-140`). Texto de usuário/ferramenta/pensamento é `SelectableText`. R: nada disso (grep `Copiar|copy_message|CopyMessage` em `chat_view.rs`/`chat_pane.rs`/`labels.rs` não acha equivalente de chat). Texto do iced não é selecionável.

### A6. Chat: anexos sem miniatura, sem prévia de imagem, sem destaque de drop; não dá para enviar só anexo  [DIFERENTE/FALTA]
- F `chat_attachments.dart:191-245` imagem enviada vira prévia acima do balão (máx 360x320, raio 10, borda border2, clique abre ampliada); `247-311` anexo pendente = miniatura 64px com "x" no canto (imagens) + fichas (thumb 24px/ícone pdf/doc + nome máx 200px + x); `48-96` `ChatDropZone` pinta overlay `accentSoft` com borda `accent` 2px enquanto arrasta.
- R `chat_composer.rs:133-150`: lista de botões de texto `"{nome} ×"` (12.5px). Mensagem enviada não mostra anexo algum (`view_user_bubble`, `chat_view.rs:859-878`, só texto). `chat_pane.rs:1838-1849` aceita `FileDropped` (um arquivo por evento) sem feedback visual. `chat_composer.rs:196-205`: botão enviar só habilita com texto, então um anexo sozinho não envia (F `chat_pane.dart:81-88` envia com `attachments.isNotEmpty`).

### A7. Chat: markdown renderizado com tema escuro fixo  [DIFERENTE]
R `chat_view.rs:762` e `:897`: `markdown::Style::from(iced::Theme::Dark)` — não segue o tema do app (tema claro fica ilegível). F `agent_markdown.dart` usa `GptMarkdownThemeData` com `colors.*`/`typo.*` (h1 22, h2 18, h3 16, h4 14.5, h5 13.5, h6 12.5), bloco de código com botão copiar, frontmatter YAML.

### A8. Grafo: menu do nó, menu do cabo, linha de arraste, pulsos e botões de time inventados  [DIFERENTE/FALTA/INVENTADO]
Ver seção E (E2, E3, E4, E6, E7).

---------------------------------------------------------------------
## B. Fazenda (F `farm/`, R `farm/` + topbar)
---------------------------------------------------------------------

| # | Elemento | Classe | Flutter | Rust |
|---|---|---|---|---|
| B1 | Título da aba | DIFERENTE | `gsh.farm.tabTitle: "Fazenda"` | `labels.rs:570` `FARM_TAB_TITLE = "Cenário"` (usado em `pane_model.rs:81`) |
| B2 | Cenário padrão | DIFERENTE | `app_settings.dart:104` `farmScenery = FarmScenery.farm` | `farm/mod.rs` `FarmPane::default` -> `Mode::Office` |
| B3 | Persistência do cenário | FALTA | alternar grava `farm.scenery` (`farm_view.dart:396-397`) | `Message::Office/Farm` só muda `self.mode` em memória (`farm/mod.rs:243-250`) |
| B4 | Faixa do colega (modo fazenda) | DIFERENTE | `peerBanner: "Fazenda de ${name}"`; no escritório sem faixa (`farm_view.dart:370-374`) | `labels.rs:850` `farm_peer_banner` = `"TIME · {name}"` |
| B5 | HUD: tokens | DIFERENTE | "{n} tokens gastos" com soma real dos loops (`farm_hud.dart:56-72`) | `farm/mod.rs:355-361` `tokens: 0` fixo (comentário "Sem fonte de tokens") |
| B6 | HUD: "N agentes · M subagentes" | DIFERENTE | agentes = loops menos subagentes | `subagents: 0` fixo; agentes = abas |
| B7 | HUD: "Redefinir posições" | FALTA | chip (10px, `_hudMuted`) com canteiro fixado (`farm_hud.dart:128-146`) | não existe |
| B8 | Arrastar canteiro para trocar vaga + lembrar por pasta | FALTA | `farm_view_plots.dart` (`farm.plots`) | não existe |
| B9 | Tooltip no hover do bicho | FALTA | `farm_view.dart:446-486`: título, "pasta · Harness", agentType, "colega de time de X"/"lidera um time de N", atividade (pensando/lendo/editando/rodando comando/pesquisando na web/delegando/usando ferramenta/esperando você/ocioso/dormindo/concluído) + ferramenta, "Gastou N tokens", "entrada · cache · gerados", "contexto X de Y (P%)", "Clique para abrir o terminal" | só o nome no hover (`farm/mod.rs:865-872`); strings `activity`, `spent`, `context` não existem no Rust (grep) |
| B10 | Clique no bicho | DIFERENTE | `revealAgentSession` (foca aba; sessão parada é retomada numa aba nova; fora do GSH: toast "Este agente está rodando fora do GSH Desktop. O terminal dele está em outra janela.") (`farm_view.dart:421-433`) | `Message::Focus(pane_id)` só foca terminal local |
| B11 | Bichinhos (cachorro/galinha/gato) e carinho (coração) | FALTA | `farm_pets.dart`, `farm_view.dart:310-316` | não existe (só o ícone `Heart` em `icons.rs`) |
| B12 | Movimento: ociosos passeiam no celeiro, dormem no curral, subagente comemora e some | FALTA | `farm_critter.dart` (`CritterMode.work/wait/wander/sleep/done`), `farm_nav.dart` | `characters.rs`: posição fixa por canteiro, braços fixos (o próprio código diz "a animação de digitação vem com o relógio da cena") |
| B13 | Minis de subagentes da máquina local | FALTA | em volta do canteiro do pai | `characters::draw_minis` só roda para `peer.subagents` (colegas) |
| B14 | Título do canteiro (plaquinha) em todo zoom | DIFERENTE | `farm_scene_overlay.dart:42-75` | só nome do agente (hover) |
| B15 | Pinch-zoom | FALTA | `onScaleUpdate` com `pointerCount` | só roda do mouse |
| B16 | Cursor "grab" sobre canteiro | FALTA | `farm_view.dart:278-284` | `mouse_interaction` padrão |
| B17 | Estado vazio (título + hint) | IGUAL | `farm.emptyTitle/emptyHint` | `labels.rs:666-667` |
| B18 | Seletor Fazenda/Escritório | IGUAL | `_SceneryToggle` | `farm/mod.rs:337-360` |
| B19 | HUD visual (caixa 2px tinta, chama, quadrados de status) | IGUAL | `farm_hud.dart` | `farm/hud.rs` |
| B20 | Mundo fazenda/escritório (sprites, props, layout, câmera, zoom `exp(-dy*0.0018)`, duplo clique recentra) | IGUAL por leitura (há goldens Dart no Rust) | `farm_world*.dart`, `office_*`, `farm_camera.dart` | `farm/world*.rs`, `office*`, `camera.rs` |
| B21 | Mensageiros do time (courier) e cartas | IGUAL por leitura | `farm_courier.dart` | `farm/courier.rs` |
| B22 | Cartões "Batendo na porta": textos | IGUAL | `farm.doorTitle/doorMessage/doorFile/doorOpen/doorRefuse` | `labels.rs:840-849` |
| B23 | Cartões da porta: onde aparecem | DIFERENTE | `gsh_page.dart:971-980` `Positioned(right:16,bottom:16)` na página inteira, sempre | `farm/mod.rs:385-405` só dentro da aba da fazenda; fora dela a batida é invisível |
| B24 | Cartões da porta: prévia | DIFERENTE | `maxLines: 4`, elipse, `typo.label` | `farm/mod.rs:448-453` altura fixa 56px, `FONT_BODY_MEDIUM` 12px |
| B25 | Mini-fazenda da topbar: visibilidade | DIFERENTE | `farm_strip.dart:60` some sem loops | `topbar.rs:249-251` sempre visível (comentário admite) |
| B26 | Mini-fazenda: pílula "N precisam de você" | DIFERENTE | clique abre menu com a fila (título, "pasta · há quanto espera · subagentes", ponto âmbar pulsante) e foca a aba (`farm_strip.dart:89-130`) | `topbar.rs:275-311` clique abre a fazenda; ponto estático |
| B27 | Mini-fazenda: zeros | DIFERENTE | número zerado em `text4` | `counter` com cor normal |
| B28 | Mini-fazenda: tooltip "{n} trabalhando · {m} precisam de você · {k} parados" | IGUAL | | `labels.rs:366-372` |

---------------------------------------------------------------------
## C. Chat (F `chat/*.dart`, `chat_*.dart`; R `chat_pane.rs`, `chat_view.rs`, `chat_composer*`)
---------------------------------------------------------------------

### C1. Cabeçalho
| Elemento | Classe | Flutter | Rust |
|---|---|---|---|
| Altura 38, borda inferior, ícone laptop, pasta em badge `panel3` raio 4 | IGUAL | `chat_pane.dart:215-262` | `chat_view.rs:98-190` |
| Renomear | DIFERENTE | clique no título abre `showRealmNameDialog` ("Renomear conversa", confirmar "Renomear") com tooltip `renameHint` ("Clique para renomear. O nome vale também no Retomar conversa, na fazenda e no claude --resume.") | edição inline com `✓` e `×` em texto (`chat_view.rs:100-119`); sem tooltip |
| Botão "Melhorar harness" (ícone `auto_fix_high`, tooltip `improveHarness` + hint, abre chat novo com `improveHarnessPrompt`) | FALTA | `chat_pane.dart:290-308`, `chat_view.dart:96-119` | grep `improve\|melhorar` em `chat_*.rs`/`main.rs` = zero |
| Selo do estado da sessão (Concluída / Pronta para revisão / Bloqueada / Falhou; ponto 6px, pílula raio 10, tooltip com detalhe) | FALTA | `chat_items.dart:464-505`, só com `classifySessionStates` ligado | — |
| Estado classificado como item da transcrição | INVENTADO | não existe | `chat_pane.rs:1315-1318` empurra `ChatItem::Classified`; `chat_view.rs:848-858` imprime `format!("{state:?}: {detail}")`, ex. "Completed: ..." (Debug do enum, em inglês) |
| Botão "Retomar conversa" | DIFERENTE | `GhostButton` com tooltip `resumeHint` ("Abrir numa aba nova uma conversa recente do Claude Code desta máquina, com o histórico.") e menu popup | painel inline de 150px sob o cabeçalho (`chat_view.rs:191-225`), sem tooltip |
| Botão "Remote Control" | DIFERENTE | tooltip `remoteControlHint` ("Continuar esta conversa num terminal com Remote Control (celular, claude.ai/code). O motor do chat não suporta Remote Control."); clique mostra toast | sem tooltip; mensagens entram na transcrição |
| Remote Control: sem sessão | DIFERENTE | "Mande uma mensagem primeiro: o Remote Control continua uma conversa que já existe." | "Ainda não há uma sessão para continuar no terminal." (`labels.rs:132`) |
| Remote Control: turno ocupado | DIFERENTE | "Espere o agente terminar este turno e tente de novo." | "Aguarde o turno terminar antes de continuar no terminal." |
| Remote Control: já ativo | DIFERENTE | "Esta conversa já está no Remote Control: continue no celular ou em ${url}" | "Esta conversa já está no Remote Control: {url}" |
| Remote Control: aviso ao passar | DIFERENTE | `handedOff`: "A conversa continua num terminal com Remote Control: abra no celular ou em claude.ai/code. O chat aqui fica pausado." | "A conversa continuará no terminal." (`labels.rs:135`) |
| Remote Control: falha ao abrir terminal | FALTA | "Não consegui abrir o terminal neste workspace." | não existe |
| Remote Control: canal do aviso | DIFERENTE | toast no canto inferior direito | item `Info` dentro da conversa (`chat_pane.rs:1014-1043`) |

### C2. Estado vazio e transcrição
| Elemento | Classe | Detalhe |
|---|---|---|
| Estado vazio (ícone 28, título 18, subtítulo, 3 dicas, "Retomar uma conversa") | IGUAL | F `chat_pane.dart:280-390`, R `chat_view.rs:240-352`; textos idênticos (`labels.rs:79-85`, `:119`) |
| Tela "Carregando a conversa…" com spinner 18 | FALTA | F `_Empty` com `loadingHistory`; R sem estado de carga |
| Margens da coluna | DIFERENTE | F `fromLTRB(side, 24, side, 32)`, `side = max(24, (largura-coluna)/2)` (`chat_pane.dart:144`); R `chat_view.rs:383` `padding([24,24])` (embaixo 24, não 32) com `max_width` na coluna |
| Espaçamento entre linhas | DIFERENTE | F `_gapAfter` (`chat_pane.dart:203`): usuário 28, ferramenta-ferramenta 4, resto 14; R `gap_between`: usuário 24 |
| Balão do usuário (máx 580, padding 12x7, raio 12, `panel3`) | IGUAL | F `chat_items.dart:231-285`, R `chat_view.rs:859-878` (sem anexos, ver A6) |
| Texto do assistente com cursor `▍` no stream | IGUAL | `labels.rs:92` |
| Linha de ferramenta | DIFERENTE | F `ChatToolLine` (`chat_items.dart:287-400`): mostra a `description` dada pelo agente; vazia => "Nome  resumo"; spinner 11px enquanto roda; chevron gira 90 graus em 120ms; hover `panel2`. R `view_tool_line` (`chat_view.rs:922`): sempre "Nome  resumo", glifo de texto como spinner, glifo trocado para o chevron |
| Detalhe da ferramenta (`$ comando`, resultado, 14 linhas) | IGUAL | F `_ToolDetails`, R `chat_view.rs:~1000-1050` (F corta com elipse, R corta linhas) |
| Visualização "Detalhado" (ferramentas já abertas) | FALTA | F `expanded: chat.view == TranscriptView.verbose`; grep `transcript_view` no R = zero (só `transcript_width`) |
| Cartão "Editou N arquivos  +A -B" | DIFERENTE | F `chat_edits_card.dart`: plural "Editou 1 arquivo"/"Editou N arquivos", ícone `difference_outlined`/`code`, `+A` verde `-B` vermelho mono 12.5, borda `border2` raio 8, diff por arquivo limitado a 40 linhas + "… mais N linhas". R `chat_view.rs:468-473`: `format!("{} {} arquivos   +{}  -{}", ✓, CHAT_EDITED_FILES, ..)` com `CHAT_EDITED_FILES = "arquivos editados"` => renderiza "✓ arquivos editados arquivos   +3  -1" (bug de string); sem plural, ícone, cor nem "… mais N linhas"; também agrupa edições negadas (F deixa de fora as com erro) |
| Raciocínio (thinking) | DIFERENTE | F `ChatThinkingBlock` (`chat_items.dart:402-462`): cabeçalho com lâmpada, "Pensando…" no stream / "Raciocínio" depois, chevron, aberto por padrão, texto itálico com barra lateral `border2` 2px. R `chat_view.rs:568-575`: parágrafo cinza 12px, sem cabeçalho nem recolher |
| Mensagem de outra sessão (peer) | DIFERENTE | F `_PeerCard` cartão `panel2`, barra `accent` 2px, ícone `forum_outlined`, "Mensagem de ${name}"; R `chat_view.rs:840-847` texto `"{from}: {texto}"` 12px |
| Aviso (`Info`) | DIFERENTE | F cor `warn` quando `warning`; R descarta o flag (`warning: _`, `chat_view.rs:834`) |
| Erros de turno (start/exit/turn) | IGUAL nos textos | `labels.rs:93-96` |
| Erro "anexo grande demais" | DIFERENTE | F "${name} é grande demais para enviar (imagem até 5 MB, PDF até 32 MB)." | R `chat/src/error.rs:41` "anexo grande demais: {name}" |
| Auto-seguir o fim | FALTA | ver A1 |
| Copiar/hora/texto selecionável | FALTA | ver A5 |
| Fonte do chat (`settings.chatFont`) | FALTA | F `chat_view.dart:70-75`; R sem a preferência |
| Largura da transcrição | IGUAL | R `main.rs:8802` usa `transcript_width.px()` |

### C3. Composer
| Elemento | Classe | Detalhe |
|---|---|---|
| Faixa de contexto (pasta mono 12.5 + branch) | IGUAL | F `chat_composer.dart:60-81`; R `chat_composer.rs:73-100` |
| Caixa de texto (raio 10, borda `border2`, 1-8 linhas) e placeholder | IGUAL | `labels.rs:86` |
| Atalhos Enter / Shift+Enter / Ctrl+V cola imagem | IGUAL | |
| Menu do `/`: busca | IGUAL | `chat_slash_menu.dart:102-120` x `chat_pane.rs:557-586` (4 faixas, máx 8, descrição só com 3+ letras) |
| Menu do `/`: visual | DIFERENTE | F cartão `panel2` borda `border2` raio 10, padding 4; linha com `/nome` mono 12.5, dica de argumento mono 12 `text4`, descrição elipsada `text3`, item ativo `panel3` raio 6 (`chat_slash_menu.dart:131-180`). R `chat_composer.rs:25-60`: uma string `"/{nome} {hint}  {descrição}"` 12px em botão ghost, contêiner sem fundo/borda, sem mono |
| Botão enviar/parar: habilitação | DIFERENTE | F envia com texto ou anexos; R só habilita com texto (`chat_composer.rs:196-205`) |
| Botão "Anexar arquivos" (+) | IGUAL | |
| Botão de sessões | DIFERENTE | F tooltip "Sessões", menu popup min 320 das últimas 24h (máx 15) + "Listar todas as sessões (pergunta ao agente)…"; R tooltip "Enviar para sessão…" (`labels.rs:130`), painel inline sob o composer (`chat_composer/details.rs:9-51`) |
| Seletor de modo | IGUAL | rótulos `Pedir permissão / Aceitar edições / Planejar / Ignorar permissões`; `warn` no bypass |
| Seletor de modelo | DIFERENTE | F lista plana de todos os modelos (nome 13px + descrição 11.5px `text3`), "Claude" quando desconhecido (`chat_model_usage.dart:28-105`). R `chat_composer/bar.rs:222-300`: 4 modelos "primários" com atalhos 1-4, divisor, submenu "Esforço", submenu "Mais modelos", selo "Requer créditos" (`app_menu.rs:214`) |
| Botão de esforço (ícone `speed` 15px + rótulo 12.5px; tooltip "Esforço: ${nível}" + hint "Quanto o modelo pensa antes de responder…") | FALTA | F `ChatEffortButton` (`chat_model_usage.dart:107-186`); R só submenu, e `CHAT_EFFORT_HINT`/`CHAT_EFFORT_TOOLTIP` existem em `labels.rs:106,113` mas não são usados |
| Anel de uso | DIFERENTE | F preenchimento `text2` (ou `warn` >=80%), trilho `border2`, mínimo 2%, tooltip "Uso", clique abre popover; R usa `accent_text` e abre painel inline sob o composer |
| Painel de uso | DIFERENTE | F `_UsagePanel` 300px, título "Uso", linha "Contexto: X de Y" + barra 5px e "%"; "Contexto: aparece depois da primeira resposta"; "Sessão (5 h)", "Semana", "${model} na semana"; "zera às HH:MM" / "zera em dd/mm às HH:MM"; "Uso do plano indisponível agora." R `details.rs:53-141`: sem barras; título "Uso da conta", "Sessão (5 horas)", "Contexto indisponível", "reinicia hoje às"/"reinicia em", "Uso da conta indisponível"; limite por modelo mostra só o nome do modelo, sem "na semana" |
| Custo no tooltip do modelo | IGUAL | `labels.rs:489` |
| Indicador de ocupado 13px | DIFERENTE | cor F `accent`/`warn`; R `accent_text`/`warn` |
| Tempo relativo ("há 5 min") | DIFERENTE | F `telemetryRelative`; R `time_ago` próprio (`chat_view.rs:226-239` com "há N d"; `details.rs:28-39` sem dias) |

### C4. Retomar conversa
| Elemento | Classe | Detalhe |
|---|---|---|
| Lista | DIFERENTE | F menu popup (min 360), 7 dias, até 30, exige caminho absoluto, "Nenhuma conversa nos últimos 7 dias." (`chat_resume.dart`); R painel inline "Conversas recentes" + "Nenhuma conversa recente." |
| Confirmação de conversa "viva" (<2 min) | DIFERENTE | F diálogo: "Esta conversa está aberta em outro lugar?" / "Ela mudou ${ago}. Se ainda estiver aberta (Claude Desktop, terminal), continuar aqui faz os dois escreverem na mesma conversa. Feche lá antes." / "Abrir mesmo assim". R `ResumePrompt` inline: "Esta sessão ainda pode estar ativa. Retomar mesmo assim?" / "Retomar mesmo assim" / "Cancelar" (`labels.rs:146-148`) |
| Onde abre | DIFERENTE | F `resumeChat` abre/foca outra aba; R `load_session` carrega na mesma aba |

### C5. Resto das strings que divergem
| Chave Flutter | Flutter | Rust |
|---|---|---|
| `planTitle` | "Plano do agente" | "Plano proposto" (`labels.rs:151`) |
| `planApprove` | "Aprovar e executar" | "Aprovar plano" |
| `planRefine` | "Continuar planejando" | "Refinar plano" |
| `planRefineHint` | "O que mudar no plano?" | "O que você gostaria de ajustar?" |
| `planSendRefine` | "Enviar ajuste" | inexistente |
| `questionTitle` | "O agente está perguntando" | "O Claude precisa de algumas respostas" |
| `other` | "Outro…" | "Outra resposta…" |
| `answer` | "Responder" | "Enviar resposta" |
| `allowAlways` | "Sempre permitir ${rule}" | "Sempre permitir" (sem regra) |
| `askPermission` | "Permitir ${tool}?" | `"{tool}: {descrição}"` |
| `allowed/denied/allowedAlways` | "Permitido/Negado/Sempre permitido: ${rule}" | inexistentes |
| `usage` | "Uso" | "Uso da conta" |
| `usageSession` | "Sessão (5 h)" | "Sessão (5 horas)" |
| `usageUnavailable` | "Uso do plano indisponível agora." | "Uso da conta indisponível" |
| `usageContextNone` | "Contexto: aparece depois da primeira resposta" | "Contexto indisponível" |
| `usageResetsToday/On` | "zera às HH:MM" / "zera em dd/mm às HH:MM" | "reinicia hoje às " / "reinicia em dd/mm às HH:MM" |
| `sessions` | "Sessões" | "Enviar para sessão…" |
| `resumeNone` | "Nenhuma conversa nos últimos 7 dias." | "Nenhuma conversa recente." |
| `editedFiles` | "Editou N arquivos" | "arquivos editados" |
| `thinkingTitle/Live` | "Raciocínio" / "Pensando…" | inexistentes |
| `peerMessageFrom` | "Mensagem de ${name}" | `"{from}: ..."` |
| `moreLines` | "… mais ${n} linhas" | inexistente |
| `copyMessage/copiedMessage` | "Copiar" / "Copiado" | inexistentes |
| `stateCompleted...` | "Concluída/Pronta para revisão/Bloqueada/Falhou" | inexistentes |
| `improveHarness*` | vários | inexistentes |
| `loadingHistory` | "Carregando a conversa…" | inexistente |

---------------------------------------------------------------------
## D. Ilha de agentes (F `agent_island*.dart`, `floating_agent_island.dart`, `island_gorilla.dart`; R `island.rs`, `agent_island_panel.rs`, `gorilla.rs`)
---------------------------------------------------------------------

| # | Elemento | Classe | Flutter | Rust |
|---|---|---|---|---|
| D1 | Pílula: altura 30, raio 15, padding 9, borda (`warn` com espera, senão `border`), fundo `panel2`/hover `panel3`, contadores (bolinha 7, gap 8/4, mono 12) | IGUAL | `agent_island.dart:560-610` | `island.rs:81-96, 728-835` |
| D2 | Gorila: sprite, poses (sleep/rest/breathe/typing/wave/cheer), intervalos 650/140/280/90ms, "!" que pisca, "z" | IGUAL por leitura (golden Dart no Rust) | `island_gorilla.dart` | `gorilla.rs` |
| D3 | Cheer 1400ms ao terminar | IGUAL | `agent_island.dart:479` | `island.rs:102` |
| D4 | Animação da largura da pílula (tween easeOutCubic 180ms) | INVENTADO | pílula muda de largura na hora | `island.rs:303-395` `WidthTween` |
| D5 | Abrir no hover (timer de saída 180ms) | FALTA | `agent_island.dart:499-533` | só clique |
| D6 | Abrir sozinha enquanto houver pedido de permissão | FALTA | `_expanded = hover \|\| pinned \|\| asks.isNotEmpty` (linha 457) | `is_expanded()` = `pinned` (`island.rs:472`) |
| D7 | Clique fixa; clique fora desfixa; perder foco da janela desfixa | DIFERENTE | `TapRegion` + `didChangeAppLifecycleState` | clique alterna e backdrop fecha (`main.rs:9466-9490`); sem lifecycle |
| D8 | Posição do painel | DIFERENTE | ancorado embaixo da pílula (`CompositedTransformFollower`), margem mínima 10 | `main.rs:9467-9473` centralizado na janela a `TOPBAR_HEIGHT + 6` |
| D9 | Painel: largura 380, raio 12, borda `border2` | IGUAL | `agent_island_panel.dart:41-60` | `agent_island_panel.rs:95-110` |
| D10 | Painel: altura máx 70% da janela, sombra | DIFERENTE | `maxHeight: 0.7*altura`, sombra `blur 18` sem deslocamento | scrollable sem limite; sombra com deslocamento (0,6) |
| D11 | Cabeçalhos de seção | DIFERENTE | 10.5px, letterSpacing 0.6, peso 600, caixa alta (`agent_island_panel.dart:115-131`) | `FONT_TITLE` 11px sem espaçamento de letras |
| D12 | Textos das seções e botões ("Pedem permissão", "Esperando você", "Terminaram", "Trabalhando", "Permitir", "Sempre", "Negar", "No terminal", "Ir para a aba", "Nenhum agente rodando agora.") | IGUAL | `gsh.agentIsland.*` | `labels.rs:309-313, 334-339, 343` |
| D13 | Ícone do harness antes do nome (SVG 14px; bolinha só sem harness) | FALTA | `_Leading` (`agent_island_panel.dart:147-170`) | `status_dot` sempre |
| D14 | Tooltips "Sempre permitir ${rule}" e "Responder no terminal do agente" | FALTA | `AppTooltip` | `ACTION_IN_TERMINAL_TOOLTIP` existe em `labels.rs:338` mas o painel não usa |
| D15 | Linha extra "Sempre: {rule}" no cartão de permissão | INVENTADO | não existe | `agent_island_panel.rs:239` |
| D16 | Cartão "Erro no servidor de status" e borda vermelha da pílula | INVENTADO | não existe | `agent_island_panel.rs:50-54, 112-133`, `island.rs:769`, `labels.rs:309` |
| D17 | Título do agente | DIFERENTE | `s.displayTitle`; workspace com máx 120px e elipse | `main.rs:5237-5240` descriptor `"{título} · t{id}"` + `format_agent_tab_name` => "claude — Terminal · t3"; workspace sem elipse; sempre o workspace ativo |
| D18 | Chats na ilha | FALTA | `_stateOf` (`agent_island.dart:253-262`) inclui `ChatSession` | `pane_descriptors` (`main.rs:5230-5247`) só `PaneTab::Terminal` |
| D19 | Agentes de todos os workspaces | FALTA (provável) | `_vm.allSessions` com nome do projeto de cada um | descriptors só de `self.panes.models()` com o nome do workspace ativo |
| D20 | Rótulo semântico da pílula ("N trabalhando · N esperando você · N terminaram") | FALTA | `Semantics(label:)` | sem acessibilidade |
| D21 | Ilha flutuante Linux (layer-shell, segundo engine, decide/reveal por canal) | FALTA | `floating_agent_island.dart` | não existe (`docs/parity-checklist.md:55` `[adiado]`) |
| D22 | Centralização no centro geométrico da janela (Linux) | FALTA | `IslandHorizontalCenter` | pílula centrada no slot do topbar |
| D23 | Notificação nativa: ação padrão | DIFERENTE | `notifications.open` = "Abrir" (`local_notifier.dart:95`) | `native_notifications.rs:37,48` rótulo "No terminal" |
| D24 | Notificação nativa: títulos e botões | IGUAL | "Agente terminou", "Agente precisa de você", "${agent} quer rodar", Permitir/Sempre/Negar | `labels.rs:633-634`, `main.rs:2673` |
| D25 | `harness_monitor.rs` | n/a (sem UI) | | detecta harness pela árvore de processos |

---------------------------------------------------------------------
## E. Grafo de agentes (F `agent_graph_*.dart`, `agent_node_card.dart`, `new_agent_dialog*.dart`; R `agent_graph*.rs`)
---------------------------------------------------------------------

| # | Elemento | Classe | Flutter | Rust |
|---|---|---|---|---|
| E1 | Geometria do nó 220x88, porta raio 7, cena 4000x3000, curva bézier (`max(48, dx/2)`), tooltip do cabo (textos "Conectado", "Mensagens: N", "Última: …", "Clique direito para remover a conexão") | IGUAL | `agent_graph_painter.dart`, `agent_graph_parts.dart:64-110` | `agent_graph.rs:13-16, 470-760`, `labels.rs:635-646` |
| E2 | Botões do topo | INVENTADO | só "Novo agente" (`OutlineButton`, ícone `add` 16px) em top 12 / right 12 | `agent_graph.rs:134-146` acrescenta "Importar time (.md)…" e "Exportar time (.md)…" (`labels.rs:655-659`, `team_markdown*.rs`); não há importação/exportação de time em Markdown no Flutter |
| E3 | Menu do nó (clique direito) | DIFERENTE | `showAgentNodeMenu` (`agent_graph_actions.dart:360-410`): menu do app com ícones (Focar `open_in_new`, Renomear `edit_outlined`, Tornar maestro `workspace_premium_outlined`, divisor, Fechar aba / Remover do canvas `close` vermelho) na posição do cursor | caixa fixa no canto superior direito (`agent_graph.rs:148-190`) com título do nó, botões padrão, item extra "Cancelar", sem ícones/divisor/vermelho |
| E4 | Menu do cabo (clique direito num link definido) | DIFERENTE | abre menu com "Remover conexão" (ícone `link_off`, perigo) (`agent_graph_view.dart:191-212`) | remove direto sem menu (`agent_graph.rs:354-357`, `Message::RemoveLink`) |
| E5 | Renomear | DIFERENTE | `showRealmNameDialog` "Renomear agente", confirmar "Renomear", valida nome único ("Outra aba já usa este nome…") | formulário flutuante `agent_graph.rs:191-221` sem validação de repetido |
| E6 | Linha tracejada ao arrastar da porta | FALTA | `pendingFrom/pendingTo` tracejados (`agent_graph_painter.dart:148-166`) | `Drag::Link` só `request_redraw`; nada desenhado |
| E7 | Pulso (bolinha colorida) que percorre o cabo quando chega mensagem (1,5s) | FALTA | `kCablePulse`, `GraphPulse` | grep `pulse` em `agent_graph*.rs` = zero (o `AgentGraphPulse` de `main.rs:6510` só pisca o ponto de status) |
| E8 | Cores/traço do cabo | DIFERENTE | definido `text2`, observado `text3`, hover `accent`; ponta redonda; tracejado 6/5; espessura 2.0 (definido) ou 1.5 + 0.35 por mensagem (máx 10) | R usa `first.color` se um dos nós é maestro, senão `text3` para todos; sem hover; tracejado de 16 segmentos; sem ponta redonda (espessura igual) |
| E9 | Hover do cabo (distância < 8, realce, sombra blur 6 na dica) | DIFERENTE | | tooltip existe; sem realce nem sombra |
| E10 | Cartão do nó: borda e sombra | DIFERENTE | borda `accent` a 70%, sombra blur 8 (`agent_node_card.dart:100-101`), sem estado de seleção | borda 100%, 2px quando selecionado (`agent_graph.rs:525-540`), sem sombra |
| E11 | Cursor (mão no nó, "precise" na porta) e tooltips ("Arraste até outro agente para conectar" na porta; "Maestro" no selo) | FALTA | `agent_node_card.dart:104-120, 150-175` | `mouse_interaction` padrão; os textos existem em `labels.rs:617,646` mas o canvas não os usa |
| E12 | Nó de chat | DIFERENTE | `ChatSession`: ícone Claude Code, "Trabalhando" quando roda, pasta do chat | `main.rs:8838-8872`: filtra chats pelo nome da pasta do workspace; detalhe `"Chat · pasta"` (texto inventado); sem ícone; status sempre "Ocioso" |
| E13 | Nó externo (app desktop): selo "desktop", "Online/Offline", "N na fila" | IGUAL (falta o estado "Trabalhando" externo) | | `agent_graph.rs:620-690` |
| E14 | Pasta mostrada no nó de terminal | DIFERENTE | `currentDirectory ?? workingDirectory` | `term.cwd()` apenas |
| E15 | Zoom e limites | DIFERENTE | `InteractiveViewer` 0.3-2, margem 400; limite do nó x = 4000-220 = 3780 | `agent_graph.rs:441` 0.3-2.5; limite x 3773 |
| E16 | Estado vazio | DIFERENTE | ícone `hub_outlined` 36 em `text4`, título `typo.title`, hint `typo.body` `text3`, largura 360, só "Novo agente" | ícone em `text3`, hint 12px, largura 420, botão extra "Importar time (.md)…" (`agent_graph.rs:86-125`) |
| E17 | Posição inicial de nó novo | DIFERENTE | `AgentGraphStore.placeMissing` | grade `80 + (slot%4)*280, 80 + (slot/4)*170` (`main.rs`) |
| E18 | Diálogo "Novo agente": abas, textos, toggles, cores | IGUAL nos textos | `new_agent_dialog.dart` | `agent_graph_dialog.rs`, `labels.rs:575-631` |
| E19 | Diálogo: campos Papel e Tarefa | DIFERENTE | `TextField` `minLines: 3, maxLines: 6` | `agent_graph_dialog.rs:494,499` `text_input` de uma linha |
| E20 | Diálogo: botões de início rápido | DIFERENTE | `OutlineButton` small com texto padrão e ícone do harness | `agent_graph_dialog.rs:596` `text(name).size(10.5)` |
| E21 | Integração desktop: toasts e erros (`invalidConfig/cliMissing/writeFailed`, "Reinicie o ${app}…") | IGUAL nos textos | | `labels.rs:579-600` |

---------------------------------------------------------------------
## F. Galeria e cartões do time
---------------------------------------------------------------------

| # | Elemento | Classe | Detalhe |
|---|---|---|---|
| F1 | 9 cartões e ordem (dbQuery, kanban, notebook, html, panel, httpRequest, tasks, layout, workspaceEnv) | IGUAL | F `gallery_template.dart` x R `gallery_template.rs:20-30` |
| F2 | Títulos e descrições pt-BR (9+9) | IGUAL | comparados um a um com `labels.rs:1183-1227` |
| F3 | Intro "Documentos especiais do GSH Desktop para que você tenha o visual do que o agente de IA esteja fazendo." | IGUAL | `labels.rs:1177` |
| F4 | Caminho do cartão (`.dbq`, `notes.notebook`, `.gsh/tasks.json`…) e ícones | IGUAL | |
| F5 | Conteúdo dos modelos | IGUAL por contrato | `assets/gallery/*.tpl` "byte a byte" (conferir com o teste de sincronia do `.panel`) |
| F6 | Cartão: padding 10, raio 8, ícone 28, gap 10, título 12/600, caminho 10, descrição 11, hover panel2->panel3 | IGUAL por leitura | `gallery_panel.dart:100-160` x `theme::GALLERY_*` |
| F7 | Sem pasta no workspace | DIFERENTE | F não oferece a aba (`galleryPanel: vm.selectedProject == null ? null`); R mostra "Nenhuma pasta — abra um workspace." (`gallery_panel.rs:21-24`) |
| F8 | Título "Não foi possível criar o arquivo" | IGUAL | `labels.rs:1178` |
| F9 | Cartões "Batendo na porta" do time | ver B22-B24 | |

---------------------------------------------------------------------
## G. Contagem por classe (linhas classificadas das tabelas B-F; os achados A1-A8 repetem itens já contados)
---------------------------------------------------------------------
Contado por script sobre as linhas de tabela B-F (a tabela C5, de strings, não entra: são ~27 divergências de texto, todas DIFERENTE/FALTA):
- IGUAL: 38
- DIFERENTE: 59
- FALTA: 30
- INVENTADO: 5 (estado classificado como item em inglês; tween de largura da ilha; "Sempre: regra" na ilha; cartão de erro do status server; botões Importar/Exportar time)
