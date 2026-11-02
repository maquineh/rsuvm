# Incorporação — usar a RSUVM como biblioteca

[English](./EMBEDDING.md) | **Português (BR)**

A RSUVM é um workspace de crates. O caminho mínimo para executar um módulo é:
`rvm-text`/`rvm-bytecode` → `rvm-verify` → `rvm-interpreter`.

> **Escopo:** execução in-process, single-tenant. O host que incorpora a VM é
> responsável por isolamento de processo/OS, limites externos e política de
> arquivos. Ver `SECURITY.pt-BR.md`.

## 1. Execução pura (sem host)

```rust
use rvm_interpreter::execute;
use rvm_runtime::{CallDepthLimit, ExecConfig};
use rvm_spec::Limits;

let limits = Limits::default();
let module = rvm_text::parse(fonte, &limits)?;
let entry = module.entry;
let verified = rvm_verify::VerifiedModule::verify(module, &limits)?;
let config = ExecConfig::new(100_000, CallDepthLimit::default());
let outcome = execute(&verified, entry, &[], &config)?;
println!("{}", outcome.value);
# Ok::<(), rvm_spec::RvmError>(())
```

`execute` usa `NoHost`: qualquer `host_read_file` falha com
`capability_denied`. O `ExecConfig` controla fuel, profundidade, memória
modelada e limite de string.

## 2. Execução com host

Implemente `rvm_runtime::Host`:

```rust
use rvm_runtime::{BuiltinHost, Capabilities, Host, Value};
use rvm_spec::{RvmError, TypeId};

struct MeuHost {
    capabilities: Capabilities,
}

impl Host for MeuHost {
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
                // O host deve revalidar o escopo e limitar os bytes.
                // Devolva um Value::Result { ty: ret, value: ... }.
                todo!("implementar o efeito")
            }
        }
    }
}
```

Depois use `rvm_interpreter::execute_with_host(&verified, entry, &args, &config,
&mut host)`. O intérprete confere a capability **antes** de chamar o host,
reserva o pior caso na quota e valida o tipo do valor devolvido.

Um exemplo completo e testado está em
`crates/rvm-interpreter/tests/host.rs`. O host do CLI, com canonicalização e
limite de 1 MiB, está em `crates/rvm-cli/src/main.rs` (`CliHost`).

## 3. Regras para hosts

- **Default deny:** comece com `Capabilities::none()`; conceda apenas o efeito e
  o escopo necessários.
- **Escopo:** caminho absoluto, sem `..`, canonicalizado; rejeite symlink/junção
  para fora; aceite apenas arquivo regular.
- **Teto:** `read_file` não pode devolver mais que `min(max_bytes, 1 MiB)`.
- **IO:** não bloqueie indefinidamente; o runtime não tem timeout.
- **Erros:** devolva `Err(String)` no `result` para falha de IO; use
  `capability_denied` para escopo não coberto e `invalid_type` para argumento
  incompatível.
- **Confiança:** o host é confiável. O convidado não pode conceder capability,
  abrir arquivo nem alterar o conjunto de grants.

## 4. Limites e isolamento

- Fuel é determinístico e conta instruções; não é tempo de parede.
- Memória é modelada, não RSS.
- Não há quota agregada de IO/tempo/taxa.
- Não há isolamento de processo/OS, side channels ou multi-tenant.
- Para código não confiável, coloque a VM dentro de um processo/VM isolado, com
  seccomp/AppArmor, cgroups/Job Objects, usuário sem privilégio e visão de
  filesystem mínima.

## 5. Versões

A API é `0.x` e experimental. Ver `docs/COMPATIBILIDADE.md`. Para produção,
congele a versão do workspace e teste o módulo com o mesmo binário que o
executará.
