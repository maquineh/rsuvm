# ADR-0039 — Enums e unions no RIR

- **Status:** Aceito; implementado no núcleo (single-module). Linker multi-módulo
  e adaptador Lift permanecem fora deste incremento.
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor para iniciar a especificação de
  enums/unions no RIR; adaptador Lift atualizado para `e04bc26`, que já expõe
  enums, unions e `match`; ADR-0029 (structs), ADR-0034 (tipos por id),
  ADR-0035 (memória retida) e ADR-0021 (adaptador Lift).
- **Especificação técnica:** [ENUMS-UNIONS-PROPOSTA](../CONTRATOS/ENUMS-UNIONS-PROPOSTA.md)

> Esta ADR foi aceita e o núcleo foi implementado: `rvm-spec`, `rvm-ir`,
> `rvm-runtime`, `rvm-verify`, `rvm-interpreter`, `rvm-text`, `rvm-bytecode` e o
> corpus. `rvm-link` rejeita módulos com enums e o adaptador Lift continua
> rejeitando esses recursos com `Unsupported` (falha fechada).

## Contexto

O Lift atual tem enums nominais com variantes vazias, posicionais e com campos
nomeados, enums recursivos/mutuamente recursivos e unions nominais desugeradas
para enums. O RIR do EXP-0 só tem `unit`, `bool`, `i64`, `string`, `f64`,
`array`, `struct`, `option` e `result`.

O adaptador Lift (`frontends/lift-rir`) foi atualizado para o Lift `e04bc26` e
passa a **rejeitar** `Enum`, `match` e construções de enum com
`AdapterError::Unsupported`. Isso é correto (falha fechada), mas bloqueia uma
parte central da linguagem atualizada.

Uma representação por “struct + tag `i64`” perderia a nominalidade, a checagem
de variante e a exaustividade. `option`/`result` não expressam N variantes com
payloads distintos. É necessário um tipo nominal próprio.

## Decisão

1. **Tipo nominal `TypeDef::Enum(EnumId)` + tabela `Module.enums`.** Cada
   `EnumDef` tem nome e variantes; cada variante tem campos posicionais ou
   nomeados, referenciando `TypeId`. `TypeDef` continua `Copy`.
2. **Unions não são um tipo RIR separado.** O frontend desugera
   `type A = B | C` para um enum nominal com variantes `B(B)` e `C(C)`, como o
   Lift já faz na HIR.
3. **Valor `Value::Enum { ty, variant, fields }`**, com igualdade estrutural,
   `Display` estável, `retained_bytes` (tag + slots + filhos) e `as_enum`.
4. **Operações `enum_new`, `enum_is_variant`, `enum_payload`, `enum_eq` e
   `enum_ne`.** `enum_payload` sobre a variante errada trapaca o novo código
   `invalid_variant` (classe `trap`); divergências estáticas são `invalid_type`.
5. **Ciclos de tipo permitidos quando o ciclo contém um enum.** A recursão é
   heap-backed (`Vec<Value>`); ciclos sem enum continuam rejeitados pela regra
   do ADR-0034. `struct S { s: S }` continua inválido.
6. **Formato textual:** declaração `enum Name { ... }` + `type <id> = enum
   Name`; `enum_new` usa `Enum.Variant`; `enum_is_variant`/`enum_payload` usam
   índice de variante e índice posicional do campo. Ordem canônica e forma
   definidas no contrato.
7. **Bytecode:** `TAG_ENUM = 9`, tabela de enums após structs e opcodes
   55–59. `IR_VERSION`/`FORMAT_VERSION` permanecem `1`; como o formato é
   experimental e não promete compatibilidade entre revisões (ADR-0004), a
   inserção da tabela altera o layout de arquivos gerados antes deste incremento.
   Decoders antigos/new decoders sobre o layout antigo falham fechado
   (`truncated_input`/`invalid_module`/`unknown_type`), nunca executam bytes
   ambíguos.
8. **Limites novos** (`max_enums`, `max_variants_per_enum`,
   `max_fields_per_enum_variant`) com checagem antes de alocar; sem teto
   agregado nesta primeira proposta.
9. **Adaptador Lift (adiado):** mapear `HirEnum`/`EnumConstruct`/`Match` para
   os novos tipos/ops; ignorar `field_boxed` (o RIR já é heap-backed); validar
   exaustividade defensivamente; `lift-encode` passa a emitir `.rsvmbc` com
   enums. Neste incremento o adaptador continua retornando `Unsupported`.
10. **Profundidade de valor:** `MAX_VALUE_DEPTH = 256`; `enum_new` calcula a
    profundidade de forma iterativa e falha com `value_too_deep`
    (`resource_limit`). O teto mantém seguros `PartialEq`, `Display`,
    `retained_bytes` e o `Drop` derivado, que continuam recursivos.

## Alternativas consideradas

1. **Struct + tag `i64` + `field_get`.** Rejeitada: sem nominalidade, sem
   variante verificada, sem exaustividade e sem payload tipado por variante.
2. **Aninhar `option`/`result` para N variantes.** Rejeitada: tags aninhadas,
   explosão combinatória e checagem de exaustividade frágil.
3. **Tipo `TypeDef::Union` separado.** Rejeitada: o Lift desugera unions para
   enums; dois mecanismos no RIR aumentariam a superfície sem ganho.
4. **Representação inline das variantes em `TypeDef`.** Rejeitada: quebraria
   `TypeDef: Copy` e espalharia a estrutura por toda a tabela; a tabela
   `EnumDef` é análoga a `StructDef`.
5. **Expor discriminantes numéricos ao convidado.** Rejeitada: o convidado
   poderia forjar variantes; `enum_is_variant`/`enum_payload` mantêm a checagem
   no runtime.
6. **Permitir ciclos por qualquer indireção heap.** Adiada; a proposta
   conservadora permite somente ciclos com enum e preserva a regra atual para
   structs/arrays/options/results.
7. **Usar `Rust` enum nativo no RIR.** Rejeitada: o RIR é independente de
   linguagem (ADR-0001) e precisa de representação serializável/verificável.

## Consequências

**Positivas**

- Desbloqueia enums/unions do Lift no adaptador e no `lift-encode`.
- Mantém identidade nominal, payload tipado e checagem de variante.
- Reutiliza a infraestrutura de tipos por id, memória retida e bytecode.
- Falha fechada para variante errada (`invalid_variant`) e para módulos
  malformados (`invalid_type`/`invalid_module`).

**Negativas**

- Aumenta a tabela de tipos, o bytecode, o decoder, o verificador e o
  interpretador; exige novos casos de corpus e testes de round-trip.
- Adiciona um código de erro estável (`invalid_variant`).
- A regra de ciclos com enum é mais complexa que a regra atual.
- Igualdade/`retained_bytes`/`Display` recursivos exigem decisão de
  profundidade para não estourar a pilha nativa.
- `match` continua sendo lowering do frontend, não uma operação RIR; a
  exaustividade não é provada pelo verificador.
- Enums multi-módulo exigem que `rvm-link` remapeie `EnumId`/`TypeDef::Enum` e
  os índices de variante; o linker já tem restrições com tipos compostos.

**Residual**

- Sem genéricos, closures, métodos de enum, discriminantes explícitos,
  wildcard/guard/nested patterns ou FFI com enums.
- Sem estabilidade de formato; a proposta é experimental.
- Sem host ABI com enums; a ABI atual continua `read_file`/`string`.
- Sem prova de ausência de bugs; fuzzing/auditoria continuam necessários.

## Validação

- Contrato técnico: `docs/CONTRATOS/ENUMS-UNIONS-PROPOSTA.md`.
- Corpus RIR: C-063..C-066 (válidos) e N-044..N-049 (inválidos/trap/limite).
- Modelo e estrutura: testes em `crates/rvm-ir/src/structure.rs` (ciclo com
  enum, tabela canônica, variantes/campos e limites).
- Texto: round-trip com enum recursivo e ciclo enum↔struct em `rvm-text`.
- Bytecode: fixture `testing::enum_simples` no round-trip do decoder.
- Interpretador: `enum_simples` em `crates/rvm-interpreter/tests/execucao.rs` e
  N-044/N-049 no corpus.
- Gates executados neste incremento: `cargo test --workspace --locked`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all -- --check` e `cargo test --locked` em `frontends/lift-rir`.
- Critério de aceitação **parcial**: o núcleo está implementado; o adaptador
  Lift e o linker multi-módulo continuam rejeitando enums, como declarado no
  escopo.

## Estado de implementação

Implementado e coberto:

- `TypeDef::Enum`, `EnumId`, `Module.enums`, `EnumDef`/`EnumVariantDef`/
  `EnumFieldDef`; `TypeDef` continua `Copy`.
- `Value::Enum`, `as_enum`, `Display`, `retained_bytes`, `depth` e
  `MAX_VALUE_DEPTH`.
- `Op::{EnumNew, EnumIsVariant, EnumPayload, EnumEq, EnumNe}` em
  `operand_ids`, `result_type`, verificação, interpretador e bytecode
  (`TAG_ENUM = 9`, opcodes 55..=59).
- Regra de ciclos com enum, `invalid_variant` e `value_too_deep`.
- Texto: declaração `enum`, `type <id> = enum <Nome>`, operações e round-trip.
- Corpus C-063..C-066 e N-044..N-049.

Adiado explicitamente:

- `rvm-link` com enums (rejeita módulos com `enums` não vazio).
- Adaptador Lift, `match` lowering e unions (continua `Unsupported`).
- `IR_VERSION`/`FORMAT_VERSION` permanecem `1`; a tabela de enums muda o layout
  do bytecode experimental. Não há promessa de compatibilidade entre revisões
  (ADR-0004): arquivos de layouts diferentes falham fechado.

## Questões abertas

1. Ciclos: decidido permitir somente ciclos que contenham enum.
2. `invalid_variant`: decidido criar o código novo (classe `trap`).
3. `IR_VERSION`: decidido manter `1`; o layout do bytecode experimental muda,
   sem promessa de compatibilidade entre revisões.
4. Limites agregados: adiados; os limites por enum/variante/campo são
   verificados antes de alocar.
5. Formato textual: decidido; `type`/`enum`/`struct` e índices numéricos nas
   operações de inspeção estão no contrato `TEXTO-EXP-0.md`.
6. Profundidade: decidido `MAX_VALUE_DEPTH = 256` + `value_too_deep`.
7. `match` de `Option`/`Result`: fora deste incremento.
8. Enums em `rvm-link`: adiados; apenas módulos planos neste incremento.
