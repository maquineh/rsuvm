# crates

## Overview

Workspace Cargo da RSUVM (EXP-0): dez crates com dependências acíclicas e
**zero dependências externas** (apenas `std`). O grafo é:

~~~text
rvm-spec
 ├── rvm-ir ── rvm-verify ── rvm-interpreter ── rvm-cli
 │        ├── rvm-text ─────────────────────────┘
 │        └── rvm-bytecode ─────────────────────┘
 │        └── rvm-link ── rvm-conformance (dev)
 └── rvm-runtime ── rvm-interpreter
                    rvm-conformance (somente dev-dependencies)
~~~

A separação é imposta pelo compilador (ADR-0009): `rvm-interpreter` não pode
importar `rvm-text`, e nenhum crate do núcleo pode importar adaptador de
linguagem. `rvm-verify` é o único produtor de `VerifiedModule`, tipo que o
executor aceita.

## Subdirectories & Boundaries

- [rvm-spec](./rvm-spec/README.md) — raiz semântica: tipos, operações com overflow
  verificado, códigos de erro, limites. Sem dependências.
- [rvm-ir](./rvm-ir/README.md) — modelo da RIR e checagem estrutural antes de
  alocar (`structure::check`). Depende de `rvm-spec`.
- [rvm-runtime](./rvm-runtime/README.md) — valores, quotas (`ExecConfig`,
  `CallDepthLimit`) e capabilities deny-by-default. Depende de `rvm-spec`.
- [rvm-verify](./rvm-verify/README.md) — tipos, CFG, dominância e a única
  construção de `VerifiedModule`. Depende de spec e ir.
- [rvm-text](./rvm-text/README.md) — parser e impressão canônica do formato
  textual experimental (entrada do corpus e do CLI). Depende de spec e ir.
- [rvm-bytecode](./rvm-bytecode/README.md) — codificação binária experimental e
  decoder limitado (não normativo; ADR-0004 aberto). Depende de spec e ir.
- [rvm-interpreter](./rvm-interpreter/README.md) — executor determinístico com
  fuel, profundidade, traps e pilha de frames explícita. Depende de spec, ir,
  runtime e verify.
- [rvm-link](./rvm-link/README.md) — ligação estática inter-módulo: resolve
  declarações externas em um módulo plano (ADR-0018). Depende de spec e ir.
- [rvm-cli](./rvm-cli/README.md) — binário `rsuvm` (`run`, `run-bc`, `check`,
  `fmt`, `encode`); não concede capabilities.
- [rvm-conformance](./rvm-conformance/README.md) — corpus em dados (21 válidos, 25 inválidos) e testes de conformidade/propriedades.
