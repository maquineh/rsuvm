# ADR-0011 — Protótipo de bytecode espelho da RIR

- **Status:** Proposto (protótipo implementado; o ADR-0004 permanece **aberto** e
  nenhum formato é congelado)
- **Data:** 2026-11-01
- **Requisitos:** RF-05, M4 do roadmap, invariantes de bytecode da fundação.

## Contexto

`docs/fundacao/ADRs/ADR-0004-BYTECODE-NAO-CONGELADO.md` pede comparar alternativas
(registradores SSA, pilha, linearização próxima da RIR) e não congelar formato no
0.1. `docs/fundacao/CONTRATOS/BYTECODE-EXP-0.md` exige decoder com limites
explícitos, validação de tamanhos antes de alocar, rejeição de opcode/versão/seção
desconhecidos e ausência de pânico em entrada inválida. N-012 (truncamento e
`length` excessivo) só é exercitável com um decoder binário.

## Decisão

Implementar **um** protótipo: codificação linear que espelha a RIR
(`docs/CONTRATOS/BYTECODE-EXP-0-IMPLEMENTACAO.md`), com:

- magic `RSVMB1`, versão `u16`, flags reservadas e deve ser `0`;
- contagens declaradas validadas contra limites **e** contra bytes restantes antes
  de qualquer alocação;
- rejeição de bytes excedentes, opcode/tipo/tag desconhecidos e versão diferente;
- `rvm_ir::structure::check` aplicado ao resultado do decoder;
- codificação determinística, sem checksum, offset de seção ou hash.

Não há promessa de estabilidade, compatibilidade ou desempenho. O formato pode ser
removido se o experimento não justificar manutenção.

## Alternativas

1. **Bytecode de pilha.** Verificador diferente e mais próximo de JVM/Wasm;
   exigiria segundo modelo de execução e não reutiliza a RIR.
2. **Registradores SSA com *phi* explícito.** Mais próximo do modelo atual, porém
   com custo de mapeamento e índices que o espelho linear já resolve.
3. **Adiar M4** e entregar só RIR + intérprete. Menor escopo; mantém N-012 e a
   análise de decoder hostil sem cobertura.

## Consequências

- Positivas: exercita a fronteira de decoder hostil, permite diferencial
  RIR↔bytecode e mede custo de codificação sem congelar decisão.
- Negativas: mais código de manutenção e mais superfície de ataque; risco de o
  protótipo ser confundido com formato estável (mitigado por documentação
  explícita e ausência de versionamento prometido).
- O ADR-0004 continua sem resposta: este protótipo **não** demonstra que a
  linearização seja a melhor opção.

## Gate e critério para fechar o ADR-0004

1. Corpus estável cobrindo C-001..C-006 e N-001..N-012.
2. Diferencial RIR↔bytecode sem divergência no corpus.
3. Testes de mutação/truncamento com orçamento registrado.
4. Comparação documentada de tamanho, custo de decode e depuração entre as três
   alternativas.

Enquanto os quatro itens não existirem, qualquer afirmação de que o formato "é
melhor" é hipótese, não resultado.
