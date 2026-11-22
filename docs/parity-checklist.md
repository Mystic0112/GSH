# Checklist de paridade Flutter → Rust

Inventário do GSH Rust em Linux, comparado às capacidades visíveis no repositório Flutter. É uma linha de base para decidir o que falta; não é aceite de uso diário nem autorização para descontinuar o Flutter.

## Legenda

- [x] Implementação identificada no Rust; a marca não substitui validação visual ou aceite de uso diário.
- [~] Implementação parcial, integração incompleta ou validação de runtime pendente.
- [ ] Não encontrei equivalente operacional na árvore Rust durante este levantamento.
- [adiado] O usuário pediu para deixar para depois.
- [fora do escopo Linux] Depende de macOS ou Windows; não implementar nesta rodada.

## Recursos do aplicativo

| Área Flutter | Estado no Rust/Linux | Evidência e trabalho restante |
| --- | --- | --- |
| Workspaces, panes, abas, árvore de arquivos e layout | [x] | Existem em `crates/app` e `crates/core`; migração de layouts Flutter em `crates/app/src/layout_store/flutter.rs`, restauração por workspace ao alternar em `Shell::restore_workspace_layout` e aplicação de layouts `.ckp` pelo editor (`EditorMsg::ApplyCkpLayout`). |
| Terminal local, hooks, estado de agente e permissões | [x] | PTY em `crates/term`; hooks e permissões em `crates/app`. Verificar o fluxo no desktop Linux com agentes reais antes de aceitar paridade diária. |
| Notificações de permissão e turno concluído | [~] | O caminho Linux usa D-Bus, ações e identidade `work.gorila.gshdesktop` em `crates/app/src/native_notifications.rs`. Testes cobrem roteamento e concorrência; a exibição e o clique ainda precisam de confirmação no COSMIC/desktop alvo. Mac e Windows estão fora do escopo desta rodada. |
| Menu e atalhos configuráveis | [x] | Modelo Linux em `crates/app/src/app_menu_model.rs`; valores vêm de `[shortcuts]` no TOML. |
| Temas claros/escuros e paletas importadas/nativas | [x] | Tokens centralizados em `crates/app/src/theme.rs`; paletas nativas do Flutter em `crates/app/src/builtin_theme_palettes.json`; importação lê tokens de UI do JSON. |
| Editor, árvore e viewers de arquivos | [~] | Há editor e viewers em `crates/app/src/editor_*` e `crates/app/src/file_viewer*`. Diagnósticos e formatação LSP para abas de arquivo integrados em `crates/editor/src/lsp` e `crates/app/src/lsp_runtime.rs`. Revisar formatos, edição e fluxos de abrir/criar contra Flutter antes de declarar paridade completa. |
| Git (status, diff, branches e commits) | [~] | Cliente e painéis existem em `crates/git` e `crates/app/src/git_*`. Sugestão de mensagem de commit com IA isolada; notas por linha persistidas para a revisão exata e enviadas ao agente escolhido; consulta/envio de checks que falharam e comentários de PR via `gh`. Confirmar os fluxos de worktree e operações assistidas contra Flutter. |
| Banco de dados SQL, MongoDB, Redis e túneis SSH | [~] | Motores e painéis estão em `crates/db` e `crates/app/src/db_*`. Túneis SSH de banco não equivalem à experiência Flutter de host/workspace remoto. |
| Terminal remoto por SSH e `gsh-server` | [~] | `crates/server` (`gsh-server` Linux com socket Unix 0600, sessões PTY persistentes e replay de 4 MiB por offset), transporte `TermSession::from_stream` em `crates/term/src/session/stream.rs` e cliente OpenSSH com diálogo em `crates/app/src/remote_ssh*` e `crates/app/src/remote_connect_dialog.rs`. Falta validar contra `sshd` real e expandir RPCs de arquivos/Git/status. |
| Manter acordado (keep-awake) e sessão com a tampa fechada | [~] | Keep-awake Linux (`systemd-inhibit`) e botão de raio no rail estão implementados. A sessão temporizada de tampa fechada também usa `handle-lid-switch:sleep:idle` e proteções térmicas/de bateria, mas só é controlada em Configurações; falta o aceite físico de fechar a tampa, observar o agente e encerrar o GSH à força. |
| Verificação de atualização (Linux notify) | [~] | Ao iniciar e a cada minuto, o GSH verifica se é hora de consultar o manifesto `latest.json` conforme a frequência escolhida. O menu força a consulta e o `UpdateCard` avisa quando há versão mais nova. Falta publicação automática do manifesto pelo release Linux. |
| Browser integrado e CDP | [~] | Cliente/daemon e aba `PaneTab::Browser` existem com endereço, histórico, validação de URL e snapshot cercado por aviso de dados externos. A integração entrega `None` no lugar dos bytes PNG, então a página não é renderizada no painel; Flutter Linux abre o navegador do sistema. |
| Chat, limites de uso e recursos Claude | [~] | Stream, histórico, ferramentas e uso da conta Claude ativa por janela/modelo existem. A ação atual faz logout/login interativo; não há perfis salvos, troca por terminal/chat, uso Codex ou créditos extras. A implementação de múltiplas contas depende da decisão sobre armazenamento OAuth do Claude CLI. |
| LocalSend e coordenação em LAN | [~] | Protocolo e interface existem em `crates/localsend`, `crates/team` e `crates/app/src/localsend_*`. Ainda requer validação entre máquinas no Linux. |
| Telemetria e diagnóstico | [~] | Coleta/armazenamento em `crates/telemetry` e telas em `crates/app/src/telemetry_*`; verificar paridade de eventos e fluxos antes do aceite. |
| Fazenda/escritório de agentes | [~] | Há UI e renderização em `crates/app/src/farm`; partes de pintura/layout estão marcadas como não integradas. Não considerar equivalente completo ainda. |

## Lacunas ou candidatos a lacuna

- [~] **Language server e pool por linguagem:** Implementado em `crates/editor/src/lsp` e `crates/app/src/lsp_runtime.rs` (cliente JSON-RPC, pool por `(linguagem, raiz)`, diagnósticos e formatação LSP ou por comando configurado com `%FILE%` em arquivo temporário). Falta validação de uso diário com servidores reais por linguagem.
- [ ] **Execução de tasks:** Flutter tem definições e execuções em `lib/app/gsh/data/tasks`; não foi encontrado fluxo equivalente na UI Rust.
- [ ] **Workbench HTTP:** Flutter tem executor e tela de requisições em `lib/app/gsh/data/http`; não foi encontrado workbench equivalente no Rust.
- [ ] **Integração Neovim:** Flutter inclui integração em `lib/app/core/data/neovim`; não foi encontrado equivalente Rust.
- [ ] **Cadernos `.notebook`:** Flutter tem `notebook_view.dart` e `NotebookSession`; não foi encontrado viewer/sessão correspondente no Rust.
- [~] **SSH remoto completo:** `gsh-server` Linux (`crates/server`) e terminal remoto por SSH (`crates/app/src/remote_ssh*`) implementados com retomada de saída por offset; ainda faltam RPCs remotos de workspace, árvore de arquivos, Git e status de agentes.
- [~] **CLI (`86e3ytwcb`):** `open`, versão `0.1.0`, quirks e os verbos portados existem. O help ainda não cobre todos os verbos do CLI Flutter (por exemplo `send-key`, `list-workspaces`, `run-task`, `db`, `http` e `note`).
- [~] **Testes de comparação golden (`86e3ytwdg`):** há fixtures para árvore de panes, abas e migração Flutter. Os testes executam só o Rust e fixtures fixas; ainda falta executar Flutter e Rust com as mesmas entradas e comparar as saídas conforme o card.
- [~] **Canais por contexto (`1dfnhq1n9fc`):** existem canais fixos que filtram workspaces, mas a associação não persiste e faltam instruções, regras, membros editáveis, pasta própria, renomeação e arquivamento.
- [~] **Varredura do Claude Desktop (`1dfnhq1n9fd`):** o scanner lê config local e lista pacotes `.mcpb`/`.dxt`; falta o inventário comparativo Claude Desktop/Code pedido pelo card. A revisão atual também mostra os comandos/URLs de MCP antes de importar.
- [~] **Modo bolsa no Linux (`1dfnhq1n9g1`):** durante a sessão temporizada de tampa fechada, o NetworkManager monitora a conectividade e alterna entre o hotspot salvo e os Wi-Fi preferidos. A vigia encerra junto com a sessão, inclusive por temperatura ou bateria. Falta executar o fluxo em um notebook com o hotspot do celular e validar o controle pelo app móvel.
- [~] **Times e bots descritos em Markdown (`1dfnhq1n9fb`):** importação/exportação local de time com revisão existe. Faltam fontes GitHub/pasta, exportação individual, atualização/versionamento, esquema estrito completo e revisão separada das conexões antes de ativá-las.
- [x] **Automações agendadas (`1dfnhq1n9f2`, Linux):** dias e horário local, execução manual, Claude/Codex/agy/Pi, histórico de resultado e log e notificação de fim/falha. Compatibilidade preserva rotinas antigas como diárias; o GSH precisa ficar aberto.
- [ ] **App móvel/controle remoto e catálogo OAuth:** aparecem na fila de cards; precisam de escopo e validação próprios. Não confundir com o release Linux manual já existente.

## Cards e decisões de plataforma

- [x] Release Linux manual: `.github/workflows/release-linux.yml` compila `gsh-app` e `gsh`, cria tarball e `SHA256SUMS`. O workflow não publica release automaticamente.
- [fora do escopo Linux] Pacotes e validação específicos de macOS e Windows.
- [adiado] Ilha flutuante no topo da janela; o card fica para depois, conforme instrução do usuário.

## Condição para descontinuação do Flutter

- [ ] Validar os fluxos Linux pendentes e confirmar que o Rust cobre o uso diário do time.
- [ ] Só depois da confirmação, arquivar `GSH Desktop` sem apagar o repositório.
- [ ] Avisar quem usa o release atual. Nenhum aviso foi enviado nesta etapa.

Este checklist pode ser atualizado conforme cada card for implementado e validado. Os testes automatizados e builds atuais não substituem verificação visual no Linux, teste entre máquinas ou confirmação de uso diário pelo time.
