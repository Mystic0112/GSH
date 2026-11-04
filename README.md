# GSH

Harness/IDE para agentes de código. Este repo é a reescrita do GSH Desktop (Flutter) em Rust, com paridade de comportamento e aparência: só a stack muda.

## Estrutura

- `crates/core` (`gsh-core`): domínio puro, sem I/O nem UI.
- `crates/cli` (binário `gsh`): CLI interna (placeholder por enquanto).

A escolha da UI ainda não foi feita; não há crate de UI.

## Checagens

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

O toolchain (stable, com `rustfmt` e `clippy`) é fixado em `rust-toolchain.toml`.
