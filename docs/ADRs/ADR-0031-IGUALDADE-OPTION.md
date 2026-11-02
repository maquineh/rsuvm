# ADR-0031 — Igualdade de `option` no núcleo (`option_eq`/`option_ne`)

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; ADR-0030 (opcionais); paridade com
  `x == null`/`x != null` do Lift.

## Contexto

A ADR-0030 deixou `x == null`/`x != null` fora do adaptador por falta de igualdade
de opcionais no núcleo. Sem ela, o adaptador não expressa comparações de opcionais
(o exemplo `option.lift` do Lift termina com `absent == null` e `null != present`).

## Decisão

1. Novas operações [`rvm_ir::Op::OptionEq`] e [`rvm_ir::Op::OptionNe`]
   (`option_eq v, w` / `option_ne v, w`), ambas produzindo `bool`.
2. **Semântica**: `none == none` é `true`; `some(a) == some(b)` compara os payloads
   por valor (`string` por conteúdo; `f64` por IEEE-754, logo `NaN != NaN`); `none`
   e `some` são diferentes. `option_ne` é a negação.
3. **Verificação** (`rvm-verify`): os dois operandos precisam ser `option` de
   **mesmo payload**; qualquer outra coisa é `invalid_type`.
4. **Formato textual**: `option_eq v, w` e `option_ne v, w`. **Bytecode**: opcodes
   `46` e `47` (`u32 lhs`, `u32 rhs`).
5. O **adaptador** mapeia `==`/`!=` quando **qualquer** operando é `option` para
   `option_eq`/`option_ne`; outros operadores sobre `option` continuam
   `Unsupported`.
6. **Corpus**: `C-047` (`none == none`), `C-048` (`some != none`), `C-049`
   (payload `string`); `N-038` (`option_eq` de não-option).

## Alternativas

1. **Baixar `x == null` no adaptador via `option_is_some` + negação**: funciona só
   para `null`, não para `x == y` entre opcionais, e duplicaria lógica de fluxo.
   Rejeitada em favor de uma operação de valor.
2. **Comparar sempre `false` para `some`/`NaN`**: divergiria de `f64`; rejeitada.

## Consequências

- Positivas: igualdade de opcionais determinística e sem fluxo; destrava
  `== null`/`!= null` (e `==`/`!=` entre opcionais) no adaptador.
- Negativas: dois opcodes novos; sem ordenação (`<`) de opcionais.

## Validação

- `crates/rvm-ir`/`rvm-verify`: `OptionEq`/`OptionNe`, mesmo payload, resultado
  `bool`.
- `crates/rvm-interpreter`: comparação por valor (payload).
- `crates/rvm-text`/`rvm-bytecode`: operações e opcodes `46`/`47`.
- Corpus `C-047`..`C-049`/`N-038` (inclui o diferencial `encode → decode → verify →
  execute`).
- Adaptador: `cargo test --manifest-path frontends/lift-rir/Cargo.toml` (oráculo
  `rsuvm-oracle13.lift` = `true false true false`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
