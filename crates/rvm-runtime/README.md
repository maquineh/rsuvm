# rvm-runtime

## Overview

Diretório de valores de runtime, configuração de quotas e capabilities do host.
É a camada de política de execução (fuel, profundidade, efeitos permitidos) que o
intérprete consome e que o CLI instancia sem nenhuma concessão.

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do componente `rvm-runtime`: `Value`,
  `ExecConfig`, `CallDepthLimit`, `Effect`, `Capabilities` e `HostApi`
  deny-by-default.
