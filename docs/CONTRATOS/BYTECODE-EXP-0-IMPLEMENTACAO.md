# Bytecode experimental — codificação linear (EXP-0)

**Estado:** protótipo não normativo. O ADR-0004 continua **aberto**: nenhum
formato wire é estável e este crate existe para medir custo e risco de decodificar
entrada hostil. ADR-0011 registra a decisão de prototipar. Não há compatibilidade
prometida entre revisões.

## 1. Layout

Todos os inteiros são little-endian. `u16`/`u32`/`i64` são fixos. Cadeias são
`u16` de comprimento seguido dos bytes UTF-8.

~~~text
arquivo:
  magic            6 bytes = "RSVMB1"
  format_version   u16 = 1
  flags            u16 = 0
  module_name      string
  module_version   u16 = 1
  entry_fn         u32
  function_count   u32
  funções[]        function_count vezes
  -- fim do arquivo; bytes excedentes são rejeitados --

function:
  id               u32
  name             string
  ret_type         u8  (0=unit, 1=bool, 2=i64)
  entry_block      u32
  param_count      u16
  params[]         param_count vezes { name: string, ty: u8 }
  block_count      u32
  blocks[]         block_count vezes

block:
  id               u32
  label            string
  block_param_count u16
  block_params[]   { value_id: u32, name: string, ty: u8 }
  instr_count      u32
  instrs[]         instr_count vezes
  terminator       terminator

instr:
  def              u32
  opcode           u8
  payload          depende do opcode

terminator:
  kind             u8
  payload          depende do kind
~~~

### Opcodes de instrução

| Código | Operação | Payload |
|---:|---|---|
| 0 | `const_i64` | `i64` |
| 1 | `const_bool` | `u8` (0 ou 1; outro valor é `invalid_number`) |
| 2 | `add_i64_checked` | `u32 lhs`, `u32 rhs` |
| 3 | `sub_i64_checked` | `u32 lhs`, `u32 rhs` |
| 4 | `mul_i64_checked` | `u32 lhs`, `u32 rhs` |
| 5 | `eq_i64` | `u32 lhs`, `u32 rhs` |
| 6 | `ne_i64` | `u32 lhs`, `u32 rhs` |
| 7 | `lt_i64` | `u32 lhs`, `u32 rhs` |
| 8 | `le_i64` | `u32 lhs`, `u32 rhs` |
| 9 | `gt_i64` | `u32 lhs`, `u32 rhs` |
| 10 | `ge_i64` | `u32 lhs`, `u32 rhs` |
| 11 | `call_static` | `u32 callee`, `u16 argc`, `u32 args[argc]` |
| 12 | `div_i64_checked` | `u32 lhs`, `u32 rhs` |
| 13 | `rem_i64_checked` | `u32 lhs`, `u32 rhs` |
| 14 | `shl_i64_checked` | `u32 lhs`, `u32 rhs` |
| 15 | `shr_i64_checked` | `u32 lhs`, `u32 rhs` |
| 16 | `and_i64` | `u32 lhs`, `u32 rhs` |
| 17 | `or_i64` | `u32 lhs`, `u32 rhs` |
| 18 | `xor_i64` | `u32 lhs`, `u32 rhs` |
| 19 | `const_str` | `u16 len`, `u8[len]` (UTF-8) |
| 20 | `str_eq` | `u32 lhs`, `u32 rhs` |
| 21 | `str_ne` | `u32 lhs`, `u32 rhs` |
| 22 | `str_len` | `u32 operand` |

Código desconhecido ⇒ `unknown_opcode`.

### Tipos de terminador

| Código | Terminador | Payload |
|---:|---|---|
| 0 | `return` sem valor | — |
| 1 | `return` com valor | `u32 value` |
| 2 | `br` | `u32 target`, `u16 argc`, `u32 args[argc]` |
| 3 | `cond_br` | `u32 cond`, `u32 then_target`, `u16 then_argc`, `u32[]`, `u32 else_target`, `u16 else_argc`, `u32[]` |

Código desconhecido ⇒ `unknown_opcode`.

## 2. Invariantes do decoder (obrigatórias)

1. `bytes.len() > Limits::max_input_bytes` ⇒ `input_too_large` antes de qualquer
   leitura de corpo.
2. Magic diferente de `RSVMB1` ⇒ `syntax_error`.
3. `format_version != 1` ou `flags != 0` ⇒ `unsupported_version`.
4. Toda leitura checa bytes restantes; faltando ⇒ `truncated_input` (nada de
   `panic`, `unwrap` ou indexação direta).
5. **Contagem antes de alocação**: cada contagem declarada (`function_count`,
   `param_count`, `block_count`, `instr_count`, `argc`, comprimentos de string) é
   comparada com o limite estrutural correspondente **e** com os bytes restantes
   antes de reservar memória. Contagem maior que os bytes restantes ⇒
   `truncated_input` (cada item consome ao menos 1 byte).
6. Tag de tipo fora de `0..=3` (`unit`, `bool`, `i64`, `string`) ⇒ `unknown_type`;
   `const_bool` fora de `0|1` ⇒
   `invalid_number`.
7. Ao final, bytes excedentes ⇒ `invalid_module`.
8. O módulo decodificado passa por `rvm_ir::structure::check` antes de ser
   devolvido; IDs não densos, nomes inválidos, alvos inexistentes e limites
   violados são rejeitados ali.
9. O decoder não confia em nenhum metadado do emissor: versão, tipos e contagens
   são reconferidos contra `Limits` e contra a estrutura.
10. O decoder nunca devolve `Module` parcial: ou o módulo completo e
    estruturalmente válido, ou erro.

`encode` é determinístico: o mesmo `Module` produz os mesmos bytes, na mesma
ordem, sem padding e sem campos derivados (não há checksum, offset de seção ou
hash).

## 3. Erros esperados no corpus de decoder

| Entrada | Código |
|---|---|
| arquivo vazio / menor que o cabeçalho | `truncated_input` |
| magic inválido | `syntax_error` |
| versão 2 ou flags ≠ 0 | `unsupported_version` |
| contagem gigante (`u32::MAX`) com arquivo curto | `truncated_input` |
| opcode 200 | `unknown_opcode` |
| tag de tipo 9 | `unknown_type` |
| nome com byte inválido/identificador inválido | `invalid_module` |
| bytes após o módulo | `invalid_module` |

Nenhum caso pode causar pânico; `crates/rvm-bytecode/tests/hostile.rs` cobre
truncamento, contagens gigantes, tags desconhecidas e mutação determinística de
bytes (xorshift com semente fixa, orçamento registrado no teste).

## 4. Diferencial RIR ↔ bytecode

Para todo módulo do corpus válido: `decode(encode(m))` precisa produzir um módulo
que, depois de verificado, executa com o mesmo valor e o mesmo consumo de fuel que
o caminho textual. Divergência vira relatório de divergência, não ajuste de
golden file.

## 5. Não objetivos

- Compatibilidade futura, versionamento estável, compressão, assinatura, seções
  opcionais, source maps, validação de checksum e otimizações de layout.
- Substituir o formato textual na depuração.
