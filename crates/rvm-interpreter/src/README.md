# rvm-interpreter — Package Contract

## Overview

`rvm-interpreter` é o executor de referência do EXP-0: interpreta um
`VerifiedModule` de forma single-thread e determinística no caminho puro. Aplica
fuel e profundidade de forma observável, produz traps semânticos e carrega
localização nos diagnósticos. Executa a RIR diretamente, sem JIT/AOT (ADR-0003).
Efeitos de host só existem quando o chamador fornece um `Host` explícito a
`execute_with_host`; `execute` usa `NoHost` e nega tudo (ADR-0036).

## Interface

- `execute(module: &VerifiedModule, entry: FnId, args: &[Value], config: &ExecConfig)
  -> Result<RunOutcome, RvmError>` (`lib.rs:execute`); usa `NoHost`.
- `execute_with_host(module, entry, args, config, host: &mut dyn Host)`
  (`lib.rs:execute_with_host`): consulta o host apenas em `host_read_file` e
  reserva o pior caso na quota antes do IO (ADR-0038).
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
- `add/sub/mul/div/rem_i64_checked` e `shl/shr_i64_checked`/`and/or/xor_i64` usam
  `rvm_spec::BinOp::eval_i64`; `None` vira trap `integer_overflow` (ou
  `division_by_zero` em divisão/resto por zero — ADR-0022 — ou `invalid_shift` em
  deslocamento fora de `0..=63` — ADR-0023). Comparações usam `CmpOp::eval_i64` e
  produzem `bool`.
- `br`/`cond_br` copiam os argumentos para os slots dos parâmetros do bloco alvo
  antes do salto (phi explícito); `return` sem valor produz `Value::Unit`.
- Erros de execução carregam `Location` de função, bloco ou instrução; trap de
  overflow é `ErrorClass::Trap`; fuel e profundidade são
  `ErrorClass::ResourceLimit`.
- Determinismo: nenhum `HashMap`, tempo, ambiente ou estado compartilhado entre
  execuções; `execute` cria todo o estado localmente. Reexecutar o mesmo módulo
  produz o mesmo valor e o mesmo fuel. `host_read_file` é a única exceção e só
  existe quando o chamador passa um `Host`; a capability é conferida antes do
  efeito e o tipo do retorno é revalidado (`capability_denied`/`invalid_type`).
- Alocação de frame usa `try_reserve_exact` e falha como `invalid_module` (não
  aborta) quando não há memória.

- Memória: antes de alocar os slots de um frame, o intérprete cobra
  `frame_cost(value_count)`; antes de alocar um buffer de argumentos de
  chamada/desvio, cobra `arg_buffer_cost(args)`. O conteúdo dos slots também é
  cobrado (`Value::retained_bytes`): a gravação cobra o valor novo e libera o
  anterior, e o retorno libera frames e slots (ADR-0035). Exceder
  `config.memory_limit()` produz `out_of_memory` antes de qualquer alocação;
  `memory_peak` registra o pico de memória viva. O modelo não mede RSS
  (ADR-0013, ADR-0016, ADR-0035).

## Side Effects

- Sem host: nenhum. Com `execute_with_host`, apenas o efeito `filesystem_read` de
  `read_file`, e somente se `Host::capabilities().allows(effect, scope)` for
  verdadeiro. O intérprete não abre arquivos; delega ao backend e valida o tipo do
  valor devolvido (ADR-0036).

## Verification

- Run: `cargo test --workspace`
