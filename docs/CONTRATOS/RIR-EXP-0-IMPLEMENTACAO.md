# Contrato de implementação da RIR — EXP-0

**Estado:** proposta experimental implementada no workspace; o formato não é
estável (ADR-0002, ADR-0004). Este documento é a especificação que o verificador,
o intérprete e o corpus devem cumprir.

## 1. Modelo

~~~text
Module    { name, version = 1, entry: FnId, types: Vec<TypeDef>, structs: Vec<StructDef>,
            functions: Vec<Function> }
TypeDef   = unit | bool | i64 | string | f64
          | array(TypeId) | struct(StructId) | option(TypeId) | result(TypeId, TypeId)
StructDef { id = índice, name, fields: Vec<FieldDef> }
FieldDef  { name, ty: TypeId }
Function  { id = índice, name, params: Vec<Param>, ret: TypeId, entry: BlockId,
            blocks: Vec<Block> }
Param     { name, ty: TypeId }               // ValueId implícito = índice
Block     { id = índice, label, params: Vec<BlockParam>, instrs, term }
BlockParam{ value: ValueId, name, ty: TypeId }
Instr     { def: ValueId, op: Op }
Op        = const_i64 | const_bool | const_str | const_f64 | bin(op,lhs,rhs) | cmp(op,lhs,rhs)
          | str_eq(lhs,rhs) | str_ne(lhs,rhs) | str_len(operand) | concat_str(lhs,rhs)
          | float(op,lhs,rhs) | array(ty,items) | array_len(v) | array_get(v,i)
          | array_set(v,i,x)
          | struct_new(sid, fields) | field_get(v, field) | field_set(v, field, x)
          | option_none(ty) | option_some(ty,v) | option_is_some(v) | option_unwrap(v)
          | coalesce(o,f) | option_eq(lhs,rhs) | option_ne(lhs,rhs)
          | result_ok(ty,v) | result_err(ty,v) | result_is_ok(v)
          | result_unwrap(v) | result_unwrap_err(v)
          | call(callee,args) | host_call(function,args)
Term      = return [v] | br(target,args) | cond_br(cond, target, target)
~~~

### Tabela de tipos (ADR-0034)

Todo módulo começa a tabela com os cinco tipos embutidos canônicos, nesta ordem:
`0 = unit`, `1 = bool`, `2 = i64`, `3 = string`, `4 = f64`. Tipos compostos
ocupam ids densos a partir de `5` e são **recursivos por id**: `array(T)`,
`option(T)` e `result(T, E)` referenciam outros `TypeId`; `struct(S)` referencia a
tabela de structs. O grafo de tipos precisa ser **acíclico** (inclusive campos de
struct).

### Declarações externas e ligação estática (ADR-0018)

Uma `Function` com `blocks` vazio é uma **declaração externa**
(`Function::is_external`): nomeia uma função a ser fornecida por outro módulo. O
formato textual a cria com `import <nome>`. Declarações externas são aceitas por
`structure::check`, mas **não** podem ser a função de entrada, e
`VerifiedModule::verify` rejeita módulos que ainda as contenham. A ligação é feita
pelo crate `rvm-link`, que resolve cada externa para a função concreta de mesmo
nome, reescreve os `call_static` para IDs globais e produz um `Module` plano sem
externas.

## 2. Invariantes estruturais (`rvm_ir::structure::check`)

Aplicadas antes de qualquer verificação de tipos e antes de alocar memória
proporcional ao módulo:

1. `version == 1` (senão `unsupported_version`).
2. `module.name`, nomes de função, rótulos de bloco e nomes de parâmetro são
   identificadores ASCII `[A-Za-z_][A-Za-z0-9_]*`, não vazios, com até
   `max_name_len` bytes.
3. Nomes de função únicos no módulo; rótulos únicos na função; nomes de
   parâmetro (de função e de bloco) únicos na função.
4. Nomes `v<N>` (prefixo `v` + somente dígitos) são reservados para a impressão de
   definições e não podem ser usados como parâmetro.
5. `functions[i].id == i`, `blocks[i].id == i` (IDs densos) e `entry`/`entry`
   de bloco dentro da faixa.
6. Bloco de entrada não declara parâmetros.
7. Todo `ValueId` de parâmetro, parâmetro de bloco e definição de instrução é
   único e o conjunto é exatamente `{0..N-1}`; parâmetros de função ocupam
   `0..params.len()`.
8. Todo operando referencia valor existente (`invalid_module`).
9. Todo alvo de desvio existe na função (`invalid_control_flow`).
10. Toda função chamada existe no módulo (`invalid_module`).
11. Contagens respeitam [`rvm_spec::Limits`], e a soma de valores usa aritmética
    verificada; a alocação do vetor de presença ocorre **depois** da checagem do
    limite.
12. A tabela de tipos é densa e começa com `unit`, `bool`, `i64`, `string`, `f64`
    (`0..=4`); todo `TypeId` referenciado existe; o grafo de tipos é **acíclico**
    (campos de struct inclusos); `module.types.len() <= max_types` (ADR-0034).

Limites padrão: 64 funções, 256 blocos/função, 16 parâmetros, 8 parâmetros por
bloco, 1024 instruções/bloco, 4096 valores/função, 16 argumentos por
chamada/desvio, 64 bytes de nome, 4096 bytes por literal `string` (ADR-0024),
1 MiB de entrada.

## 3. Verificação de tipos e fluxo (`rvm-verify::VerifiedModule::verify`)

Depois de `structure::check`:

1. **CFG e alcançabilidade.** Sucessores vêm de `br`/`cond_br`. Blocos
   inalcançáveis a partir do bloco de entrada são rejeitados com
   `invalid_control_flow` (o EXP-0 não define semântica para código morto).
2. **Dominância.** Calculada sobre o CFG (algoritmo iterativo de conjuntos de
   dominadores; `dom[entry] = {entry}`, `dom[b] = {b} ∪ ∩ dom[pred]`).
3. **Definição antes do uso.** Um valor usado em um ponto precisa ter definição
   que domine esse ponto; dentro do mesmo bloco, a definição precisa vir antes
   (índices de instrução) — caso contrário `invalid_control_flow`.
   - Parâmetros de bloco são definidos na entrada do próprio bloco.
   - Argumentos de `br`/`cond_br` são usados no terminador (fim do bloco) e
     precisam dominar o terminador.
4. **Tipos por operação:**
   | Operação | Operandos | Resultado |
   |---|---|---|
   | `const_i64` | — | `i64` |
   | `const_bool` | — | `bool` |
   | `add/sub/mul/div/rem_i64_checked` e `shl/shr_i64_checked`, `and/or/xor_i64` | `i64`, `i64` | `i64` |
   | `const_str` | — | `string` |
   | `str_eq`, `str_ne` | `string`, `string` | `bool` |
   | `str_len` | `string` | `i64` |
   | `concat_str` | `string`, `string` | `string` |
   | `const_f64` | — | `f64` |
   | `add_f64`/`sub_f64`/`mul_f64`/`div_f64` | `f64`, `f64` | `f64` |
   | `eq_f64`/`ne_f64`/`lt_f64`/`le_f64`/`gt_f64`/`ge_f64` | `f64`, `f64` | `bool` |
   | `array` | `1..=max_array_len` valores do mesmo tipo `T` | `array(T)` (no op) |
   | `array_len` | `array(T)` | `i64` |
   | `array_get` | `array(T)`, `i64` | `T` |
   | `array_set` | `array(T)`, `i64`, `T` | `array(T)` |
   | `struct_new` | um valor por campo, no tipo do campo | `struct(S)` |
   | `field_get` | `struct(S)`, nome de campo existente | tipo do campo |
   | `field_set` | `struct(S)`, campo existente, valor do tipo do campo | `struct(S)` |
   | `option_none` | — | `option(T)` (no op) |
   | `option_some` | `T` | `option(T)` (no op) |
   | `option_is_some` | `option(T)` | `bool` |
   | `option_unwrap` | `option(T)` | `T` |
   | `coalesce` | `option(T)`, fallback `T` ou `option(T)` | tipo do fallback |
   | `option_eq`/`option_ne` | `option(T)`, `option(T)` | `bool` |
   | `result_ok`/`result_err` | valor no tipo da variante | `result(T, E)` (no op) |
   | `result_is_ok` | `result(T, E)` | `bool` |
   | `result_unwrap`/`result_unwrap_err` | `result(T, E)` | `T`/`E` |
   | `host_read_file` | `string`, `i64` (ABI fechada, ADR-0036) | `result string string` |
   | comparações `*_i64` | `i64`, `i64` | `bool` |
   | `call_static` | casam com `params` do alvo | `ret` do alvo |
   Qualquer divergência ⇒ `invalid_type`.
5. **Chamada:** aridade e tipos dos argumentos casam com a assinatura; erro de
   aridade ou tipo ⇒ `invalid_type`. Chamadas são estáticas: `callee` é um
   `FnId` já validado por `structure::check`.
6. **Terminadores:**
   - `return` (sem valor) só em função `-> unit`;
   - `return v` exige tipo de `v` igual a `ret` e `ret != unit`;
   - `br`/`cond_br` exigem aridade e tipos exatamente iguais aos `params` do bloco
     alvo;
   - `cond_br.cond` precisa ser `bool`.
   Violações ⇒ `invalid_type`.
7. **Ponto de entrada:** `module.entry` precisa existir (estrutural) e pode ter
   parâmetros; os argumentos da chamada do host são checados em tempo de execução
   (`invalid_type`).

Ordem de checagem: funções por índice, blocos por índice, instruções por índice,
terminador por último. O primeiro erro vence.

## 4. Semântica de execução

- Estado: um frame por chamada com `Vec<Value>` do tamanho de
  `Function::value_count()`; slots não inicializados são erro interno
  (`invalid_module`), nunca pânico.
- `const_*` grava o valor; `bin` aplica `rvm_spec::BinOp::eval_i64` e mapeia
  `None` para `integer_overflow`, exceto divisão/resto por zero
  (`division_by_zero`, ADR-0022) e deslocamento fora de `0..=63` (`invalid_shift`,
  ADR-0023); `cmp` usa `CmpOp::eval_i64`; `call` avalia
  argumentos, verifica profundidade, entra no frame do callee e grava o retorno no
  slot de `def`.
- `const_str`/`str_eq`/`str_ne`/`str_len` operam sobre `string`; `concat_str`
  retém os bytes do resultado na quota e falha com `out_of_memory` (quota) ou
  `string_too_long` (acima de `ExecConfig::max_string_bytes`, ADR-0025); a
  liberação ocorre ao sobrescrever o slot ou no retorno do frame (ADR-0035).
- `const_f64`/`Float` operam sobre `f64` com **IEEE-754 sem trap** (divisão por zero
  → `inf`/`NaN`; comparações com `NaN` são falsas exceto `!=`, ADR-0026).
- `array` cria um array imutável (não-vazio, homogêneo) do tipo declarado no op e
  retém `len × 24` bytes mais o custo dos filhos na quota; a cobrança é liberada
  quando o slot é sobrescrito ou o frame retorna (ADR-0035); `array_len` devolve o
  comprimento; `array_get` faz **bounds check** e produz `index_out_of_bounds` fora
  de `0..len` (ADR-0027).
- `array_set` é uma **atualização funcional**: devolve um array **novo** com o
  elemento substituído e **não altera** o array de origem (não há referências nem
  aliasing no núcleo, ADR-0028). Exige `value` no tipo do elemento e faz bounds
  check (`index_out_of_bounds`); o array novo retém os mesmos bytes e a liberação
  ocorre ao sobrescrever o slot (ADR-0035).
- `struct_new` cria um valor de struct (campos de qualquer `TypeId`, ADR-0029/0034)
  e retém `campos × 24` + filhos; `field_get` lê um campo pelo nome (o campo
  precisa existir e o objeto ser struct); `field_set` é uma **atualização
  funcional** que devolve uma struct **nova** com o campo substituído, sem alterar
  a de origem (ADR-0032). A memória é liberada ao sobrescrever o slot/retornar
  (ADR-0035).
- `option_none`/`option_some` criam `none`/`some` do tipo `option(T)` declarado no
  op (ADR-0030/0034); `option_is_some` testa; `option_unwrap` extrai o payload e
  produz o trap `unwrap_none` sobre `none`; `coalesce` devolve o payload se `some`,
  senão o fallback. O fallback **preguiçoso** do `??` do Lift é obtido pelo
  adaptador via fluxo de controle, não pelo op.
- `option_eq`/`option_ne` comparam dois `option` de mesmo payload por valor
  (`none == none` é `true`; `some` compara payloads; `NaN != NaN`, ADR-0031).
- `result_ok`/`result_err` constroem um `result` do tipo declarado no op
  (ADR-0033/0034); `result_is_ok` testa; `result_unwrap`/`result_unwrap_err`
  extraem o payload e produzem o trap `unwrap_error` sobre a variante errada.
- `host_read_file` é a única chamada de host do EXP-0 (ADR-0036): exige a
  capability `filesystem_read` para o escopo do argumento `path` **antes** de
  chamar o backend; sem concessão ⇒ `capability_denied`. O host devolve
  `result string string` e o tipo do valor é conferido pelo intérprete
  (`invalid_type` se divergir). A string devolvida entra na quota de memória ao
  ser gravada (ADR-0035).
- `br`/`cond_br` copiam os argumentos para os slots dos parâmetros do bloco alvo
  antes de saltar (phi explícito).
- `return` devolve `Value::Unit` quando não há valor.

### Fuel

~~~text
antes_de_cada_instrucao:      se fuel == 0 { out_of_fuel } senão fuel -= 1
antes_de_cada_terminador:     se fuel == 0 { out_of_fuel } senão fuel -= 1
~~~

`fuel_used = fuel_inicial - fuel_restante`. Chamadas **não** resetam fuel.
Programas triviais: `constante(42)` consome 2 (1 instrução + 1 terminador);
`soma` chamada por `main` consome 6 no total (4 no chamador + 2 no chamado).

### Profundidade

~~~text
frame de entrada: depth = 1
antes_de_entrar_no_callee: se depth + 1 > max_call_depth { call_depth_exceeded }
~~~

### Memória (ADR-0013, ADR-0016)

~~~text
custo_do_frame  = 256 + slots_do_frame x 24 bytes
custo_do_buffer = args x 24 bytes (buffer transitório de chamada/desvio)
cobranca        = antes de alocar; conta memória VIVA, não cumulativa
liberacao       = no retorno do frame / ao fim do desvio
limite_padrao   = 16 MiB (MemoryLimit); 0 e aceito e nada cabe
~~~

`slots_do_frame` e `Function::value_count()` (parâmetros + parâmetros de bloco +
definições). Estouro produz `out_of_memory` com `Location::Function`. O modelo é
determinístico e **não** mede RSS; buffers transitórios de argumentos de
chamada/desvio entram na conta enquanto existem (ADR-0016).

`max_depth` no resultado é a maior profundidade observada. O teto
`CallDepthLimit::HARD_MAX = 256` é aplicado sobre uma pilha de frames **na heap**
(sem recursão na linguagem hospedeira). A versão recursiva anterior estourou a
pilha nativa com 256 frames; ADR-0012 registra o resultado e a decisão.

## 5. Determinismo

- Nenhuma dependência de `HashMap`, tempo, ambiente, ordem de sistema de arquivos
  ou `cfg!(debug_assertions)`.
- Mesmo módulo + limites + configuração + argumentos ⇒ mesmo valor ou mesmo
  código de erro, sempre.
- Um `VerifiedModule` pode ser executado repetidamente sem compartilhar estado.

## 6. Fora do EXP-0

Formato wire estável, imports, heap de objetos, GC, exceções recuperáveis,
closures, concorrência, async, `f64`, arrays, registros, referências,
concatenação/slicing de `string` (ADR-0024), `unsafe`, FFI e capabilidades
concedidas.
