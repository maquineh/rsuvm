# rvm-spec

## Overview

Diretório do crate que define o núcleo semântico do EXP-0: tipos, operações com
overflow verificado, tabela de códigos de erro, diagnóstico com localização e
limites estruturais. É a raiz do grafo de dependências: nenhum outro crate do
workspace é importado aqui.

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do componente `rvm-spec`: `Type`, `BinOp`,
  `CmpOp`, `ErrorCode`/`ErrorClass`, `RvmError`, `Limits`, `IR_VERSION`.
