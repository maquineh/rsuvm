# Contrato de erros — EXP-0

**Estado:** experimental, mas os **nomes** dos códigos são estáveis dentro do
EXP-0: o corpus e os testes de conformidade dependem deles. Renomear, remover ou
reclassificar um código exige ADR e atualização do corpus.

## 1. Estrutura

Todo diagnóstico é um `RvmError` com:

- `code`: `ErrorCode` (nome estável em `snake_case`);
- `category`: `ErrorCategory` — estágio do pipeline (`decode`, `verify`, `trap`,
  `resource`, `capability`);
- `class`: `ErrorClass` — resposta observável (`module`, `trap`,
  `resource_limit`, `capability`);
- `detail`: texto humano, **instável**;
- `location`: opcional (`módulo`, `função`, `bloco`, `instrução`, `valor`).

Comparações de conformidade usam `code` (e, quando relevante, `class`). Mensagens
humanas nunca são contrato.

## 2. Tabela de códigos

| Código | Categoria | Classe | Condição | Emitido por |
|---|---|---|---|---|
| `syntax_error` | decode | module | texto/bytes fora da gramática; magic inválido; token inesperado | `rvm-text`, `rvm-bytecode` |
| `invalid_number` | decode | module | literal inteiro fora de `i64`; número malformado | `rvm-text`, `rvm-bytecode` |
| `unknown_opcode` | decode | module | nome/tag de instrução ou terminador desconhecido | `rvm-text`, `rvm-bytecode` |
| `unknown_type` | decode | module | nome/tag de tipo desconhecido | `rvm-text`, `rvm-bytecode` |
| `unsupported_version` | decode | module | versão de módulo ou de codificação ≠ suportada | `rvm-ir`, `rvm-text`, `rvm-bytecode` |
| `truncated_input` | decode | module | faltam bytes para o que foi declarado | `rvm-bytecode` |
| `input_too_large` | decode | module | entrada maior que `Limits::max_input_bytes` | `rvm-text`, `rvm-bytecode` |
| `invalid_module` | verify | module | IDs não densos/duplicados, nome inválido/reservado, referência a função inexistente, limite estrutural, contagem transbordando | `rvm-ir` |
| `invalid_type` | verify | module | tipo ou aridade incompatível (operandos, argumentos, retorno, condição, chamada) | `rvm-verify` |
| `invalid_control_flow` | verify | module | alvo de desvio inexistente, bloco inalcançável, terminador ausente/duplicado, violação de dominância | `rvm-ir`, `rvm-verify`, `rvm-text` |
| `unsupported_feature` | verify | module | imports, capability concedida sem executor, recurso fora do EXP-0 | `rvm-ir`, `rvm-verify`, `rvm-text`, `rvm-runtime` |
| `integer_overflow` | trap | trap | soma/subtração/multiplicação fora de `i64`; `i64::MIN / -1` e `i64::MIN % -1` | `rvm-interpreter` |
| `division_by_zero` | trap | trap | divisão ou resto por zero (ADR-0022) | `rvm-interpreter` |
| `out_of_fuel` | resource | resource_limit | saldo de fuel insuficiente antes do despacho | `rvm-interpreter` |
| `call_depth_exceeded` | resource | resource_limit | profundidade de chamadas excederia o limite | `rvm-interpreter` |
| `out_of_memory` | resource | resource_limit | quota de memória modelada de frames excedida | `rvm-interpreter` |
| `capability_denied` | capability | capability | requisição de efeito de host sem concessão | `rvm-runtime` |

## 3. Ordem de falha (determinismo de diagnóstico)

O primeiro erro encontrado vence, nesta ordem:

1. tamanho da entrada (`input_too_large`) e forma textual/binária;
2. versão do módulo (`unsupported_version`);
3. invariantes estruturais, na ordem de `rvm_ir::structure::check`
   (módulo → funções em ordem → blocos em ordem → valores → operandos →
   terminadores);
4. tipos e CFG por função, em ordem de índice de função e de bloco;
5. execução, na ordem de despacho.

Nenhuma falha de verificação pode ser convertida em trap: códigos de módulo nunca
aparecem durante a execução de um `VerifiedModule`.

## 4. Mapeamento de casos de conformidade

| Caso | Código esperado | Estágio |
|---|---|---|
| N-001 tipo incompatível em argumento | `invalid_type` | verify |
| N-002 aridade incorreta | `invalid_type` | verify |
| N-003 ID/valor não definido | `invalid_module` | decode/verify (resolução de nomes) |
| N-004 definição duplicada | `invalid_module` | decode/verify |
| N-005 bloco sem terminador ou com duplicação | `invalid_control_flow` | decode/verify |
| N-006 destino de branch inexistente | `invalid_control_flow` | decode/verify |
| N-007 retorno difere da assinatura | `invalid_type` | verify |
| N-008 opcode/tipo/versão desconhecidos | `unknown_opcode` / `unknown_type` / `unsupported_version` | decode |
| N-009 overflow em `i64` | `integer_overflow` | execução (trap) |
| N-010 loop com fuel finito | `out_of_fuel` | execução (limite) |
| N-011 recursão acima da profundidade | `call_depth_exceeded` | execução (limite) |
| N-012 truncamento/`length` excessivo | `truncated_input` / `input_too_large` | decode (bytecode) |
| N-021 quota de memória | `out_of_memory` | execução (limite) |
| N-025 divisão por zero | `division_by_zero` | execução (trap) |
| N-026 `i64::MIN / -1` | `integer_overflow` | execução (trap) |

`invalid_module` também cobre limite estrutural excedido: a distinção entre
"estrutura inválida" e "quota de execução" é feita por `class`, não pelo texto.

## 5. Códigos de saída do CLI

| Saída | Significado |
|---:|---|
| 0 | sucesso |
| 1 | uso incorreto do CLI ou falha de IO do host |
| 2 | erro de módulo (`class = module`) |
| 3 | trap semântico (`class = trap`) |
| 4 | limite de recurso (`class = resource_limit`) |
| 5 | capability negada (`class = capability`, não alcançável no MVP) |

O CLI imprime `código: detalhe [localização]` em `stderr` e nunca converte erro em
pânico.

## 6. Estabilidade

- Adicionar código novo: permitido com ADR; o corpus pode ignorar códigos que não
  exercita.
- Renomear/reclassificar: exige atualização simultânea de `rvm-spec`, desta
  tabela e do corpus, com nota no relatório.
- `detail` e `location` são auxiliares de depuração e podem mudar a qualquer
  momento sem ADR.
