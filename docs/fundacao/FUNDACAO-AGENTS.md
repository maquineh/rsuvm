# Instruções para agentes de implementação

Estas instruções valem para agentes de código que usem este pacote. A especificação está em fase de pesquisa. Leia primeiro README.md, docs/SEGURANCA/GUARDRAILS-DEEPSEEK.md e os ADRs relevantes.

## Regras obrigatórias

1. Antes de editar, inventarie o repositório e leia instruções locais, manifests, testes e CI. Relate fatos com caminhos e linhas. Não invente módulos, tipos, APIs ou testes.
2. Trate Proposta, Hipótese, Desconhecido e Resultado conforme definidos no README. Só promova uma proposta a decisão validada após critérios de evidência do ADR.
3. Preserve a semântica declarada do Lift e do Rio. Não altere compiladores, semântica, branches remotas ou dependências externas sem tarefa explícita do responsável pelo repositório.
4. Não afirme que existe backend Lift→RIR, AST pública, HIR tipada ou API de integração sem verificar fonte e testes no commit escolhido.
5. Não invente o baseline do Rio. Se o código não estiver disponível, registre a dependência e continue apenas no trabalho independente da RVM.
6. Mantenha o runtime independente das linguagens hospedadas. Nenhum branch especial do executor por linguagem de origem.
7. Entradas, bytecode, nomes, tamanhos, índices e offsets são não confiáveis. Verifique limites antes de alocar, indexar, recursar ou despachar.
8. Por padrão, use Rust seguro, sem unsafe, FFI nativa, carregamento dinâmico ou execução de código nativo convidado. Qualquer exceção exige ADR, justificativa de ameaça, testes e aprovação explícita do mantenedor.
9. Nenhuma capacidade de SO é implícita. Arquivos, rede, ambiente, relógio e processos precisam de APIs/capacidades concedidas pelo host; no MVP, não oferecer essas capacidades ao convidado.
10. Nunca edite testes de referência para mascarar divergência. Determine primeiro se implementação viola contrato ou se contrato precisa de proposta de mudança.
11. Nunca declare teste, fuzzing, benchmark, revisão de segurança ou auditoria como concluídos sem comando, ambiente, commit e resultado registráveis.
12. Não use dependências novas sem justificar necessidade, manutenção, licença, superfície de ataque e versão fixada.

## Forma de trabalho

- Mantenha a mudança no escopo pedido e divida-a em passos pequenos.
- Se documento e código divergirem, crie relatório de divergência com evidências e proponha atualização do contrato/ADR.
- Atualize testes de conformidade para cada regra semântica implementada; não amplie a linguagem implicitamente.
- Registre limitações, casos não cobertos e resultados negativos.
- Use nomes de APIs conforme o estilo do repositório. A preferência por português nos documentos não exige traduzir identificadores de código.

## Relatório final de cada tarefa

Inclua arquivos alterados; requisito/ADR atendido; decisão tomada; comandos executados e resultados exatos; testes não executados; risco ou lacuna restante; próxima tarefa sugerida. Não diga “seguro”, “isolado” ou “compatível” sem delimitar a propriedade medida.
