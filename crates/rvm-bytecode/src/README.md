# rvm-bytecode — Package Contract

## Overview

`rvm-bytecode` implementa uma codificação binária **experimental e não normativa**
da RIR (ADR-0011; ADR-0004 continua aberto). Existe para exercitar a fronteira de
decoder hostil e permitir o diferencial `RIR ↔ bytecode`. Nenhum formato wire é
estável, não há checksum e bytes não são confiáveis. Depende de `rvm-spec` e
`rvm-ir`.

## Interface

- `MAGIC` — `RSVMB1` (`lib.rs:MAGIC`).
- `FORMAT_VERSION` — `1` (`lib.rs:FORMAT_VERSION`).
- `encode(module: &Module) -> Vec<u8>` — determinístico, espelho linear da RIR
  (`lib.rs:encode`).
- `decode(bytes: &[u8], limits: &Limits) -> Result<Module, RvmError>`
  (`lib.rs:decode`).

## Invariants

- Layout little-endian documentado em
  `docs/CONTRATOS/BYTECODE-EXP-0-IMPLEMENTACAO.md`: magic, `u16` versão, `u16`
  flags (precisa ser `0`), strings `u16`+UTF-8, contagens `u16`/`u32`, opcodes e
  tags fixos.
- `bytes.len() > Limits::max_input_bytes` é `input_too_large` antes de ler o
  corpo; magic inválido é `syntax_error`; versão ≠ 1 ou flags ≠ 0 é
  `unsupported_version`.
- Toda leitura verifica bytes restantes; faltando, `truncated_input`. Nenhuma
  indexação direta, `unwrap` ou `panic` no caminho de decodificação.
- **Contagem antes de alocação**: cada contagem declarada é comparada com o limite
  estrutural correspondente **e** com os bytes restantes antes de reservar
  memória; contagem maior que os bytes restantes é `truncated_input`.
- Tag de tipo fora de `0..=2` é `unknown_type`; `const_bool` fora de `0|1` é
  `invalid_number`; opcode/kind desconhecido é `unknown_opcode`.
- Bytes excedentes ao final são `invalid_module`; o resultado passa por
  `rvm_ir::structure::check` antes de ser devolvido.
- O decoder nunca devolve módulo parcial: ou módulo completo e estruturalmente
  válido, ou erro. O decoder não confia em metadado do emissor: versão, tipos e
  contagens são reconferidos.
- `encode` é determinístico (mesmo `Module` → mesmos bytes) e não inclui padding,
  hash, checksum ou offset de seção — nenhum deles é mecanismo de segurança.

## Side Effects

- `None`. Sem IO, rede, ambiente, relógio ou processos. `encode` aloca
  proporcional ao módulo (já validado); `decode` aloca somente depois das
  checagens de limite e de bytes restantes.

## Verification

- Run: `cargo test --workspace`
