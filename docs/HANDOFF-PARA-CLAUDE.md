# Handoff para Claude — GSH Desktop

Atualizado em 23/11/2026. Repositório: `/home/heliodev/projetos/GSH`.

## Estado do Git

- Branch: `codex/native-permission-notifications`.
- `HEAD` e `origin/main` estão ambos em `10c3bed` (`docs: add Linux completion report`).
- As alterações listadas abaixo estão na working tree, sem commit e sem push.
- Não reverter alterações existentes. O diff inclui trabalho anterior desta tarefa, não apenas a integração recente.

## O que foi feito nesta retomada

### Canais

- Integrei a nova UI/editor de canais ao Shell e ao rail.
- Canais têm IDs estáveis, pasta, instruções, membros por referência à sessão existente, regra de resposta e arquivamento/restauração.
- Atribuições e o canal ativo são salvos no layout; contexto de canal/workspace de chats e terminais também é persistido.
- Chats e terminais criados passam a carregar o contexto do canal. Um chat novo aplica as instruções do canal com `--append-system-prompt` do Claude CLI.
- Regras disponíveis: Manual (padrão), Só ao mencionar e Todos. O roteador resolve menções, deduplica sessões e encaminha a mensagem para os chats/terminais já vinculados. Pula o próprio emissor, chats ocupados e mensagens com anexos; em terminais ignora sessão em estado Working.
- O caminho de envio local do terminal continua presente; não remover o LocalSend nem o envio local de mensagens.
- Foram atualizados testes de persistência, de seleção de workspace/canal e dos argumentos do Claude CLI.

### Perfil e uso do Claude

O trabalho anterior mantido nesta mesma working tree dá suporte a perfis oficiais do Claude CLI via `CLAUDE_CONFIG_DIR`, troca/login por terminal do CLI, associação do perfil a chats/terminais novos e painel de uso do Claude. O GSH não copia tokens do CLI.

## Validação executada

Com o estado atual do código, passaram:

- `cargo fmt --all -- --check`
- `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
- `CARGO_INCREMENTAL=0 cargo test --workspace --locked --offline` — exit 0. A suíte do app executou 487 testes; também passaram os testes dos demais crates.
- `CARGO_INCREMENTAL=0 cargo build -p gsh-app --release --locked --offline`
- Testes focados de persistência de canais/workspaces, regras Manual/Menção/Todos e argumento `--append-system-prompt`.

Neste ambiente foi necessário chamar Cargo por `/home/heliodev/.cargo/bin/cargo`, porque `~/.cargo/bin` não estava no `PATH`. Os checks acima foram feitos sem acesso visual ao app em execução; não afirmar validação por clique ou screenshot.

## Revisão ainda pendente

Pedi a revisão de segurança ao agente, mas ele falhou antes de começar por limite de uso da conta. Não houve revisão de segurança nem alterações feitas por esse agente. A revisão geral posterior também não aconteceu. Rodar ambas em sequência, corrigir achados e repetir os checks antes de publicar.

## Cards que ainda não devem ser marcados concluídos

- **Canais por contexto (`1dfnhq1n9fc`)**: a UI, persistência e roteamento básico foram integrados. A descrição também pede transcrição própria por canal; verificar se o fluxo existente cobre isso de verdade. O card apareceu como pendente no ClickUp e não foi atualizado.
- **Times e bots em Markdown (`1dfnhq1n9fb`)**: há parser/exportador de Markdown e importação de arquivo com modal obrigatório de revisão, mas o card pede mais: importar de URL GitHub/pasta, revisar canais/skills/rotinas/conexões, compartilhar/exportar pela UI e versionar/atualizar pacote. A implementação atual não cobre tudo isso; não fechar ainda.
- **Uso/limites de contas (`1dfnhq1n9f8`)**: perfis e uso Claude foram implementados. Confirmar os critérios de outros provedores antes de fechar; o ClickUp ainda mostrava “Em progresso”.
- **F4/paridade Flutter** continua aguardando validação da equipe, conforme contexto anterior.

A integração MCP do ClickUp atingiu o limite diário. A sessão autenticada no navegador mostrou os critérios de Canais e Times, mas os estados dos cards não foram modificados nesta retomada.

## Escopo a preservar

- Deixar a Ilha de Agentes para depois.
- Pular trabalho exclusivo de macOS e Windows; priorizar o que pode ser compilado e verificado no Linux.
- Não fazer o teste físico do WhatsApp sem a condição externa já mencionada pelo usuário.

## Próximos passos

1. Revisar os gaps dos cards Canais e Times contra as descrições do ClickUp; completar o que falta ou registrar claramente o limite real.
2. Fazer revisão de segurança e, depois, revisão geral; corrigir achados e repetir os gates acima.
3. Verificar o status dos cards elegíveis e só fechar os que estiverem integralmente concluídos.
4. Gerar o PDF final solicitado pelo usuário, publicar as alterações revisadas na `main` sem force-push e reportar o resultado.

## Arquivos da retomada

Principais arquivos com alterações nesta retomada: `crates/app/src/main.rs`, `crates/app/src/rail.rs`, `crates/app/src/channel_editor.rs`, `crates/app/src/channel_routing.rs`, `crates/app/src/layout_store.rs`, `crates/app/src/layout_store/workspaces.rs`, `crates/app/src/chat_pane.rs`, `crates/chat/src/protocol.rs`, `crates/core/src/workspace.rs`, `crates/core/src/workspace/channel.rs`, `crates/config/src/flutter_state.rs` e os módulos de Configurações/Claude.
