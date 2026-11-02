# Meta de cobertura do corpus — EXP-0

**Estado:** meta declarada e verificada por teste; não é cobertura de código
(linhas/branches) nem prova de correção. O corpus é **dado**
(`crates/rvm-conformance/corpus/`); a cobertura aqui é presença de operação nos
casos declarados.

## 1. Meta por operação

Cada operação do subset EXP-0 precisa aparecer em **pelo menos um caso válido**
com valor esperado, e cada terminador em pelo menos um caso válido. A verificação
é executada por `crates/rvm-conformance/tests/coverage.rs`
(`corpus_valido_cobre_todas_as_operacoes_do_subset`).

| Operação | Caso(s) válido(s) |
|---|---|
| `const_i64` | C-001, C-017, C-018, C-019 (todos os casos com literais) |
| `const_bool` | C-003 |
| `const_str` | C-027, C-028, C-029, C-030 |
| `add_i64_checked` | C-001, C-004, C-005, C-017, C-018, C-019 |
| `sub_i64_checked` | C-010 |
| `mul_i64_checked` | C-011 |
| `div_i64_checked` | C-020 |
| `rem_i64_checked` | C-021 |
| `shl_i64_checked` | C-022 |
| `shr_i64_checked` | C-023 |
| `and_i64` | C-024 |
| `or_i64` | C-025 |
| `xor_i64` | C-026 |
| `str_eq` | C-027 |
| `str_ne` | C-029 |
| `str_len` | C-028 |
| `eq_i64` | C-002 |
| `ne_i64` | C-012 |
| `lt_i64` | C-008 |
| `le_i64` | C-013 |
| `gt_i64` | C-014 |
| `ge_i64` | C-015 |
| `call_static` | C-004, C-005, C-006, C-016, C-018 |

| Terminador | Caso(s) válido(s) |
|---|---|
| `return` | todos |
| `br` | C-003, C-007, C-017, C-019 |
| `cond_br` | C-003, C-007, C-017 |

## 2. Meta por código de erro

Cada código de erro **central** precisa aparecer em pelo menos um caso inválido
com a expectativa declarada. A verificação é executada por
`corpus_invalido_cobre_os_codigos_de_erro_centrais`.

| Código | Caso(s) inválido(s) |
|---|---|
| `invalid_type` | N-001, N-002, N-007, N-015, N-017 |
| `invalid_module` | N-003, N-004, N-022, N-023 |
| `invalid_control_flow` | N-005, N-006, N-013, N-016, N-018 |
| `unknown_opcode` | N-008 |
| `unsupported_version` | N-014 |
| `integer_overflow` | N-009, N-019, N-020, N-026 |
| `division_by_zero` | N-025 |
| `invalid_shift` | N-027 |
| `out_of_fuel` | N-010 |
| `call_depth_exceeded` | N-011 |
| `out_of_memory` | N-021, N-024 |

## 3. Lacunas declaradas (não cobertas pelo corpus)

Códigos e operações sem caso no corpus, cobertos apenas por testes de unidade ou
do decoder binário. A ausência é **lacuna declarada**, não aprovação:

| Item | Onde é exercitado |
|---|---|
| `syntax_error`, `unknown_type`, `invalid_number`, `unsupported_feature` | parser textual (`crates/rvm-text/tests`) |
| `truncated_input`, `input_too_large` | decoder binário (`crates/rvm-bytecode/tests/hostile.rs`) |
| `capability_denied` | `rvm-runtime` e `tests/properties.rs` |
| limites estruturais (funções, blocos, instruções, valores) | `rvm-ir/src/structure.rs` (unitários, com `Limits` customizados) |
| `call_static` entre módulos | fora do EXP-0 (H03 não testada) |

## 4. Regras

- Nenhuma expectativa é ajustada para "fazer passar"; divergência vira relatório
  em `docs/RELATORIOS/`.
- A meta é verificada por teste; remover um caso que cobre uma operação obrigatória
  faz `coverage.rs` falhar.
- Cobertura de operação **não** é cobertura de caminho, tipos de borda ou
  aritmética exaustiva: ela só garante que cada operação aparece em um caso.
