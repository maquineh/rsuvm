# lift-rir — Package Contract

## Overview

`lift-rir` traduz um subconjunto de programas **Lift** para a RIR do EXP-0. A
entrada é código-fonte `.lift` (via `lift_lexer`/`lift_parser`/`lift_sema`) ou uma
`HirProgram` já analisada; a saída é um `rvm_ir::Module` pronto para
`rvm_verify::VerifiedModule::verify`. O crate fica fora do núcleo (ADR-0001) e é um
workspace separado (ADR-0021).

## Interface

- `pub fn lower_source(source: &str, entry: &str) -> Result<Module, AdapterError>`
  (`src/lib.rs:lower_source`) — roda o pipeline do Lift e baixa para a RIR.
- `pub fn lower(program: &HirProgram, entry: &str) -> Result<Module, AdapterError>`
  (`src/lib.rs:lower`) — baixa uma HIR já analisada.
- `pub enum AdapterError { LiftPipeline(String), Unsupported(String) }`
  (`src/lib.rs:AdapterError`).
- Binário **`lift-run`** (`src/bin/lift_run.rs`): CLI que roda um `.lift` ponta a
  ponta na RVM e imprime o valor (`i64:30`, `bool:true`, `unit`), com os mesmos
  códigos de exit do `rsuvm` (0/2/3/4/1).

## Invariants

- **Subconjunto explícito**: só funções puras `int`/`boolean`/`void`, `let`/`const`
  e `return`, um `if/else` **terminal** (ambos os ramos retornam), `+ - *`,
  comparações, negação unária e chamadas internas. Qualquer outra construção é
  `AdapterError::Unsupported` — nunca um módulo incorreto.
- **Sem phi de junção**: o `if/else` terminal passa o ambiente como parâmetros de
  bloco para os dois ramos; como ambos retornam, não há merge.
- **Overflow é trap**: `+ - *` viram `*_i64_checked`, como no EXP-0.
- **Determinismo**: a ordem das declarações define os `FnId`; sem `HashMap`,
  relógio ou ambiente.
- **Fronteira**: o núcleo não importa este crate; ele depende do núcleo, nunca o
  contrário.

## Side Effects

- `None`. Transformação pura em memória; não lê arquivos, rede, ambiente ou
  relógio. (O teste diferencial compara com o oráculo do Lift registrado, sem
  invocar o compilador do Lift em tempo de teste.)

## Verification

- `cargo test --manifest-path frontends/lift-rir/Cargo.toml` (8 testes em
  `tests/diferencial.rs` e 5 em `tests/cli.rs`).
- Oráculo do Lift e comparação: `docs/RELATORIOS/ADAPTADOR-LIFT-RIR.md`.
