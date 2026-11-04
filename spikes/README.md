# Spikes de UI (iced vs gpui)

Dois apps com o mesmo cenário, para comparar RAM, CPU e uso de GPU. Não fazem parte do produto.

Cenário: grid de 48 linhas de texto monoespaçado, uma linha nova a cada 16 ms, mais uma ilha preta no topo com largura oscilando e texto "N agentes".

Cada pasta é um workspace próprio (por isso o `[workspace]` vazio no `Cargo.toml`).

```bash
cd spikes/iced && cargo build --release && ./target/release/spike-iced
cd spikes/gpui && cargo build --release && ./target/release/spike-gpui
```

No Windows, use `target\release\spike-iced.exe` e `target\release\spike-gpui.exe`.

Roteiro de medição: card "[SPIKE] Comparar iced vs gpui no Windows" no ClickUp.
