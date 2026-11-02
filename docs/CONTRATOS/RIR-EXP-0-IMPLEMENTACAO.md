# Contrato de implementação da RIR — EXP-0

**Estado:** proposta experimental implementada no workspace; o formato não é
estável (ADR-0002, ADR-0004). Este documento é a especificação que o verificador,
o intérprete e o corpus devem cumprir.

## 1. Modelo

~~~text
Module    { name, version = 1, entry: FnId, functions: Vec<Function> }
Function  { id = índice, name, params: Vec<Param>, ret: Type, entry: BlockId,
            blocks: Vec<Block> }
Param     { name, ty }                       // ValueId implícito = índice
Block     { id = índice, label, params: Vec<BlockParam>, instrs, term }
BlockParam{ value: ValueId, name, ty }
Instr     { def: ValueId, op: Op }
Op        = const_i64 | const_bool | bin(op,lhs,rhs) | cmp(op,lhs,rhs) | call(callee,args)
Term      = return [v] | br(target,args) | cond_br(cond, target, target)
~~~

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

Limites padrão: 64 funções, 256 blocos/função, 16 parâmetros, 8 parâmetros por
bloco, 1024 instruções/bloco, 4096 valores/função, 16 argumentos por
chamada/desvio, 64 bytes de nome, 1 MiB de entrada.

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
   | `add/sub/mul_i64_checked` | `i64`, `i64` | `i64` |
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
  `None` para `integer_overflow`; `cmp` usa `CmpOp::eval_i64`; `call` avalia
  argumentos, verifica profundidade, entra no frame do callee e grava o retorno no
  slot de `def`.
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
closures, concorrência, async, `f64`, strings, arrays, registros, referências,
`unsafe`, FFI e capabilidades concedidas.
