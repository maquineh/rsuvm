# ADR-0028 — Mutação de array por atualização funcional (`array_set`)

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; ADR-0027 (arrays imutáveis); invariante
  de **SSA puro** do EXP-0 (`docs/CONTRATOS/RIR-EXP-0-IMPLEMENTACAO.md`).

## Contexto

O EXP-0 é **SSA puro**: cada `ValueId` é definido exatamente uma vez e todo uso lê
um valor imutável. A ADR-0027 introduziu arrays imutáveis e deixou a mutação
(`array_set`) em aberto porque mutação in-place colide com essa semântica: se
`v3 = array v0, v1, v2` e depois se alterasse um elemento de `v3` no lugar, todos
os leitores de `v3` mudariam de valor retroativamente, quebrando a
transparência referencial e o determinismo garantidos pelo verificador.

É preciso decidir o **modelo de mutação** e o **aliasing** antes de implementar.

## Decisão

1. `array_set { array, index, value }` (`array_set v, i, x`) é uma **atualização
   funcional**: produz um **novo** array com o elemento em `index` substituído; o
   array de origem **não é alterado**. Não há identidade/ponteiro de array no
   núcleo — arrays são **valores**, não referências.
2. Consequência direta de SSA: **não existe aliasing** no núcleo. Dois leitores do
   mesmo `ValueId` sempre veem o mesmo array; quem quiser o array atualizado deve
   usar o resultado de `array_set`.
3. **Verificação** (`rvm-verify`): `array` precisa ser do tipo `array`; `index`
   precisa ser `i64`; `value` precisa ter **exatamente** o tipo do elemento do
   array (sem coerção). O resultado tem o tipo do array de entrada.
4. **Bounds check** em execução: `index < 0` ou `index >= len` produz o trap
   **`index_out_of_bounds`** (mesmo código de `array_get`, ADR-0027).
5. **Quota**: o array novo cobra `len × SLOT_BYTES` (24) na memória modelada,
   **cumulativo** (como literal e concatenação) → `out_of_memory`. O array antigo
   não é liberado (o modelo é cumulativo).
6. **Formato textual**: `array_set v, i, x`. **Bytecode**: opcode `38`
   (`u32 array`, `u32 index`, `u32 value`). Sem tag nova.
7. **Adaptador** Lift→RIR: `xs[i] = v` vira `array_set` + **religação** do binding
   `xs` ao novo array. Como Lift tem semântica de **referência** (arrays mutáveis)
   e a RIR tem semântica de **valor**, o adaptador só baixa a atribuição quando o
   binding é um **local não-aliased**: declarado por `let`/`const` na própria
   função e usado **apenas** como objeto de indexação (`xs[...]`), nunca passado
   como argumento, devolvido nem copiado para outro binding. Caso contrário,
   `AdapterError::Unsupported` (nunca um módulo incorreto).
8. **Corpus**: `C-036` (leitura após `array_set`), `C-037` (origem inalterada),
   `C-038` (elemento `string`), `N-030` (fora dos limites → `index_out_of_bounds`),
   `N-031` (valor de tipo errado → `invalid_type`).

## Alternativas

1. **Mutação in-place com células mutáveis / referências:** quebraria SSA e a
   pureza de `Value`, exigiria identidade e ordem de avaliação observável, e
   tornaria o determinismo dependente de aliasing. Rejeitada.
2. **Não ter mutação (status quo da ADR-0027):** mantém o núcleo puro, mas deixa
   coleções inutilizáveis para qualquer acumulador. Rejeitada a pedido do
   mantenedor.
3. **`array_set` in-place com `&mut` só no adaptador:** empurraria a decisão de
   aliasing para fora do núcleo e criaria dois modelos de array. Rejeitada.

## Consequências

- Positivas: mutação expressiva **sem** introduzir referências; SSA e
  determinismo preservados; bounds check fail-closed; um único trap reutilizado.
- Negativas: cópia O(n) por atualização; quota cumulativa cobra o array novo a
  cada `array_set` (conservador); o adaptador precisa de análise de aliasing
  conservadora (rejeita programas válidos que aliasam arrays).
- Residual: sem arrays aninhados, sem `push`/`pop`/slicing, sem identidade de
  array; objetos/closures continuam fora.

## Validação

- `crates/rvm-ir`: `Op::ArraySet` (resultado = tipo do array; operandos
  `array`/`index`/`value`).
- `crates/rvm-verify`: `require_array`/`array_element_type`; `index: i64`;
  `value` no tipo do elemento; tipo do resultado resolvido em `build_values`.
- `crates/rvm-interpreter`: bounds check → `index_out_of_bounds`; array novo;
  quota cumulativa.
- `crates/rvm-text`/`rvm-bytecode`: op `array_set` (texto) e opcode `38`.
- Corpus `C-036`/`C-037`/`C-038`/`N-030`/`N-031` (inclui o diferencial
  `encode → decode → verify → execute`).
- Adaptador: `cargo test --manifest-path frontends/lift-rir/Cargo.toml` (oráculo
  `rsuvm-oracle9.lift`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
