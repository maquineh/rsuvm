# rvm-bytecode

## Overview

Diretório da codificação binária experimental. O decoder é a superfície hostil do
projeto: recebe bytes arbitrários, aplica limites antes de alocar e rejeita
truncamento, opcodes/tags desconhecidos, versão/flags inválidas e bytes
excedentes. O formato não é normativo e pode mudar sem compatibilidade.

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do componente `rvm-bytecode`: layout,
  `encode` determinístico e invariantes do decoder.
