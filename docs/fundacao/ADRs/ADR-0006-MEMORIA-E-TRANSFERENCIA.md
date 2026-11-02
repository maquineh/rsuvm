# ADR-0006 — Memória e transferência entre contextos

- **Status:** Aberto para pesquisa
- **Data:** 2026-11-01
- **Contexto:** Futuras linguagens podem divergir em ownership, aliasing e mutabilidade; decidir GC/compartilhamento agora seria prematuro.

## Decisão experimental

EXP-0 começa sem heap de objetos compartilhado e sem concorrência. Pesquisar cópia, transferência exclusiva (move/handle), compartilhamento imutável e referência serializada. Compartilhamento mutável sem protocolo explícito fica proibido.

## Alternativas a avaliar

- Heap único com GC comum.
- Heap separado por ator e cópia de mensagens.
- Ownership linear/exclusivo e transferência por handle.
- Objetos imutáveis compartilhados.

## Consequências

Cada opção muda custo de chamadas, identidade, coleta, falhas e compatibilidade. Não chamar nenhuma de segura sem provar propriedades no threat model. Protótipos devem registrar aliases e custo.

## Perguntas

Transferir valor mutável remove acesso antigo? Imutabilidade é enforcement ou convenção? Qual custo para 1 KB–16 MB? O que ocorre se receiver falha? Escolher com especificação, testes de propriedade, benchmarks e revisão independente.
