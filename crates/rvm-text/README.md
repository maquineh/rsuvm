# rvm-text

## Overview

Diretório do formato textual experimental usado pelo corpus de conformidade e
pelo comando `rsuvm fmt`. Entrada não confiável: o parser aplica limites antes de
lexar, rejeita bloco sem terminador e só devolve módulo estruturalmente válido.

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do componente `rvm-text`: `parse`, `print`,
  gramática, resolução de nomes e mapeamento de códigos de erro.
