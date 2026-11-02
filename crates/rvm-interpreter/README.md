# rvm-interpreter

## Overview

Diretório do executor de referência. Consome somente `VerifiedModule` e executa
com quotas explícitas: fuel por despacho e profundidade por frame. É o oráculo
semântico usado pelo corpus e pelo CLI; não emite código nativo nem concede
efeitos de host.

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do componente `rvm-interpreter`: `execute`,
  `RunOutcome`, semântica de fuel/profundidade, traps e pilha de frames explícita.
