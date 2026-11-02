# Relatório das etapas A — RSUVM

Este relatório acompanha a execução dos itens do "Próximo passo sugerido" (§12) do
`RELATORIO-IMPLEMENTACAO.md`, agrupados nas etapas A1–A4 e publicados por push em
`https://github.com/maquineh/rsuvm.git`.

Mapeamento aprovado pelo mantenedor:

| Etapa | Itens do §12 | Tema |
|---|---|---|
| A1 | 1–2 | Remoto/CI + licença do repositório e política de reuso do Rio |
| A2 | 3–4 | Ampliação do corpus/cobertura + quota de memória por frame |
| A3 | 5–6 | Campanha de fuzzing + benchmarks pré-registrados (H06/H03) |
| A4 | 7 | Auditoria do Lift ou fechamento do ADR-0007 como bloqueado |

Convenção: **Resultado** (executado, com comando) é separado de **Proposta** e
**Desconhecido**. Nada aqui foi auditado por terceiros.

## A1 — Remoto/CI, licença e política de reuso do Rio (itens 1–2)

### Entregue

- `LICENSE-MIT` e `LICENSE-APACHE` (Apache License 2.0 integral) na raiz.
- `license = "MIT OR Apache-2.0"` em `[workspace.package]` (`Cargo.toml`) e
  `license.workspace = true` nas nove crates.
- `docs/ADRs/ADR-0014-LICENCA-DO-REPOSITORIO.md` — decisão de licença dupla.
- `docs/ADRs/ADR-0015-POLITICA-DE-REUSO-DO-RIO.md` — Rio apenas como referência
  comportamental; proibido copiar código/AST; sem linkagem.
- Índice `docs/ADRs/README.md` atualizado; seção "Licença" no `README.md`.
- CI ganhou o passo de build do gate documentado em `AGENTS.md §4`
  (`cargo build --workspace --all-targets`): `.github/workflows/ci.yml` e
  `.gitlab-ci.yml`.

### Item 1 (remoto e CI verde)

O item 1 é de processo: manter o remoto atualizado e o CI verde a cada mudança.
Ele é atendido por este fluxo (commit + push por etapa) e pela verificação dos
gates antes de cada push. Os comandos e resultados estão em "Verificação" abaixo e
são repetidos em cada etapa.

### Item 2 (licença e reuso do Rio)

Decisões do mantenedor nesta rodada: licença **dupla MIT OR Apache-2.0**; reuso do
Rio **apenas como referência comportamental**. Registradas nos ADRs 0014 e 0015.

### Verificação (Resultado)

| Comando | Resultado |
|---|---|
| `cargo metadata --no-deps --format-version 1` | `license = "MIT OR Apache-2.0"` em cada crate |
| `cargo build --workspace --all-targets` | `Finished`, exit 0 |
| `cargo test --workspace` | exit 0 (120 testes verdes) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, sem warnings |
| `cargo fmt --all -- --check` | exit 0, sem diferenças |

## A2 — Corpus/cobertura e quota de memória (itens 3–4)

### Item 3 (ampliar o corpus e fixar meta de cobertura por operação)

- Casos válidos novos:
  - `C-017-encadeamento-de-merges.rvm` — um merge alimenta outro merge;
  - `C-018-aridade-maxima-de-chamada.rvm` — `call_static` com 16 parâmetros e 16
    argumentos (limites máximos);
  - `C-019-limite-de-parametros-de-bloco.rvm` — bloco com exatamente 8 parâmetros.
- Casos inválidos novos:
  - `N-022-parametros-de-bloco-em-excesso.rvm` — 9 > 8 ⇒ `invalid_module`;
  - `N-023-argumentos-de-chamada-em-excesso.rvm` — 17 > 16 ⇒ `invalid_module`;
  - `N-024-limite-de-buffer-de-argumentos.rvm` — quota de memória no limiar do
    buffer (ADR-0016) ⇒ `out_of_memory`.
- Meta de cobertura por operação: `docs/CONTRATOS/CORPUS-COBERTURA.md`, verificada
  por `crates/rvm-conformance/tests/coverage.rs` (toda operação do subset e todo
  terminador em ≥1 caso válido; códigos de erro centrais em ≥1 caso inválido).
- Limites estruturais: `rvm-ir/src/structure.rs` ganhou teste de fronteira exata
  (aceita no limite) e casos acima do limite para parâmetros de função, parâmetros
  de bloco, argumentos de chamada, argumentos de desvio, blocos e valores.
- Corpus agora: **19 válidos** (C-001..C-019) e **23 inválidos**
  (N-001..N-011, N-013..N-024).

### Item 4 (quota de memória e consumo por frame)

- `rvm-runtime` ganhou `arg_buffer_cost(args) = args × SLOT_BYTES` (saturado).
- O intérprete passou a cobrar o buffer transitório de argumentos de **chamada**
  (`Op::Call`, antes de `collect_args`) e de **desvio** (`br`/`cond_br`, em `jump`),
  liberando-o ao fim. O buffer de desvio deixou de ser reutilizado entre saltos,
  para que o modelo corresponda à memória viva.
- `RunOutcome::memory_peak` passa a incluir frames **e** buffers.
- Decisão e limites em `docs/ADRs/ADR-0016-MEMORIA-DE-BUFFERS-DE-ARGUMENTOS.md`;
  ADR-0013 atualizado para apontar a extensão.

### Verificação (Resultado)

| Comando | Resultado |
|---|---|
| `cargo test --workspace` | **131 passed, 0 failed** (antes: 120) |
| `cargo test --release --workspace` | 131 passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, sem warnings |
| `cargo fmt --all -- --check` | exit 0, sem diferenças |
| pico de `soma` com buffer | 704 = 328 (main) + 48 (2 args) + 328 (soma) |
| pico de `com_merge` com buffer de desvio | 424 = 400 (frame de 6 valores) + 24 |
| corpus N-024 (`memory: 703`) | `limit out_of_memory` (só falha por causa do buffer) |

Lacuna remanescente: o modelo ainda **não** mede RSS real nem a capacidade
reservada do `Vec`; é quota determinística, não medição de consumo.

## A3 — Fuzzing e benchmarks (itens 5–6)

### Item 5 (campanha de fuzzing)

- Novo alvo `crates/rvm-conformance/tests/fuzzing.rs`: campanha determinística
  sobre **parser textual**, **decoder binário** e **verificador** (com execução
  limitada do que verifica).
- Orçamento declarado: 4 sementes fixas × base × 100 mutações
  (texto 7 600; bytecode 11 200; verificador 11 200 operações).
- Registro da rodada: `docs/RELATORIOS/FUZZING-EXP-0.md` (ferramenta, alvos,
  sementes, corpus, orçamento, resultado, limitações).
- Resultado: nenhum pânico observado; 2 842 mutações ainda verificaram e foram
  executadas. Não é fuzzer guiado por cobertura (política de zero dependências).

### Item 6 (benchmarks pré-registrados)

- `docs/RELATORIOS/BENCHMARKS-PREREGISTRADOS.md`: método, métrica e limiar de
  H06/H03 fixados **antes** de medir.
- H03 e H06 permanecem **não medidas**: exigem decidir a ligação estática
  inter-módulo, que não existe no EXP-0 (item 6 reconhece a dependência).
- Substituto exploratório: `crates/rvm-interpreter/benches/chamada.rs`
  (`harness = false`), custo marginal de `call_static` **intra-módulo**
  (≈ 164–205 ns/chamada no ambiente registrado). Não classifica H03/H06.

### Verificação (Resultado)

| Comando | Resultado |
|---|---|
| `cargo test -p rvm-conformance --test fuzzing` | 3 passed, 0 failed; texto 7 600, bytecode 11 200, verificador 11 200 operações, 0 pânico |
| `cargo bench -p rvm-interpreter --bench chamada` | executou; ver tabela em `BENCHMARKS-PREREGISTRADOS.md` |
| `cargo build --workspace --all-targets` | compila o alvo de bench |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |

Lacuna remanescente: fuzzing não guiado por cobertura e benchmark inter-módulo
bloqueado pela ausência de ligação estática (ADR pendente).

## A4 — Lift (item 7)

A preencher na etapa A4.
