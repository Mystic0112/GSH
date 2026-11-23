# Auditoria de paridade: Configurações (Flutter -> Rust)

Somente leitura. Flutter = verdade (`/home/heliodev/projetos/GSH Desktop/lib/app/settings/ui/`, textos em `lib/i18n/pt_BR.i18n.json`). Rust = `/home/heliodev/projetos/GSH/crates/app/src/settings/` + `crates/app/src/labels.rs`.

Método: li os dois lados inteiros (nav, cada painel, diálogos) e rodei um cruzamento automático: cada string pt-BR de `settings.*` do Flutter procurada em todo `crates/**/*.rs` (134 strings sem correspondência, todas no lado GSH Desktop ou extras da Claude). Antes de dizer FALTA, fiz grep dos campos de config no app inteiro.

Legenda de caminhos: `F:` = Flutter, `R:` = Rust. `settings_page.dart` = `settings/ui/settings_page.dart`.

## Resumo por classe (itens de tela, não linhas de código)

| Classe | Qtde | Onde se concentra |
|---|---|---|
| IGUAL | 14 | Lado Claude inteiro (Geral, Privacidade, Recursos, Memória, Design systems, Claude Code, Cowork, Desenvolvedor, Skills, Conectores, Plugins), nav do lado Claude, rodapé de versão, diálogos de armazenamento/logout |
| DIFERENTE | 12 | Lado inicial, nav GSH, placeholder da busca, Recursos do GSH, Notificações, Linguagem, Commits com IA, idioma, Uso, Sistema, faixa do tamanho da interface, botão do diálogo de marketplace |
| FALTA no Rust | ~35 | Quase todo o lado "GSH Desktop": Geral, Aparência, Terminal, Atalhos, Hosts remotos, Notificações (sons), Commits com IA (descoberta/modelos) |
| INVENTADO no Rust | 12 | Aba Editor, campos de shell/histórico/fonte no Terminal, Rotinas agendadas, Perfis do Claude Code, "Trocar de conta", Créditos extras, sessão de tampa fechada, Modo bolsa, scanner de Extensões, faixa de erro |
| Pedido do dono (não conta) | 3 | Aba LocalSend (nav + painel + popup), toggle de Navegador (o Flutter também tem), nada de Obscura em Configurações |

Padrão geral: o lado **Claude** foi portado com fidelidade (textos idênticos, medidas copiadas, mesmas seções e ordem), com extras inventados por cima. O lado **GSH Desktop** não é um port: é uma tela própria, com abas, rótulos, controles e layout diferentes do Flutter.

---

## 1. Navegação e estrutura (impacto: visível na primeira tela)

### 1.1 Lado e aba em que a tela abre — DIFERENTE
- F: `settings_page.dart:82` `_category = SettingsCategory.general`. O lado vem de `side`, e `general` é lado Claude. A tela abre em **Claude > Geral**.
- R: `settings/mod.rs:258` `nav_claude: false` e `:261` `section: Section::General`. Abre em **GSH Desktop > Geral**.
- Correção: abrir com `nav_claude = true` e `claude.tab = Tab::General`.

### 1.2 Menu do lado "GSH Desktop" — DIFERENTE (ordem, itens e rótulos)
F: `settings_nav.dart:96-153`, grupo "GSH Desktop", 9 itens nesta ordem, rótulos de `pt_BR` `page.nav.*`:

| # | Flutter | Ícone Flutter |
|---|---|---|
| 1 | Geral | settings_outlined |
| 2 | Recursos do GSH | widgets_outlined |
| 3 | Aparência | brightness_6_outlined |
| 4 | Terminal | terminal_outlined |
| 5 | Linguagem | data_object |
| 6 | Commits com IA | auto_awesome_outlined |
| 7 | Atalhos | keyboard_outlined |
| 8 | Notificações | notifications_outlined |
| 9 | Hosts remotos | cloud_outlined |

R: `settings/view.rs:300-347`, 10 itens:

| # | Rust | Observação |
|---|---|---|
| 1 | Geral | igual |
| 2 | Aparência | posição diferente |
| 3 | Terminal | posição diferente |
| 4 | **Editor** | INVENTADO (ver 3.1) |
| 5 | Notificações | posição diferente |
| 6 | **Fazenda do time** | só aparece com Fazenda ligada. No Flutter isto é uma seção dentro de "Recursos do GSH" (ver 2.2) |
| 7 | **LocalSend** | pedido do dono |
| 8 | **Automações** | no Flutter chama "Commits com IA" |
| 9 | **Servidores de linguagem** | no Flutter chama "Linguagem" |
| 10 | Recursos do GSH | no Flutter é o item 2 |

- FALTA: **Atalhos** e **Hosts remotos**.
- Rótulos errados: "Automações" (deve ser "Commits com IA"), "Servidores de linguagem" (deve ser "Linguagem").
- Ícones: o Rust reaproveita `ICON_FILE_ITEM` para Editor e LocalSend, `ICON_FARM` para Fazenda, `ICON_BOLT` para Automações. Os ícones do Flutter acima não são usados.

### 1.3 Placeholder da busca do menu — DIFERENTE
- F: `page.nav.search` = **"Procurar"**.
- R: `labels.rs:1133` `SETTINGS_NAV_SEARCH` = **"Buscar configurações"**.
- O Flutter também tem botão de limpar (`InputFeature.clear`, `settings_nav.dart:~290`). O `text_input` do Rust (`view.rs:419-433`) não tem. O mesmo vale para a busca dos catálogos (`catalog.rs` `catalog_header`).

### 1.4 Resto do nav — IGUAL
- Alternador de lado "GSH Desktop | Claude", rodapé "GSH Desktop <versão>", largura 220, grupos do lado Claude, "Nenhum resultado" e "Chaves de API" externo (ícone north_east): tudo idêntico (`view.rs:299-481` vs `settings_nav.dart:96-420`).
- Grupos do lado Claude: "Configurações" (Geral, Conta, Privacidade, Uso, Recursos, Memória, Design systems, Claude Code, Cowork), "Este computador" (Sistema, Extensões, Desenvolvedor), "Personalização" (Skills, Conectores, Plugins), "Plataforma" (Chaves de API).

---

## 2. Lado "GSH Desktop", aba por aba (impacto: aqui está o grosso da divergência)

Estilo do Flutter nestas abas: Aparência, Terminal, Notificações, Commits com IA, Linguagem, Atalhos e Hosts remotos usam cartão (`_Section/_Card/_Row`, `settings_page.dart:1892-1985`): coluna de 680 px, rótulo de seção em `text3`, cartão `panel2` com raio 10 e divisores. Geral e Recursos do GSH usam o estilo plano (`SettingsSection`, 1010 px). No Rust, **todas** as abas GSH usam um `setting_row` plano (rótulo à esquerda, controle à direita, `view.rs:700-743`) com título `theme::SIZE_TITLE`. Nenhuma tem cartão.

### 2.1 Geral (GSH) — FALTA quase tudo, INVENTADO no lugar
F: `settings_page.dart:267-359`, seção "GSH Desktop" + seção "Diagnóstico".

| Item (Flutter) | Controle | Padrão | No Rust |
|---|---|---|---|
| Mostrar terminal do GSH Desktop / "Mantém um workspace sem pasta, só de terminal, fixado no topo da barra lateral. Desligar fecha seus terminais." | Switch | ligado (`showGsh = true`) | **FALTA**. Campo `show_gsh` existe em `config/schema.rs:208` mas nenhum código do app lê |
| Motor (editor) / "Procurando o Neovim…" / "O Neovim não foi encontrado na PATH do seu shell." | Botão atualizar + dropdown "GSH Desktop (padrão)" / "Neovim" | GSH | **FALTA**. `editor.engine` só existe no schema |
| Verificar atualizações / "Com que frequência o GSH Desktop deve procurar novas versões." | Dropdown Diariamente / Semanalmente / Mensalmente / Nunca | Diariamente | **FALTA a linha**. `update_check_frequency` é lido em `main.rs:5414`, mas sem controle em Configurações |
| Idioma | Dropdown com ícones: Sistema, **Inglês**, **Espanhol**, **Português (BR)** | Sistema | DIFERENTE: `view.rs:169-205` usa `pick_list` com "English", "Español", "Português (Brasil)" (`labels.rs:1137-1141`), sem ícones |
| Diagnóstico > Arquivo de log / "Erros e eventos de inicialização são registrados aqui, mantidos por N dias." + caminho / botão "Revelar" | Texto + botão | | **FALTA** |
| Diagnóstico > Reportar um problema / "Abre uma issue pré-preenchida com sua versão, SO e log recente. Nada é enviado automaticamente — você revisa antes." / botão "Reportar…" + diálogo "Relatório de problema" | Botão + diálogo | | **FALTA** |

R: `view.rs:143-192`:
- Seção "Aparência" com **Tema** segmentado Sistema/Claro/Escuro (`labels.rs:709-711,1137`). INVENTADO nesta aba. No Flutter, o modo claro/escuro mora em Aparência (dropdown "Modo") e em Claude > Geral.
- Seção "GSH Desktop" só com Idioma.

### 2.2 Recursos do GSH — DIFERENTE
F: `gsh_features_panel.dart:1-147`. Título de seção "Recursos do GSH". Cada linha tem rótulo **e descrição**:

| Rótulo Flutter | Descrição Flutter |
|---|---|
| Chat com o Claude (beta) | Botão de chat na barra do workspace (Claude Code em stream-json). |
| Fazenda | Loops de agente lidos dos transcripts do Claude Code. |
| (se Fazenda ligada) Cenário da fazenda: segmentado Fazenda / Escritório | Onde os agentes ficam: a fazenda ou um escritório em pixel art. |
| Harness | Botão que abre o que o agente carrega no workspace: instruções, skills, subagentes, hooks, permissões e MCP. |
| Grafo de agentes | Visualiza os agentes do workspace e quem fala com quem. |
| Navegador embutido | Aba "Navegador" no menu da pane (onde a plataforma tem webview embutida). |
| Ilha de agentes | Pílula na barra do GSH e, no Linux com suporte a layer-shell, também sobre outras janelas. Desative para ocultá-la nos dois lugares. |

Depois, só com Fazenda ligada, seção **"Fazenda do time"** com as linhas Ver a fazenda dos colegas, Time, Seu nome, Mostrar títulos das sessões, Senha do time, Quem pode bater na porta, Perguntar antes de abrir a porta e Rede do trabalho. Os textos são os mesmos de `labels.rs:752-775`.

R: `view.rs:585-607`, lista `labels.rs:735-740`:
- Só o rótulo, **sem descrição**, com nomes encurtados: "Chat" (deve ser "Chat com o Claude (beta)"), "Navegador" (deve ser "Navegador embutido"), "Harness", "Grafo de agentes", "Fazenda", "Ilha de agentes" (esses coincidem).
- A ordem do Rust é Chat, Fazenda, Harness, Grafo, Navegador, Ilha. A do Flutter é Chat, Fazenda, (Cenário), Harness, Grafo, Navegador, Ilha.
- **FALTA** "Cenário da fazenda". `farm.scenery` existe no schema, mas não há controle.
- **DIFERENTE (lugar)**: o Rust faz "Fazenda do time" uma aba própria (`Section::Team`, `view.rs:294-378`), com título de aba "Fazenda do time". Os textos das linhas são idênticos, mas no Flutter é uma seção dentro de "Recursos do GSH".

### 2.3 Aparência — FALTA quase tudo
F: `settings_page.dart:936-1123`. Seções, na ordem:

1. **Layout**: "Inverter panes" / "Coloca os workspaces à direita e arquivos, busca, git e banco à esquerda." (switch, padrão desligado). **FALTA**. `swap_side_panels` só no schema.
2. **Tema**:
   - "Tema" / "Cores do app, realce de código e paleta do terminal." (dropdown da lista de temas, importados depois de um divisor com ícone). **FALTA**.
   - "Modo" / "Qual variante do tema usar." (dropdown Sistema/Claro/Escuro, descrição vira aviso "só traz a variante escura/clara" quando o tema tem uma só). **FALTA** (o Rust tem um segmentado sem descrição, ver 2.1).
   - "Arquivo de tema" / "Importe um tema de um arquivo JSON, ou exporte o tema ativo." com botões Importar… / Exportar… / Remover. **FALTA** (o Rust só importa pela aba Design systems).
   - Preview do tema com abas "Código" e "Terminal" (`_ThemePreview`, `:1389-1543`). **FALTA**.
3. **Fontes**: Fonte da interface (+ descrição), Tamanho da interface (stepper 11 a 22), Fonte do código, Tamanho do código (stepper 9 a 20), Fonte do terminal, Tamanho do terminal (stepper opcional 9 a 24, "Seguir o código"), Peso do terminal (Automático (pela tela) / Fino / Normal / Médio / Seminegrito). **FALTA tudo**, e nada disso é lido pelo app (`terminal_font`, `terminal_font_weight` só no schema).
4. **Conversa**: "Fixar mensagem do usuário" / "A pergunta fica fixa no topo enquanto a resposta rola." (switch, padrão ligado). **FALTA** (`pin_user_message` só no schema).

R: `view.rs:193-227`:
- Título "Aparência", Tema segmentado, **Tamanho da interface** como `text_input` com placeholder "De 10 a 20" (`labels.rs:1125-1127`, faixa 10 a 20 em `appearance.rs:12-14`). DIFERENTE: no Flutter é stepper com faixa 11 a 22.
- **Largura da transcrição** (Estreito/Médio/Largo). INVENTADO nesta aba. No Flutter só existe em Claude > Geral, onde o Rust também tem.

### 2.4 Terminal — INVENTADO + FALTA
F: `settings_page.dart:690-780`. Seção "Terminal padrão" (Motor [só se o motor for selecionável], Shell [só Windows]) e seção "Comportamento" ("Fechar a aba quando o shell encerrar" / "Quando um shell termina sozinho (exit, Ctrl-D), fecha a aba em vez de deixar um terminal morto.", padrão desligado). Aviso de WSL só no Windows.

R: `view.rs:228-258`:
- INVENTADO: "Shell padrão" (text input), "Histórico do terminal (1000 a 100000 linhas)", "Fonte do terminal (8 a 32)" e o aviso "As alterações valem para novos terminais." O Flutter não tem nenhum desses controles. O tamanho do terminal fica em Aparência como stepper, e shell/histórico não existem na UI.
- FALTA: "Fechar a aba quando o shell encerrar". `close_tab_on_shell_exit` só no schema, ninguém lê.
- "Motor" (Ghostty/xterm) não se aplica à stack Rust. Decisão do dono: omitir, ou documentar como não portável.

### 2.5 Linguagem — DIFERENTE
F: `settings_page.dart:2443-2722`.
- Seção "FORMATAÇÃO" > "Formatar ao salvar" / "Formata o arquivo automaticamente ao salvar (⌘S)." (switch, padrão desligado).
- Seção "SERVIDORES DE LINGUAGEM": um cartão com **uma linha expansível por linguagem** (`kLanguageDefs`): nome, extensões (`.dart · .xxx`), bolinha de status (verde "Servidor responde" / vazada "Servidor não encontrado ou comando inválido" / "Verificando…"). Ao expandir: "Comando do language server", "Comando do formatador (opcional)", dica "Formatador externo com o placeholder %FILE%…", "Redefinir para o padrão" e "Salvar e reiniciar".
- Rodapé: "Erros e formatação usam o language server de cada linguagem. O GSH Desktop não instala servidores — ele usa o que já está na sua máquina. ● responde · ○ não encontrado ou comando inválido (…)".

R: `view.rs:558-584`. Título de aba "Servidores de linguagem" (rótulo errado) e dois `text_editor` multilinha de 160 px: "Servidores de linguagem" e "Formatadores", com ajuda "Uma entrada por linha: linguagem = comando. Exemplo: rust = rust-analyzer" (`labels.rs:730-735`).
- Todos os elementos do Flutter acima são FALTA (formatar ao salvar, lista por linguagem, status, reset, "Salvar e reiniciar", rodapé).
- O editor `linguagem = comando` é INVENTADO.

### 2.6 Commits com IA — DIFERENTE + INVENTADO
F: `settings_page.dart:362-621`. Seção "Mensagens de commit", cartão com:
- "Harness": a descrição muda (Procurando harnesses de linha de comando instalados… / Nenhum harness compatível foi encontrado no PATH. / "X está configurado, mas indisponível." / "Escolha a CLI usada para gerar mensagens de commit." / caminho do executável). Controle: dropdown só com os harnesses **descobertos no PATH** (logo SVG de cada um, "Não configurado") + botão de atualizar com tooltip "Atualizar harnesses instalados".
- "Modelo" (só com harness escolhido): dropdown com busca ("Buscar entre N modelos…"), "Auto" ou "Padrão da CLI", "· Recomendado". Descrições "Este harness escolhe o modelo automaticamente." / "Este harness usa o modelo padrão da própria CLI." / "Só aparecem os modelos liberados na sua conta." / "A lista de modelos fica indisponível até o harness ser encontrado.".
- "Gerar pelo Controle de Versão" / "O GSH Desktop envia apenas o diff selecionado e os assuntos dos commits recentes. Padrões comuns de credenciais e arquivos sensíveis são redigidos antes de o harness rodar." com ícone auto_awesome.
- Erro "Não foi possível descobrir os harnesses de automação instalados.", aviso de modelo obsoleto.

R: `view.rs:380-557`:
- Título "Automações". "Harness padrão": `pick_list` **fixa** com os 7 harnesses + "Desativado" (rótulos "Antigravity, Claude Code, Codex, Cursor, GitHub Copilot, OpenCode, Pi"), sem descoberta, sem logos, sem botão de atualizar, sem descrição.
- "Modelo padrão": **campo de texto livre** com placeholder "Padrão do harness" (string solta, `view.rs:413`), sem lista de modelos, sem busca.
- FALTA: linha "Gerar pelo Controle de Versão" e todas as mensagens de estado.
- INVENTADO: **"Rotinas agendadas"** inteira: nome, HH:MM, harness, dias da semana (a string "Dias" é solta, `view.rs:450`), repositório, prompt, "Agendar rotina", lista com "Testar agora" / "Remover", "Histórico de execuções" (`view.rs:538`) e "Abrir log". O Flutter não tem nenhum agendamento (`grep rotina|routine|schedul` no Flutter não acha nada ligado a isso). Vêm de `crate::scheduled_automations` e de `labels.rs:720-729`.

### 2.7 Atalhos — FALTA a aba inteira
F: `settings_page.dart:1544-1625` e `shortcut_catalog.dart`. Aviso "Os atalhos de teclado ainda não são personalizáveis." e 5 seções read-only de keycaps: Application (Open Settings ⌘, ; Open Workspace ⌘O), Workspace (New Terminal ⌘T; New Chat ⌘⇧L; New Chat to the Right ⌘⇧J; Toggle Workspace Panel ⌘B; Toggle Files ⌘⇧B; Split Pane Right ⌘D; Split Pane Down ⌘⇧D; Focus Pane ⌘⌥←→↑↓; Select Tab 1–8; Select Last Tab ⌘9; Next/Previous Realm ⌘`), Navigation & Search (Go to File ⌘P; Search in Files ⌘⇧F; Focus Composer ⌘L), Editor (Save File ⌘S; Format File ⌘⇧F), View (Zoom In/Out/Actual Size). Os rótulos do catálogo são em **inglês** de propósito no Flutter.
R: não existe. `grep "Atalhos"` em `crates/` só acha "Atalhos /nome" do Harness.

### 2.8 Notificações — DIFERENTE
F: `settings_page.dart:1126-1210` (+ linhas de som em `:1210-1330`).
- Seção "Notificações": "Ativar notificações" / "Avisar quando um agente terminar uma resposta e a janela não estiver em foco." (switch, padrão ligado). Só no macOS e ligado: "Permissão do sistema" ("O GSH Desktop tem permissão para enviar notificações." / "O macOS ainda não concedeu acesso a notificações.", "Concedido" / "Solicitar permissão").
- Seção "Sons": **Volume** (slider 0 a 100, mostra "50%", padrão 50); "Turno concluído" / "Um agente terminou o turno." e "Ação necessária" / "Um agente está esperando sua aprovação ou resposta." Cada linha tem Ouvir, Escolher arquivo (.wav/.mp3), Voltar ao padrão (só com arquivo), "Tocar também com a aba ativa" e switch. Descrição "Personalizado: nome".

R: `view.rs:281-293`: título "Notificações" e **uma** linha: "Conclusão de respostas e permissões do Claude Code" (switch). Rótulo, texto e escopo diferem.
- FALTA: slider de volume, as duas linhas de som e seus controles, permissão do sistema. `sound.volume`, `sound.events`, `on_active_tab` existem no schema; só `ActionRequired` é lido (`mod.rs:permission_sound`), e ele aparece apenas em Claude > Geral.

### 2.9 Hosts remotos — FALTA a aba inteira
F: `settings_page.dart:1627-1890`. Cartão "Como funciona" (explica SSH + gsh-server + chave), botão "Adicionar host", lista com nome, `alvo ssh · Conectado/Conectando…/Reconectando…/Offline/Não conectado · N workspace(s)` e botões Reconectar / Editar / Remover (diálogo "Remover host" / `Remover "X" e todos os workspaces dele? Nada é apagado no host.`), estado vazio "Nenhum host remoto ainda." e (mobile) "Chave deste dispositivo". Textos em `pt_BR` `settings.remoteHosts.*`.
R: não existe nenhuma tela de gestão de hosts. O Rust tem só o diálogo de conectar (`remote_connect_dialog.rs`, `remote_ssh.rs`) sem lista persistida.

### 2.10 Aba LocalSend — pedido do dono (não conta)
R: `Section::LocalSend` (`view.rs:379`, `settings/localsend.rs`, `labels.rs:752-840`). Sem equivalente no Flutter, esperado.

---

## 3. Itens INVENTADOS no Rust (fora do lado GSH já citado)

### 3.1 Aba "Editor" — INVENTADO
`view.rs:259-280`: "Tabulação (2 ou 4)", "Realce de sintaxe", "Quebra de linha" (`labels.rs:1130-1132`). O Flutter não tem essas opções em nenhum lugar de Configurações (`AppSettings` não possui `tabSize`, `wordWrap` nem `syntaxHighlight`). O único ajuste de editor no Flutter é o "Motor" (ver 2.1).

### 3.2 Conta — INVENTADO (2 itens)
- **"Perfis do Claude Code"** (`claude/panels.rs:493-570`, só Linux): "Conta padrão", "Em uso"/"Usar", "Desvincular", "Nome do perfil", "/caminho/da/pasta", "Adicionar perfil". Todas as strings são soltas no código (fora de `labels.rs`). Não existe no Flutter.
- **"Trocar de conta"** (`panels.rs:593`, diálogo `claude/view.rs` `confirm_switch_account_dialog`, `labels.rs:1417-1420` `SETTINGS_ACCOUNT_SWITCH*`). O Flutter tem só Entrar / Sair (`account_panel.dart:156-163`).
- O restante da aba (Perfil/Instruções, Conectado/Desconectado, E-mail, Organização, ID da organização, Plano, Método de login, diálogo de logout) é IGUAL.

### 3.3 Uso — INVENTADO + DIFERENTE
- INVENTADO: botão **"Trocar de conta"** e **e-mail** no cabeçalho (`secondary.rs:25-60`). O Flutter mostra só o nome do plano (`usage_panel.dart:46-54`).
- INVENTADO: seção **"Créditos extras"** (`secondary.rs:152-197`: "Uso dos créditos", "Créditos extras desativados nesta conta.", "N usados de M", "Perto do limite", "Limite atingido", "Percentual de uso indisponível."). Não existe no Flutter.
- DIFERENTE: "Última atualização: …" com botão "Atualizar" rotulado (outline com texto). No Flutter é `IconButton.ghost` só com ícone (`usage_panel.dart:70-78`).
- Barras de janela, títulos, "Reinicia em/às", "% usado", planos (Pro, Max, Equipe, Empresa): IGUAL.

### 3.4 Sistema — INVENTADO + DIFERENTE
- IGUAL: "Versão do aplicativo", "Iniciar ao ligar", seção "Armazenamento" inteira (local, Usar padrão / Alterar…, Redefinir…, diálogos de reset e de reinicialização).
- DIFERENTE: "Manter computador ativo" aparece sempre no Rust (`secondary.rs:688`). O Flutter só mostra se `keepAwake.isSupported` (`system_panel.dart:41`).
- INVENTADO (só Linux, `secondary.rs:699-800`): **"Sessão com a tampa fechada"** (durações, "Iniciar sessão", "Encerrar agora", avisos de bateria e temperatura) e **"Modo bolsa (hotspot + Wi-Fi)"** (campos "SSID do hotspot" e "Wi-Fi separados por vírgula", "Aplicar"). Strings soltas no código. Não existem no Flutter.

### 3.5 Extensões — INVENTADO
- Flutter: `SettingsPlaceholderPanel` (ícone extension 64 px, "Nenhuma extensão instalada", nota sobre .MCPB/.DXT). `settings_page.dart:109-114`.
- Rust (`secondary.rs:575-668`): além do placeholder (usado só quando vazio), varre o Claude Desktop (`claude_desktop_scanner`) e lista itens com "MCP stdio / MCP HTTP / Bundle (.mcpb/.dxt)", "env: … (valores ocultos)", botão "Importar após revisão", chip "Somente revisão", "Bloqueado: …". Não existe no Flutter.

### 3.6 Faixa de erro genérica — INVENTADO
`view.rs:221-238`: mensagens "Idioma inválido…", "Informe um tamanho entre 10 e 20.", "Tema inválido…", etc., em `warn`, empilhadas no fim do painel. O Flutter não valida texto livre nessas telas (usa steppers e dropdowns).

---

## 4. Lado Claude: itens que batem — IGUAL (conferidos)

Conferidos linha a linha por texto, ordem, tipo de controle e medida (as constantes `SETTINGS_*` em `theme.rs:513-618,1520-1618` repetem os números do Flutter: 28/26/48 de padding, 1010 de largura, 12 de linha, 40 de gap, 13.5/14/15 de fonte, 860+56 nos catálogos):
- **Geral** (Claude): Aparência (Tema com ícones, Fonte da interface, Fonte do chat, Tamanho do texto 13/14/16, Largura da transcrição, Movimento reduzido) e Notificações (4 linhas). `panels.rs:282-420` vs `general_panel.dart`.
- **Privacidade**, **Recursos** (Geral + "As Skills foram movidas para Personalização"), **Memória**, **Cowork**, **Desenvolvedor** (MCP locais, "Modo desenvolvedor", Telemetria, arquivo de configurações, cartão "Quer uma chave de API?"), **Claude Code** (7 seções: Geral, Aparência do código, Aparência, Remote Control [linha do chat oculta no Windows em ambos], Sessões, Segurança, Navegador).
- **Design systems**, **Skills**, **Conectores**, **Plugins** e diretório de plugins: mesma estrutura (cabeçalho com Meus/Descobrir, busca 255 px, "+ Adicionar", grupos com contagem, linhas com ícone 36 px). Não conferi pixel a pixel, só texto, estrutura e medidas.
- Diálogos: logout, reset de armazenamento, reinicialização, aviso sem workspace, seletor de fonte.

DIFERENTE pequeno: no diálogo de nome do marketplace o botão de confirmar é **"Adicionar"** no Rust (`claude/view.rs:119` `SETTINGS_COMMON_ADD`). No Flutter os dois diálogos (nova skill e marketplace) usam **"Criar"** (`skills_panel.dart:~52` `confirmLabel: context.t.common.create`).

---

## 5. Pedidos do dono e itens neutros

- **LocalSend**: aba "LocalSend" (4 campos: ficar disponível, nome do dispositivo, pasta, limites) e popup. Não conta como inventado.
- **Navegador**: o toggle existe nos dois lados e `Abrir links no navegador integrado` também. Só difere o rótulo ("Navegador" vs "Navegador embutido") e a falta da descrição.
- **Obscura**: nenhuma referência em `crates/app/src/settings/`.

## 6. Observações transversais

1. Strings pt-BR soltas fora de `labels.rs`, por onde o Rust escapa do mapa de textos: `view.rs:413` "Padrão do harness", `:450` "Dias", `:538` "Histórico de execuções", "Abrir log"; `panels.rs` perfis ("Conta padrão", "Em uso", "Usar", "Desvincular", "Nome do perfil", "Adicionar perfil", "Perfis do Claude Code"); `secondary.rs` ("Créditos extras", "Importar após revisão", "Somente revisão", "Modo bolsa…", "SSID do hotspot", "Aplicar").
2. **Campos de config sem UI e sem consumo** (declarados em `config/src/schema.rs`, mas nenhum código fora de `config/` os lê): `show_gsh`, `swap_side_panels`, `close_tab_on_shell_exit`, `format_on_save`, `pin_user_message`, `terminal_font`, `terminal_font_weight`, `editor.engine`, `sound.volume`, `sound.on_active_tab`. Implementar a tela sem implementar o efeito deixaria os controles mortos. A UI e o comportamento precisam andar juntos.
3. Onde grava: o Flutter guarda `AppSettings` (Hive) pelo `SettingsController`, e as chaves do Claude no `~/.claude/settings.json`. O Rust grava em `config.toml` (`toml_edit`) e nas mesmas chaves do Claude. Não é diferença visível ao usuário.
4. Cabeçalho da tela: Flutter usa `WindowTitleBar` com controles de janela, seta de voltar (30 px, ícone 18) e título 14 px. O Rust reproduz botão, título e medidas (`theme.rs:531-534`). Não conferi os controles de janela, fora do escopo.

---

## 7. Ordem de impacto sugerida para correção

1. Abrir em Claude > Geral (1.1).
2. Refazer o menu GSH: 9 itens na ordem do Flutter, rótulos "Linguagem", "Commits com IA", e mover "Fazenda do time" para dentro de "Recursos do GSH" (1.2, 2.2).
3. Aparência completa (2.3), com o Rust consumindo `swap_side_panels`, fontes, peso, `pin_user_message`.
4. Geral GSH (2.1): Mostrar terminal, Motor do editor, Verificar atualizações, Idioma com rótulos do Flutter, Diagnóstico.
5. Notificações (2.8), Terminal (2.4), Linguagem (2.5), Commits com IA (2.6).
6. Abas Atalhos (2.7) e Hosts remotos (2.9).
7. Remover o inventado: aba Editor, campos de shell/histórico/fonte, Rotinas agendadas, Perfis, Trocar de conta, Créditos extras, tampa/Modo bolsa, scanner de Extensões, faixa de erro (3).
8. Ajustes finos: "Procurar", botão limpar da busca, "Criar" no marketplace, faixa 11 a 22, refresh só com ícone em Uso, `keepAwake.isSupported`.
