# Implementation Overview — RSUVM 0.1.0-alpha.1

**English** | [Português (BR)](./RELATORIOS/RELATORIO-ETAPAS-A.md)

This document summarizes what is implemented in the first public alpha. Deep
contracts, ADRs, and evidence reports remain in Portuguese; this is the
community-facing map.

> **Status:** experimental research prototype. Single-tenant, **not a sandbox**,
> no multi-tenant support. Formats and API are experimental.

## 1. Architecture

| Crate | Responsibility |
|---|---|
| `rvm-spec` | types, operations, stable error codes, limits, host ABI declarations |
| `rvm-ir` | RIR model and structural invariants (`structure::check`) |
| `rvm-runtime` | `Value`, execution config, quotas, capabilities, `Host` trait |
| `rvm-verify` | CFG, reachability, dominance, types, call/host signatures, `VerifiedModule` |
| `rvm-text` | experimental textual parser and canonical printer |
| `rvm-bytecode` | experimental linear encoder and bounded decoder |
| `rvm-interpreter` | deterministic executor with fuel, depth, memory, traps, host calls |
| `rvm-link` | static inter-module linking |
| `rvm-cli` | `rsuvm` binary (`run`, `run-bc`, `check`, `fmt`, `encode`) |
| `rvm-conformance` | data corpus and conformance/property tests |

The core has **zero external dependencies** and forbids `unsafe`. The `fuzz/`
workspace and the Lift adapter are separate; they are not part of the core build.

## 2. Language and RIR

- Scalars: `unit`, `bool`, `i64`, `string`, `f64`.
- Composites: `array T`, `struct Name`, `option T`, `result T E`, recursive by a
  module-level type-id table (ADR-0034).
- Arithmetic: checked `i64` operations, division/remainder by zero traps,
  shift-range validation, bitwise operations.
- Strings: equality, length, concatenation, with `max_string_bytes`.
- `f64`: IEEE-754 without traps.
- Arrays: immutable literals, `array_len`, bounds-checked `array_get`, functional
  `array_set`.
- Structs: `struct_new`, `field_get`, functional `field_set`.
- Options: `option_none`, `option_some`, `option_is_some`, `option_unwrap`,
  `coalesce`, `option_eq`, `option_ne`.
- Results: `result_ok`, `result_err`, `result_is_ok`, `result_unwrap`,
  `result_unwrap_err`.
- Control flow: `return`, `br`, `cond_br`, explicit block parameters (phi).
- Calls: static intra-module and linked inter-module calls.
- Host: `host_read_file` (see §4).

Verification runs before execution and checks structure, IDs, CFG,
reachability, dominance, definition-before-use, types, arity, and terminators.
The executor only accepts `VerifiedModule`.

## 3. Execution and quotas

- One frame per call, explicit heap frame stack (no native recursion).
- Fuel: 1 per dispatched instruction/terminator; no reset across calls.
- Depth: 1..=256 (`CallDepthLimit::HARD_MAX`), default 64.
- Modeled memory: frame cost (`256 + slots × 24`), argument buffers
  (`args × 24`), retained value bytes (strings, arrays, structs, options,
  results). Writes charge the new value and release the previous one; frame
  return releases its slots (ADR-0035).
- Traps: `integer_overflow`, `division_by_zero`, `invalid_shift`,
  `index_out_of_bounds`, `unwrap_none`, `unwrap_error`, `out_of_fuel`,
  `call_depth_exceeded`, `out_of_memory`, `capability_denied`.
- `execute` uses `NoHost`; `execute_with_host` accepts an explicit host.

## 4. Host ABI and capabilities

- Single built-in function:
  `read_file(path: string, max_bytes: i64) -> result string string`.
- Effect: `filesystem_read`; scope is argument 0 (`path`); bytecode function
  code `0`, opcode `54`.
- Default deny: `Capabilities::none()`; grants are explicit and host-side.
- `Capabilities::allows` compares path components and rejects `..`.
- The CLI requires an absolute, existing directory, canonicalizes root and
  target, rejects symlinks/junctions pointing outside, reads only regular files,
  and limits each read to 1 MiB (ADR-0037).
- Before I/O, the interpreter reserves `SLOT_BYTES + min(max_bytes, 1 MiB)` in the
  memory quota; if it does not fit, `out_of_memory` is raised and the host is not
  called (ADR-0038).
- `execute` without a host denies every host call.

## 5. Formats and CLI

- Experimental textual format (`EXP-0-TEXT`) with canonical printing and
  reparse.
- Experimental linear bytecode (`RSVMB1`) with bounded decoding and hostile-input
  checks.
- `rsuvm run`, `run-bc`, `check`, `fmt`, `encode`.
- `--entry`, `--arg` (`i64`, `bool`, `f64`, `str`, `unit`), `--fuel`,
  `--max-depth`, `--memory`, `--allow-fs-read`.
- Stable exit codes 0–5 (success, usage/I/O, module, trap, resource, capability).

## 6. Tests and evidence

| Item | Result |
|---|---|
| `cargo test --workspace --locked` | **176 passed, 0 failed** (Linux and Windows) |
| `cargo test --release --workspace --locked` | **176 passed, 0 failed** (Linux and Windows) |
| MSRV `cargo +1.75.0 check --workspace --all-targets --locked` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo doc` with `-D warnings` | exit 0 |
| Conformance corpus | 62 valid + 42 invalid |
| H06 benchmark | confirmed on dedicated runner runs `41429076866` and `41520845695` |
| Short fuzzing | run `41543777109`: 24,751,525 executions, 0 observed crashes |
| Release dry-run | run `41543773653`: Linux/Windows build success; publish skipped |
| Clean install / Docker | local dry-run: Windows zip, Linux tar, `SHA256SUMS`, `docker run` verified |

Detailed reports (Portuguese):

- `docs/RELATORIOS/RELATORIO-ETAPAS-A.md`
- `docs/RELATORIOS/RELATORIO-IMPLEMENTACAO.md`
- `docs/RELATORIOS/DRYRUN-RELEASE-0.1.0-alpha.1.md`
- `docs/RELATORIOS/FUZZING-EXP-0.md`
- `docs/RELATORIOS/BENCHMARKS-PREREGISTRADOS.md`

## 7. Key ADRs

| ADR | Decision |
|---|---|
| 0005 | Capabilities deny-by-default |
| 0009 | Crate workspace and dependency graph |
| 0010 | Experimental textual format |
| 0011 | Experimental bytecode |
| 0012 | Depth ceiling and explicit frame stack |
| 0013/0016 | Modeled memory quota and argument buffers |
| 0018 | Static inter-module linking |
| 0020 | Coverage-guided fuzzing |
| 0024–0033 | `string`, `f64`, arrays, structs, options, results |
| 0034 | Type-id table |
| 0035 | Retained value memory and slot release |
| 0036 | Minimal host ABI with capabilities |
| 0037 | File scope canonicalization; handles deferred |
| 0038 | Quota reservation before host I/O |

Full index: `docs/ADRs/README.md` (Portuguese).

## 8. Security posture

- `unsafe` forbidden; zero external dependencies in the core.
- Default deny; narrow, absolute file scope; canonicalization; regular files
  only; 1 MiB per read; quota reservation before I/O.
- **Not** a sandbox and **not** multi-tenant.
- Residual risks: TOCTOU between canonicalization and open, hard links, no
  aggregate I/O/time/rate quota, no independent audit, no long fuzzing campaign.
- Host code is trusted; custom hosts must enforce their own policy.

## 9. Release status

- Version: `0.1.0-alpha.1`.
- Distribution: GitHub Releases for Linux x86_64 and Windows x86_64; crates.io not
  published (`publish = false`); macOS untested.
- Security policy: `SECURITY.md`; compatibility: `docs/COMPATIBILITY.md`.
- The first tag exercises the `publish` job; the dry-run validated build,
  packaging, checksums, and artifacts.
