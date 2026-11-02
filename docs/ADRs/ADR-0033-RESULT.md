# ADR-0033 — `Result` no núcleo (payloads escalares)

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor ("Option aninhado / Result"); ADR-0030
  (opcionais); invariante de `Type: Copy`.

## Contexto

O Lift tem `Result<T, E>` com `result.is_ok`/`result.unwrap`/`result.unwrap_err`
(construído por funções de host). Faltava o tipo no núcleo. Ao mesmo tempo, o
pedido mencionava "Option aninhado". Ambos esbarram no mesmo ponto: `Type` precisa
continuar `Copy`, e um payload recursivo (`Option<Box<Type>>` ou `Type::Optional`
de agregado) exigiria uma representação por tabela de tipos no módulo — um
refatoramento amplo.

## Decisão

1. **`Result` escalar**: novo tipo [`rvm_spec::Type::Result(ScalarKind, ScalarKind)`]
   (payloads `ok`/`err` escalares) e valor
   [`rvm_runtime::Value::Result`]. `Type` continua `Copy`.
2. **`ScalarKind`**: o enum escalar compartilhado por elementos de array, payloads
   de `option` e payloads de `result` (`ArrayElem`/`OptionalElem` passam a ser
   **aliases** de `ScalarKind`). Sem mudança de comportamento para arrays/options.
3. Novas operações em [`rvm_ir::Op`]:
   - `ResultOk { ok, err, value }` (`result_ok <ok> <err> v`) e
     `ResultErr { ok, err, value }` (`result_err <ok> <err> v`): constroem um
     `result`; `value` precisa ter o tipo da variante;
   - `ResultIsOk { value }` (`result_is_ok v`): → `bool`;
   - `ResultUnwrap { value }` (`result_unwrap v`): → payload `ok`; sobre `err`
     produz o trap **`unwrap_error`**;
   - `ResultUnwrapErr { value }` (`result_unwrap_err v`): → payload `err`; sobre
     `ok` produz `unwrap_error`.
4. **Verificação** (`rvm-verify`): construtores exigem `value` no tipo da variante;
   inspeção exige operando `result`. `build_values` resolve os tipos.
5. **Formato textual**: tipo `result <ok> <err>` (parâmetros/retorno/parâmetros de
   bloco) e as cinco operações. **Bytecode**: tag de tipo `8` (`result` + tag `ok` +
   tag `err`) e opcodes `49`..`53`.
6. **Adaptador**: **não** há caminho no Lift para construir `Result` no subconjunto
   (só `result.is_ok`/`unwrap`/`unwrap_err`, que exigem um valor vindo de host). O
   adaptador continua devolvendo `Unsupported` para tipos `Result`; o núcleo fica
   pronto para quando houver uma fonte de `Result`.
7. **Corpus**: `C-053` (`is_ok`), `C-054` (`unwrap_err`), `C-055` (`unwrap`);
   `N-041` (`unwrap` de `err` → `unwrap_error`), `N-042` (`is_ok` de não-`result`).

## Alternativas

1. **`Option` aninhado / `Option` de agregado** (também pedido): exigiria payload
   não-escalar em `Type::Optional`, ou seja, `Box<Type>` (quebra `Copy`) **ou** uma
   tabela de tipos no módulo e sintaxe textual para payloads agregados. Fica
   **adiado**; a decisão de representação precisa de ADR próprio.
2. **Payloads de `Result` não-escalares:** mesma barreira; adiado.
3. **Sem `Result` no núcleo:** não atende o pedido; rejeitada.

## Consequências

- Positivas: `Result` escalar completo (construção, inspeção, traps) sem quebrar
  `Type: Copy`; enum escalar unificado reduz duplicação.
- Negativas: payloads só escalares; sem `Option` aninhado; o adaptador não usa
  `Result` (sem fonte Lift); um trap novo (`unwrap_error`).
- Residual: `Option` de agregado/aninhado, `Result` de agregados, `?` (propagação).

## Validação

- `crates/rvm-spec`: `ScalarKind` + aliases, `Type::Result`; `unwrap_error` como
  `Trap`.
- `crates/rvm-ir`/`rvm-verify`: cinco operações, tipos e trap.
- `crates/rvm-interpreter`: construção, `is_ok`, `unwrap`/`unwrap_err` (trap).
- `crates/rvm-text`/`rvm-bytecode`: tipo, operações, tag `8` e opcodes `49`..`53`.
- Corpus `C-053`..`C-055`/`N-041`/`N-042` (inclui o diferencial
  `encode → decode → verify → execute`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
