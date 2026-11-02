# ADR-0032 — Mutação de campo por atualização funcional (`field_set`)

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; ADR-0028 (mutação de array); ADR-0029
  (structs); invariante de SSA puro.
- **Revisão (ADR-0035):** a cobrança cumulativa da struct nova foi substituída pelo
  modelo de bytes retidos por slot.

## Contexto

A ADR-0029 introduziu structs com leitura de campo (`field_get`), mas sem escrita.
Como no EXP-0 os valores são imutáveis e SSA, mutação in-place de campo colidiria
com a semântica (um leitor de `v` mudaria retroativamente). É preciso decidir o
modelo de mutação e o aliasing, como na ADR-0028.

## Decisão

1. `FieldSet { object, field, value }` (`field_set v, campo, x`) é uma **atualização
   funcional**: devolve uma **nova** struct com o campo substituído; a struct de
   origem **não é alterada**. Structs são **valores**, não referências.
2. **Sem aliasing** no núcleo (SSA): dois leitores do mesmo `ValueId` veem a mesma
   struct.
3. **Verificação** (`rvm-verify`): `object` precisa ser `struct`; `field` precisa
   existir; `value` precisa ter **exatamente** o tipo do campo. O resultado tem o
   tipo da struct de entrada.
4. **Quota**: a struct nova cobra `campos × SLOT_BYTES` (24) na memória modelada,
   **cumulativo** → `out_of_memory`.
5. **Formato textual**: `field_set v, campo, x`. **Bytecode**: opcode `48`
   (`u32 object`, `string field`, `u32 value`). Sem tag nova.
6. **Adaptador** Lift→RIR: `obj.campo = v` vira `field_set` + **religação** do
   binding `obj` à struct nova. A mesma análise conservadora de aliasing da
   ADR-0028 é **generalizada** para structs: só é baixado quando `obj` é um **local
   não-aliasado** (declarado por literal de array/struct e usado apenas como
   "lugar" — objeto de indexação ou acesso a campo). Caso contrário,
   `AdapterError::Unsupported`.
7. **Corpus**: `C-050` (leitura após `field_set`), `C-051` (origem inalterada),
   `C-052` (campo `string`); `N-039` (valor de tipo errado), `N-040` (campo
   inexistente).

## Alternativas

1. **Mutação in-place com referências:** quebraria SSA e a pureza de `Value`;
   rejeitada (mesma razão da ADR-0028).
2. **Sem `field_set`:** mantém o núcleo puro, mas deixa structs somente-leitura;
   rejeitada a pedido do mantenedor.
3. **Análise de aliasing nova para structs:** como structs do subset têm campos
   escalares (Copy), a cópia é independente; ainda assim, a análise conservadora
   existente é reutilizada para cobrir passagem por referência mutável.

## Consequências

- Positivas: mutação de campo expressiva **sem** referências; SSA e determinismo
  preservados; reutiliza a análise de aliasing e o modelo de quota.
- Negativas: cópia O(campos) por atualização; quota cumulativa cobra a struct nova;
  o adaptador recusa programas que aliasam a struct.
- Residual: sem campos aninhados, sem `field_set` de campo struct/array, sem
  métodos.

## Validação

- `crates/rvm-ir`: `Op::FieldSet` (resultado = tipo da struct).
- `crates/rvm-verify`: campo existente e tipo exato; resultado resolvido em
  `build_values`.
- `crates/rvm-interpreter`: struct nova; quota cumulativa.
- `crates/rvm-text`/`rvm-bytecode`: operação e opcode `48`.
- Corpus `C-050`..`C-052`/`N-039`/`N-040` (inclui o diferencial
  `encode → decode → verify → execute`).
- Adaptador: `cargo test --manifest-path frontends/lift-rir/Cargo.toml` (oráculo
  `rsuvm-oracle14.lift` = `11 3 bia`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
