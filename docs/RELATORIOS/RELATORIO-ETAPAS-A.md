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
| B1 | lacuna §9 (H03/H06) | Ligação estática inter-módulo (crate `rvm-link`, ADR-0018) |
| B2 | lacuna §9 (fuzzing/benchmark) | Fuzzing guiado por cobertura + benchmark inter-módulo |

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

- Auditoria executada: `docs/RELATORIOS/BASELINE-LIFT.md` (LIFT-BASELINE-001),
  commit imutável `691c344880ac85056bc4bc85bf1e513654769bb1`.
- Licença **desconhecida** (sem `LICENSE`/`COPYING`/`NOTICE`, sem campo `license`);
  AST/HIR públicas, mas execução por compilação nativa + Cargo (não VM); 6
  dependências externas; subset incompatível.
- Decisão: integração Lift **bloqueada** para este repositório —
  `docs/ADRs/ADR-0017-INTEGRACAO-LIFT-BLOQUEADA.md`. ADR-0007 da fundação
  preservado intacto. Nenhum código do Lift foi copiado.

### Verificação (Resultado)

| Comando | Resultado |
|---|---|
| `git ls-remote https://github.com/maquineh/lift` | `HEAD = 691c3448…` |
| inspeção estática (arquivos, deps, `unsafe`, CI) | registrada em `BASELINE-LIFT.md` |
| `cargo test --workspace` | 134 passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |

Limite: a auditoria é **inspeção estática**; o Lift não foi compilado nem
executado, e a licença continua pendente de decisão do mantenedor.

## B1 — Ligação estática inter-módulo

Lacuna remanescente do §9: H03/H06 não medidas por falta de ligação entre
módulos. Decisão e limites em
`docs/ADRs/ADR-0018-LIGACAO-ESTATICA-INTER-MODULO.md`.

### Entregue

- **Declaração externa** na RIR: `Function` sem blocos (`Function::is_external`);
  `structure::check` aceita externas, mas a entrada não pode ser externa;
  `rvm-verify` rejeita módulos com externas não linkadas.
- **Formato textual**: `import <nome>` declara a externa; a impressão canônica
  reemite `import`.
- **Novo crate `rvm-link`** (10º crate): `link(modules, entry_module,
  entry_function, limits)` resolve externas por nome único, reescreve `call_static`
  para IDs globais, renomeia funções para unicidade e produz um `Module` plano sem
  externas. Sem serialização (H03).
- Testes em `crates/rvm-conformance/tests/linking.rs` (ligação, diferencial de
  bytecode, externa sem provedor, provedor ambíguo, nome de módulo duplicado,
  verificação de módulo não linkado) e em `rvm-ir`/`rvm-text`.
- Contratos e arquitetura atualizados: `docs/CONTRATOS/RIR-EXP-0-IMPLEMENTACAO.md`,
  `docs/CONTRATOS/TEXTO-EXP-0.md`, `docs/ARQUITETURA.md`, `crates/README.md`.

### Verificação (Resultado)

| Comando | Resultado |
|---|---|
| `cargo test --workspace` | **143 passed, 0 failed** (antes: 134) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `linking.rs` | liga `app`+`prov`, verifica e executa `i64:30`; diferencial de bytecode ok; falhas fechadas |

Limite: resolução por **nome único** (sem namespaces qualificados nem ligação
dinâmica); os nomes das funções mudam no módulo plano. H06 continua dependendo de
executar o benchmark (B2).

## B2 — Fuzzing e benchmark ampliados

Lacunas remanescentes de §9: fuzzing limitado (3, 8) e H06/H03 sem medição (2).
Decisão de fuzzing em `docs/ADRs/ADR-0019-ESTRATEGIA-DE-FUZZING.md`.

### Entregue

- **Fuzzing ampliado** (`crates/rvm-conformance/tests/fuzzing.rs`): 6 sementes
  (antes 4) × 200 mutações (antes 100) em **quatro** alvos — parser textual,
  decoder binário, verificador e **linker** (`rvm-link`).
- **Benchmark inter-módulo** (`crates/rvm-interpreter/benches/chamada.rs`): mede a
  variante inter (via `rvm-link`) contra a intra, com 7 amostras × 5 000 execuções,
  mediana e dispersão, e aplica o limiar pré-registrado.
- Registros atualizados: `docs/RELATORIOS/FUZZING-EXP-0.md` e
  `docs/RELATORIOS/BENCHMARKS-PREREGISTRADOS.md`.

### Verificação (Resultado)

| Comando | Resultado |
|---|---|
| `cargo test -p rvm-conformance --test fuzzing` | 4 passed; texto 22 800, bytecode 33 600, verificador 33 600 (8 599 aceitos), linker 34 800 (5 160 ligados), **0 pânico** |
| `cargo bench -p rvm-interpreter --bench chamada` | executou; H06 **inconclusiva** (dispersão 38,8% > 10%); custo de ligação ≈ 46 420 ns |
| `cargo test --workspace` | **144 passed, 0 failed** |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |

### Resultado por hipótese

- **H03 — confirmada estruturalmente**: a ligação estática resolve `call_static`
  para índice global; não há serialização por chamada (ADR-0018).
- **H06 — inconclusiva**: a dispersão da medição nesta máquina excedeu o limiar
  pré-registrado de 10%; nenhum limiar foi ajustado. Classificação exige runner
  isolado ou mais amostras.

Limites: fuzzing **não** guiado por cobertura (ADR-0019); H06 pendente de medição
estável.

## Próximos passos — execução

### 1. Benchmark em runner dedicado (H06)

- Harness endurecido: 11 amostras × 20 000 execuções, `k` até 256, aquecimento
  longo, mediana e dispersão (`crates/rvm-interpreter/benches/chamada.rs`).
- Resultado na máquina de desenvolvimento (**não isolada**): dispersão máxima
  **65%**; H06 permanece **inconclusiva** pela regra pré-registrada. Nenhum limiar
  foi ajustado.
- Caminho de runner dedicado: workflow **manual** `.github/workflows/bench.yml`
  (GitHub-hosted, `workflow_dispatch`) publica o log como artefato `bench-log`.

### 2. Fuzzing guiado por cobertura (decisão)

- `docs/ADRs/ADR-0020-FUZZING-GUIADO-POR-COBERTURA.md` (**Proposto**): fixa
  `libfuzzer-sys` **0.4.13** (licença `(MIT OR Apache-2.0) AND NCSA`) e
  `cargo-fuzz` **0.13.2** (licença `MIT OR Apache-2.0`), uso de **nightly**,
  isolamento em `fuzz/` como workspace separado e excluído.
- **Não implementado**: exige aprovação explícita do mantenedor (regra 12).
  A campanha determinística do ADR-0019 permanece.

### 3. Licença de Rio e Lift

- `docs/RELATORIOS/PEDIDO-LICENCA-RIO-LIFT.md`: pedido pronto para envio, com
  fatos, usos contemplados e perguntas. É **ação humana**; nada foi copiado.

### Verificação (Resultado)

| Comando | Resultado |
|---|---|
| `cargo bench -p rvm-interpreter --bench chamada` | executou; dispersão até 65%; H06 inconclusiva |
| `cargo build --workspace --all-targets` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
