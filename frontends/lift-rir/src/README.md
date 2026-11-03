# lift-rir — Package Contract

## Overview

`lift-rir` translates a subset of **Lift** programs to the EXP-0 RIR. Input is
`.lift` source (via `lift_lexer`/`lift_parser`/`lift_sema`) or an analyzed
`HirProgram`; output is an `rvm_ir::Module` ready for
`rvm_verify::VerifiedModule::verify`. It lives outside the core (ADR-0001) and is
a separate workspace (ADR-0021).

## Interface

- `pub fn lower_source(source: &str, entry: &str) -> Result<Module, AdapterError>`
  (`src/lib.rs:lower_source`) — runs the Lift pipeline and lowers to RIR.
- `pub fn lower(program: &HirProgram, entry: &str) -> Result<Module, AdapterError>`
  (`src/lib.rs:lower`) — lowers an already analyzed HIR.
- `pub enum AdapterError { LiftPipeline(String), Unsupported(String) }`
  (`src/lib.rs:AdapterError`).
- Binary **`lift-run`** (`src/bin/lift_run.rs`): executes a `.lift` end-to-end on
  the RVM and prints the value (`i64:30`, `bool:true`, `unit`), with the same
  exit codes as `rsuvm` (0/2/3/4/1).
- Binary **`lift-encode`** (`src/bin/lift_encode.rs`): lowers, verifies, and
  writes `.rsvmbc`; it does not execute.

## Invariants

- **Explicit subset:** functions with `int`/`number`/`boolean`/`string`/`void`,
  arrays, interfaces/structs, `T?`, `let`/`const`, **assignment**, `return`,
  `if`/`else if`/`else` (with merge and `if` without `else`), **`while`**;
  arithmetic/comparisons/string comparison, unary negation, short-circuit
  `!`/`&&`/`||`, and internal calls. Any other construct is
  `AdapterError::Unsupported` — never an incorrect module.
- **CFG/SSA with explicit phi:** each `BindingId` is tracked to the current
  `ValueId`; `if`/`else` and `while` carry the environment as **block
  parameters**; `&&`/`||`/`!` become control flow with a `bool` merge.
- **Overflow is a trap:** `+ - *` become `*_i64_checked`, as in EXP-0.
- **Determinism:** declaration order defines `FnId`; no `HashMap`, clock, or
  environment.
- **Boundary:** the core never imports this crate; this crate depends on the
  core, never the reverse.
- **New Lift features fail closed:** enums, unions, generics, closures,
  function values, higher-order array methods, `match`, `for-of`, `break`,
  `continue`, `throw`, `try/catch`, `?`, imports/modules, FFI, and
  `result.ok`/`result.err` are rejected with `Unsupported`.

## Side Effects

- `None` for lowering. `lift-run` reads one `.lift` file and writes to
  stdout/stderr; `lift-encode` reads one `.lift` file and writes one `.rsvmbc`.
  Neither grants filesystem/network/clock/process effects to the Lift program.
  (The differential test compares against the recorded Lift oracle, without
  invoking the Lift compiler at test time.)

## Verification

```bash
cargo test --manifest-path frontends/lift-rir/Cargo.toml
```

Current tests: 9 CLI + 34 differential + 4 encode = **47 passed**.

Lift oracle and matrix: `docs/RELATORIOS/ADAPTADOR-LIFT-RIR.md`.
