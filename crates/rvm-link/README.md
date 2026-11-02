# rvm-link

## Overview

Ligação **estática inter-módulo** do EXP-0 (ADR-0018). A RIR representa um módulo
e `call_static` só alcança o próprio módulo; este crate resolve **declarações
externas** (`Function` sem blocos, criadas por `import` no texto) para a função
concreta de mesmo nome em outro módulo e devolve um `Module` plano, sem externas,
pronto para `rvm-verify`. Sem serialização de argumentos: a chamada inter-módulo
vira chamada direta por índice. Depende de `rvm-spec` e `rvm-ir`.

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do pacote: `link` e invariantes
  (determinismo, falha fechada, resolução por nome único).
- `tests/` — não há alvo próprio; a verificação de ponta a ponta vive em
  `crates/rvm-conformance/tests/linking.rs`.

Verificação: `cargo test --workspace`.
