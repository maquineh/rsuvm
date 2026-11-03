# Lift → RVM: writing Lift programs and generating `.rsvmbc`

**English** | [Português (BR)](./RELATORIOS/ADAPTADOR-LIFT-RIR.md)

This guide shows how to write a Lift program that the adapter supports, execute
it directly, and compile it to RVM bytecode (`.rsvmbc`).

> **Scope:** prototype adapter (`frontends/lift-rir`, ADR-0021). It covers a
> subset of Lift and does **not** implement enums, unions, generics, closures,
> `match`, `for-of`, `try/catch`, FFI, modules, or `console.log`. Unsupported
> constructs fail with `AdapterError::Unsupported` instead of producing a wrong
> module.

## 1. Prerequisites

- The Lift checkout pinned by the adapter: commit `e04bc26`
  (`https://github.com/maquineh/lift`).
- Rust/Cargo **1.94.1+** for Lift (the RSUVM core still declares MSRV 1.75; the
  adapter workspace inherits Lift's requirement).
- The RSUVM workspace checked out next to `frontends/lift-rir`.

Build the adapter binaries:

```bash
cargo build --manifest-path frontends/lift-rir/Cargo.toml --bins
```

Binaries are under `frontends/lift-rir/target/debug/`:
`lift-run` (execute) and `lift-encode` (emit bytecode).

## 2. A minimal Lift program

Lift requires a **function-only** file for this adapter (no top-level
statements). Save as UTF-8 **without BOM**:

```lift
function soma(a: int, b: int): int {
  return a + b;
}
```

Run it directly:

```bash
cargo run --manifest-path frontends/lift-rir/Cargo.toml --bin lift-run -- \
  examples/lift/soma.lift soma --arg i64:10 --arg i64:20
# i64:30
```

## 3. Generate RVM bytecode

```bash
cargo run --manifest-path frontends/lift-rir/Cargo.toml --bin lift-encode -- \
  examples/lift/soma.lift soma /tmp/soma.rsvmbc
# 116 bytes em /tmp/soma.rsvmbc

rsuvm run-bc /tmp/soma.rsvmbc --arg i64:10 --arg i64:20
# i64:30
```

`lift-encode` runs Lift lexer/parser/analyzer, lowers HIR to RIR, verifies the
module with `rvm_verify`, and only then encodes. It does not execute the program
and does not grant host effects.

## 4. Supported subset

- Functions with `int`, `number` (`f64`), `boolean`, `string`, `void`, arrays,
  interfaces/structs, `T?`, and calls.
- `let`/`const`, assignment to a mutable binding, array element, or struct field
  (only when the target is a non-aliased local; functional update).
- `return`, `if`/`else if`/`else`, `while`.
- Literals (`int`, `number`, `string`, `boolean`, arrays, object literals with a
  type annotation), indexing, field access, `null`, `option.some`,
  `option.is_some`, `option.unwrap`, `??`, `== null`/`!= null`.
- Arithmetic `+ - * / %`, comparisons, unary `-`, logic `!`/`&&`/`||` with
  short-circuit, and internal calls.
- Array/struct element/field updates are lowered only when alias analysis proves
  the local is not used as a value; otherwise `Unsupported`.

### Not supported by this adapter

- Enums and unions (nominal tagged types).
- Generics/type parameters, generic interfaces/enums.
- Closures, function values, higher-order array methods.
- `match`, `for-of`, `break`, `continue`, `throw`, `try/catch`, `?` propagation.
- `console.log`, top-level statements, imports/modules, FFI/`import interface`.
- `result.ok`/`result.err` constructors and `try` propagation.

These are Lift features that the RIR subset cannot represent directly. The
adapter fails closed instead of approximating them.

## 5. Examples

| File | Entry | Command | Expected |
|---|---|---|---|
| `examples/lift/soma.lift` | `soma` | `lift-run ... soma --arg i64:10 --arg i64:20` | `i64:30` |
| `examples/lift/fib.lift` | `fib` | `lift-run ... fib --arg i64:10` | `i64:55` |
| `examples/lift/segundo.lift` | `segundo` | `lift-run ... segundo` | `i64:20` |
| `examples/lift/soma_ponto.lift` | `soma_ponto` | `lift-run ... soma_ponto` | `i64:7` |
| `examples/lift/resolve.lift` | `resolve` | `lift-run ... resolve` | `i64:42` |
| `examples/lift/saudacao.lift` | `saudacao` | `lift-run ... saudacao` | `str:ola` |

For each, replace `lift-run` with `lift-encode ... <out.rsvmbc>` and execute the
result with `rsuvm run-bc`.

## 6. Common errors

- `ObjectLiteralNeedsContext`: annotate the binding, e.g.
  `let p: Ponto = { x: 3, y: 4 };`.
- `pipeline do Lift falhou`: Lift syntax/type error; run `lift check` or
  `lift run` in the Lift checkout for the full diagnostic.
- `fora do subconjunto: ...`: the program uses a feature listed in §4.
- Empty output or BOM error: save the `.lift` file as UTF-8 **without BOM**.

## 7. Verification

```bash
cargo test --manifest-path frontends/lift-rir/Cargo.toml
```

The differential suite compares the RSUVM result with the recorded Lift oracle;
`tests/encode.rs` decodes, verifies, and executes the emitted bytecode. See
`docs/RELATORIOS/ADAPTADOR-LIFT-RIR.md` for the detailed matrix.

## 8. Limits

The adapter is a research prototype, not a Lift backend:
it does not preserve ownership/borrowing semantics outside the lowered subset,
does not support effects, and does not prove equivalence for programs it lowers.
The generated module is subject to RVM quotas and the same single-tenant
security model as any `.rvm` module.
