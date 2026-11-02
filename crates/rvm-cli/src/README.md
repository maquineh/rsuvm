# rvm-cli — Package Contract

## Overview

`rvm-cli` implementa o binário de desenvolvimento `rsuvm`. É a única interface de
linha de comando do protótipo e **não concede capabilities**: nenhum subcomando
oferece arquivo, rede, ambiente, relógio ou subprocesso ao convidado. Lê módulos
textuais ou binários, verifica antes de executar e traduz
`ErrorClass`/`ErrorCode` em códigos de saída estáveis
(`docs/CONTRATOS/ERROS-EXP-0.md` §5).

## Interface

- Binário `rsuvm` (`Cargo.toml:[[bin]]`).
- Subcomandos: `run`, `run-bc`, `check`, `fmt`, `encode`, `version`, `help`
  (`main.rs:dispatch`).
- Opções: `--entry <fn>`, `--arg <tipo:valor>`, `--fuel <n>`, `--max-depth <n>`
  (`main.rs:parse_options`). Tipos de argumento: `i64:<inteiro>`, `bool:true|false`,
  `unit` (`main.rs:parse_argument`).
- `encode` recebe exatamente dois posicionais: entrada `.rvm` e saída binária
  (`main.rs:command_encode`).
- Saída de valor: `unit`, `bool:true`, `i64:30` (formato de `Value::Display`).
- Códigos de saída: `0` sucesso; `1` uso/IO; `2` módulo; `3` trap; `4` limite de
  recurso; `5` capability (`main.rs:Failure::exit_code`).

## Invariants

- Antes de executar, o módulo passa por `rvm_text::parse`/`rvm_bytecode::decode` e
  por `rvm_verify::VerifiedModule::verify`; não há atalho de execução
  (`main.rs:command_run`).
- `--max-depth` fora de `1..=CallDepthLimit::HARD_MAX` é erro de uso (exit 1), não
  é ajustado silenciosamente nem truncado (`main.rs:Options::config`).
- `--fuel` padrão é `ExecConfig::default().fuel()`; `--fuel 0` é aceito e produz
  `out_of_fuel` na primeira operação.
- O nome em `--entry` precisa existir no módulo verificado; ausente é erro de uso
  (exit 1) e não uma execução silenciosa da entrada padrão (`main.rs:resolve_entry`).
- Não existe opção de concessão de capability; qualquer `--allow-*` é opção
  desconhecida e sai com 1 (`main.rs:parse_options`, teste `opcao_de_capability_nao_existe`).
- Erros de módulo, trap e limite são distinguidos por `ErrorClass`, não por texto:
  a mensagem em `stderr` segue `código: detalhe [localização]`
  (`main.rs:Failure::Display`).
- IO falho (arquivo inexistente, binário ilegível) é exit 1, nunca pânico
  (`main.rs:load_module`).
- `fmt` imprime a forma canônica de `rvm_text::print` sem reexecutar nem verificar;
  `check` parseia e verifica, imprimindo módulo e entrada.

## Side Effects

- Lê exatamente o arquivo de entrada indicado e, em `encode`, escreve exatamente o
  arquivo de saída indicado (`std::fs::read`, `std::fs::read_to_string`,
  `std::fs::write`). Nenhum outro acesso a filesystem.
- Escreve em `stdout`/`stderr` do processo. Nenhuma rede, ambiente, relógio,
  subprocesso ou capability.

## Verification

- Run: `cargo test --workspace`
