# ADR-0027 — Arrays imutáveis no núcleo

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; `docs/ARQUITETURA.md` §3 (que listava
  arrays como fora do EXP-0); ADR-0025 (quota cumulativa para strings).
- **Revisão (ADR-0035):** a quota cumulativa dos arrays foi substituída pelo modelo
  de bytes retidos por slot.

## Contexto

Arrays eram o último item grande do subset. Sem eles, um frontend não cobre
coleções. Como o EXP-0 é **SSA puro** (valores definidos uma vez), mutação
in-place de elementos quebraria a semântica; a decisão abaixo adota arrays
**imutáveis** com leitura verificada.

## Decisão

1. Novo tipo [`rvm_spec::Type::Array(ArrayElem)`], com **elemento escalar**
   (`bool`/`i64`/`string`/`f64`); **sem arrays aninhados**. Usar `ArrayElem` (Copy)
   mantém `Type: Copy` (um `Box<Type>` tornaria `Type` não-`Copy`, refator grande).
2. Novo valor [`rvm_runtime::Value::Array(Vec<Value>)`].
3. Novas operações em [`rvm_ir::Op`]:
   - `ArrayLiteral(Vec<ValueId>)` (`array v0, v1, …`): **não-vazio** (array vazio é
     `invalid_module`); o tipo do elemento vem do primeiro item e todos precisam
     ser do mesmo tipo;
   - `ArrayLen { array }` (`array_len`): comprimento → `i64`;
   - `ArrayGet { array, index }` (`array_get`): leitura com **bounds check**;
     índice `< 0` ou `>= len` produz o trap **`index_out_of_bounds`**.
4. Arrays são **imutáveis**: não há `array_set` nesta rodada (mutação exigiria
   decisão de SSA/aliasing). O array é criado por literal e lido.
5. **Quota**: `ArrayLiteral` cobra `len × SLOT_BYTES` (24) na memória modelada,
   **cumulativo** (como a concatenação) → `out_of_memory`; limite estrutural
   [`rvm_spec::Limits::max_array_len`] (4096).
6. **Formato textual**: `array v0, v1, …`, `array_len v`, `array_get v, i`.
   **Bytecode**: opcodes `35` (`array`), `36` (`array_len`), `37` (`array_get`);
   tag de tipo `5` (`array` + tag do elemento).
7. O **adaptador** Lift→RIR mapeia literais `[...]` e indexação `xs[i]`.
8. **Corpus**: `C-034` (`array_len`), `C-035` (`array_get`), `N-029` (fora dos
   limites → `index_out_of_bounds`).

## Alternativas

1. **Arrays mutáveis (`array_set`):** quebra SSA/aliasing e a pureza do valor;
   rejeitado nesta rodada.
2. **`Type::Array(Box<Type>)` (aninháveis):** torna `Type` não-`Copy`, com
   refatoramento amplo; adiado.
3. **Sem arrays:** mantém o núcleo mínimo, mas incompleto; rejeitado a pedido do
   mantenedor.

## Consequências

- Positivas: coleções de escalares com leitura verificada e determinística;
  bounds check fail-closed; `Type` continua `Copy`.
- Negativas: arrays imutáveis (sem `array_set`), sem aninhamento, sem slicing nem
  concatenação de arrays; quota cumulativa conservadora; três opcodes e um trap
  novos.
- Residual: mutação, arrays de arrays, `map`/`filter` e formatação ficam fora;
  objetos/closures também.

## Validação

- `crates/rvm-spec`: `ArrayElem`/`Type::Array`; `index_out_of_bounds` como `Trap`;
  `max_array_len`.
- `crates/rvm-ir`/`rvm-verify`: literal não-vazio e homogêneo; `array_len`/`array_get`
  exigem array; `array_get` exige índice `i64`.
- `crates/rvm-interpreter`: bounds check → `index_out_of_bounds`; quota cumulativa.
- `crates/rvm-text`/`rvm-bytecode`: ops e tags.
- Corpus `C-034`/`C-035`/`N-029` (inclui o diferencial `encode → decode → verify →
  execute`).
- Adaptador: `cargo test --manifest-path frontends/lift-rir/Cargo.toml` (oráculo
  `rsuvm-oracle8.lift` = `20`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
