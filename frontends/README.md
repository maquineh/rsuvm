# frontends

## Overview

Adaptadores de linguagem hospedeira para a RIR da RSUVM. Vivem **fora** de
`crates/` para preservar a ADR-0001 (o núcleo não conhece Lift/Rio) e o invariante
de zero dependências do núcleo. Cada adaptador é um **workspace Cargo separado**,
excluído do workspace principal (`exclude = ["frontends"]` em `../Cargo.toml`), e
pode depender de crates da linguagem de origem.

## Subdirectories & Boundaries

- [lift-rir](./lift-rir/README.md) — protótipo Lift→RIR (ADR-0021): consome a HIR
  pública do Lift e emite a RIR do EXP-0 para um subconjunto de funções com
  escalares, arrays, structs e `T?`; também gera `.rsvmbc` via `lift-encode`.
  Enums/unions existem na RIR (ADR-0039), mas o lowering do adaptador e `match`
  ainda não; esses recursos e generics, closures, `for-of`, `try/catch`,
  módulos e FFI são rejeitados. Guia: [LIFT-TO-RVM](../docs/LIFT-TO-RVM.md).

O núcleo (`crates/`) não importa nada daqui; o fluxo é sempre
`linguagem → frontend → rvm_ir::Module → rvm-verify → rvm-interpreter`.
