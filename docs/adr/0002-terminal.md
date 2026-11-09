# ADR 0002: emulador de terminal e PTY

Status: aceito em 2026-11-09.

## Contexto

O terminal é o recurso central do GSH: os agentes rodam dentro dele. O Flutter usava libghostty via flterm. No Rust precisamos de um emulador VT e de um PTY que funcionem nos três sistemas.

Candidatos avaliados:
- **libghostty** (via FFI): exige build com Zig 0.16 e a API C ainda muda entre versões. Descartado.
- **alacritty_terminal** + **portable-pty**: ambos em Rust puro.

## Decisão

- **Emulador VT:** `alacritty_terminal` 0.26 (Apache-2.0). Faz o parse de VT/ANSI e mantém o grid, o cursor e o scrollback.
- **PTY:** `portable-pty` 0.9 (MIT). Usa forkpty no Unix e ConPTY no Windows.

## Evidências

- Teste headless: um bash real sobe num PTY, recebe um comando e a saída chega ao emulador (`gsh-42`).
- Janela de teste com iced: mostra o prompt do bash renderizado.
- Ainda não testado: digitação na janela e resize.

## Consequências

- O grid é desenhado em canvas do iced (ADR 0001), célula por célula.
- O comportamento deve ser igual ao do Flutter, não o motor. A paridade é conferida por teste, não por motor.
- Se a renderização ou o desempenho não atenderem, este ADR é revisto.

## Em aberto

- Digitação, resize e cores de fundo por célula na UI (card "Terminal na UI").
- Conferir a licença das dependências transitivas antes do primeiro release.
