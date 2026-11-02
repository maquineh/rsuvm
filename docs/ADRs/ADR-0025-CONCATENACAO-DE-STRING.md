# ADR-0025 — Concatenação de `string` com contabilização de memória

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; ADR-0024 (tipo `string`); ADR-0013/ADR-0016
  (quota de memória modelada).

## Contexto

O ADR-0024 adicionou `string` como literais imutáveis, mas **sem** concatenação —
o tipo ficava limitado a comparação e comprimento. Concatenar exige produzir um
valor novo em execução, o que precisa de **quota** (senão um laço cresce sem
limite) e de um **limite por string**.

## Decisão

1. Nova operação [`rvm_ir::Op::ConcatStr`] (`concat_str`): concatena dois `string`
   e produz `string`.
2. **Quota de memória**: o comprimento do resultado (`lhs.len() + rhs.len()`) é
   cobrado na quota modelada **antes** de materializar a string; exceder
   `ExecConfig::memory_limit` produz `out_of_memory`. A cobrança é
   **cumulativa** (não é liberada ao fim do frame): isso limita o crescimento total
   em laços. É um modelo **conservador**, não uma arena real — não há reciclagem.
3. **Limite por string**: `ExecConfig::max_string_bytes`
   (`DEFAULT_MAX_STRING_BYTES = 4096`); resultado acima disso produz o novo trap
   **`string_too_long`** ([`rvm_spec::ErrorCode`], `Trap`).
4. **Formato textual**: `concat_str a, b`. **Bytecode**: opcode `23`.
5. O **adaptador** Lift→RIR mapeia `+` de `string` do Lift (`a + b`) para
   `concat_str`.
6. **Corpus**: `C-031` (concatenação) e `N-028` (estouro da quota com `memory:
   700`). `string_too_long` é coberto por teste unitário
   (`crates/rvm-interpreter/tests/memoria.rs`), pois o corpus não ajusta
   `max_string_bytes`.

## Alternativas

1. **Arena explícita com índices (`Value::Str(u32)`):** mantém `Value` `Copy`, mas
   exige formatação/igualdade cientes da arena em CLI e corpus; mais complexo.
2. **Sem quota:** falha aberta para DoS por concatenação; rejeitado.
3. **Liberar bytes por frame:** modelo mais fino, porém aproxima a vida útil de
   valores que escapam (retorno); adiado.

## Consequências

- Positivas: concatenação real e determinística; DoS de string limitado por quota e
  por `max_string_bytes`; trap `string_too_long` distinto de `out_of_memory`.
- Negativas: a quota cumulativa é **conservadora** (pode rejeitar programas com
  muitas strings temporárias antes do consumo real); novo código de erro estável e
  um opcode novo; a string resultante é copiada (sem arena).
- Residual: slices, `replace`, formatação e comparação de ordem de `string`
  continuam fora; `f64`, arrays e objetos também.

## Validação

- `crates/rvm-spec`: `string_too_long` na tabela, `Trap`.
- `crates/rvm-ir`/`rvm-verify`: `ConcatStr` (operandos `string`, resultado
  `string`).
- `crates/rvm-interpreter/tests/memoria.rs`: concatenação dentro do limite
  (`max_string_bytes = 8`) e acima (`6`) → `string_too_long`; corpus `N-028`
  estoura a quota → `out_of_memory`.
- `crates/rvm-conformance/corpus`: `C-031`; diferencial `encode → decode → verify →
  execute`.
- Adaptador: `cargo test --manifest-path frontends/lift-rir/Cargo.toml` (inclui
  `+` de string; oráculo do Lift `rsuvm-oracle6.lift` = `ola mundo`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
