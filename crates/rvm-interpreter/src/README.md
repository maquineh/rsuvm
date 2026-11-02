# rvm-interpreter — Package Contract

## Overview

`rvm-interpreter` é o executor de referência do EXP-0: interpreta um
`VerifiedModule` de forma single-thread, determinística e sem efeitos de host.
Aplica fuel e profundidade de forma observável, produz traps semânticos e
carrega localização nos diagnósticos. Executa a RIR diretamente, sem JIT/AOT
(ADR-0003) e sem tocar o sistema operacional.

## Interface

- `execute(module: &VerifiedModule, entry: FnId, args: &[Value], config: &ExecConfig)
  -> Result<RunOutcome, RvmError>` (`lib.rs:execute`).
- `RunOutcome` — `value: Value`, `fuel_used: u64`, `max_depth: u32`,
  `memory_peak: u64` (`lib.rs:RunOutcome`).

## Invariants

- Só aceita `VerifiedModule`; não existe caminho de execução para `Module` cru.
  Toda leitura de slot usa acesso verificado (`get`), sem indexação direta.
- A chamada do host é validada **antes** do primeiro débito de fuel: função de
  entrada inexistente é `invalid_module`; aridade ou tipo de argumento divergente
  é `invalid_type` (`lib.rs:validate_host_call`).
- Fuel: cada instrução despachada custa 1 e cada terminador despachado custa 1; o
  débito acontece **antes** do despacho, então saldo 0 produz `out_of_fuel` sem
  executar a operação. Chamadas não resetam o saldo; `fuel_used` é
  `inicial - restante`.
- Profundidade: frame de entrada é 1; entrar no callee quando `depth + 1` excede
  `config.max_call_depth()` produz `call_depth_exceeded`; `max_depth` registra a
  maior profundidade observada.
- A execução usa uma **pilha de frames explícita na heap** (`Vec<Frame>`), sem
  recursão na linguagem hospedeira; a versão recursiva anterior estourou a pilha
  nativa com 256 frames (ADR-0012).
- `add/sub/mul_i64_checked` usam `rvm_spec::BinOp::eval_i64`; `None` vira trap
  `integer_overflow`. Comparações usam `CmpOp::eval_i64` e produzem `bool`.
- `br`/`cond_br` copiam os argumentos para os slots dos parâmetros do bloco alvo
  antes do salto (phi explícito); `return` sem valor produz `Value::Unit`.
- Erros de execução carregam `Location` de função, bloco ou instrução; trap de
  overflow é `ErrorClass::Trap`; fuel e profundidade são
  `ErrorClass::ResourceLimit`.
- Determinismo: nenhum `HashMap`, tempo, ambiente ou estado compartilhado entre
  execuções; `execute` cria todo o estado localmente. Reexecutar o mesmo módulo
  produz o mesmo valor e o mesmo fuel.
- Alocação de frame usa `try_reserve_exact` e falha como `invalid_module` (não
  aborta) quando não há memória.

- Memória: antes de alocar os slots de um frame, o intérprete cobra
  `frame_cost(value_count)`; a conta é de memória viva e o custo é liberado no
  retorno. Exceder `config.memory_limit()` produz `out_of_memory` com localização
  da função, antes de qualquer alocação; `memory_peak` registra o pico. O modelo não
  mede RSS (ADR-0013).

## Side Effects

- `None`. Não há arquivo, rede, ambiente, relógio, processo ou capability. O
  intérprete só consome CPU e memória do host, limitado por fuel e profundidade;
  a memória total de frames não é contabilizada em bytes contra quota explícita
  (lacuna registrada no relatório de implementação).

## Verification

- Run: `cargo test --workspace`
