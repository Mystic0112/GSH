# Auditoria de paridade Flutter -> Rust (23/11/2026)

Feita só por leitura de código (nada foi rodado). Relatórios por área nesta pasta.

| Área | Arquivo | Igual | Diferente | Falta | Inventado |
|---|---|---|---|---|---|
| Rail, topo, diálogos | rail-topbar-dialogs.md | 14 | 24 | 17 | 9 |
| Panes, abas, terminal | panes-tabs-terminal.md | 22 | 24 | 30 | 5 |
| Arquivos, editor, git | files-editor-git.md | 4 blocos | 12 | 8 grupos | 11 |
| Bancos, HTTP, tasks, telemetria | db-http-tasks-telemetry.md | 45+ | 14 | 8 | 6 |
| Chat, agentes, ilha, fazenda | chat-agents-island-farm.md | 38 | 59 | 30 | 5 |
| Configurações | settings.md | 14 | 12 | ~35 | 12 |

## Ordem sugerida
1. Terminal e abas (uso diário com Claude/vim): Alt, setas com modificador, mouse, bracketed paste, Ctrl+V; menu de clique direito da aba; status do agente na aba; links.
2. Rail/workspaces: tirar exemplos falsos, avatar com cor/foto, ícone `layers`, não bloquear troca, menu e diálogo de workspace (codex em andamento).
3. Chat: auto-scroll, bug "Editou N arquivos", anexos, cartões de permissão/plano, copiar/hora.
4. Arquivos: menu da árvore, Ctrl+F, Ctrl+P, rodapé do viewer, markdown rolando; depois kanban/notebook.
5. Configurações: reestruturar menu como o Flutter, Atalhos, Hosts remotos, Aparência.
6. Ilha, fazenda, grafo.
7. Tasks e HTTP workbench (dependem de telas novas) + verbos da CLI.

## Decisão do Hélio
"Inventados" que foram pedidos por ele nos cards (não são erro): LocalSend, navegador/Obscura, notas por linha no diff, PR e checks, mensagem de commit por IA, canais com instruções, perfis Claude, modo tampa fechada/bolsa. Decidir um a um se ficam.
