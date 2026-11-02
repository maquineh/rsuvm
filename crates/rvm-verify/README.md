# rvm-verify

## Overview

Diretório do verificador experimental. Recebe um `Module` estruturalmente
validado e entrega `VerifiedModule` — a única entrada aceita pelo intérprete. É o
ponto onde tipos, fluxo de controle e dominância são decididos, antes de qualquer
despacho.

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do componente `rvm-verify`:
  `VerifiedModule::verify`, regras de CFG/dominância/tipos e garantia de que
  falhas são erros de módulo.
