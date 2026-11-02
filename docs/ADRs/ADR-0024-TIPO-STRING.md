# ADR-0024 — Tipo `string` (texto UTF-8 imutável) no núcleo

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; `docs/ARQUITETURA.md` §3 (que listava
  strings como fora do EXP-0); ADR-0001 (núcleo independente de linguagem).

## Contexto

O EXP-0 só tinha `unit`, `bool` e `i64`. Sem texto, um frontend não cobre literais
de string nem comparação/length, e o adaptador Lift→RIR não consegue mapear o tipo
`string` do Lift. Este ADR fecha a lacuna com o mínimo determinístico.

## Decisão

1. Novo tipo [`rvm_spec::Type::Str`] (`string`) e valor
   [`rvm_runtime::Value::Str(String)`] (UTF-8, imutável). `Value` **deixa de ser
   `Copy`** e passa a `Clone`; slots de frame clonam ao ler.
2. Novas operações em [`rvm_ir::Op`]:
   - `ConstStr(String)` (`const_str`): o literal **vive no módulo**;
   - `StrEq { lhs, rhs }` (`str_eq`) e `StrNe { lhs, rhs }` (`str_ne`): comparam
     **conteúdo**, produzem `bool`;
   - `StrLen { operand }` (`str_len`): comprimento em **bytes**, produz `i64`.
3. **Sem concatenação, slicing nem alocação em execução**: os literais são
   imutáveis e fazem parte do módulo (bounded pela entrada). Não há arena de
   runtime nem nova contabilidade de memória nesta rodada. Concatenação exigiria
   arena + quota (trabalho futuro).
4. **Formato textual**: literal `"..."` com escapes `\"`, `\\`, `\n`, `\r`, `\t`;
   operações `const_str`, `str_eq`, `str_ne`, `str_len`. A impressão canônica
   reescapa o conteúdo.
5. **Bytecode**: tag de tipo `3` (`string`); opcodes `19..=22`; o literal é
   `u16` de comprimento + bytes UTF-8.
6. **Limite** [`rvm_spec::Limits::max_string_bytes`] (padrão 4096), validado
   **antes de alocar** no parser textual, no decoder binário e em
   `structure::check` (para módulos construídos por API).
7. **Corpus**: `C-027` (`str_eq`), `C-028` (`str_len`), `C-029` (`str_ne`),
   `C-030` (retorno de string e impressão).
8. O adaptador Lift→RIR pode mapear `string` do Lift (fora do escopo desta
   rodada; o lowering de strings no adaptador fica como próximo passo).

## Alternativas

1. **Pool indexado (`Value::Str(u32)`), mantendo `Value` Copy:** exigiria
   formatação/igualdade cientes do pool em CLI e corpus; mais complexo e frágil.
2. **`Value::Str` + concatenação com arena e quota de memória:** mais útil, porém
   muito maior (arena, contabilidade, determinismo); adiado.
3. **Não suportar strings:** mantém o núcleo mínimo, mas incompleto; rejeitado a
   pedido do mantenedor.

## Consequências

- Positivas: tipo de texto real com literais, igualdade e comprimento;
  determinístico e sem alocação em execução; bounded por `max_string_bytes`.
- Negativas: `Value` deixa de ser `Copy` (refactor pequeno em leitura de slots);
  novo tipo e opcodes no formato experimental; sem concatenação/slicing.
- Residual: concatenação, slices, comparação de ordem e conversões numéricas
  continuam fora; `f64`, arrays e objetos também.

## Validação

- `crates/rvm-spec`: `Type::Str` bidirecional; `Limits::max_string_bytes`.
- `crates/rvm-runtime`: `Value::Str` (`ty`, `as_str`, `matches_type`, `Display`).
- `crates/rvm-text`: lexing de `"..."` com escapes; round-trip
  `parse(print(m)) == m`; string acima do limite → `invalid_module`.
- `crates/rvm-bytecode`: tag `3` e opcodes `19..=22` codificam/decodificam;
  literal acima do limite → `invalid_module` antes de alocar.
- `crates/rvm-interpreter`: `const_str`/`str_eq`/`str_ne`/`str_len`.
- Corpus `C-027..C-030` (inclui o diferencial `encode → decode → verify →
  execute`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
