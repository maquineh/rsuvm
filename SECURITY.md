# Security Policy — RSUVM

**English** | [Português (BR)](./SECURITY.pt-BR.md)

## Project status

RSUVM is an **experimental research prototype** (EXP-0). It is not a validated
sandbox, not an authorization for production, and must not be used to execute
mutually untrusted code.

The supported model is **single-tenant**: one host process executes one trusted
module or a module owned by a single party. The runtime does not implement
process/OS isolation, does not separate tenants, and is not a security boundary
against hostile code.

## Supported versions

| Version | Support |
|---|---|
| `0.1.x` (alpha) | Latest published version only; security fixes on a best-effort basis |
| Older versions | Unsupported |

There is no LTS and no compatibility promise across versions of the textual
format, bytecode, RIR, or Rust API. See `docs/COMPATIBILITY.md`.

## How to report

**Do not open a public issue for a vulnerability.** Use the private channel:

- GitHub Security Advisories:
  <https://github.com/maquineh/rsuvm/security/advisories/new>

Include, if possible:

- version/commit and operating system;
- command and `.rvm`/bytecode module that reproduces it;
- expected and observed behavior;
- impact (denial of service, unauthorized read, verification bypass, etc.);
- whether the finding depends on specific configuration (`--allow-fs-read`, a
  custom host, a filesystem with symlinks/hard links).

Do not send secrets, personal data, or the contents of sensitive files. If
reproduction requires a sensitive file, describe its structure instead of its
contents.

## Scope

In scope:

- verifier (`rvm-verify`), structural check (`rvm-ir`), and decoder
  (`rvm-bytecode`);
- interpreter and quotas (`rvm-interpreter`, `rvm-runtime`);
- host ABI and capability checks (`BuiltinHost`, `Capabilities`, `Host`);
- CLI `rsuvm` and its file host (`CliHost`);
- textual parser/printer (`rvm-text`);
- linker (`rvm-link`).

Out of scope (and frequently operator configuration):

- multi-tenant isolation, containers, VMs, seccomp, cgroups/Job Objects;
- broad, writable scopes or scopes containing hard links to sensitive files;
- TOCTOU with a concurrent writer in the granted directory;
- side channels, resource DoS without external isolation, time/rate limits;
- Lift/Rio frontends under `frontends/` and fuzzing dependencies;
- the underlying OS, filesystem, hardware, and Rust runtime.

## Known limitations

These are **known and declared** limitations, not new vulnerabilities:

- TOCTOU between canonicalization and open in the CLI host;
- hard links inside the scope pointing to inodes outside it are not detected;
- 1 MiB ceiling **per read**, with no aggregate byte, time, or rate quota;
- no independent audit and no long-running fuzzing campaign;
- experimental API/format, with no cross-version compatibility;
- a custom `Host` is trusted and must apply its own policy;
- no process/OS isolation.

## Disclosure

- Coordinated process, with an effort to fix before disclosure.
- Target of 90 days for confirmed findings, adjustable by complexity and risk.
- Credit to the reporter, if desired.
- No bug bounty program.

## Good-faith research

Good-faith security research on this repository is welcome. Do not encourage
access to third-party data, do not exfiltrate content, and do not cause harm. If
a test requires a multi-tenant environment or real data, stop and describe the
scenario in the private report.
