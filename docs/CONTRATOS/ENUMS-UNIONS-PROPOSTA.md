# Enums e unions no RIR — proposta técnica

**Status:** proposta, **não implementada**. Este documento não é contrato vigente;
o RIR atual continua sem enums/unions e o adaptador Lift os rejeita com
`Unsupported`. A decisão está em
[ADR-0039](../ADRs/ADR-0039-ENUMS-UNIONS-RIR.md).

## 1. Escopo

Adicionar ao RIR um tipo **enum nominal com variantes etiquetadas** capaz de
representar:

- enums Lift com variante sem payload, payload posicional ou campos nomeados;
- enums recursivos (`enum List { Nil, Cons(int, List) }`) e mutuamente
  recursivos;
- unions Lift (`type Shape = Circle | Square`), que o frontend desugara para um
  enum com uma variante posicional por membro;
- enums como payload de `array`, `struct`, `option` e `result`;
- igualdade estrutural, construção e inspeção de variante.

Fora do escopo desta proposta:

- genéricos, closures/function values, métodos de enum, discriminantes
  explícitos, `match` como operação do RIR, wildcard/guard/nested patterns;
- FFI/host ABI com enums (a ABI atual continua `read_file`/`string`);
- unions estruturais/anônimas: unions Lift são nominais e desugaradas no
  frontend;
- layout binário estável ou compatibilidade entre versões.

## 2. Tipo e tabela

### 2.1 Identificadores

```rust
pub type EnumId = u32;
pub type EnumVariantId = u32;
```

`EnumId` identifica a declaração na tabela `Module::enums`; `EnumVariantId` é o
índice da variante na ordem de declaração. Nenhum dos dois é um discriminante
numérico exposto ao convidado.

### 2.2 `TypeDef`

Adicionar uma variante:

```rust
pub enum TypeDef {
    // ... existentes ...
    Enum(EnumId),
}
```

`TypeDef` continua `Copy`; a estrutura das variantes fica na tabela de enums
(como `Struct` referencia `StructId`). `TypeDef::name()` retorna `"enum"`;
`is_scalar()` continua falso. O `Display` do tipo é `enum` (o nome nominal é
resolvido na tabela).

### 2.3 `EnumDef`

```rust
pub struct EnumDef {
    pub id: EnumId,
    pub name: String,
    pub variants: Vec<EnumVariantDef>,
}

pub struct EnumVariantDef {
    pub name: String,
    pub fields: Vec<EnumFieldDef>,
}

pub struct EnumFieldDef {
    /// `None` = payload posicional; `Some(name)` = campo nomeado.
    pub name: Option<String>,
    pub ty: TypeId,
}
```

Invariantes:

1. `enums[i].id == i` (IDs densos).
2. Nome do enum é identificador ASCII válido, único entre enums e structs do
   módulo (namespace nominal).
3. `variants` tem pelo menos uma entrada; nomes de variantes são únicos dentro do
   enum.
4. Em cada variante, os campos são **todos posicionais** (`name == None`) ou
   **todos nomeados** (`name == Some`); não há mistura. Campos nomeados têm nomes
   únicos e válidos.
5. Todo `ty` referencia um `TypeId` existente.
6. Existe **exatamente um** `TypeDef::Enum(id)` na tabela de tipos para cada
   `EnumDef`; e todo `TypeDef::Enum(id)` referencia um enum existente. Isso
   preserva identidade nominal por `TypeId`.
7. `Module.enums` é indexado por `EnumId`; a ordem é a ordem de declaração.

### 2.4 `Module`

Adicionar:

```rust
pub struct Module {
    // ... existentes ...
    pub enums: Vec<EnumDef>,
}
```

`Module::enum_def(id)`, `Module::find_enum(name)` e `Module::enum_type(id)`
(resolve o `TypeId` canônico) são auxiliares previstos.

### 2.5 Ciclos de tipo

O ADR-0034 rejeita ciclos na tabela de tipos. A proposta permite **ciclos que
passem por pelo menos um enum**:

- `enum List { Nil, Cons(int, List) }` é válido;
- `enum A { B(B) } enum B { A(A) }` é válido;
- `enum E { S(S) } struct S { e: E }` é válido;
- `struct S { s: S }` continua inválido (ciclo sem enum), preservando a regra
  atual.

Justificativa: `Value::Enum` guarda os payloads em `Vec<Value>`, então a
recursão é heap-backed; o enum fornece o ponto nominal/tag que quebra o ciclo de
tamanho. A verificação de ciclos deve usar DFS com estado (branco/cinza/preto) e
permitir a aresta de retorno somente se o ciclo contiver um nó de enum. A
travessia precisa ser limitada por `max_types` e não pode recursar sem
orçamento.

**Questão aberta:** permitir ciclos por qualquer indireção heap (array, struct,
option, result) em vez de somente enum. A proposta conservadora é enum-only.

## 3. Valor

```rust
pub enum Value {
    // ... existentes ...
    Enum {
        ty: TypeId,
        variant: EnumVariantId,
        fields: Vec<Value>,
    },
}
```

- `ty` precisa ser `TypeDef::Enum(_)`; `variant` é o índice da variante;
  `fields` está na ordem de declaração da variante.
- `Value::ty()` devolve `ty`; `Value::matches_type(ty)` exige igualdade exata.
- `Value::as_enum() -> Option<(EnumVariantId, &[Value])>`.
- `PartialEq` é estrutural: mesmo `ty`, mesma variante e campos iguais. `f64`
  segue a semântica existente (`NaN != NaN`).
- `Display` estável: `enum:<ty>:<variant>` para variante vazia;
  `enum:<ty>:<variant>[<campo>, ...]` quando há payload. Exemplos:
  - `enum:5:0` (variante vazia);
  - `enum:5:1[i64:2, i64:3]`.
- `retained_bytes` (ADR-0035): `SLOT_BYTES` do tag + `SLOT_BYTES * fields.len()`
  + soma recursiva dos filhos, saturando em `u64::MAX`. Variante vazia custa
  `SLOT_BYTES`.

### 3.1 Igualdade e ordenação

- `==`/`!=` são estruturais e exigem o **mesmo** `TypeId`.
- Não há ordenação (`<`, `>`) de enums.
- Não há coerção entre enums diferentes, mesmo com variantes iguais.

### 3.2 Recursão e profundidade

Valores recursivos podem ser profundos. As operações de igualdade,
`retained_bytes` e `Display` devem ser **depth-safe** (iterativas ou com pilha
explícita/limite) ou documentar explicitamente o risco de stack overflow. O
orçamento de memória não limita a profundidade sozinho; a proposta exige um
teste adversarial de lista profunda e uma decisão de implementação.

## 4. Operações

Adicionar a `Op`:

```rust
EnumNew {
    ty: TypeId,
    variant: EnumVariantId,
    fields: Vec<ValueId>,
},
EnumIsVariant {
    value: ValueId,
    variant: EnumVariantId,
},
EnumPayload {
    value: ValueId,
    variant: EnumVariantId,
    field: u32,
},
EnumEq { lhs: ValueId, rhs: ValueId },
EnumNe { lhs: ValueId, rhs: ValueId },
```

### 4.1 `EnumNew`

- Verificador: `ty` é `TypeDef::Enum`; `variant` em `0..variants.len()`;
  `fields.len()` e tipos casam exatamente com a variante, na ordem de declaração.
- Execução: constrói `Value::Enum`; pré-checa a memória do valor (tag + slots +
  filhos) e grava no slot com a cobrança normal (ADR-0035).
- Erro estático: `invalid_type` para tipo/variante/aridade/tipo de campo.

### 4.2 `EnumIsVariant`

- Verificador: `value` tem tipo `TypeDef::Enum`; `variant` em faixa.
- Execução: devolve `bool`; não copia o payload.
- Erro estático: `invalid_type`.

### 4.3 `EnumPayload`

- Verificador: `value` tem tipo enum; `variant`/`field` em faixa; o tipo do
  resultado é o `ty` do campo.
- Execução: se a variante do valor não for `variant`, trap
  `invalid_variant` (novo código, classe `trap`); se `field` for inválido,
  `invalid_module` (defesa contra valor corrompido; o verificador já rejeitou).
  Caso contrário, copia o campo para o slot de destino (cobrança normal).
- Erro estático: `invalid_type`; dinâmico: `invalid_variant`.

### 4.4 `EnumEq`/`EnumNe`

- Verificador: ambos os operandos têm o mesmo `TypeId` enum.
- Execução: igualdade estrutural; não trapam. `EnumNe` é a negação.
- Erro estático: `invalid_type` se os tipos diferirem.

### 4.5 `operand_ids` e `result_type`

- `operand_ids` inclui `fields` (EnumNew), `value` (IsVariant/Payload) e
  `lhs`/`rhs` (Eq/Ne).
- `Op::result_type`:
  - `EnumNew` → `Some(ty)`;
  - `EnumIsVariant`/`EnumEq`/`EnumNe` → `Some(TYPE_BOOL)`;
  - `EnumPayload` → `None` (o verificador resolve pelo campo).
- O verificador resolve `EnumPayload` em `resolve_op_type` e valida os demais
  operandos.

## 5. Erros

Adicionar ao contrato de erros:

| Código | Categoria | Classe | Situação |
|---|---|---|---|
| `invalid_variant` | trap | trap | `enum_payload` sobre variante diferente da solicitada |

`invalid_type` cobre variante fora de faixa, campo fora de faixa, aridade/tipo
divergente e igualdade entre enums diferentes. `invalid_module` cobre tabela de
enums inconsistente, `TypeDef::Enum` órfão/duplicado e ciclo sem enum.

## 6. Formato textual

### 6.1 Declaração

Forma canônica proposta:

```text
enum Shape {
  Circle(number),
  Rect(number, number),
  Empty,
  Named { radius: number, label: string },
}

type 5 = enum Shape
```

- `enum` declara as variantes; `type <id> = enum <Name>` registra o tipo na
  tabela densa (como `type` já faz para `option`/`result`).
- A ordem canônica de impressão é: `module`, `entry`, declarações `enum`,
  declarações `type`, declarações `struct`, `import`, funções. O parser pode
  aceitar referências para frente, mas a forma impressa é fixa.
- Variantes vazias, posicionais e nomeadas são mutuamente exclusivas; vírgula
  final é opcional. Sem discriminantes explícitos.

### 6.2 Operações

```text
enum_new Shape.Circle, v0, v1
enum_is_variant v0, Shape.Circle
enum_payload v0, Shape.Circle, 0
enum_eq v0, v1
enum_ne v0, v1
```

- `enum_new` usa o nome qualificado da variante e os valores na **ordem de
  declaração** dos campos (campos nomeados são reordenados pelo frontend).
- `enum_is_variant`/`enum_payload` usam `Enum.Variant`; o campo é o índice
  posicional (para variantes nomeadas, a ordem de declaração).
- O printer emite nomes qualificados e índices; o parser resolve para
  `EnumId`/`EnumVariantId`.

## 7. Bytecode

### 7.1 Tipo

Adicionar `TAG_ENUM = 9` e, após `type_def`:

```text
type_def:
  tag u8 = 9
  enum_id u32
```

Adicionar após a tabela de structs:

```text
enum_count u16
enums[]:
  id u32
  name string
  variant_count u16
  variants[]:
    name string
    named u8            (0 = posicional, 1 = nomeado)
    field_count u16
    fields[]:
      [name string se named = 1]
      ty u32
```

Contagens validadas contra `Limits` e bytes restantes antes de alocar. Tag
desconhecida continua `unknown_type`; enum_id fora da tabela é
`invalid_module` após a checagem estrutural.

### 7.2 Opcodes

| Código | Operação | Payload |
|---:|---|---|
| 55 | `enum_new` | `u32 ty`, `u32 variant`, `u16 argc`, `u32 args[argc]` |
| 56 | `enum_is_variant` | `u32 value`, `u32 variant` |
| 57 | `enum_payload` | `u32 value`, `u32 variant`, `u32 field` |
| 58 | `enum_eq` | `u32 lhs`, `u32 rhs` |
| 59 | `enum_ne` | `u32 lhs`, `u32 rhs` |

Códigos 55–59 são novos; decoders antigos os rejeitam com `unknown_opcode`
(falha fechada). A proposta **não** altera `IR_VERSION` nem `FORMAT_VERSION`:
a mudança é aditiva e o formato continua experimental. Se o mantenedor preferir
versionar, a alternativa é `IR_VERSION = 2` com migração do corpus.

## 8. Limites

Adicionar a `Limits`:

- `max_enums` (proposta inicial: 256);
- `max_variants_per_enum` (proposta: 256);
- `max_fields_per_enum_variant` (proposta: 256).

A soma total de variantes/campos é limitada indiretamente por
`max_input_bytes` e `max_types`; se a medição mostrar risco de memória, adicionar
um teto agregado (`max_enum_variants_total`, `max_enum_fields_total`). Toda
contagem é checada antes de alocar.

## 9. Verificação

Ordem proposta (após a checagem estrutural existente):

1. IDs densos, nomes únicos, variantes não vazias, campos homogêneos.
2. `TypeDef::Enum` ↔ `EnumDef` exatamente um-para-um.
3. Tipos de campo válidos; `EnumPayload`/`EnumNew` com índices em faixa.
4. Operandos definidos e com tipo correto; igualdade exige mesmo enum.
5. Ciclos: permitir ciclo com enum; rejeitar ciclo sem enum.
6. `EnumPayload` resolve o tipo de retorno pelo campo.

A verificação não prova exaustividade de `match`; isso é responsabilidade do
frontend (Lift já exige todas as variantes). O RIR não tem `match` próprio.

## 10. Execução

- `EnumNew`/`EnumPayload` cobram memória na gravação do slot; `EnumPayload`
  copia o campo e o `write_current` cobra os bytes retidos.
- `EnumIsVariant`/`EnumEq`/`EnumNe` não alocam (exceto clones temporários já
  existentes no modelo).
- `EnumPayload` sobre variante errada: trap `invalid_variant`, antes de copiar
  o payload.
- `EnumEq`/`EnumNe` comparam recursivamente; valores profundos exigem a decisão
  de implementação depth-safe (§3.2).
- Fuel: cada op continua custando 1 unidade; não há desconto.
- Não há efeito de host; enums são valores puros do convidado.

## 11. Mapeamento Lift

| Lift/HIR | RIR proposto |
|---|---|
| `HirDeclaration::Enum(HirEnum)` | `EnumDef` + `TypeDef::Enum` |
| `HirEnumVariant.fields` posicionais | `EnumFieldDef { name: None, ty }` |
| `HirEnumVariant.named_fields` | `EnumFieldDef { name: Some, ty }` |
| `HirEnumVariant.field_boxed` | ignorado (payload em `Vec<Value>`) |
| `HirExprKind::EnumConstruct` | `Op::EnumNew` (índice da variante; campos na ordem) |
| `HirMatchPattern::EnumVariant` | `EnumIsVariant` + `EnumPayload` |
| `type A = B | C` (union) | enum com variantes `B(B)`, `C(C)` (desugaring no frontend) |
| enums em array/struct/option/result | `TypeId` normal; sem caso especial |
| enums recursivos | permitidos pela regra de ciclo com enum |
| FFI/host | fora do escopo |

### 11.1 Lowering de `match`

1. Avaliar o scrutinee uma vez em um valor temporário.
2. Para cada variante, exceto a última: `EnumIsVariant` + `cond_br` para o braço.
3. Último braço: sem teste, direto (exaustividade garantida pelo Lift).
4. Em cada braço, para cada binding: `EnumPayload` com variante/campo; se a
   variante não casar, o trap `invalid_variant` protege o caso de um valor
   inesperado.
5. O braço executa o corpo e converge para o merge normal; se o corpo termina em
   `return`, o fluxo já divergiu.
6. O adaptador deve validar defensivamente que cada variante aparece exatamente
   uma vez; caso contrário, `Unsupported` (não confiar apenas no frontend).

## 12. Plano de testes

### 12.1 Corpus RIR

Casos válidos propostos (numeração a definir após o próximo C):

- enum vazio (`enum E { A, B }`);
- payload posicional (`Circle(f64)`);
- campos nomeados;
- enum recursivo (`List`/`Cons`);
- enums mutuamente recursivos;
- enum em array, struct, option e result;
- igualdade estrutural (`eq`/`ne`, inclusive variantes diferentes);
- construção + payload + match exaustivo;
- union desugerada (via adaptador);
- round-trip texto ↔ bytecode, incluindo recursão.

Casos inválidos propostos:

- enum sem variantes;
- variante duplicada;
- campos mistos posicionais/nomeados;
- campo nomeado duplicado;
- `TypeDef::Enum` órfão/duplicado;
- variante/campo fora de faixa;
- payload com aridade/tipo errado;
- igualdade entre enums diferentes;
- ciclo sem enum (deve continuar inválido);
- `enum_payload` sobre variante errada em execução (`invalid_variant`).

### 12.2 Adaptador Lift

- `HirEnum` posicional, nomeado, vazio;
- recursivo e mutuamente recursivo;
- `EnumConstruct` + `match` exaustivo;
- unions Lift;
- enums em arrays/structs/options;
- erro para pattern não exaustivo/duplicado (defensivo);
- teste diferencial com oráculo Lift e execução na RVM;
- `lift-encode` → `rsuvm run-bc` para os casos.

### 12.3 Fuzzing

Adicionar entradas de enum ao alvo `verificador`/`decoder` ou um alvo dedicado;
contrato: nenhum pânico; erros são `RvmError`. Não alegar ausência de bugs.

## 13. Critérios de aceitação

- `cargo test --workspace` verde com os novos casos.
- Corpus válido/inválido cobre todos os novos códigos e operações.
- Bytecode round-trip e decoder hostil rejeitam contagens/tags/índices
  inválidos antes de alocar.
- Adaptador Lift executa enums/unions e `lift-encode` produz `.rsvmbc`
  consumível por `rsuvm run-bc`.
- `clippy -D warnings`, `fmt --check`, MSRV e docs verdes.
- Nenhuma alegação de estabilidade de formato ou de segurança multi-tenant.

## 14. Impacto de implementação

| Crate/área | Mudança |
|---|---|
| `rvm-spec` | `TypeDef::Enum(EnumId)`, `EnumId`, `invalid_variant`, novos `Limits` |
| `rvm-ir` | `Module.enums`, `EnumDef`/`EnumVariantDef`/`EnumFieldDef`, `Op::{EnumNew, EnumIsVariant, EnumPayload, EnumEq, EnumNe}`, checagem estrutural e fixture |
| `rvm-runtime` | `Value::Enum`, `as_enum`, `Display`, `retained_bytes` |
| `rvm-verify` | tabela de enums, tipos das ops, igualdade nominal, travessia de ciclos |
| `rvm-interpreter` | dispatch, construção, inspeção, trap `invalid_variant`, memória |
| `rvm-text` | declaração `enum`, `type = enum`, impressão e operações qualificadas |
| `rvm-bytecode` | `TAG_ENUM`, tabela de enums, opcodes 55–59, contagens/limites |
| `rvm-link` | fundir/remap `EnumId`, `TypeDef::Enum`, campos e ops; hoje o linker já é restrito a módulos sem tipos compostos — enums multi-módulo exigem resolver essa lacuna |
| `rvm-conformance` | casos C/N, expectativas e cobertura de ops |
| `rvm-cli` | sem mudança própria; `encode`/`run-bc` passam a transportar enums |
| `frontends/lift-rir` | `HirEnum`, `EnumConstruct`, `Match`, unions e testes |
| `docs` | contratos RIR/texto/bytecode/erros/corpus e este ADR |

O linker é o ponto mais delicado: `TypeDef::Enum(EnumId)` e os índices de
variante precisam ser remapeados junto com `TypeId`/`StructId`, e a identidade
nominal não pode colidir entre módulos. A proposta inicial pode exigir que
enums sejam usados apenas em módulos já ligados/planos (como o adaptador Lift,
que produz um módulo único), deixando a ligação multi-módulo com enums para um
incremento separado.

## 15. Questões abertas

1. Ciclos: permitir só com enum ou qualquer indireção heap?
2. `invalid_variant` novo ou reutilizar `unwrap_error`?
3. `IR_VERSION` aditivo (1) ou bump para 2?
4. Limites agregados para variantes/campos?
5. Formato textual de `type`/`enum` e ordem canônica.
6. `Display`/igualdade depth-safe: iterativo ou limite explícito?
7. Suportar `match` de `Option`/`Result` junto ou apenas enums neste incremento?
8. Enums em módulos ligados por `rvm-link` neste incremento ou apenas em módulos
   planos (single-module), adiando o remapeamento de `EnumId`?
