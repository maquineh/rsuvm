# Formato textual experimental — EXP-0-TEXT

**Estado:** gramática fechada para o protótipo, **não normativa** para a RIR nem
para nenhum formato wire. ADR-0010 registra a decisão. A fundação declara
(`docs/fundacao/CONTRATOS/RIR-EXP-0.md` §4) que a notação legível de exemplo não
define gramática; este documento define a gramática usada pelo corpus e pelo CLI,
sujeita a mudança sem compatibilidade.

## 1. Gramática

~~~text
arquivo    = comentário* módulo import* entrada declaração_fn+ EOF ;
módulo     = "module" ident "version" inteiro ;
import     = "import" ident ;
entrada    = "entry" ident ;
declaração_fn = "fn" ident "(" [parâmetros] ")" "->" tipo bloco+ ;
parâmetros = parâmetro { "," parâmetro } ;
parâmetro  = ident ":" tipo ;
tipo       = "unit" | "bool" | "i64" | "string" ;
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
import <nome_da_funcao_externa>   ; opcional, zero ou mais

fn <nome>(<param>: <tipo>, ...) -> <tipo> {
  <rótulo>:
    v<id> = <operação> <operandos>
    <terminador>
}
~~~

Regras:

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

Literais de string com interpolação, caracteres de escape não listados, arrays,
registros, tipos genéricos, comentários de bloco, anotações, módulos múltiplos por
arquivo, ligação dinâmica, imports qualificados por módulo (`modulo.funcao`) e
qualquer forma de açúcar sintático. A ligação inter-módulo é **estática** e feita
pelo crate `rvm-link` (ADR-0018), fora do formato textual.
