# Incremento de enums no RIR — 2026-11-02

**Escopo:** implementação do núcleo do ADR-0039 (enums nominais) em
`rvm-spec`, `rvm-ir`, `rvm-runtime`, `rvm-verify`, `rvm-interpreter`,
`rvm-text`, `rvm-bytecode` e corpus de conformidade. Linker multi-módulo com
enums e lowering de enums/unions no adaptador Lift ficaram fora e continuam
falhando fechado.

## Resultado

- `cargo test --workspace --locked`: **187 passed, 0 failed** (30 alvos;
  inclui doc-tests vazios).
- Corpus: **66 válidos** e **48 inválidos**.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo fmt --all -- --check`: exit 0.
- `cargo +1.75.0 check --workspace --all-targets --locked`: exit 0 (MSRV).
- `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --locked`: exit 0.
- `frontends/lift-rir`: `cargo test --locked` — **47 passed, 0 failed**
  (9 CLI + 34 diferenciais + 4 encode).

Estes resultados são da execução local em Windows, rustc 1.94.1; não houve
execução de CI neste incremento.

## O que foi implementado

- `TypeDef::Enum(EnumId)`, `Module.enums`, `EnumDef`/`EnumVariantDef`/
  `EnumFieldDef`; `TypeDef` continua `Copy`.
- `Value::Enum { ty, variant, fields }`, `as_enum`, `Display`, `retained_bytes`
  e `depth`; `MAX_VALUE_DEPTH = 256`.
- `Op::{EnumNew, EnumIsVariant, EnumPayload, EnumEq, EnumNe}` em
  `operand_ids`, `result_type`, verificação, interpretador e bytecode
  (`TAG_ENUM = 9`, opcodes 55..=59).
- Regra de ciclos: permitido somente quando o ciclo contém enum; ciclos sem
  enum continuam `invalid_module`.
- Novos erros: `invalid_variant` (trap) e `value_too_deep` (limite de recurso).
- Texto: declaração `enum`, `type <id> = enum <Nome>`, referências para frente,
  `enum_new Enum.Variant` e índices numéricos em `enum_is_variant`/
  `enum_payload`; round-trip com enum recursivo e ciclo enum↔struct.
- Corpus: C-063..C-066 (válidos) e N-044..N-049 (inválidos/trap/limite).

## Limitações e riscos residuais

- `rvm-link` rejeita qualquer módulo com `enums` não vazio; remapeamento de
  `EnumId`/`TypeId` entre módulos não foi implementado.
- O adaptador Lift continua retornando `Unsupported` para enums, unions e
  `match`; não há teste diferencial Lift↔RIR para enums.
- O bytecode continua em `FORMAT_VERSION = 1`, mas a inserção de `enum_count`
  após as structs altera o layout. Não há promessa de compatibilidade entre
  revisões (ADR-0004); arquivos de layouts diferentes devem falhar fechado.
- A profundidade é limitada por `MAX_VALUE_DEPTH` porque `PartialEq`, `Display`,
  `retained_bytes` e o `Drop` derivado são recursivos. O teto não foi validado
  contra todos os tamanhos de pilha de todas as plataformas; é uma mitigação,
  não uma prova.
- O decoder hostil inclui um fixture de enum na campanha de mutação, mas não há
  teste dedicado para cada contagem/tag de enum corrompida.
- Nada aqui prova ausência de bugs, segurança multi-tenant ou estabilidade de
  formato.
