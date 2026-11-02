# lift-rir

## Overview

Protótipo do adaptador **Lift → RIR** (ADR-0021). Consome a **HIR pública** do Lift
(`lift_sema::analyze` → `HirProgram`) e emite um `rvm_ir::Module` para um
subconjunto de funções puras `int`/`boolean`/`void`. É um **workspace Cargo
separado**: depende do Lift por git fixado (`5e59af5`) e não entra no build/CI do
núcleo. O teste diferencial compara o resultado executado na RSUVM com o oráculo
do compilador do Lift.

## Subdirectories & Boundaries

- [src](./src/README.md) — contrato do pacote: `lower_source`/`lower`,
  `AdapterError` e o subconjunto suportado; binário `lift-run`
  (`src/bin/lift_run.rs`).
- `tests/` — `diferencial.rs` (Lift→RIR→verificação→execução) e `cli.rs` (binário
  `lift-run`: saída e códigos de exit).

## Binário `lift-run`

```bash
cargo run --manifest-path frontends/lift-rir/Cargo.toml --bin lift-run -- \
    fib.lift fib --arg i64:10
# i64:55
```

Verificação: `cargo test --manifest-path frontends/lift-rir/Cargo.toml`.
Relatório: `docs/RELATORIOS/ADAPTADOR-LIFT-RIR.md`.
