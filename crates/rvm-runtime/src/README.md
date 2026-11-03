# rvm-runtime — Package Contract

## Overview

`rvm-runtime` define valores de runtime, quotas de execução e a fronteira de
capabilities. Implementa a regra **deny-by-default** do ADR-0005: o conjunto
concedido nasce vazio e só cresce por construtores explícitos do host
(`Capabilities::grant`/`with_grant`, ADR-0036). Nenhuma função do crate toca
sistema operacional; o efeito só existe se um `Host` externo implementar
`call`. Depende apenas de `rvm-spec`.

## Interface

- `Value` — `Unit`, `Bool(bool)`, `I64(i64)`, `Str(String)`, `F64(f64)`,
  `Array { ty, items }`, `Struct { ty, struct_id, fields }`, `Option { ty, value }`,
  `Result { ty, value }`, `Enum { ty, variant, fields }` (Clone, não `Copy` nem `Eq`);
  `ty() -> TypeId`, `as_bool`,
  `as_i64`, `as_str`, `as_f64`, `as_array`, `as_struct`, `as_option`, `as_result`,
  `as_enum`, `matches_type(TypeId)`, `depth()`, `retained_bytes()`, `Display`
  estável (`i64:30`, `bool:true`, `unit`,
  `str:<texto>`, `f64:1.5`, `array:[i64:1, i64:2]`, `struct:0[i64:1]`, `none`,
  `some:i64:1`, `ok:i64:5`, `err:str:e`, `enum:5:1[i64:2]`) (`lib.rs:Value`).
- `MAX_VALUE_DEPTH` — teto de profundidade de valor (256); `enum_new` recusa
  valores acima dele com `value_too_deep` (ADR-0039).
- `DEFAULT_FUEL` — saldo padrão de execução (`100_000`).
- `DEFAULT_MEMORY_LIMIT_BYTES` (16 MiB), `FRAME_BASE_BYTES` (256), `SLOT_BYTES`
  (24), `frame_cost(slots) -> u64` e `arg_buffer_cost(args) -> u64` — modelo
  determinístico de memória por frame e por buffer transitório de argumentos
  (`lib.rs:frame_cost`, `lib.rs:arg_buffer_cost`).
- `MemoryLimit` — `new(bytes)`, `get()`, `DEFAULT` (16 MiB); `0` é aceito.
- `CallDepthLimit` — `HARD_MAX = 256`, `DEFAULT = 64`, `new(u16) -> Option<Self>`,
  `get()` (`lib.rs:CallDepthLimit`).
(- `ExecConfig` — `new(fuel, depth)`, `fuel()`, `max_call_depth()`,
  `with_memory(MemoryLimit)`, `memory_limit()`, `Default`
  (`lib.rs:ExecConfig`).
- `Effect` — `FilesystemRead`, `FilesystemWrite`, `Network`, `Environment`,
  `Clock`, `Process`; `name()`, `from_name(&str)`. Reexportado de `rvm-spec`.
- `CapabilityGrant` — `new(effect, scope)`, `effect()`, `scope()`.
- `Capabilities` — `none()`, `grant(effect, scope)`, `with_grant(effect, scope)`,
  `granted() -> &[CapabilityGrant]`, `is_granted(Effect)`,
  `allows(Effect, scope)` (componentes de caminho; escopo vazio = qualquer).
- `Host` — trait de backend: `capabilities()`, `call(BuiltinHost, args, ret)`;
  `NoHost` nega toda chamada.
- `HostApi` — `standalone()`, `new(Capabilities)`, `capabilities()`,
  `request(Effect, scope) -> Result<(), RvmError>` (`lib.rs:HostApi`).

## Invariants

- `Capabilities` tem campo privado e construtores explícitos; não há
  desserialização nem `Default` com conteúdo. `none()` é o padrão e nenhum efeito
  é concedido sem uma chamada explícita a `grant`/`with_grant`.
- `Capabilities::allows` compara o escopo por componentes de caminho
  (`Path::starts_with`), não por prefixo textual, e nega caminhos com componente
  `..` (sem canonicalização, `..` escaparia do escopo); escopo vazio concede
  qualquer caminho sem `..`. `HostApi::request` devolve `Ok(())` quando `allows` é
  verdadeiro e `Err(capability_denied)` caso contrário.
- Nenhuma função do crate abre arquivo, socket, variável de ambiente, relógio ou
  processo; o `Host` é um trait implementado fora do crate (o CLI implementa
  `CliHost` com `std::fs`), e a negação padrão é do código, não de flag.
- `CallDepthLimit` só existe em `1..=HARD_MAX`; `new(0)` e `new(257)` devolvem
  `None`. O teto de 256 é aplicado pelo intérprete sobre uma pilha de frames na
  heap (ADR-0012).
- `Value` não carrega ponteiro, referência ou handle do processo hospedeiro; todo
- A quota de memória é um **modelo em bytes RSUVM** (`frame_cost` e
  `arg_buffer_cost`), não medição de RSS: frame `256 + slots × 24` e buffer
  `args × 24`, saturados. `MemoryLimit` não é validado contra teto; o host
  escolhe, e `0` significa "nada cabe" (ADR-0013, ADR-0016).
  valor conhece seu `TypeId` e não há conversão implícita.
- `fuel` é `u64` sem validação extra; quem executa debita antes do despacho.

## Side Effects

- `None`. O crate não lê nem escreve recursos do host. `request` produz um
  diagnóstico de negação sem efeito colateral observável; `Host` é apenas um
  trait, sem implementação de efeito neste crate.

## Verification

- Run: `cargo test --workspace`
