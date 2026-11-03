# RSUVM 0.1.0-alpha.1 — an experimental VM in Rust

**English** | [Português (BR)](./ANUNCIO-0.1.0-alpha.1.md)

I am publishing the first alpha of **RSUVM**, a research virtual machine written
in Rust. It executes a verified intermediate representation (RIR) with a
deterministic interpreter, execution quotas, and a minimal host ABI.

## What works

- RIR verifier (structure, CFG, dominance, types, and signatures).
- Interpreter with fuel, depth, modeled memory, and stable traps.
- Types `unit`, `bool`, `i64`, `string`, `f64`, `array`, `struct`, `option`, and
  `result`.
- Experimental textual format and bytecode, with round-trip.
- Static inter-module linking (`rvm-link`).
- `rsuvm` CLI: `run`, `run-bc`, `check`, `fmt`, `encode`.
- `read_file`/`filesystem_read` host ABI with capability, canonicalized scope,
  1 MiB ceiling per read, and quota reservation before I/O.
- 176 tests on Linux/Windows (debug/release), MSRV 1.75, CI, and a
  pre-registered benchmark.

## What it is **not**

- **It is not a sandbox.** It is single-tenant; it does not isolate process/OS
  and does not support multi-tenant use.
- It has no independent audit and no long-running fuzzing campaign.
- TOCTOU and hard links remain residual in file access.
- There is no aggregate I/O/time/rate quota.
- IR/text/bytecode formats and the Rust API are experimental, with no
  cross-version compatibility.
- There is a Lift→RIR prototype adapter (subset: no enums, unions, generics,
  closures, `match`, `for-of`, `try/catch`, or modules); `lift-encode` emits
  `.rsvmbc` for the supported subset.

## Try it

```bash
cargo install --path crates/rvm-cli --locked
rsuvm run examples/soma.rvm --arg i64:10 --arg i64:20   # i64:30
rsuvm check examples/soma.rvm
```

Or download the release artifacts and verify `SHA256SUMS`.

## Security

Read [SECURITY.md](../SECURITY.md) and [USER-GUIDE](./USER-GUIDE.md). For
untrusted code, use external process/OS isolation; the runtime does not replace
containers, VMs, seccomp, or cgroups.

Vulnerabilities: GitHub Security Advisories, not a public issue.

## Feedback

The most useful things for this alpha:

- run your own modules and report friction;
- review the verifier/decoder/interpreter;
- point out documentation gaps;
- discuss the use case and a viable frontend.

Thank you for testing. This is a research artifact, not a finished product.
