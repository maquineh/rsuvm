# Embedding — Using RSUVM as a Library

**English** | [Português (BR)](./INCORPORACAO.md)

RSUVM is a workspace of crates. The minimal path to execute a module is:
`rvm-text`/`rvm-bytecode` → `rvm-verify` → `rvm-interpreter`.

> **Scope:** in-process, single-tenant execution. The host embedding the VM is
> responsible for process/OS isolation, external limits, and file policy. See
> [SECURITY.md](../SECURITY.md).

## 1. Pure execution (no host)

```rust
use rvm_interpreter::execute;
use rvm_runtime::{CallDepthLimit, ExecConfig};
use rvm_spec::Limits;

let limits = Limits::default();
let module = rvm_text::parse(source, &limits)?;
let entry = module.entry;
let verified = rvm_verify::VerifiedModule::verify(module, &limits)?;
let config = ExecConfig::new(100_000, CallDepthLimit::default());
let outcome = execute(&verified, entry, &[], &config)?;
println!("{}", outcome.value);
# Ok::<(), rvm_spec::RvmError>(())
```

`execute` uses `NoHost`: any `host_read_file` fails with `capability_denied`.
`ExecConfig` controls fuel, depth, modeled memory, and the string limit.

## 2. Execution with a host

Implement `rvm_runtime::Host`:

```rust
use rvm_runtime::{BuiltinHost, Capabilities, Host, Value};
use rvm_spec::{RvmError, TypeId};

struct MyHost {
    capabilities: Capabilities,
}

impl Host for MyHost {
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn call(
        &mut self,
        function: BuiltinHost,
        _args: &[Value],
        ret: TypeId,
    ) -> Result<Value, RvmError> {
        match function {
            BuiltinHost::ReadFile => {
                // The host must revalidate the scope and bound the bytes.
                // Return a Value::Result { ty: ret, value: ... }.
                todo!("implement the effect")
            }
        }
    }
}
```

Then use `rvm_interpreter::execute_with_host(&verified, entry, &args, &config,
&mut host)`. The interpreter checks the capability **before** calling the host,
reserves the worst case in the quota, and validates the returned value's type.

A complete, tested example is in `crates/rvm-interpreter/tests/host.rs`. The CLI
host, with canonicalization and a 1 MiB limit, is in
`crates/rvm-cli/src/main.rs` (`CliHost`).

## 3. Rules for hosts

- **Default deny:** start with `Capabilities::none()`; grant only the required
  effect and scope.
- **Scope:** absolute path, no `..`, canonicalized; reject symlinks/junctions
  pointing outside; accept only regular files.
- **Ceiling:** `read_file` must not return more than `min(max_bytes, 1 MiB)`.
- **I/O:** do not block indefinitely; the runtime has no timeout.
- **Errors:** return `Err(String)` inside the `result` for I/O failure; use
  `capability_denied` for an uncovered scope and `invalid_type` for an
  incompatible argument.
- **Trust:** the host is trusted. The guest cannot grant a capability, open a
  file, or change the grant set.

## 4. Limits and isolation

- Fuel is deterministic and counts instructions; it is not wall-clock time.
- Memory is modeled, not RSS.
- There is no aggregate I/O/time/rate quota.
- There is no process/OS isolation, side-channel protection, or multi-tenant
  support.
- For untrusted code, put the VM inside an isolated process/VM, with
  seccomp/AppArmor, cgroups/Job Objects, an unprivileged user, and a minimal
  filesystem view.

## 5. Versions

The API is `0.x` and experimental. See [COMPATIBILITY](./COMPATIBILITY.md). For
production, pin the workspace version and test the module with the same binary
that will execute it.
