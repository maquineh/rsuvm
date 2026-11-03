# ADR-0039 — Enums e unions no RIR (proposta)

- **Status:** Proposto
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor para iniciar a especificação de
  enums/unions no RIR; adaptador Lift atualizado para `e04bc26`, que já expõe
  enums, unions e `match`; ADR-0029 (structs), ADR-0034 (tipos por id),
  ADR-0035 (memória retida) e ADR-0021 (adaptador Lift).
- **Especificação técnica:** [ENUMS-UNIONS-PROPOSTA](../CONTRATOS/ENUMS-UNIONS-PROPOSTA.md)

> Esta ADR **propõe** o desenho. Nenhum código, tipo, opcode, corpus ou
> adaptador foi alterado para enums/unions nesta rodada. O adaptador Lift
> continua rejeitando esses recursos com `Unsupported`.

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

## Decisão proposta

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
   Name`; operações qualificadas por `Enum.Variant`; forma canônica e ordem de
   impressão definidas no contrato.
7. **Bytecode aditivo:** `TAG_ENUM = 9`, tabela de enums após structs e opcodes
   55–59. Sem bump de `IR_VERSION`/`FORMAT_VERSION` nesta proposta; decoders
   antigos rejeitam os novos tags/opcodes com falha fechada. Se o mantenedor
   preferir versionar, `IR_VERSION = 2` é a alternativa.
8. **Limites novos** (`max_enums`, `max_variants_per_enum`,
   `max_fields_per_enum_variant`) com checagem antes de alocar; sem teto
   agregado nesta primeira proposta.
9. **Adaptador Lift:** mapear `HirEnum`/`EnumConstruct`/`Match` para os novos
   tipos/ops; ignorar `field_boxed` (o RIR já é heap-backed); validar
   exaustividade defensivamente; `lift-encode` passa a emitir `.rsvmbc` com
   enums.

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

## Validação proposta

- Contrato técnico: `docs/CONTRATOS/ENUMS-UNIONS-PROPOSTA.md`.
- Corpus RIR: casos válidos/inválidos listados no contrato (§12).
- Adaptador Lift: enums posicionais/nomeados/recursivos, `match`, unions,
  enums em arrays/structs/options, teste diferencial e `lift-encode`.
- Bytecode: round-trip e decoder hostil para tags/contagens/índices.
- Gates: `cargo test --workspace`, `clippy -D warnings`, `fmt --check`, MSRV,
  docs; `cargo test --manifest-path frontends/lift-rir/Cargo.toml`.
- Critério de aceitação: todos os casos acima executam/verificam e o adaptador
  deixa de rejeitar enums/unions suportados.

## Questões abertas

1. Ciclos: só com enum ou qualquer indireção heap?
2. `invalid_variant` novo ou reutilizar `unwrap_error`?
3. `IR_VERSION` aditivo (1) ou bump para 2?
4. Limites agregados para variantes/campos?
5. Formato textual de `type`/`enum` e ordem canônica.
6. Igualdade/`retained_bytes`/`Display` depth-safe: iterativo ou limite?
7. Suportar `match` de `Option`/`Result` no mesmo incremento ou só enums?
8. Enums em módulos ligados por `rvm-link` neste incremento ou apenas em módulos
   planos (single-module), adiando o remapeamento de `EnumId`?
