# ADR-0010 — Formato textual experimental como entrada do corpus

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-01
- **Requisitos:** RF-07 (testes diferenciais registrados), corpus independente da
  implementação, PRD §8(d).

## Contexto

O corpus precisa ser automatizável e, idealmente, independente da implementação.
`docs/fundacao/CONTRATOS/RIR-EXP-0.md` §4 mostra uma notação legível e diz
explicitamente que ela "não define gramática, JSON, binário ou API Rust".
`docs/fundacao/TESTES/CONFORMIDADE.md` exige casos C-001..C-006 e N-001..N-012 sem
definir como representá-los.

Sem um formato de entrada, o corpus teria de ser código Rust (builder API), o que
o acopla à implementação e dificulta revisão por humanos e por subagentes com
contexto novo.

## Decisão

Definir a gramática EXP-0-TEXT em `docs/CONTRATOS/TEXTO-EXP-0.md` como formato
**de teste e de CLI**, explicitamente não normativo para a RIR e para qualquer
formato wire:

- corpus em arquivos `.rvm` com expectativas em comentários (`; expect:`);
- parser com limites de bytes e contagens, tratamento hostil e ausência de pânico;
- impressão canônica reparseável, usada por `rsuvm fmt`;
- nenhuma promessa de compatibilidade entre revisões do protótipo.

## Alternativas

1. **Corpus como código Rust** (`rvm_ir::testing` estendido). Menor superfície de
   ataque, mas corpus acoplado à API e ilegível para revisor de especificação.
2. **JSON.** Exigiria parser/serializador próprio ou dependência externa
   (proibida sem ADR); verboso para revisão manual.
3. **S-expression.** Boa legibilidade, mas exigiria definir árvore genérica e
   validação semântica separada, com mais código que a gramática atual.

## Consequências

- Positivas: corpus inspecionável e versionável; CLI com entrada legível; testes
  negativos de nível textual (N-005, N-006) ficam expressáveis.
- Negativas: o parser passa a ser superfície de ataque documentada no modelo de
  ameaça; a impressão canônica renomeia definições (`v<id>`) e pode renumerar
  valores de módulos construídos por API/bytecode.
- Risco: divergência entre gramática documentada e parser. Mitigação: testes de
  gramática, round-trip e corpus; divergência vira relatório, não edição de golden
  file.

## Validação

- Corpus C-001..C-006 e N-001..N-012 executado por `cargo test --workspace`.
- Round-trip `parse(print(parse(texto)))` preservando estrutura e IDs para módulos
  originados do texto.
- Testes de entrada hostil em `crates/rvm-text/tests` sem pânico.
- Reavaliar se o formato exigir açúcar sintático (strings, arrays) ou se o corpus
  precisar expressar módulos com imports.
