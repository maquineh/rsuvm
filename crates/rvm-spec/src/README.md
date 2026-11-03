# rvm-spec — Package Contract

## Overview

`rvm-spec` é o núcleo semântico do EXP-0: define tipos, operações aritméticas
verificadas, comparações, códigos de erro estáveis, localização de diagnóstico e
limites estruturais. Não depende de nenhum outro crate do workspace e não conhece
Lift, Rio nem qualquer linguagem hospedeira (ADR-0001). É a camada que fixa a
semântica de overflow independente do perfil de compilação: toda aritmética usa
`i64::checked_*` (`rvm-spec/lib.rs:BinOp::eval_i64`).

## Interface

- `TypeDef`/`TypeId` — tipos escalares e compostos (`array`, `struct`, `enum`,
  `option`, `result`), recursivos por id; builtins canônicos (`TYPE_UNIT`..`TYPE_F64`),
  `builtin_types`, `TypeDef::name`/`is_scalar`, `Display` (`lib.rs:TypeDef`).
- `BinOp` — `AddI64Checked`, `SubI64Checked`, `MulI64Checked`;
  `eval_i64(lhs, rhs) -> Option<i64>` (devolve `None` no transbordo),
  `operand_type`, `result_type`, `name`, `from_name` (`lib.rs:BinOp`).
- `CmpOp` — `Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`; `eval_i64(lhs, rhs) -> bool`,
  `operand_type`, `result_type` (`lib.rs:CmpOp`).
- `Effect` — efeitos de host (`filesystem_read`, `filesystem_write`, `network`,
  `environment`, `clock`, `process`); `name`, `from_name`, `Display`
  (`lib.rs:Effect`).
- `BuiltinHost` — ABI fechada do EXP-0: `ReadFile`, com `name`, `effect`, `code`,
  `scope_arg` e `max_effect_bytes`; `MAX_HOST_READ_BYTES` é o teto de 1 MiB
  (`lib.rs:BuiltinHost`, ADR-0036/ADR-0038).
- `ErrorCategory` — `Decode`, `Verify`, `Trap`, `Resource`, `Capability`.
- `ErrorClass` — `Module`, `Trap`, `ResourceLimit`, `Capability` (RF-05).
- `ErrorCode::OutOfMemory` — quota de memória de frames excedida (classe
  `ResourceLimit`, categoria `Resource`).
- `ErrorCode` — códigos estáveis; `as_str`, `from_stable_name`, `category`,
  `class` (`lib.rs:ErrorCode`). Inclui `invalid_variant` e `value_too_deep`.
- `Location` — `Module`, `Function`, `Block`, `Instruction`, `Value`.
- `RvmError` — `new(code, detail)`, `at(location)`, `code`, `category`, `class`,
  `detail`, `location`, `Display`, `std::error::Error`.
- `Limits` — contagens máximas de funções, blocos, parâmetros, instruções,
  valores, argumentos, nomes, bytes de entrada, structs, enums, variantes e
  campos; `Default` com valores pequenos.
- `IR_VERSION` — versão do formato implementado (valor `1`).

## Invariants

- Semântica de overflow é determinística e igual em debug e release: só
  `checked_add`/`checked_sub`/`checked_mul`; nunca `+`, `-`, `*` diretos em
  operandos `i64` de convidado (`lib.rs:BinOp::eval_i64`).
- `ErrorCode::category` e `ErrorCode::class` são derivadas do código: falha de
  decode/verificação é sempre `ErrorClass::Module`, overflow é `Trap`, fuel e
  profundidade são `ResourceLimit`.
- Nomes estáveis de código não mudam sem ADR + atualização de
  `docs/CONTRATOS/ERROS-EXP-0.md`; teste `tabela_de_erros_e_estavel` fixa os
  nomes usados pelo corpus.
- Não existe coerção implícita entre tipos: operações declaram tipo de operando e
  resultado (`BinOp::operand_type`, `CmpOp::result_type`).
- `Limits::default()` é pequeno de propósito; `rvm-ir` e `rvm-text` comparam
  contagens contra ele **antes** de alocar.
- Códigos não reconhecidos por `from_stable_name` devolvem `None`; o chamador
  falha, nunca assume um valor padrão.

## Side Effects

- `None`. O crate é puro: não lê arquivos, rede, ambiente, relógio ou processos,
  não tem `unsafe` (o workspace aplica `unsafe_code = "forbid"`), não mantém
  estado global mutável e não aloca memória proporcional a entrada não validada.

## Verification

- Run: `cargo test --workspace`
