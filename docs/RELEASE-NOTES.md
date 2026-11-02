# RSUVM — Experimental Release

**English** | [Português (BR)](./RELEASE-NOTES.pt-BR.md)

> **This is a research alpha.** Single-tenant, not a sandbox, no process/OS
> isolation, no multi-tenant support. IR/text/bytecode formats and the Rust API
> are experimental and may change without compatibility.

## Highlights

- RIR verifier, deterministic interpreter, and fuel/depth/modeled-memory quotas.
- Types `unit`, `bool`, `i64`, `string`, `f64`, `array`, `struct`, `option`, and
  `result`.
- Experimental textual format and bytecode, with round-trip.
- Static inter-module linking.
- Host ABI `read_file`/`filesystem_read` with capability, canonicalized scope,
  1 MiB ceiling per read, and quota reservation before I/O.
- `rsuvm` CLI (`run`, `run-bc`, `check`, `fmt`, `encode`).

## Security and limits

- Default deny; `execute` without a host denies effects.
- Absolute file scope, no `..`, canonicalized; symlinks/junctions pointing
  outside are denied; regular files only.
- Quota reservation before I/O; 1 MiB ceiling per read.
- **No** aggregate I/O/time/rate quota, OS isolation, or independent audit.
- TOCTOU and hard links remain residual.

Read [SECURITY.md](../SECURITY.md), [USER-GUIDE](./USER-GUIDE.md), and
[COMPATIBILITY](./COMPATIBILITY.md).

## Verification

- `cargo test --workspace --locked`
- `cargo test --release --workspace --locked`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo fmt --all -- --check`
- CI and results: link to this tag's run.

## Checksums

Download `SHA256SUMS` with the artifacts and verify it before running.

## Security reporting

Do not open a public issue. Use
<https://github.com/maquineh/rsuvm/security/advisories/new>.
