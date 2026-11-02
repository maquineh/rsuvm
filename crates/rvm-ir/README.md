# rvm-ir

## Overview

Diretório do modelo de RIR experimental e da checagem estrutural que separa
"módulo representável" de "módulo inválido". Fica entre `rvm-spec` (semântica) e
os crates que consomem módulos (`rvm-verify`, `rvm-text`, `rvm-bytecode`,
`rvm-interpreter`).

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do componente `rvm-ir`: modelo `Module` /
  `Function` / `Block` / `Op` / `Terminator`, invariantes de IDs densos e
  `structure::check`.
