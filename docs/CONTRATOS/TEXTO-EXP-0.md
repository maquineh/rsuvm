# Formato textual experimental — EXP-0-TEXT

**Estado:** gramática fechada para o protótipo, **não normativa** para a RIR nem
para nenhum formato wire. ADR-0010 registra a decisão. A fundação declara
(`docs/fundacao/CONTRATOS/RIR-EXP-0.md` §4) que a notação legível de exemplo não
define gramática; este documento define a gramática usada pelo corpus e pelo CLI,
sujeita a mudança sem compatibilidade.

## 1. Gramática

~~~text
arquivo    = comentário* módulo declaração_tipo* declaração_enum* declaração_struct* import* entrada declaração_fn+ EOF ;
módulo     = "module" ident "version" inteiro ;
declaração_tipo = "type" inteiro "=" tipo ;
declaração_enum = "enum" ident "{" variante { "," variante } [ "," ] "}" ;
variante   = ident [ "(" tipo { "," tipo } [ "," ] ")"
                   | "{" campo { "," campo } [ "," ] "}" ] ;
declaração_struct = "struct" ident "{" campo+ "}" ;
campo      = ident ":" tipo ;
import     = "import" ident ;
entrada    = "entry" ident ;
declaração_fn = "fn" ident "(" [parâmetros] ")" "->" tipo bloco+ ;
parâmetros = parâmetro { "," parâmetro } ;
parâmetro  = ident ":" tipo ;
tipo       = "unit" | "bool" | "i64" | "string" | "f64"
           | "array" tipo | "struct" ident | "enum" ident
           | "option" tipo | "result" tipo tipo ;
bloco      = rótulo [ "(" parâmetros ")" ] ":" instrução* terminador ;
instrução  = ident "=" operação ;
operação   = "const_i64" inteiro
           | "const_bool" ( "true" | "false" )
           | "const_str" string
           | "const_f64" real
           | op_binário valor "," valor
           | "str_eq" valor "," valor
           | "str_ne" valor "," valor
           | "str_len" valor
           | "concat_str" valor "," valor
           | op_f64 valor "," valor
           | "array" tipo valor { "," valor }
           | "array_len" valor
           | "array_get" valor "," valor
           | "array_set" valor "," valor "," valor
           | "struct_new" ident valor { "," valor }
           | "field_get" valor "," ident
           | "field_set" valor "," ident "," valor
           | "option_none" tipo
           | "option_some" tipo valor
           | "option_is_some" valor
           | "option_unwrap" valor
           | "coalesce" valor "," valor
           | "option_eq" valor "," valor
           | "option_ne" valor "," valor
           | "result_ok" tipo tipo valor
           | "result_err" tipo tipo valor
           | "result_is_ok" valor
           | "result_unwrap" valor
           | "result_unwrap_err" valor
           | "enum_new" ident "." ident [ valor { "," valor } ]
           | "enum_is_variant" valor "," inteiro
           | "enum_payload" valor "," inteiro "," inteiro
           | "enum_eq" valor "," valor
           | "enum_ne" valor "," valor
           | "host_read_file" valor "," valor
           | "call_static" ident "(" [valores] ")" ;
op_binário = "add_i64_checked" | "sub_i64_checked" | "mul_i64_checked"
           | "div_i64_checked" | "rem_i64_checked"
           | "shl_i64_checked" | "shr_i64_checked"
           | "and_i64" | "or_i64" | "xor_i64"
           | "eq_i64" | "ne_i64" | "lt_i64" | "le_i64" | "gt_i64" | "ge_i64" ;
valores    = valor { "," valor } ;
valor      = ident ;
terminador = "return" [ valor ]
           | "br" alvo
           | "cond_br" valor "," alvo "," alvo ;
alvo       = ident [ "(" [valores] ")" ] ;
inteiro    = [ "-" ] dígito+ ;
real       = [ "-" ] dígito+ "." dígito+ ;
op_f64     = "add_f64" | "sub_f64" | "mul_f64" | "div_f64"
           | "eq_f64" | "ne_f64" | "lt_f64" | "le_f64" | "gt_f64" | "ge_f64" ;
string     = '"' { caractere | escape } '"' ;
escape     = "\\" ( '"' | "\\" | "n" | "r" | "t" ) ;
ident      = ( letra | "_" ) { letra | dígito | "_" } ;
comentário = ";" { qualquer caractere exceto quebra de linha } quebra_de_linha ;
~~~

`letra` é ASCII `A-Z a-z`. Espaços, tabs e quebras de linha separam tokens e não
têm significado, exceto por encerrarem comentários. Não há ponto e vírgula como
terminador de instrução.

## 2. Regras léxicas e de forma

1. A primeira declaração precisa ser `module`. Falta de `module` ou `entry` é
   `syntax_error` (`module`) ou `invalid_module` (`entry` ausente).
2. `version` aceita `0..=65535`; versão diferente de `1` é `unsupported_version`;
   número inválido ou fora de `u16` é `invalid_number`.
3. Literais `i64` são decimais com sinal opcional; fora de `[-2^63, 2^63-1]` é
   `invalid_number`.
4. Tipo fora de `unit|bool|i64` é `unknown_type`; instrução/terminador fora da
   gramática é `unknown_opcode`; qualquer outro erro de token é `syntax_error`.
5. `import <nome>` declara uma **função externa** (sem corpo) que a ligação
   inter-módulo (`rvm-link`, ADR-0018) resolve para a função concreta de mesmo
   nome em outro módulo. A entrada do módulo não pode ser externa. Um módulo com
   externas não é executável direto: precisa ser linkado antes da verificação.
6. Entrada maior que `Limits::max_input_bytes` é rejeitada com
   `input_too_large` **antes** do lexer.
7. Bloco termina com exatamente um terminador, que precisa ser a última
   instrução: zero terminadores ou terminador seguido de mais instruções é
   `invalid_control_flow`.
8. Um bloco de entrada (`entry`) não pode declarar parâmetros (regra estrutural).
9. `entry` duplicado, função duplicada, rótulo duplicado, valor duplicado ou
   nome `v<N>` usado como parâmetro são `invalid_module`.
10. Quando existir mais de um erro, vence o primeiro na ordem do arquivo.
11. `host_read_file` (ADR-0036) exige `string` e `i64` como argumentos e o tipo
    `result string string` na tabela (`type <id> = result string string`). O texto
    não escolhe o efeito nem o escopo: ambos vêm da ABI; a capability é conferida
    em execução.
12. `enum` declara variantes; `type <id> = enum <Nome>` registra o tipo na tabela
    densa (ADR-0039). Structs e enums compartilham o espaço de nomes nominal.
    Variantes vazias, posicionais e nomeadas são mutuamente exclusivas; a vírgula
    final é opcional. `enum_new` usa `Enum.Variant`; `enum_is_variant` e
    `enum_payload` usam o índice da variante e o índice posicional do campo.
13. Referências para frente a structs/enums são aceitas; uma referência sem
    declaração correspondente é `unknown_type`. Um enum declarado sem
    `type <id> = enum <Nome>` é `invalid_module` (regra estrutural).

## 3. Resolução de nomes e identificadores de valor

Dentro de uma função, parâmetros, parâmetros de bloco e definições de instrução
compartilham um único espaço de nomes. Todos os nomes são resolvidos para
`ValueId` densos, na ordem:

1. parâmetros de função, na ordem de declaração (`0..params.len()`);
2. para cada bloco, na ordem do arquivo: seus parâmetros de bloco e depois as
   definições de suas instruções.

Nome usado mas não declarado é `invalid_module`; rótulo de alvo não declarado é
`invalid_control_flow`. O parser resolve nomes em duas passadas (declarações
depois usos), o que permite referências a definições posteriores — a legalidade
(dominância) é decidida por `rvm-verify`, não pelo parser.

## 4. Impressão canônica

`rvm_text::print` produz texto reparseável:

~~~text
module <nome> version 1
entry <nome_da_entrada>
type <id> = <tipo>                  ; tipos compostos (ids >= 5), na ordem

enum <Nome> {
  <Variante>
  <Variante>(<tipo>, ...)
  <Variante> { <campo>: <tipo>, ... }
}

import <nome_da_funcao_externa>     ; opcional, zero ou mais

struct <Nome> { <campo>: <tipo> ... }   ; opcional, zero ou mais

fn <nome>(<param>: <tipo>, ...) -> <tipo> {
  <rótulo>:
    v<id> = <operação> <operandos>
    <terminador>
}
~~~

Regras:

- A ordem canônica de impressão é `module`, `entry`, `type`, `enum`, `struct`,
  `import`, funções. Cada enum é separado por uma linha em branco; variantes usam
  dois espaços de recuo. Uma variante vazia não tem payload; uma posicional usa
  `(...)`; uma nomeada usa `{ ... }` com vírgulas.
- Parâmetros e parâmetros de bloco usam o nome armazenado na IR (validado como
  identificador não reservado).
- Definições de instrução são impressas como `v<id>`, independentemente do nome
  usado na entrada.
- Blocos e funções são impressos na ordem dos IDs; sem comentários, sem linhas em
  branco entre blocos e com dois espaços de recuo.
- Para módulos originados do próprio texto, `parse(print(m))` preserva a
  estrutura **e os IDs**. Para módulos construídos por API ou bytecode com IDs
  densos fora da ordem textual, a reimpressão preserva a estrutura e a semântica,
  podendo renumerar valores.

## 5. Exemplo

~~~text
; soma dois inteiros
module exemplo version 1
entry soma

fn soma(a: i64, b: i64) -> i64 {
  entry:
    v2 = add_i64_checked a, b
    return v2
}
~~~

## 6. Por que um formato de texto

- O corpus de conformidade vira **dado** inspecionável e revisável por humanos,
  não código Rust acoplado à implementação.
- O CLI ganha entrada legível para depuração de semântica.
- O custo aceito é que o parser passa a ser superfície de ataque: entrada hostil,
  limites antes de alocar e ausência de pânico são testados em
  `crates/rvm-text/tests`.

Literal `string` usa `"..."` com escapes `\"`, `\\`, `\n`, `\r` e `\t`; quebra de
linha dentro do literal é `syntax_error`; literal maior que
`Limits::max_string_bytes` é `invalid_module` (ADR-0024).

## 7. Fora do EXP-0

Literais de string com interpolação, caracteres de escape não listados, tipos
genéricos, comentários de bloco, anotações, módulos múltiplos por arquivo, ligação
dinâmica, imports qualificados por módulo (`modulo.funcao`) e qualquer forma de
açúcar sintático. A ligação inter-módulo é **estática** e feita pelo crate
`rvm-link` (ADR-0018), fora do formato textual. Tipos compostos são recursivos por
id (ADR-0034), então `array array i64`, `option option i64` e structs com campos
de outros tipos são expressáveis; ciclos são rejeitados.
