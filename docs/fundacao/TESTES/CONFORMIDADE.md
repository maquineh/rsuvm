# Plano de conformidade e testes

Este arquivo especifica casos-alvo. Não afirma que implementação ou testes já existam ou tenham sido executados.

## 1. Camadas

1. Semântica: entradas válidas e valores/traps definidos.
2. Verificador: estrutura, tipos, CFG, referências e versão.
3. Decoder: entradas malformadas, tamanho e truncamento.
4. Runtime: fuel, chamadas, depth, determinismo e isolamento entre execuções.
5. Diferencial: resultado de frontend(s) e runtime no subset definido.
6. Propriedades/fuzzing: invariantes e ausência de panic observada no orçamento declarado.
7. Segurança: capabilities negadas e quotas adversariais conforme threat model.

## 2. Corpus positivo

| ID | Caso | Resultado esperado |
|---|---|---|
| C-001 | soma(10,20) em i64 | retorna 30 |
| C-002 | igualdade entre i64 iguais | true |
| C-003 | branch com condição true | só executa bloco correspondente |
| C-004 | chamada estática com tipos/arity corretos | retorno confere |
| C-005 | fuel suficiente | execução termina normalmente |
| C-006 | repetir módulo puro | mesmo valor/categoria de erro |

## 3. Corpus negativo

| ID | Entrada | Resultado esperado |
|---|---|---|
| N-001 | tipo incompatível em argumento | invalid_type antes da execução |
| N-002 | aridade incorreta | rejeição do verificador |
| N-003 | uso de ID não definido | rejeição do verificador |
| N-004 | definição duplicada | rejeição do verificador |
| N-005 | bloco sem terminador ou com duplicação | invalid_control_flow |
| N-006 | destino de branch inexistente | rejeição do verificador |
| N-007 | retorno difere da assinatura | rejeição do verificador |
| N-008 | opcode/tipo/versão desconhecidos | erro de formato/feature, sem execução |
| N-009 | soma/sub/mul transborda i64 | trap integer_overflow |
| N-010 | loop sem fim com quota finita | out_of_fuel dentro do limite |
| N-011 | recursão excede depth | call_depth_exceeded |
| N-012 | truncamento/length excessivo | erro decode sem panic |

## 4. Teste diferencial

Para cada programa compartilhado registrar fonte, subset, versão/commit do frontend, comando, saída, valor/erro esperado e ambiente. Excluir casos sem comportamento definido. Divergência vira caso mínimo e classificação: tradutor, RIR, verificador, runtime, oracle ou especificação ambígua.

Lift→Rust nativo só pode ser executado como fixture confiável e baseline semântico; compilação nativa não constitui isolamento. Semântica de Rio é desconhecida até baseline.

## 5. Propriedades

- Verificador aceita módulos válidos gerados e rejeita mutações inválidas conhecidas.
- Módulo aceito não causa acesso fora de índices/vetores controlados do executor.
- Overflow checked sempre produz o trap contratado.
- Fuel é monotônico e não reseta em chamadas.
- Entrada inválida não causa panic em decoder/verificador.
- Reexecutar módulo puro não compartilha estado acidental.

## 6. Fuzzing e release gate

Registrar ferramenta/versão, targets, seed, corpus, tempo, recursos, crashes e minimizações. “Zero crash no corpus em X horas” descreve só a rodada, não prova ausência de vulnerabilidade.

Não chamar protótipo de sandbox seguro sem threat model explícito, quotas verificadas, testes adversariais, revisão independente, análise de capabilities e riscos residuais. Backend nativo Lift não substitui RVM isolada.
