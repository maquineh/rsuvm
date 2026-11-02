# Changelog

**English** | [Português (BR)](./CHANGELOG.pt-BR.md)

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The
project uses semantic versioning `0.x`: incompatible changes may occur in minor
releases. IR/text/bytecode formats are **experimental** and have no cross-version
compatibility.

## [0.1.0-alpha.1] - 2026-11-02

### Added

- Workspace of `rvm-spec`, `rvm-ir`, `rvm-runtime`, `rvm-verify`, `rvm-text`,
  `rvm-bytecode`, `rvm-interpreter`, `rvm-link`, `rvm-cli`, and
  `rvm-conformance` crates, with no external dependencies in the core.
- RIR verifier with structural checks, CFG, dominance, types, and signatures.
- Deterministic interpreter with fuel, depth, modeled memory, and traps.
- Types `unit`, `bool`, `i64`, `string`, `f64`, `array`, `struct`, `option`, and
  `result`, with a type-id table.
- Experimental textual format and linear bytecode.
- Static inter-module linking (`rvm-link`).
- Minimal host ABI `read_file`/`filesystem_read`, with `Capabilities`, `Host`,
  `NoHost`, and `execute_with_host`.
- `rsuvm` CLI with `run`, `run-bc`, `check`, `fmt`, `encode`, and
  `--allow-fs-read`.
- Conformance corpus with 62 valid and 42 invalid cases.
- 176 workspace tests, Linux/Windows CI, MSRV, and a pre-registered benchmark.

### Security

- Default deny: `execute` without a host denies every effect.
- Absolute file scope, no `..`, canonicalized root and target, rejection of
  symlinks/junctions pointing outside, and regular-files-only reads.
- 1 MiB ceiling per read and quota reservation before I/O.
- `unsafe` forbidden in the workspace; no external dependencies in the core.
- Short coverage-guided fuzzing campaign (run `41543777109`, 3 × 120 s,
  **24,751,525 executions, 0 observed crashes**); not a long campaign or audit.

### Known limitations

- TOCTOU between canonicalization and open; hard links are not detected.
- No aggregate I/O/time/rate quota; the 1 MiB ceiling is per call.
- No process/OS isolation and no multi-tenant support.
- No independent audit and no long-running fuzzing campaign.
- Experimental formats and API, with no cross-version compatibility.

[0.1.0-alpha.1]: https://github.com/maquineh/rsuvm/releases/tag/v0.1.0-alpha.1
