# ADR-0029 — Objetos/structs no núcleo

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; ADR-0027/ADR-0028 (arrays); invariante de
  `Type: Copy`; `docs/ARQUITETURA.md`.

## Contexto

Faltava um tipo **produto** no EXP-0: um valor com campos nomeados. Sem ele, um
frontend não expressa registros. Como o núcleo é tipado estaticamente e `Type`
precisa continuar `Copy` (usado por valor em toda a RIR), um tipo de struct
recursivo (`Box<Type>`) ou por nome (`String`) forçaria um refatoramento amplo.

## Decisão

1. Novo tipo [`rvm_spec::Type::Struct(StructId)`], onde `StructId = u32` indexa a
   tabela de structs do módulo. Mantém `Type: Copy`.
2. [`rvm_ir::Module`] ganha `structs: Vec<StructDef>` (IDs densos), com
   [`StructDef`] = nome + `Vec<FieldDef>`. **Campos são escalares**
   (`bool`/`i64`/`string`/`f64`); **sem** campos struct/array/unit nesta rodada
   (evita recursão e interação com arrays).
3. Novo valor [`rvm_runtime::Value::Struct { ty: StructId, fields: Vec<Value> }`].
4. Novas operações em [`rvm_ir::Op`]:
   - `StructNew { struct_id, fields }` (`struct_new Nome v0, v1, …`): constrói a
     struct; exige exatamente um valor por campo, no tipo do campo;
   - `FieldGet { object, field }` (`field_get v, campo`): lê um campo; o objeto
     precisa ser struct e o campo precisa existir.
   Não há `field_set`: mutação de campo fica fora desta rodada (como arrays
   tiveram leitura antes de `array_set`).
5. **Verificação** (`rvm-verify`): o tipo do resultado de `struct_new` é a própria
   struct; o de `field_get` é o tipo do campo, resolvido a partir do tipo do
   objeto. `structure::check` valida IDs densos, nomes/únicos, campos escalares e
   que todo `Type::Struct(id)` referencie struct existente.
6. **Quota**: `struct_new` cobra `campos × SLOT_BYTES` (24) na memória modelada,
   **cumulativo** → `out_of_memory`.
7. **Formato textual**: declaração `struct Nome { campo: tipo … }`; tipo
   `struct Nome` (em parâmetros, retorno e parâmetros de bloco); operações
   `struct_new Nome v0, v1` e `field_get v, campo`. **Bytecode**: tag de tipo `6`
   (`struct` + `u32 id`), opcodes `39` (`struct_new`), `40` (`field_get`) e uma
   **tabela de structs** após `entry` (contagem `u16`; por struct: `u32 id`, nome,
   `u16` campos, cada campo nome + tipo).
8. **Ligação inter-módulo** (`rvm-link`) **não** combina tabelas de structs nesta
   rodada: linkar um módulo que declare structs é rejeitado.
9. O **adaptador** Lift→RIR mapeia `interface Nome { … }` para a tabela de
   structs, o literal `{ campo: valor, … }` para `struct_new` (na ordem declarada)
   e `obj.campo` para `field_get`.
10. **Corpus**: `C-039` (`struct_new`/`field_get`), `C-040` (campos `string`/`bool`),
    `C-041` (struct como parâmetro); `N-032` (`field_get` em não-struct),
    `N-033` (campo inexistente), `N-034` (valor de tipo errado em `struct_new`).

## Alternativas

1. **`Type::Struct(Box<Type>)` recursivo:** permitiria structs aninhadas, mas
   torna `Type` não-`Copy` e exige refatoramento amplo. Adiado.
2. **Structs por nome (`String`) em `Type`:** idem (não-`Copy`). Rejeitada.
3. **Campos com arrays/structs:** interage com ADR-0027/0028 e recursão; fora
   desta rodada.
4. **`field_set` (mutação de campo):** exigiria decisão de aliasing (análoga a
   ADR-0028); adiada.

## Consequências

- Positivas: tipo produto com campos tipados e acesso verificado; `Type`
  permanece `Copy`; dois opcodes e uma tag novos; sem trap novo.
- Negativas: campos só escalares; sem mutação de campo nem aninhamento; ligação
  com structs indisponível; `Display` de valor usa o **id** da struct
  (`struct:<id>[...]`) por não ter acesso à tabela.
- Residual: objetos mutáveis, structs aninhadas, métodos, ligação com structs.

## Validação

- `crates/rvm-spec`: `StructId`, `Type::Struct`; limites `max_structs`/
  `max_fields_per_struct`.
- `crates/rvm-ir`: `StructDef`/`FieldDef`, `Op::StructNew`/`Op::FieldGet`,
  `structure::check` da tabela e das referências.
- `crates/rvm-verify`: tipos de `struct_new`/`field_get` e existência de campo.
- `crates/rvm-interpreter`: construção e leitura com quota cumulativa.
- `crates/rvm-text`/`rvm-bytecode`: declaração, tipo, ops, tabela e opcodes.
- Corpus `C-039`/`C-040`/`C-041`/`N-032`/`N-033`/`N-034` (inclui o diferencial
  `encode → decode → verify → execute`).
- Adaptador: `cargo test --manifest-path frontends/lift-rir/Cargo.toml` (oráculo
  `rsuvm-oracle10.lift` = `7 ana 42`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
