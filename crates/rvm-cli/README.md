# rvm-cli

## Overview

Diretório do binário `rsuvm`. Ferramenta de desenvolvimento para executar,
verificar, formatar e codificar módulos do protótipo. Nega efeitos por padrão e
concede `filesystem_read` somente com `--allow-fs-read <dir>`; não é um serviço:
lê um arquivo, aplica o pipeline verificado e imprime valor ou diagnóstico com
código de saída estável.

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do componente `rvm-cli`: subcomandos, opções,
  códigos de saída e ausência de concessão de capability.
