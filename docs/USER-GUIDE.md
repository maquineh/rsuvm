# User Guide — RSUVM (EXP-0)

**English** | [Português (BR)](./GUIA-USUARIO.md)

This guide covers the `rsuvm` binary in the experimental alpha. Read
[SECURITY.md](../SECURITY.md) and [COMPATIBILITY.md](./COMPATIBILITY.md) before
using it in any shared environment.

> **Scope:** single-tenant prototype. It is not a sandbox, does not isolate
> process/OS, and does not support multi-tenant use. Formats and API may change
> without compatibility.

## 1. Installation

### From source

```bash
cargo install --path crates/rvm-cli --locked
rsuvm --version
```

### Local build without installing

```bash
cargo build --release --locked -p rvm-cli
./target/release/rsuvm --version
```

### Release binaries

Official artifacts (when published) are at
<https://github.com/maquineh/rsuvm/releases>. Verify `SHA256SUMS` before running.
Tested platforms: Linux x86_64 and Windows x86_64; macOS is not distributed
without testing.

### Docker

```bash
docker build -t rsuvm:alpha .
docker run --rm rsuvm:alpha --version
docker run --rm -v "$PWD/examples:/work:ro" rsuvm:alpha \
  run /work/soma.rvm --arg i64:10 --arg i64:20
```

The image runs as UID `10001` and mounts examples read-only. The container is
**not** a sandbox by itself; for untrusted code, apply external isolation
(seccomp, unprivileged user, read-only filesystem).

## 2. Quick start

```bash
# Run examples/soma.rvm with two i64 arguments.
rsuvm run examples/soma.rvm --arg i64:10 --arg i64:20
# output: i64:30

# Verify without executing.
rsuvm check examples/soma.rvm

# Print the canonical form.
rsuvm fmt examples/soma.rvm

# Encode to bytecode and run the binary.
rsuvm encode examples/soma.rvm /tmp/soma.rsvmbc
rsuvm run-bc /tmp/soma.rsvmbc --arg i64:10 --arg i64:20
```

## 3. Subcommands

| Command | Purpose |
|---|---|
| `run <file.rvm>` | parse, verify, and execute the textual format |
| `run-bc <file.bin>` | decode, verify, and execute bytecode |
| `check <file.rvm>` | parse + verify, without executing |
| `fmt <file.rvm>` | print the canonical form |
| `encode <file.rvm> <out.bin>` | encode the module as bytecode |
| `version` / `--version` | binary version |
| `help` / `--help` | help |

### `run` and `run-bc` options

| Option | Effect |
|---|---|
| `--entry <fn>` | choose the entry function |
| `--arg <type:value>` | argument; may be repeated |
| `--fuel <n>` | fuel balance (default 100,000) |
| `--max-depth <n>` | maximum depth (1..=256; default 64) |
| `--memory <bytes>` | modeled memory quota (default 16 MiB) |
| `--allow-fs-read <dir>` | grant `filesystem_read` under a directory |

Argument types: `i64:10`, `bool:true`, `f64:1.5`, `str:text`, `unit`.

### Exit codes

| Code | Meaning |
|---:|---|
| 0 | success |
| 1 | usage error or tool I/O |
| 2 | module error (parse/decode/verification) |
| 3 | semantic trap |
| 4 | resource limit |
| 5 | capability denied |

## 4. File reads (`--allow-fs-read`)

Without the flag, `host_read_file` fails with `capability_denied` (exit 5). With
the flag:

- the directory must be **absolute**, existing, and a directory;
- `..` is rejected in the scope and in the path;
- root and target are canonicalized; a symlink/junction pointing outside is
  denied;
- only regular files are read;
- the ceiling is **1 MiB per read**; the interpreter reserves the worst case in
  the quota before I/O.

Operational recommendations:

- grant the smallest possible directory, preferably **read-only**;
- do not grant a directory writable by untrusted users (TOCTOU);
- do not grant a directory containing hard links to sensitive files;
- do not use in multi-tenant mode; the runtime is not an isolation boundary.

Example:

```bash
rsuvm run examples/host_read.rvm \
  --arg "str:$PWD/examples/soma.rvm" \
  --arg i64:1024 \
  --allow-fs-read "$PWD/examples"
```

## 5. Examples

| File | Demonstrates | Expected output |
|---|---|---|
| `examples/soma.rvm` | parameters and checked addition | `i64:30` |
| `examples/strings.rvm` | `concat_str` | `str:RSUVM` |
| `examples/array.rvm` | immutable array and `array_get` | `i64:20` |
| `examples/struct.rvm` | struct and `field_get` | `i64:7` |
| `examples/option.rvm` | `option` and `coalesce` | `i64:42` |
| `examples/result.rvm` | `result` and `result_is_ok` | `bool:true` |
| `examples/host_read.rvm` | `host_read_file` | depends on the file |

Examples without a host run without extra options. `host_read.rvm` requires
`--allow-fs-read` and two arguments.

## 6. Errors and diagnostics

The CLI prints `code: detail [location]` to `stderr`. Stable codes are in
`docs/CONTRATOS/ERROS-EXP-0.md`. Human messages may change; depend only on the
code/class.

## 7. Limits

- Fuel: 1 per dispatched instruction/terminator; calls do not reset it.
- Depth: 1..=256; entry frame = 1.
- Memory: deterministic model (`frame + slots + buffers`), not RSS.
- String: concatenation limit (`ExecConfig::max_string_bytes`).
- Host: 1 MiB per read; no aggregate I/O/time/rate quota.

## 8. Next documents

- [EMBEDDING](./EMBEDDING.md) — use the VM as a library.
- [COMPATIBILITY](./COMPATIBILITY.md) — versioning and formats.
- [IMPLEMENTATION](./IMPLEMENTATION.md) — what is implemented.
- `docs/ARQUITETURA.md` — design and limits (Portuguese).
- [SECURITY](../SECURITY.md) — reporting and security scope.
