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
  `AdapterError` e o subconjunto suportado.
- `tests/` — `diferencial.rs` (Lift→RIR→verificação→execução).

Verificação: `cargo test --manifest-path frontends/lift-rir/Cargo.toml`.
Relatório: `docs/RELATORIOS/ADAPTADOR-LIFT-RIR.md`.
