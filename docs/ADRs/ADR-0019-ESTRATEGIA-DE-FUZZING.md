# ADR-0019 — Estratégia de fuzzing do EXP-0

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** etapa B2 (lacuna remanescente de §9 do
  `RELATORIO-IMPLEMENTACAO.md`); lacunas 3 e 8 ("fuzzing limitado", "parser é
  superfície de ataque"); `AGENTS.md` regra 2 (zero dependências externas).

## Contexto

O relatório de implementação registrou fuzzing **limitado e determinístico** e
que o parser textual é superfície de ataque. A etapa B2 pede ampliar a campanha.
Fuzzers guiados por cobertura usuais (`cargo-fuzz`/libFuzzer, AFL) exigem uma
**dependência externa de desenvolvimento** (`libfuzzer-sys`) e, em geral, toolchain
**nightly** com `-Z sanitizer-coverage`.

A política do repositório é **zero dependências externas**, inclusive de
desenvolvimento, e o CI roda em toolchain **estável**. Adicionar `cargo-fuzz`
quebraria ambos os invariantes sem ADR específico e aprovação do mantenedor.

## Decisão

1. A campanha de fuzzing do EXP-0 permanece **determinística, em processo e
   orçada** (sem fuzzer guiado por cobertura nesta rodada).
2. A campanha é **ampliada** em `crates/rvm-conformance/tests/fuzzing.rs`:
   - 6 sementes fixas (antes 4) e 200 mutações por base (antes 100);
   - quatro alvos: parser textual, decoder binário, verificador (+execução
     limitada) e **linker** (`rvm-link`, ADR-0018).
3. A adoção de um fuzzer guiado por cobertura exige **ADR próprio**, com licença
   verificada, versão fixada e aprovação explícita do mantenedor (regra 12 de
   `AGENTS.md`); fica registrada como próximo passo, não como decisão.
4. O orçamento é medido em **número de operações** (determinístico), não em tempo
   de parede.

## Alternativas

1. **`cargo-fuzz`/libFuzzer:** feedback de cobertura real, porém dependência
   externa + nightly; rejeitado nesta rodada.
2. **AFL++ via processo externo:** fora do build Cargo; rejeitado.
3. **Sem fuzzing:** descartado — a campanha determinística já cobre parser,
   decoder, verificador e linker.

## Consequências

- Positivas: reprodutível, sem dependências, roda no CI estável; amplia alvos e
  orçamento; cobre o linker novo.
- Negativas: **não** tem feedback de cobertura; não explora caminhos que a mutação
  aleatória não alcança; "zero pânico" descreve a rodada, não ausência de bug.
- Residual: uma campanha guiada por cobertura (ou por tempo, em runner dedicado)
  continua pendente de aprovação.

## Validação

- `crates/rvm-conformance/tests/fuzzing.rs`: quatro alvos, orçamento declarado e
  cumprido (asserção de total), sem pânico observado.
- Registro da rodada em `docs/RELATORIOS/FUZZING-EXP-0.md`.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
