# rvm-link — Package Contract

## Overview

`rvm-link` faz a **ligação estática inter-módulo** do EXP-0 (ADR-0018). A RIR
representa um módulo; `call_static` só alcança funções do próprio módulo. O
linker recebe vários [`rvm_ir::Module`] e devolve **um** módulo plano, sem
externas, pronto para `rvm_verify::VerifiedModule::verify`.

O ponto de entrada é `link` (`src/lib.rs:link`). Funções **externas** são
`rvm_ir::Function` sem blocos (`Function::is_external`), criadas pelo `import` do
formato textual. O linker resolve cada externa para a função concreta de mesmo
nome em outro módulo e reescreve os `call_static` para IDs globais.

## Interface

- `pub fn link(modules: &[Module], entry_module: usize, entry_function: FnId, limits: &Limits) -> Result<Module, RvmError>`
  (`src/lib.rs:link`).
- Nada mais é público. O resultado é um `Module` sem externas; a verificação e a
  execução seguem o pipeline normal.

## Invariants

- **Sem serialização**: a chamada inter-módulo vira chamada direta por índice; os
  argumentos não são convertidos para bytes (H03).
- **Resolução por nome único**: duas funções concretas com o mesmo nome em módulos
  diferentes tornam a ligação **ambígua** e falham (`invalid_module`).
- **Falha fechada**: módulo sem provedor, nome de módulo duplicado, entrada não
  concreta e `call_static` para ID inexistente produzem `invalid_module`; o linker
  nunca devolve módulo com externas.
- **Determinismo**: a ordem dos módulos define os IDs globais; sem `HashMap`,
  relógio, ambiente ou estado global. Mesma entrada ⇒ mesma saída.
- **Núcleo puro**: não importa `rvm-text`, `rvm-verify` nem `rvm-interpreter`; só
  `rvm-spec` e `rvm-ir`.

## Side Effects

- `None`. Transformação pura em memória; não lê nem escreve recursos do host.

## Verification

- Run: `cargo test --workspace`
- Alvo dedicado: `crates/rvm-conformance/tests/linking.rs` (ligação, diferencial de
  bytecode e modos de falha).
