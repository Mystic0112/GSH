//! Modelos de texto dos itens novos (copiados de `claude_harness_source.dart`).
//!
//! Seguem duas lições do Harness² (google-research/harness2): instrução separa
//! **fatos** de **procedimento**, e hook que bloqueia **ensina**: diz ao agente
//! por que parou e o que fazer no lugar.

/// `CLAUDE.md` novo.
pub(crate) const CLAUDE_MD: &str = r##"# Instruções do projeto

<!-- Este arquivo entra inteiro em TODA mensagem: fique no essencial.
     Fatos aqui; procedimento longo vira skill (.claude/skills/). -->

## Fatos

- Stack: <linguagem, framework, versões que importam>
- Comandos: `<instalar>` · `<testar>` · `<lint>` · `<rodar>`
- Estrutura: <onde mora o quê; o que é gerado e não se edita à mão>

## Regras

- <o que nunca fazer aqui, e por quê>
- <convenção que o agente erra se ninguém avisar>

## Procedimentos

<!-- Só o nome e quando usar; os passos ficam na skill. -->
- Para <tarefa recorrente>, use a skill `<nome>`.
"##;

/// `SKILL.md` novo; `$name` vira o nome do item.
const SKILL_MD: &str = r##"---
name: $name
description: Use quando <situação concreta que o usuário descreve>. Faz <resultado>.
---

# $name

<!-- A descrição acima é o que o agente vê sempre (custa em todo turno);
     este corpo só carrega quando a skill é acionada. -->

## Quando usar

- <gatilho 1>
- Não use para <caso parecido que é de outra skill>.

## Passos

1. <primeiro passo, com o comando exato>
2. <...>

## Como conferir

- <o comando ou o sinal de que deu certo antes de dizer que terminou>
"##;

/// Subagente novo; `$name` vira o nome do item.
const AGENT_MD: &str = r##"---
name: $name
description: Use para <tipo de tarefa>. Devolve <o que volta para quem chamou>.
tools: Read, Grep, Glob
---

Você é o subagente $name.

## Papel

<o que ele faz e o que fica fora do escopo dele>

## O que devolver

<formato da resposta: só o que quem chamou precisa, sem despejar arquivos>
"##;

/// Comando novo.
pub(crate) const COMMAND_MD: &str = r##"---
description: O que este comando faz.
argument-hint: <argumento>
---

Instruções do comando. Use $ARGUMENTS para o que vier depois do nome.
"##;

/// Hook que ensina: bloqueia com `exit 2` e o stderr volta para o agente como
/// instrução, por que parou e o que fazer no lugar.
pub(crate) const HOOK_SH: &str = r##"#!/usr/bin/env bash
# Hook PreToolUse (Bash), ligado em .claude/settings.json.
# Recebe o pedido da ferramenta em JSON no stdin.
#   exit 0 = deixa passar
#   exit 2 = bloqueia, e o que sair no stderr volta para o AGENTE.
# Bloqueio bom ensina: diz por que parou e o que fazer no lugar, senão o
# agente só tenta de novo de outro jeito.
input=$(cat)

# --force e -f, mas não --force-with-lease (que é o caminho certo).
if printf '%s' "$input" | grep -Eq 'git push[^"]*( --force([^-]|$)| -f([ "]|$))'; then
  cat >&2 <<'MSG'
Bloqueado: `git push --force` reescreve o histórico do remoto e apaga o
trabalho de quem já puxou.
Faça no lugar: `git push --force-with-lease`, ou pergunte ao usuário.
MSG
  exit 2
fi

exit 0
"##;

pub(crate) fn skill_md(name: &str) -> String {
    SKILL_MD.replace("$name", name)
}

pub(crate) fn agent_md(name: &str) -> String {
    AGENT_MD.replace("$name", name)
}

/// Comando que o `settings.json` usa para chamar um hook do projeto (fecha com
/// `<slug>.sh"`). Aspas por causa de pasta com espaço.
pub(crate) const HOOK_COMMAND_PREFIX: &str = r#"bash "$CLAUDE_PROJECT_DIR/.claude/hooks/"#;
