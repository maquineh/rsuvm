# ADR-0002 — RIR como fronteira experimental

- **Status:** Proposto
- **Data:** 2026-11-01
- **Contexto:** Separar regras das linguagens dos mecanismos de execução e permitir verificação prévia.

## Decisão proposta

Criar RIR experimental, tipada e verificável. EXP-0 usa funções, tipos primitivos, blocos e fluxo explícito. Forma SSA é hipótese de representação, não contrato congelado.

## Alternativas

- Interpretar AST de cada linguagem no runtime.
- Compartilhar bytecode completo imediatamente.
- Usar IR externa já existente.

## Consequências

- RIR separa runtime de frontend, mas cria custo de tradução e manutenção.
- Interpretação de AST por linguagem evita tradução inicial, mas dificulta independência e chamadas interlinguagem.
- Reutilizar IR existente só após comparar semântica, licença e segurança.

## Validação

Corpus de dois frontends, regras independentes, rejeição pré-execução de inválidos e comparação com intérprete de referência. Reabrir se RIR não expressar subset comum sem exceções por linguagem.
