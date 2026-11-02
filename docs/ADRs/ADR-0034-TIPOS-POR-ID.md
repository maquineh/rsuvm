# ADR-0034 — Representação de tipos por id (tabela no módulo)

- **Status:** **Proposto** — decisão de design registrada; **não implementada**.
- **Data:** 2026-11-02
- **Requisitos:** destravar `Option`/`Result` de agregados e aninhados; manter
  determinismo, ausência de `unsafe` e zero dependências externas.

## Contexto

Hoje [`rvm_spec::Type`] é um `enum` **`Copy`** e fechado:

```
Type = unit | bool | i64 | string | f64 | array(ArrayElem)
     | struct(StructId) | option(OptionalElem) | result(OptionalElem, OptionalElem)
```

`ArrayElem`/`OptionalElem`/`ScalarKind` são escalares. Isso mantém `Type: Copy` e
tornou arrays, structs, opcionais e `Result` viáveis sem refatoramento amplo, mas
**impede** payloads agregados e aninhados (`Option<Option<T>>`, `int[]?`,
`int?[]`, `Result<User, Erro>`, campos de struct opcionais). A barreira é
estrutural: um tipo recursivo por valor exige indireção (`Box`), e `Box` remove
`Copy` de `Type`, que é passado por valor em toda a RIR.

## Decisão proposta

1. Introduzir `TypeId = u32` e uma **tabela de tipos** no módulo:
   `Module { ..., types: Vec<TypeDef>, ... }`, com IDs densos (`types[i].id == i`).
2. `TypeDef` é recursivo **por id**, não por `Box`:

   ```text
   TypeDef = unit | bool | i64 | string | f64
           | array(TypeId)
           | struct(StructId)
           | optional(TypeId)
           | result(TypeId, TypeId)
   ```

3. Parâmetros, retorno, parâmetros de bloco, campos de struct e operandos de
   operação passam a carregar `TypeId` (não `Type`). `Type` deixa de existir como
   enum público de valor; sobra o alias `TypeId`.
4. **Validação estrutural** (`rvm_ir::structure`): IDs densos; todo `TypeId`
   referenciado existe; o grafo de tipos é **acíclico** (uma definição não pode
   conter a si mesma, direta ou indiretamente); campos de struct precisam ser
   "sized" (sem recursão infinita). `option`/`result`/`array` podem referenciar
   qualquer tipo válido, inclusive agregados e outros opcionais/resultados.
5. **Runtime**: `Value` carrega `TypeId` onde hoje carrega `StructId`/
   `OptionalElem`/`ScalarKind`; a tabela do módulo resolve layout e nomes.
6. **Texto**: `tipo` vira `unit|bool|i64|string|f64|array tipo|struct Nome|
   option tipo|result tipo tipo`. A tabela é reconstruída no parse (a ordem de
   declaração define os IDs) ou emitida explicitamente.
7. **Bytecode**: tabela de tipos após `entry` (contagem + definições); tags de
   tipo passam a ser `u32 TypeId`. `struct` continua com `StructId`.
8. **Migração em fases** (cada uma com gates verdes):
   1. adicionar `types`/`TypeId` **ao lado** de `Type` e converter só o
      `rvm-ir`/`rvm-verify`;
   2. converter `rvm-runtime`/`rvm-interpreter`;
   3. converter `rvm-text`/`rvm-bytecode` (round-trip);
   4. converter o adaptador Lift→RIR;
   5. remover o enum `Type` antigo e os aliases escalares.
9. **Critério de validação** (quando implementado):
   - corpus com `option<option<i64>>`, `result<i64, string>`, `int[]?`,
     `int?[]`, `Result<User, Erro>` e campo de struct opcional;
   - `encode → decode → verify → execute` idêntico ao caminho textual;
   - determinismo (mesma tabela ⇒ mesmos bytes e mesmo resultado);
   - `cargo test --workspace`, `clippy -D warnings` e `fmt --check` verdes;
   - nenhuma dependência externa, nenhum `unsafe`.

## Alternativas

1. **`Type::Optional(Box<Type>)` (recursivo direto):** elimina `Copy` de `Type`
   (usado por valor em `Op::result_type`, `Param`, `BlockParam`, `Function::ret`,
   `Function::value_type`), com refatoramento semelhante ao da tabela, porém sem
   IDs estáveis para bytecode/serialização. Preterida.
2. **Manter o enum `Copy` e adicionar variantes agregadas com `Box` só onde
   necessário:** híbrido confuso; `Type` continuaria não-`Copy`. Preterida.
3. **Tipos por nome (`String`) ou internados (`&'static`):** não-`Copy`, exigem
   arena global e quebram determinismo/serialização. Rejeitada.
4. **Não fazer nada:** mantém o núcleo limitado a payloads escalares; rejeitada
   pelo pedido do mantenedor.

## Consequências

- Positivas: tipos verdadeiramente recursivos sem `Box` no valor; `TypeId: Copy`
  preserva a ergonomia; IDs densos são determinísticos e serializáveis; destrava
  `Option`/`Result` de agregados e aninhados.
- Negativas: mudança grande e transversal (todos os crates e o bytecode);
  validação de ciclos nova; risco de regressão exige migração faseada; o adaptador
  precisa mapear `ResolvedType` recursivo para `TypeId`.
- Residual (até implementar): `Option`/`Result` de agregados e aninhados
  continuam indisponíveis; a ADR-0033 e a ADR-0034 ficam em aberto.

## Observação de escopo

Esta ADR **não** altera código nesta rodada: o repositório permanece no enum
`Type: Copy` e nos limites das ADR-0027/0029/0030/0033. A implementação exige a
migração faseada acima e uma nova rodada de revisão.
