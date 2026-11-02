# Contributing to RSUVM

**English** | [Português (BR)](./CONTRIBUTING.pt-BR.md)

Thank you for your interest. RSUVM is a research prototype: changes must preserve
the core's security invariants, determinism, and dependency-free design.

## Before opening a PR

Read, in order:

1. `AGENTS.md`;
2. `README.md` and `docs/ARQUITETURA.md`;
3. the contracts under `docs/CONTRATOS/`;
4. the ADR for the area you are changing;
5. `SECURITY.md` if the topic is security.

## Core rules

- `unsafe`, FFI, `transmute`, raw pointers, and dynamic loading are forbidden.
  An exception requires an ADR, threat model, test, and approval.
- Only `std` in core crates. A new dependency requires justification, verified
  license, pinned version, and approval.
- Input is hostile: validate limits before allocating, indexing, recursing, or
  dispatching.
- Libraries must not `panic!`, `unwrap`, `expect`, or index directly; propagate
  `RvmError`.
- Determinism: no `HashMap`, clock, environment, file order, or
  `debug_assertions` on the execution path.
- No implicit effects. `read_file` is the only host function and requires
  `filesystem_read` with a scope.
- Do not weaken a test or limit to hide a failure; open a divergence report.

## Build and tests

```bash
cargo build --workspace --locked
cargo test --workspace --locked
cargo test --release --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

Declared MSRV: Rust `1.75`. Also verify:

```bash
cargo +1.75.0 check --workspace --all-targets --locked
```

The Lift adapter has a separate workspace:

```bash
cargo test --manifest-path frontends/lift-rir/Cargo.toml
```

## Decision process

Decisions that close an open question require an ADR under `docs/ADRs/`, with
status, alternatives, consequences, and validation criteria. Contracts under
`docs/CONTRATOS/` describe what the code **must** do; if code and contract
diverge, open a report under `docs/RELATORIOS/`; do not "fix" the test.

## PRs

- One topic per PR, with tests and affected documentation.
- Describe the requirement/ADR addressed, the decision, the commands executed,
  and remaining gaps.
- Do not include build artifacts, binaries, `target/`, or secrets.
- For security changes, follow `SECURITY.md` instead of opening a public issue.

## License

By contributing, you agree that your contribution is licensed under
`MIT OR Apache-2.0`, as described in `LICENSE-MIT` and `LICENSE-APACHE`.
