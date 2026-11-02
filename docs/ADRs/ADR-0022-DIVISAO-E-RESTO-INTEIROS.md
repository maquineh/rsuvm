# ADR-0022 — Divisão e resto inteiros verificados no núcleo

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; etapa de ampliação do adaptador Lift→RIR
  (ADR-0021); `docs/ARQUITETURA.md` §3 (que listava divisão/resto como fora do
  EXP-0).

## Contexto

O EXP-0 tinha apenas `add/sub/mul_i64_checked`. O adaptador Lift→RIR (ADR-0021)
não conseguia mapear `/` e `%`, deixando um subconjunto prático de Lift
incompleto. Implementar divisão no adaptador (por subtração repetida) seria
`O(n)`, erraria sinais e não cobriria bordas; a operação pertence ao núcleo.

## Decisão

1. Adicionar duas operações a [`rvm_spec::BinOp`]: **`DivI64Checked`**
   (`div_i64_checked`) e **`RemI64Checked`** (`rem_i64_checked`), com semântica de
   **truncamento para zero** (`i64::checked_div` / `i64::checked_rem`).
2. **Divisão/resto por zero** produz o novo trap **`division_by_zero`**
   ([`rvm_spec::ErrorCode`], categoria/`ErrorClass` = `Trap`). O código estável
   entra na tabela de `docs/CONTRATOS/ERROS-EXP-0.md`.
3. **`i64::MIN / -1`** e **`i64::MIN % -1`** (resultado fora de `i64`) produzem
   **`integer_overflow`**, mantendo o padrão das demais operações.
4. **Bytecode** ganha os opcodes `12` (`div_i64_checked`) e `13`
   (`rem_i64_checked`); o restante do pipeline (`rvm-verify`, `rvm-interpreter`,
   `rvm-text`) é genérico sobre `BinOp` e não muda de forma.
5. **Corpus**: `C-020` (divisão), `C-021` (resto), `N-025` (divisão por zero →
   `division_by_zero`), `N-026` (`i64::MIN / -1` → `integer_overflow`).
6. O **adaptador** Lift→RIR mapeia `/` → `div_i64_checked` e `%` →
   `rem_i64_checked` (ADR-0021 atualizado).

## Alternativas

1. **Reusar `integer_overflow` para divisão por zero:** mais simples, mas funde
   dois traps distintos e perde diagnóstico; rejeitado.
2. **Implementar divisão no adaptador por subtração repetida:** `O(n)`, erra
   negativos e bordas; rejeitado.
3. **Não suportar divisão:** mantém o EXP-0 mínimo, mas deixa o frontend
   incompleto; rejeitado a pedido do mantenedor.

## Consequências

- Positivas: EXP-0 ganha aritmética inteira completa (`+ - * / %`); o adaptador
  cobre `/` e `%`; `division_by_zero` dá diagnóstico distinto de overflow.
- Negativas: novo código de erro estável (contrato) e dois opcodes novos no
  bytecode experimental; o formato textual ganha duas palavras-chave. Nada muda
  em capabilities ou efeitos de host.
- Residual: shifts, `f64` e strings continuam fora do EXP-0.

## Validação

- `crates/rvm-spec`: `eval_i64` para `/` e `%` (truncamento, zero, `MIN / -1`);
  round-trip de nomes; `division_by_zero` na tabela e como `ErrorClass::Trap`.
- `crates/rvm-conformance/corpus`: `C-020`, `C-021`, `N-025`, `N-026`; o
  diferencial `encode → decode → verify → execute` cobre as ops novas.
- `crates/rvm-bytecode`: opcodes 12/13 codificam/decodificam.
- `crates/rvm-interpreter`: `division_by_zero` e `integer_overflow` distintos.
- Adaptador: `cargo test --manifest-path frontends/lift-rir/Cargo.toml` (inclui
  `/` e `%` e o trap de divisão por zero).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
