# ADR-0023 — Deslocamentos e operações bit a bit inteiras no núcleo

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; `docs/ARQUITETURA.md` §3 (que listava
  shifts como fora do EXP-0); ADR-0022 (padrão de ops inteiras verificadas).

## Contexto

O EXP-0 tinha `+ - * / %` e comparações. Faltavam deslocamentos e operações bit a
bit, comuns em código de sistema. A ADR-0022 fixou o padrão: ops inteiras
verificadas com traps distintos.

## Decisão

1. Adicionar cinco operações a [`rvm_spec::BinOp`]:
   - **`ShlI64Checked`** (`shl_i64_checked`) e **`ShrI64Checked`**
     (`shr_i64_checked`): deslocamento com quantidade verificada em `0..=63`;
     quantidade fora disso devolve [`None`] e produz o trap **`invalid_shift`**
     (novo [`rvm_spec::ErrorCode`], `Trap`). `shr` é **aritmético** (sinal
     preservado, como `i64::checked_shr`).
   - **`AndI64`** (`and_i64`), **`OrI64`** (`or_i64`) e **`XorI64`** (`xor_i64`):
     operações bit a bit, **sem trap**.
2. Os bits deslocados para fora são descartados (semântica de `<<`/`>>` do Rust);
   `checked_shl`/`checked_shr` verificam apenas a **quantidade**.
3. **Bytecode** ganha os opcodes `14..=18`; o restante do pipeline é genérico sobre
   `BinOp`.
4. **Corpus**: `C-022` (`shl`), `C-023` (`shr`), `C-024` (`and`), `C-025` (`or`),
   `C-026` (`xor`) e `N-027` (`shl` com 64 → `invalid_shift`).
5. O adaptador Lift→RIR **não** mapeia estas ops nesta rodada: a `BinaryOperator`
   do Lift não expõe bit a bit nem shifts (só aritmética, comparação e lógica).

## Alternativas

1. **Reusar `integer_overflow` para deslocamento inválido:** funde diagnósticos;
   rejeitado (padrão da ADR-0022 é trap distinto).
2. **`shl` sem verificação (wrap):** silencioso e inconsistente com a aritmética
   verificada do EXP-0; rejeitado.
3. **Não adicionar bit a bit:** mantém o núcleo mínimo, mas incompleto para
   aritmética inteira; rejeitado a pedido do mantenedor.

## Consequências

- Positivas: aritmética inteira mais completa (`+ - * / % << >> & | ^`); trap
  `invalid_shift` distinto; bit a bit sem trap (não pode transbordar).
- Negativas: novo código de erro estável e cinco opcodes novos; o formato textual
  ganha cinco palavras-chave. Nada muda em capabilities/efeitos.
- Residual: `f64`, strings, arrays e objetos continuam fora do EXP-0.

## Validação

- `crates/rvm-spec`: `eval_i64` para `<< >> & | ^` (incluindo `1 << 64` → `None` e
  `shr` aritmético); round-trip de nomes; `invalid_shift` como `ErrorClass::Trap`.
- `crates/rvm-conformance/corpus`: `C-022..C-026`, `N-027`; o diferencial
  `encode → decode → verify → execute` cobre as ops novas.
- `crates/rvm-bytecode`: opcodes 14..18 codificam/decodificam.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
