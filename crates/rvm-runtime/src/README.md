# rvm-runtime — Package Contract

## Overview

`rvm-runtime` define valores de runtime, quotas de execução e a fronteira de
capabilities. Implementa a regra **deny-by-default** do ADR-0005 de forma
estrutural: não existe construtor que conceda capability e nenhuma função do
crate toca sistema operacional. Depende apenas de `rvm-spec`.

## Interface

- `Value` — `Unit`, `Bool(bool)`, `I64(i64)`, `Str(String)`, `F64(f64)` (Clone,
  não `Copy` nem `Eq`); `ty`, `as_bool`, `as_i64`, `as_str`, `as_f64`,
  `matches_type`, `Display` estável (`i64:30`, `bool:true`, `unit`, `str:<texto>`,
  `f64:1.5`) (`lib.rs:Value`).
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
  `Clock`, `Process`; `name()`.
- `Capabilities` — `none()`, `granted() -> &[Effect]`, `is_granted(Effect)`.
- `HostApi` — `standalone()`, `new(Capabilities)`, `capabilities()`,
  `request(Effect, scope) -> Result<(), RvmError>` (`lib.rs:HostApi`).

## Invariants

- `Capabilities` tem campo privado e **um único** construtor efetivo, `none()`;
  não há `grant`, `insert`, `with` nem `Default` com conteúdo. No EXP-0 o conjunto
  concedido é sempre vazio.
- `HostApi::request` devolve sempre `Err(capability_denied)`; o ramo `Ok` usa
  `UnsupportedFeature` e é inalcançável porque nenhum conjunto não vazio pode ser
  construído.
- Nenhuma função do crate abre arquivo, socket, variável de ambiente, relógio ou
  processo; a negação é do código, não de flag.
- `CallDepthLimit` só existe em `1..=HARD_MAX`; `new(0)` e `new(257)` devolvem
  `None`. O teto de 256 é aplicado pelo intérprete sobre uma pilha de frames na
  heap (ADR-0012).
- `Value` não carrega ponteiro, referência ou handle do processo hospedeiro; todo
- A quota de memória é um **modelo em bytes RSUVM** (`frame_cost` e
  `arg_buffer_cost`), não medição de RSS: frame `256 + slots × 24` e buffer
  `args × 24`, saturados. `MemoryLimit` não é validado contra teto; o host
  escolhe, e `0` significa "nada cabe" (ADR-0013, ADR-0016).
  valor conhece seu `Type` e não há conversão implícita.
- `fuel` é `u64` sem validação extra; quem executa debita antes do despacho.

## Side Effects

- `None`. O crate não lê nem escreve recursos do host. `request` produz um
  diagnóstico de negação sem efeito colateral observável.

## Verification

- Run: `cargo test --workspace`
