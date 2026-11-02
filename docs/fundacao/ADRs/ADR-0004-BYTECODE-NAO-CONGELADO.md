# ADR-0004 — Não congelar formato binário no Foundation 0.1

- **Status:** Aberto
- **Data:** 2026-11-01
- **Contexto:** RIR e semântica não foram validadas; decidir endianidade, versionamento e layout agora criaria compatibilidade prematura.

## Decisão proposta

Usar representação interna/humana experimental e testar alternativas de registradores, pilha e linearização próxima da RIR. Nenhum wire format estável no 0.1.

## Alternativas

- Escolher agora bytecode registrador SSA.
- Escolher bytecode de pilha.
- Executar RIR diretamente até o experimento justificar encoding.

## Consequências

- Evita prometer compatibilidade sem evidência.
- Ferramentas/fixtures devem marcar revisão de formato.
- Comparação inclui verificação, decode hostil, depuração e custo.

## Gate

Corpus estável, decoder limitado, diferenciais RIR↔bytecode, resultados de tamanho/desempenho e revisão independente. Se nenhuma opção melhorar simplicidade ou desempenho de forma útil, manter execução da RIR.
