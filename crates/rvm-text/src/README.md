# rvm-text — Package Contract

## Overview

`rvm-text` implementa o formato textual experimental EXP-0 (ADR-0010): um lexer
ASCII, um parser de duas passadas e uma impressão canônica reparseável. Existe
para que o corpus de conformidade seja **dado** e para dar ao CLI uma entrada
legível. Não é formato wire estável e não define a semântica da RIR; apenas
constrói módulos estruturalmente válidos. Depende de `rvm-spec` e `rvm-ir`.

## Interface

- `parse(input: &str, limits: &Limits) -> Result<Module, RvmError>`
  (`lib.rs:parse`).
- `print(module: &Module) -> String` (`lib.rs:print`).

## Invariants

- Entrada acima de `Limits::max_input_bytes` é rejeitada com `input_too_large`
  antes do lexer (`lib.rs:parse`).
- Lexer ASCII: comentário `;` até fim da linha; identificadores
  `[A-Za-z_][A-Za-z0-9_]*`; inteiros decimais com sinal opcional; `->` e `=`
  como tokens próprios. Caractere inesperado, EOF inesperado e token fora da
  gramática são `syntax_error`.
- `module` é obrigatório e primeiro; `entry` aparece uma vez; uma ou mais `fn`.
  `entry` ausente ou duplicado é `invalid_module`; `import` é
  `unsupported_feature`.
- `version != 1` é `unsupported_version`, decidido imediatamente após ler o
  número; número fora de `i64`/`u16` é `invalid_number`.
- Tipo fora de `unit|bool|i64|string|f64|array|struct|enum|option|result` é
  `unknown_type`; operação/terminador desconhecido é `unknown_opcode`.
- Declarações `enum` + `type <id> = enum <Nome>` são resolvidas com referências
  para frente a structs/enums; campos de variante são posicionais ou nomeados.
  `enum_new` usa `Enum.Variant`; `enum_is_variant`/`enum_payload` usam índices
  (ADR-0039).
- `host_read_file` (opcode 54) representa `HostCall` com `u8 function` (0 =
  `read_file`), `u16 argc` e `u32 args[]`; função desconhecida é `unknown_opcode`.
  A semântica e o efeito vêm da ABI (ADR-0036), não do texto/bytecode.
- Cada bloco tem exatamente um terminador, necessariamente a última instrução do
  bloco; zero terminadores, dois terminadores ou instrução após o terminador são
  `invalid_control_flow`.
- Resolução de nomes em duas passadas: parâmetros primeiro, depois, por bloco,
  parâmetros de bloco e definições de instrução, exatamente nessa ordem. Nome
  duplicado ou referência a valor não declarado é `invalid_module`; rótulo alvo
  não declarado é `invalid_control_flow`.
- Ao final, `rvm_ir::structure::check` é aplicado; `parse` **só devolve módulo
  estruturalmente válido**. Tipos, CFG e dominância são responsabilidade de
  `rvm-verify`.
- Ambiguidade conhecida: `return <nome>` é distinguido de `return` seguido de
  novo bloco (`<nome>:`) olhando o token seguinte; se for `:` ou `(`, o nome abre
  bloco e o retorno não tem valor (`lib.rs` doc do crate).
- Impressão canônica: `module`, `entry`, `type`, `enum`, `struct`, `import`,
  funções e blocos na ordem dos IDs, recuo de dois espaços, definições como
  `v<id>`, parâmetros e parâmetros de bloco
  pelo nome armazenado. Para módulos originados do texto, `parse(print(m))`
  preserva estrutura **e** IDs; para módulos de API/bytecode com IDs fora da
  ordem textual, pode renumerar valores.
- `print` nunca entra em pânico com módulo inválido: alvos/chamadas ausentes são
  impressos como `bloco<N>`/`fn<N>`.

## Side Effects

- `None`. Parser e impressão são funções puras sobre `&str`/`&Module`; não leem
  arquivos, rede, ambiente, relógio ou processos. As alocações (tokens, árvore
  bruta) são proporcionais à entrada, que é limitada por `max_input_bytes`.

## Verification

- Run: `cargo test --workspace`
