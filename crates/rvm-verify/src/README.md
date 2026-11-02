# rvm-verify — pacote fonte

## Overview

Pacote fonte do verificador de RIR: um único arquivo, `lib.rs`, que separa
módulos não confiáveis do executor. A única forma de obter um `VerifiedModule` é
`VerifiedModule::verify`, que roda primeiro a checagem estrutural de
`rvm_ir::structure::check` e depois valida fluxo de controle, alcançabilidade,
dominância, definição antes do uso, tipos, chamadas, chamadas de host e
terminadores (`lib.rs:1-40`).

Ordem efetiva dentro de `verify` (`lib.rs:81-87`) e de `verify_function`
(`lib.rs:408-426`):

1. `rvm_ir::structure::check(&module, limits)` (`lib.rs:82`).
2. Para cada função, na ordem de índice (`lib.rs:83-85`): `build_cfg`
   (`lib.rs:412`), `check_reachability` (`lib.rs:413`), `compute_dominators`
   (`lib.rs:414`), `build_values` (`lib.rs:415`) e, por fim, cada bloco na ordem
   de índice (`lib.rs:422-424`).

O crate depende apenas de `rvm-spec` e `rvm-ir` (incluindo `test-helpers` só em
desenvolvimento).

## Interface

- `VerifiedModule` (`lib.rs:64`): struct com o campo privado `module: Module`
  (`lib.rs:65`), `Clone + Debug` (`lib.rs:63`) e nenhum `From<Module>` ou
  construtor público (`lib.rs:59-62`).
- `VerifiedModule::verify(module: Module, limits: &Limits) -> Result<Self, RvmError>`
  (`lib.rs:81`): consome o módulo e é a única construção.
- `VerifiedModule::module(&self) -> &Module` (`lib.rs:91`), `entry(&self) -> FnId`
  (`lib.rs:97`), `entry_function(&self) -> Option<&Function>` (`lib.rs:103`) e
  `function(&self, id: FnId) -> Option<&Function>` (`lib.rs:109`).
- Superfície interna (privada, sem `pub`): `Def` (`lib.rs:116`), `Cfg`
  (`lib.rs:134`), `ValueMaps` (`lib.rs:144`), `FunctionFacts` (`lib.rs:152`),
  `verify_function` (`lib.rs:408`), `build_cfg` (`lib.rs:432`),
  `check_reachability` (`lib.rs:483`), `compute_dominators` (`lib.rs:525`),
  `build_values` (`lib.rs:630`), `operation_type` (`lib.rs:687`), `invalid`
  (`lib.rs:792`).
- Sem subdiretórios neste pacote.

## Invariants

- Construção única: campo `module` privado, sem `From<Module>` e sem `new`
  público; `verify` é o único caminho (`lib.rs:59-66`, `lib.rs:81-87`). As
  leituras devolvem referências imutáveis (`lib.rs:91-111`), então um
  `VerifiedModule` não muda depois de verificado.
- A checagem estrutural precede toda verificação de semântica: `structure::check`
  é a primeira instrução de `verify` (`lib.rs:82`) e as alocações proporcionais ao
  módulo só ocorrem depois (`build_cfg` em `lib.rs:432-480`, `build_values` em
  `lib.rs:630-681`).
- `Limits` só é consumido na etapa estrutural: `verify` repassa `limits` a
  `structure::check` (`lib.rs:82`) e `verify_function` não recebe limites
  (`lib.rs:408`).
- Ordem de diagnóstico determinística: funções por índice (`lib.rs:83-85`),
  blocos por índice (`lib.rs:422-424`), instruções por índice e terminador por
  último (`lib.rs:165-184`); o primeiro erro vence (`lib.rs:29-30`).
- Bloco inalcançável a partir do bloco de entrada é `invalid_control_flow`
  (`lib.rs:502-518`), verificado antes de qualquer cálculo de dominância
  (`lib.rs:413-414`).
- Dominância por ponto fixo sobre bitsets `Vec<u64>`: `dom[entry] = {entry}` e
  `dom[b] = {b} ∪ ∩ dom[pred]` (`lib.rs:520-602`); como a alcançabilidade já foi
  exigida, todo bloco não-entrada tem ao menos um predecessor (`lib.rs:522-524`).
- Definição antes do uso: no mesmo bloco, o índice da definição precisa ser menor
  que a posição do uso; entre blocos, a definição precisa dominar o uso
  (`lib.rs:349-362`). Parâmetro de função está sempre disponível (`lib.rs:350`) e
  parâmetro de bloco é definido na entrada do próprio bloco (`lib.rs:351`).
- Posição de uso do terminador é `block.instrs.len()` (`lib.rs:249`), logo
  `return <v>`, condição e argumentos de `br`/`cond_br` são checados no fim do
  bloco.
- Cada valor tem no máximo uma definição e um tipo: `record_definition`
  (`lib.rs:707-729`) e `record_type` (`lib.rs:732-754`) recusam repetição e
  posição fora da faixa.
- O mapa de tipos é próprio do verificador e usa `TypeId` da tabela do módulo
  (ADR-0034); `resolve_op_type` deriva o tipo de `call_static` de `Function::ret`
  da função alvo e resolve construções agregadas.
- Aridade e tipos de chamada vêm da assinatura do alvo: aridade divergente,
  argumento com tipo divergente, `return` incompatível e condição não-`bool` são
  `invalid_type` (`lib.rs:219-237`, `lib.rs:251-290`); argumentos de desvio
  seguem a mesma regra contra os parâmetros do bloco alvo (`lib.rs:309-327`).
- `host_read_file` é verificado contra a ABI fechada (`host_signature`):
  argumentos `string` e `i64`; o retorno `result string string` precisa existir na
  tabela do módulo. Aridade/tipo divergentes são `invalid_type` (ADR-0036).
- A verificação de disponibilidade (`check_use`) roda antes da checagem de tipo
  do mesmo operando (`lib.rs:197-210`), então um uso não dominante com tipo
  errado produz `invalid_control_flow`.
- `module.entry` existe após a verificação e pode declarar parâmetros
  (`lib.rs:101-105`; teste `entrada_com_parametros_verifica`, `lib.rs:857`).

## Side Effects

- Nenhum efeito externo: sem I/O, rede, ambiente, relógio, processo ou estado
  global; a verificação é sequencial e não usa `HashMap` (`lib.rs:32-40`).
- Alocação proporcional ao módulo só depois de `structure::check`
  (`lib.rs:82`): sucessores/predecessores por bloco (`lib.rs:432-480`), bitset de
  dominadores e mapa de valores. Custo declarado: até `O(B² · ceil(B/64))`
  operações de palavra de 64 bits e memória `O(B · ceil(B/64) + V)` para `B`
  blocos e `V` valores (`lib.rs:32-37`).
- Nenhum `unwrap`, `expect`, `panic!` ou indexação direta fora de
  `#[cfg(test)]` (módulo de testes em `lib.rs:796`); o crate aplica
  `#![forbid(unsafe_code)]` (`lib.rs:42`) e `#![deny(missing_docs)]`
  (`lib.rs:43`).
- Diagnósticos são construídos por `invalid` (`lib.rs:792-794`) com código
  estável e `Location` de módulo, função, bloco ou instrução.
- Todo erro emitido é de módulo: `invalid_module`, `invalid_type` ou
  `invalid_control_flow`, com classe `ErrorClass::Module`, nunca trap
  (`lib.rs:39-40`, `lib.rs:823`).

## Verification

- Run: `cargo test --workspace`
