# rvm-conformance — Package Contract

## Overview

`rvm-conformance` reúne o corpus independente da implementação e os testes que o
executam. O pacote de biblioteca não expõe API de runtime: `src/lib.rs` contém
apenas a documentação de crate que fixa o papel do corpus. O comportamento
observável é verificado em `tests/conformance.rs` (pipeline e diferencial
`RIR ↔ bytecode`) e `tests/properties.rs` (fuel exato, determinismo, truncamento,
mutação), sobre arquivos `.rvm` em `corpus/valid` e `corpus/invalid`.

## Interface

- Nenhum item público de biblioteca: o crate é um contêiner de dados de teste e
  de alvos de integração (`src/lib.rs`).
- Corpus: arquivos `.rvm` com expectativa declarada em comentário
  (`; id:`, `; expect:`), conforme [corpus/README.md](../corpus/README.md).
- Alvos de teste (não são contratos de código):
  `tests/conformance.rs` — percorre o corpus e compara `value|error|trap|limit`;
  `tests/properties.rs` — propriedades e orçamentos declarados;
  `tests/coverage.rs` — meta de cobertura por operação
  (`docs/CONTRATOS/CORPUS-COBERTURA.md`);
  `tests/fuzzing.rs` — campanha determinística de parser/decoder/verificador
  (`docs/RELATORIOS/FUZZING-EXP-0.md`).

## Invariants

- O corpus é **dado**, não código: casos válidos (C-001..C-046) e inválidos
  (N-001..N-011, N-013..N-037) vivem em arquivos `.rvm`; N-012
  (truncamento/`length` excessivo) é exercitado no decoder binário
  (`crates/rvm-bytecode/tests/hostile.rs`). A meta de cobertura por operação está
  em `docs/CONTRATOS/CORPUS-COBERTURA.md` e é verificada por
  `tests/coverage.rs`.
- Toda expectativa `error <código>` precisa falhar **antes** da execução e com
  `ErrorClass::Module`; `trap` exige `ErrorClass::Trap`; `limit` exige
  `ErrorClass::ResourceLimit` (`tests/conformance.rs`).
- IDs de caso são únicos entre `valid/` e `invalid/`; todo arquivo precisa de
  `id:` e exatamente um `expect:`.
- Todo caso `value` é executado duas vezes e precisa repetir valor e fuel
  (C-006 generalizado), e também passa por `encode → decode → verify → execute`
  com o mesmo valor e fuel.
- Nenhuma expectativa ou golden file é ajustado para acomodar a implementação;
  divergência vira relatório em `docs/RELATORIOS/`.
- Os testes declaram orçamento (semente e número de mutações) e verificam apenas o
  que executam: "sem pânico observado" descreve a rodada, não ausência de
  vulnerabilidade.

## Side Effects

- `None` em produção: o crate não é usado por runtime algum.
- Em testes: lê os arquivos do corpus no próprio diretório do crate e não escreve
  arquivos. Sem rede, ambiente, relógio ou processos.

## Verification

- Run: `cargo test --workspace`
