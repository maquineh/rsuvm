# ADR-0008 — Baseline e integração com Rio

- **Status:** Aberto; código-fonte pendente
- **Data:** 2026-11-01
- **Contexto:** Conversa anterior mencionou Rio e possível localização local, mas nenhum repositório/código foi entregue ou encontrado no workspace atual.

## Decisão provisória

Não declarar recursos existentes nem criar compilador substituto. Obter fonte autorizada, produzir RIO-BASELINE-001 e então propor integração. RVM segue independente enquanto isso.

## Consequências

- Evita reescrita prematura e preserva eventual interpretador como oracle.
- Integração Rio depende de artefato faltante, sem bloquear runtime genérico.

## Validação

Commit/hash, licença, inventário de módulos e testes, comportamento observável e primeiro caso diferencial. Atualizar ADR quando baseline estiver disponível.
