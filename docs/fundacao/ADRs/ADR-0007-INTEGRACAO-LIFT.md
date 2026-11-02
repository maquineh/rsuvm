# ADR-0007 — Rota de integração com Lift

- **Status:** Aberto; auditoria pendente
- **Data:** 2026-11-01
- **Contexto:** O link fornecido identifica Lift (maquineh/lift). O README declara transpilar subset tipado para Rust/Cargo. Nesta preparação não houve clone, auditoria de código ou teste local. Ponto HIR/API RIR não confirmado.

## Decisão provisória

Não alterar Lift upstream nem assumir backend/AST/semântica internos. Auditar commit fixo e comparar opções em protótipo separado. Só então escolher API de crate, adapter externo ou frontend independente.

## Consequências

- Preserva backend/documentação enquanto viabilidade é medida.
- Pode requerer mais esforço que estender internamente o compilador.
- Licença, estabilidade e suporte precisam ser analisados.

## Validação

Relatório de fonte/testes, subset e protótipo diferencial reproduzível sem regressão. Reabrir se API não for reutilizável, semântica não mapear ou licença/arquitetura não permitir integração.
