# ADR-0030 — `Option`/`coalesce` no núcleo

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; ADR-0027/0028/0029; invariante de
  `Type: Copy`; fallback **preguiçoso** do `??` do Lift.

## Contexto

O Lift tem opcionais (`T?`), `null`, `??`, `option.some`, `option.is_some` e
`option.unwrap`. Sem um tipo opcional no núcleo, o adaptador não pode baixar `??`
nem esses construtos. Como `Type` precisa continuar `Copy`, um `Option<Box<Type>>`
forçaria um refatoramento amplo.

## Decisão

1. Novo tipo [`rvm_spec::Type::Optional(OptionalElem)`], com **payload escalar**
   (`bool`/`i64`/`string`/`f64`); sem `option` de array/struct/option aninhada.
   Usar `OptionalElem` (Copy) mantém `Type: Copy`.
2. Novo valor [`rvm_runtime::Value::Option { elem, value: Option<Box<Value>> }`].
3. Novas operações em [`rvm_ir::Op`]:
   - `OptionNone(elem)` (`option_none <elem>`): `none` do payload;
   - `OptionSome { value }` (`option_some v`): envolve um escalar;
   - `OptionIsSome { value }` (`option_is_some v`): → `bool`;
   - `OptionUnwrap { value }` (`option_unwrap v`): → payload; `none` produz o trap
     **`unwrap_none`**;
   - `Coalesce { option, fallback }` (`coalesce o, f`): payload se `some`, senão o
     fallback. O resultado é o tipo do fallback (payload ou o próprio `option`).
4. **Verificação** (`rvm-verify`): `option_some` exige payload escalar; `is_some`/
   `unwrap` exigem `option`; `coalesce` exige fallback do payload ou do próprio
   `option`. `build_values` resolve o tipo de cada um a partir dos operandos.
5. **Sem quota** adicional: `option` não aloca coleções (só um `Box` do payload).
6. **Formato textual**: tipo `option <elem>` (parâmetros/retorno/parâmetros de
   bloco) e as cinco operações. **Bytecode**: tag de tipo `7` (`optional` + tag do
   payload) e opcodes `41`..`45`.
7. O **adaptador** Lift→RIR mapeia `null`, `option.some(x)`, `option.is_some(x)`,
   `option.unwrap(x)` e `a ?? b`. O `??` é baixado com **fluxo de controle**
   (`option_is_some` + ramos), preservando o **fallback preguiçoso** do Lift: o
   fallback só é avaliado quando o `option` é `none` (um trap no fallback não
   dispara se o `option` for `some`). `x == null`/`x != null` ficam fora (devolvem
   `Unsupported`).
8. **Corpus**: `C-042` (`none`+`coalesce`), `C-043` (`some`+`coalesce`), `C-044`
   (`is_some`), `C-045` (`unwrap`), `C-046` (`option` como parâmetro); `N-035`
   (`unwrap` de `none` → `unwrap_none`), `N-036` (`is_some` de não-option),
   `N-037` (`option_some` de não-escalar).

## Alternativas

1. **`Coalesce` com fallback avaliado sempre:** mais simples, mas divergiria do
   Lift (fallback preguiçoso) e trapava em casos que não deveriam trapar. Rejeitada.
2. **`Type::Optional(Box<Type>)` (aninhável):** torna `Type` não-`Copy`; adiado.
3. **Sem `option` no núcleo:** mantém o núcleo menor, mas não destrava `??`;
   rejeitada a pedido do mantenedor.

## Consequências

- Positivas: opcionais escalares com `??` preguiçoso e determinístico; `Type`
  permanece `Copy`; um trap novo (`unwrap_none`) reutilizado de forma direta.
- Negativas: payload só escalar; sem `option` aninhado, sem `== null` no adaptador;
  `Display` de `option` usa `none`/`some:<valor>`.
- Residual: `Result`, `option` de agregados, `==`/`!=` de opcionais, métodos.

## Validação

- `crates/rvm-spec`: `OptionalElem`, `Type::Optional`; `unwrap_none` como `Trap`.
- `crates/rvm-ir`/`rvm-verify`: cinco operações, tipos e trap.
- `crates/rvm-interpreter`: `some`/`none`, `is_some`, `unwrap` (trap) e `coalesce`.
- `crates/rvm-text`/`rvm-bytecode`: tipo, operações, tag `7` e opcodes `41`..`45`.
- Corpus `C-042`..`C-046`/`N-035`..`N-037` (inclui o diferencial
  `encode → decode → verify → execute`).
- Adaptador: `cargo test --manifest-path frontends/lift-rir/Cargo.toml` (oráculos
  `rsuvm-oracle11.lift` = `hi missing 7 9 5 true false`, `rsuvm-oracle12.lift` = `5`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
