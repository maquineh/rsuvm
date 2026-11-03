# lift-rir

## Overview

Prototype **Lift → RIR** adapter (ADR-0021). It consumes Lift's public HIR
(`lift_sema::analyze` → `HirProgram`) and emits an `rvm_ir::Module` for a subset
of `int`/`number`/`boolean`/`string`/`void` functions with `let`/`const`,
**assignment**, `if`/`else if`/`else` (with merge), **`while`**, arrays,
interfaces/structs, `T?`, `??`, `== null`/`!= null`, and short-circuit
`!`/`&&`/`||`.

It is a **separate Cargo workspace**: it depends on Lift pinned by git
(`e04bc26`) and is not part of the core build/CI. The core RSUVM remains free of
external dependencies. The differential test compares the RSUVM result with the
recorded Lift compiler oracle.

## Subdirectories & Boundaries

- [src](./src/README.md) — package contract: `lower_source`/`lower`,
  `AdapterError`, supported subset; `lift-run` and `lift-encode` binaries.
- `tests/` — `diferencial.rs` (Lift→RIR→verify→execute), `cli.rs` (`lift-run`
  output/exit codes), `encode.rs` (`lift-encode` bytecode + execution).

## Binaries

### `lift-run` — execute directly

```bash
cargo run --manifest-path frontends/lift-rir/Cargo.toml --bin lift-run -- \
    examples/lift/fib.lift fib --arg i64:10
# i64:55
```

### `lift-encode` — emit RVM bytecode

```bash
cargo run --manifest-path frontends/lift-rir/Cargo.toml --bin lift-encode -- \
    examples/lift/soma.lift soma /tmp/soma.rsvmbc
rsuvm run-bc /tmp/soma.rsvmbc --arg i64:10 --arg i64:20
# i64:30
```

`lift-encode` runs the Lift pipeline, lowers to RIR, verifies, and then encodes;
it does not execute and does not grant host effects.

## Supported / unsupported

Supported: functions, scalars, arrays, interfaces/structs, `T?`, `??`,
`== null`/`!= null`, `let`/`const`, assignment (non-aliased local), `if`/`else`,
`while`, arithmetic/comparisons/logic, internal calls.

Unsupported (fail closed with `AdapterError::Unsupported`): enums, unions,
generics, closures/function values, higher-order array methods, `match`,
`for-of`, `break`, `continue`, `throw`, `try/catch`, `?` propagation,
`console.log`, top-level statements, imports/modules, FFI, `import interface`,
and `result.ok`/`result.err`.

The RIR subset cannot represent these features directly; the adapter rejects
them instead of approximating semantics. See [LIFT-TO-RVM](../../docs/LIFT-TO-RVM.md)
for the user guide and examples. The RIR now has nominal enums (ADR-0039 and
[ENUMS-UNIONS-PROPOSTA](../../docs/CONTRATOS/ENUMS-UNIONS-PROPOSTA.md)), but the
adapter lowering for enums/unions/`match` remains out of scope in this increment
and still fails closed.

## Verification

```bash
cargo test --manifest-path frontends/lift-rir/Cargo.toml
```

Current tests: 9 CLI + 34 differential + 4 encode = **47 passed**.

Report: `docs/RELATORIOS/ADAPTADOR-LIFT-RIR.md`.
