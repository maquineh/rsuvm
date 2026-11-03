# RSUVM Blueprint — Rust Universal Virtual Machine

**English** | [Português (BR)](./README.pt-BR.md)

[![CI](https://github.com/maquineh/rsuvm/actions/workflows/ci.yml/badge.svg)](https://github.com/maquineh/rsuvm/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)

**Release:** [v0.1.0-alpha.1](https://github.com/maquineh/rsuvm/releases/tag/v0.1.0-alpha.1)

## System Overview

RSUVM is an **experimental research prototype** (EXP-0): a Rust virtual machine
that executes a common intermediate representation (RIR) produced by independent
frontends. The core knows nothing about Lift, Rio, or any host language: it
accepts `unit`/`bool`/`i64`, pure functions, explicit block control flow, and
static calls, verifies the whole module **before** execution, and runs under fuel
and depth quotas. No format is stable; nothing here is a validated sandbox or an
authorization for production.

**Documentation base:** the RVM-Foundation-0.1 package, copied unmodified into
[docs/fundacao](./docs/fundacao/ORIGEM.md) (24 files verified by SHA-256).

Pipeline:

~~~text
.rvm text ── rvm-text::parse ──┐
                               ├── Module ── structure::check ── verify
.rsvmbc bytes ─ rvm-bytecode ──┘                                   │
                                                                   ▼
                     rvm-interpreter::execute(VerifiedModule)  →  value | trap | limit
~~~

## Quick Start

```bash
# Install the binary into ~/.cargo/bin.
cargo install --path crates/rvm-cli --locked
rsuvm run examples/soma.rvm --arg i64:10 --arg i64:20   # i64:30
rsuvm check examples/soma.rvm

# Or run without installing:
cargo run -p rvm-cli -- run examples/soma.rvm --arg i64:10 --arg i64:20
```

- Full guide: [USER-GUIDE](./docs/USER-GUIDE.md)
- Embedding the VM: [EMBEDDING](./docs/EMBEDDING.md)
- Versioning: [COMPATIBILITY](./docs/COMPATIBILITY.md)
- What is implemented: [IMPLEMENTATION](./docs/IMPLEMENTATION.md)
- Lift → RVM bytecode: [LIFT-TO-RVM](./docs/LIFT-TO-RVM.md)
- Security policy: [SECURITY](./SECURITY.md)
- Contributing: [CONTRIBUTING](./CONTRIBUTING.md)
- Changelog: [CHANGELOG](./CHANGELOG.md)

Portuguese versions: [README](./README.pt-BR.md),
[user guide](./docs/GUIA-USUARIO.md), [compatibility](./docs/COMPATIBILIDADE.md),
[embedding](./docs/INCORPORACAO.md), [security](./SECURITY.pt-BR.md).

## Subdirectories & System Boundaries

- [crates](./crates/README.md) — workspace of ten acyclic crates, with no
  external dependencies: spec, ir, runtime, verify, text, bytecode, interpreter,
  link, cli, and conformance. Dependency boundaries are enforced by the compiler.
- [docs](./docs/ARQUITETURA.md) — architecture, contracts (`docs/CONTRATOS/`),
  repository ADRs (`docs/ADRs/`), foundation ADRs (`docs/fundacao/ADRs/`), and
  reports (`docs/RELATORIOS/`).
- [examples](./examples/soma.rvm) — demonstration `.rvm` modules in the
  experimental textual format; they are not a code contract.
- [scripts](./scripts/gerar-evidencias.ps1) — generates the execution evidence
  log; it is not a runtime artifact.
- [fuzz](./fuzz/README.md) — **separate** coverage-guided fuzzing workspace
  (nightly, dev dependency; not part of the main workspace; ADR-0020).
- [frontends](./frontends/README.md) — language adapters outside the core; the
  Lift→RIR prototype (ADR-0021), also a separate workspace. It can execute a
  supported Lift subset or emit `.rsvmbc` with `lift-encode`; see
  [LIFT-TO-RVM](./docs/LIFT-TO-RVM.md).

## Workspace Invariants

- Zero external dependencies: only `std`; a new crate requires an ADR and
  approval.
- `unsafe_code = "forbid"` and `missing_docs = "deny"` on all targets
  (`Cargo.toml:[workspace.lints.rust]`).
- `overflow-checks = true` also in the release profile: overflow semantics come
  from `checked_*`, but accidental arithmetic must not silently wrap
  (`Cargo.toml:[profile.release]`).
- Full verification precedes execution; the executor only receives
  `rvm_verify::VerifiedModule`.
- ADR-0004 remains open: no wire format is stable; bytecode is experimental
  (ADR-0011).

## Global Invariants

- **Frontend independence** (ADR-0001): no core crate imports or mentions
  Lift/Rio; adapters, when present, live outside `crates/`.
- **Fail closed**: textual/binary input, names, indices, and counts are
  untrusted; limits are checked before allocating, indexing, recursing, or
  dispatching.
- **No implicit effects** (ADR-0005): no capability is granted by default. The
  only exception is `read_file`/`filesystem_read`, explicitly granted by
  `Capabilities::grant` + `execute_with_host` or by the CLI with
  `--allow-fs-read <dir>` (ADR-0036).
- **Determinism**: same module + limits + configuration + arguments ⇒ same value
  or same error code on the pure path; no `HashMap`, clock, environment, or
  filesystem. `host_read_file` is an explicit, gated call whose result depends on
  the backend and system state; it is not deterministic by construction.
- **Stable diagnostics**: `rvm-spec` codes and classes are contract; human
  messages are not (`docs/CONTRATOS/ERROS-EXP-0.md`).
- **Depth ceiling** applied over a heap-allocated frame stack (ADR-0012): the
  recursive version overflowed the native stack with 256 frames.

## Global Side Effects

- The core (`rvm-spec`, `rvm-ir`, `rvm-runtime`, `rvm-verify`, `rvm-text`,
  `rvm-bytecode`, `rvm-interpreter`) is pure: no filesystem, network,
  environment, clock, subprocess, or `unsafe`. `rvm-interpreter` only executes an
  effect when given an explicit `Host` in `execute_with_host`; `execute` denies
  everything.
- `rvm-cli` reads the indicated input file, writes only the `encode` output file,
  uses process `stdout`/`stderr`, and, with `--allow-fs-read <dir>`, reads files
  under the granted directory during `host_read_file` (ADR-0036).
- The test suite reads `crates/rvm-conformance/corpus` and creates temporary
  files in CLI tests; it does not write to the repository.
- `scripts/gerar-evidencias.ps1` writes `docs/RELATORIOS/EVIDENCIAS-EXECUCAO.md`
  (execution log), never code.

## System Verification

- Run: `cargo test --workspace --locked`.
- Additional gates: `cargo clippy --workspace --all-targets --locked -- -D
  warnings` and `cargo fmt --all -- --check`.
- CI: `.gitlab-ci.yml` (GitLab) and `.github/workflows/ci.yml` (GitHub Actions)
  run `fmt --check`, `clippy -D warnings`, `test`, and `test --release` on Linux
  and Windows; there is an MSRV (`1.75`) job and `cargo doc` with `-D warnings`.
  Manual auxiliary workflows: `bench.yml` (pre-registered benchmark) and
  `fuzz.yml` (coverage-guided fuzzing, nightly). Releases on `v*` tags use
  `.github/workflows/release.yml`; see `docs/RELEASE-CHECKLIST.md`.
- Current results and raw log:
  [RELATORIO-IMPLEMENTACAO](./docs/RELATORIOS/RELATORIO-IMPLEMENTACAO.md) and
  [EVIDENCIAS-EXECUCAO](./docs/RELATORIOS/EVIDENCIAS-EXECUCAO.md): 187 tests
  green (the symlink test is `cfg(unix)`, the junction test is `cfg(windows)`),
  66 valid + 48 invalid corpus cases.
- Packaging and clean install:
  [DRYRUN-RELEASE-0.1.0-alpha.1](./docs/RELATORIOS/DRYRUN-RELEASE-0.1.0-alpha.1.md).
- Short fuzzing campaign:
  [FUZZING-EXP-0](./docs/RELATORIOS/FUZZING-EXP-0.md) §7.

## Security (Declared Limits)

Private vulnerability channel: [SECURITY.md](./SECURITY.md).

- Memory-safe Rust reduces classes of memory errors, but the prototype is **not**
  process/OS isolation; hostile multi-tenant use would require reviewed external
  isolation.
- "Zero panic in the corpus" describes the test run, not the absence of
  vulnerabilities. **Long** fuzzing campaigns and independent audits were **not**
  performed; there was a short campaign (3 × 120 s, run `41543777109`, 24.75 M
  executions, 0 observed crashes), which does not prove the absence of flaws. The
  pre-registered benchmark has recorded measurements (dedicated runner and local
  re-execution after ADR-0034/0035); H06 was confirmed in runs `41429076866` and
  `41520845695`; see
  [BENCHMARKS-PREREGISTRADOS](./docs/RELATORIOS/BENCHMARKS-PREREGISTRADOS.md).
- Lift/Rio integration was **not** done: technical blockers are recorded in
  [BASELINE-RIO](./docs/RELATORIOS/BASELINE-RIO.md) and
  [BASELINE-LIFT](./docs/RELATORIOS/BASELINE-LIFT.md). The Rio and Lift licenses
  were declared `MIT OR Apache-2.0` by the maintainer.

## License

Dual **`MIT OR Apache-2.0`**
([ADR-0014](./docs/ADRs/ADR-0014-LICENCA-DO-REPOSITORIO.md)): reuse may choose
either. Full texts in [LICENSE-MIT](./LICENSE-MIT) and
[LICENSE-APACHE](./LICENSE-APACHE). The Rio reuse policy is in
[ADR-0015](./docs/ADRs/ADR-0015-POLITICA-DE-REUSO-DO-RIO.md). The license allows
code reuse; it is not a security claim and does not replace legal advice.
