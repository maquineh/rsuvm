# Compatibility and Versioning

**English** | [Português (BR)](./COMPATIBILIDADE.md)

## Project version

The workspace uses `0.1.0-alpha.1` (the first public alpha). While it is `0.x`:

- **minor** may contain incompatible changes;
- **patch** should contain compatible fixes;
- relevant format/API changes are recorded in `CHANGELOG.md` and, when they close
  a decision, in an ADR.

There is no LTS. Support covers the latest published version on a best-effort
basis.

## IR, text, and bytecode formats

- RIR is experimental; `IR_VERSION = 1` identifies the current shape, not a
  stability promise.
- The textual format (`EXP-0-TEXT`) and bytecode (`RSVMB1`) are **experimental
  and non-normative**. There is no cross-version compatibility.
- The CLI may reject bytecode from another revision with
  `unsupported_version`, `unknown_opcode`, or `invalid_module`.
- To store modules, keep the source text and the binary version used; do not
  treat bytecode as a stable archival format.

## Rust API

- Crates are `0.x`; the public API may change in a minor release.
- `rvm-spec` fixes stable error codes; renaming/reclassifying requires updating
  `docs/CONTRATOS/ERROS-EXP-0.md` and an ADR.
- `VerifiedModule` is the only execution gate; there is no stability promise for
  internal details.
- `Host`, `Capabilities`, and `BuiltinHost` are an experimental ABI; extensions
  require an ADR and threat model.

## CLI

- Subcommand names, flags, and exit codes are documented in
  [USER-GUIDE](./USER-GUIDE.md) and `docs/CONTRATOS/ERROS-EXP-0.md`.
- Error messages, `Display` formatting, and diagnostic order may change.
- The canonical `fmt` output is reparseable by the same version; it is not
  guaranteed across versions.

## Platforms

| Platform | Status |
|---|---|
| Linux x86_64 | CI and development |
| Windows x86_64 | CI and development |
| macOS | untested; do not distribute without validation |
| other architectures | untested |

MSRV: Rust `1.75` (verified by `cargo +1.75.0 check --workspace --locked`).

## Distribution channels

- Source code on GitHub (`main`).
- Binaries on GitHub Releases, with `SHA256SUMS`.
- crates.io: **not published** in this alpha (`publish = false`). A future
  publication will require API stability, metadata, docs.rs, and a semver policy.

## Security changes

Security fixes may be released without detailed prior notice, with a note in
`CHANGELOG.md` and, when appropriate, a GitHub Security Advisory. See
[SECURITY.md](../SECURITY.md).
