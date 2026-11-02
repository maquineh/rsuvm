# rvm-conformance

## Overview

Diretório do corpus de conformidade em dados e dos testes de integração. É a
prova executável do EXP-0: casos válidos e inválidos derivados de
`docs/fundacao/TESTES/CONFORMIDADE.md` (C-001..C-006, N-001..N-012) mais casos
adicionais identificados como tal. A meta de cobertura por operação está em
`docs/CONTRATOS/CORPUS-COBERTURA.md`.

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do pacote: nenhum símbolo público de
  biblioteca; define invariantes do corpus e dos testes.
- [corpus](./corpus/README.md) — formato do corpus (`.rvm` + metadados `; id:` e
  `; expect:`) e regras de unicidade/estágio.
- `tests/` — alvos de integração `conformance.rs` e `properties.rs` (sem contrato
  de código; são o mecanismo de verificação).
