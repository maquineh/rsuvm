# ADR-0026 — Tipo `f64` (ponto flutuante IEEE-754) no núcleo

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; `docs/ARQUITETURA.md` §3 (que listava `f64`
  como fora do EXP-0).

## Contexto

O EXP-0 só tinha inteiros (`i64`), `bool` e `string`. Sem ponto flutuante, um
frontend não cobre literais/aritmética `number`. Este ADR adiciona `f64` com
semântica **IEEE-754 determinística**, sem traps.

## Decisão

1. Novo tipo [`rvm_spec::Type::F64`] (`f64`) e valor [`rvm_runtime::Value::F64`].
   `Value` **deixa de implementar `Eq`** (por causa de `NaN`); continua `PartialEq`.
2. Novas operações em [`rvm_ir::Op`]:
   - `ConstF64(u64)` (`const_f64`): o literal é guardado como **bits IEEE-754**
     (`f64::to_bits`), o que preserva `Eq`/`Hash` na RIR;
   - `Float { op: FloatOp, lhs, rhs }`: aplica [`rvm_spec::FloatOp`].
3. [`rvm_spec::FloatOp`] = `{ Add, Sub, Mul, Div, Eq, Ne, Lt, Le, Gt, Ge }`
   (`add_f64` … `ge_f64`). Aritmética **IEEE-754 sem trap**: divisão por zero
   produz `inf`/`NaN`; comparações seguem IEEE (com `NaN`, tudo é falso exceto
   `!=`). Não há `-0.0`/`NaN` literais na fronteira textual.
4. **Formato textual**: literal `[-]d+.d+` (exige ponto **e** ao menos um dígito
   após); operações `const_f64`, `add_f64`…`ge_f64`. A impressão usa forma
   **reparseável** (garante `.` ou expoente).
5. **Bytecode**: tag de tipo `4` (`f64`); opcodes `24` (`const_f64`) e `25..=34`
   (uma por `FloatOp`). O literal são 8 bytes (bits).
6. O **adaptador** Lift→RIR mapeia `number` do Lift → `f64`: literais, `+ - * /`,
   comparações e negação unária.
7. **Corpus**: `C-032` (aritmética `f64`) e `C-033` (comparações `f64`).

## Alternativas

1. **Guardar `f64` direto no `Op`:** quebraria `Eq` em toda a RIR
   (`Op`/`Instr`/`Block`/`Function`/`Module`); rejeitado — usa-se bits.
2. **Trap em divisão por zero `f64`:** contraria IEEE-754 e a intuição de
   ponto flutuante; rejeitado.
3. **Não suportar `f64`:** mantém o núcleo mínimo, mas incompleto; rejeitado a
   pedido do mantenedor.

## Consequências

- Positivas: tipo numérico real com aritmética/comparações IEEE-754; determinístico
  entre plataformas para operações básicas; sem trap surpresa.
- Negativas: `Value` deixa de ser `Eq`; o literal `const_f64` é guardado como bits;
  `NaN`/`-0.0`/`inf` não são expressáveis como literais textuais.
- Residual: `%` de `f64`, `pow`, conversões `i64↔f64`, `f32`, formatação decimal
  controlada e comparação total (ordenável) ficam fora; arrays/objetos/closures
  também.

## Validação

- `crates/rvm-spec`: `FloatOp` bidirecional; IEEE (`1.0/0.0 = inf`, `NaN != NaN`);
  `Type::F64`.
- `crates/rvm-ir`/`rvm-verify`: `ConstF64`/`Float` (operandos `f64`; resultado por
  op).
- `crates/rvm-text`: literal `f64` (ponto obrigatório), round-trip
  `parse(print(m)) == m`, impressão reparseável.
- `crates/rvm-bytecode`: tag `4` e opcodes `24..=34`.
- Corpus `C-032`/`C-033` (inclui o diferencial `encode → decode → verify →
  execute`).
- Adaptador: `cargo test --manifest-path frontends/lift-rir/Cargo.toml` (oráculo
  `rsuvm-oracle7.lift` = `1.5 true false`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
