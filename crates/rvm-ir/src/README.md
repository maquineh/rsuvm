# rvm-ir — Package Contract

## Overview

`rvm-ir` define o modelo da RIR experimental EXP-0 (`Module`, `Function`,
`Block`, `Instr`, `Op`, `Terminator`) e a checagem estrutural
`structure::check`, que roda antes de qualquer verificação de tipos e antes de
alocar memória proporcional ao módulo. Depende apenas de `rvm-spec`. O crate não
executa, não valida tipos e não conhece texto nem bytecode: é a fronteira de
dados compartilhada (ADR-0002).

## Interface

- `FnId`, `BlockId`, `ValueId` — aliases `u32` com invariante de densidade.
- `Module` — `name`, `version`, `entry`, `types`, `structs`, `enums`, `functions`
  (`lib.rs:Module`); `function(id)`, `function_by_name(name)`, `struct_def(id)`,
  `enum_def(id)`, `enum_by_name(name)`, `type_def(id)`, `find_type(def)`.
- `EnumDef`/`EnumVariantDef`/`EnumFieldDef` — enum nominal, variantes e campos
  posicionais (`name == None`) ou nomeados (ADR-0039).
- `Function` — `id`, `name`, `params`, `ret`, `entry`, `blocks`
  (`lib.rs:Function`); `block(id)`, `value_count()`.
- `Param` — `name`, `ty: TypeId`.
- `Block` — `id`, `label`, `params`, `instrs`, `term` (`lib.rs:Block`).
- `BlockParam` — `value`, `name`, `ty` (merge por phi explícito).
- `Instr` — `def`, `op`.
- `Op` — variantes do subset (`ConstI64`, `ConstBool`, `ConstStr`, `ConstF64`,
  `Bin`, `Cmp`, operações de `string`/`f64`/array/struct/option/result/enum,
  `Call` e `HostCall`); `result_type`, `operand_ids` (`lib.rs:Op`).
- `Terminator` — `Return`, `Br`, `CondBr`; `operand_ids` (`lib.rs:Terminator`).
- `BranchTarget` — `target`, `args`.
- `structure::check(&Module, &Limits) -> Result<(), RvmError>`
  (`structure.rs:check`).
- `testing` — construtores de módulos válidos para testes, incluindo
  `host_read_file`, habilitados por `cfg(test)` ou pela feature `test-helpers`
  (`lib.rs:testing`).

## Invariants

- `functions[i].id == i` e `blocks[i].id == i`; IDs não densos ou duplicados são
  `invalid_module` (`structure.rs:check_function`).
- Parâmetros de função ocupam os `ValueId` `0..params.len()`; os demais valores
  (parâmetros de bloco e definições) completam `{0..N-1}` sem repetição.
- O bloco de entrada não declara parâmetros.
- Nomes são identificadores ASCII `[A-Za-z_][A-Za-z0-9_]*`, não vazios, com no
  máximo `Limits::max_name_len` bytes; `v` + apenas dígitos é reservado para
  impressão canônica e é rejeitado em parâmetros (`structure.rs:is_reserved_value_name`).
- Nomes de função são únicos no módulo; rótulos e nomes de valor são únicos na
  função.
- Todo operando referencia valor existente (`invalid_module`); todo alvo de
  desvio referencia bloco existente (`invalid_control_flow`); toda função chamada
  existe (`invalid_module`).
- Toda contagem é comparada com `Limits` antes da alocação; a soma de valores usa
  aritmética verificada e o vetor de presença só é alocado depois da checagem.
- O terminador é obrigatório por tipo (`Block::term` não é `Option`); bloco sem
  terminador é rejeitado na fronteira textual, não representável aqui.
- Tipos vivem na tabela do módulo (`types`, ADR-0034); construções resolvidas
  pelos operandos (`array`, `option_some`, `coalesce`, `field_get`,
  `enum_payload`) não expõem tipo fixo em `Op::result_type` (`None`).
- `enums[i].id == i`; nomes de enum são únicos e não colidem com structs; cada
  variante tem campos homogêneos e cada enum tem exatamente um `TypeDef::Enum`
  canônico; os limites `max_enums`/`max_variants_per_enum`/
  `max_fields_per_enum_variant` são checados antes de alocar (ADR-0039).
- O grafo de tipos é acíclico, exceto ciclos que contenham um enum; ciclos sem
  enum continuam `invalid_module`.
- `structure::check` não olha tipos de operandos nem dominância; isso é
  responsabilidade de `rvm-verify`.

## Side Effects

- `None`. `structure::check` é função pura; a única alocação relevante é o vetor
  de presença de valores, limitado por `Limits::max_values_per_function` e alocado
  depois da checagem. Sem IO, rede, ambiente, relógio, processos ou `unsafe`.

## Verification

- Run: `cargo test --workspace`
