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
- **Runner dedicado (GitHub Actions, run `41428569139`)**: dispersão no ponto
  classificador (k=256) = **0,28%**; overhead relativo **−0,0269** ⇒ **H06
  confirmada** (limiares pré-registrados inalterados). Custo de ligação ≈ 11 317 ns.
- Na máquina de desenvolvimento (não isolada) a dispersão foi até 65% e H06 ficou
  inconclusiva; addendum em `BENCHMARKS-PREREGISTRADOS.md` §5.3 explicita que a
  classificação usa o ponto classificador (maior `k`).
- Caminho de runner dedicado: workflow **manual** `.github/workflows/bench.yml`
  (GitHub-hosted, `workflow_dispatch`) publica o log como artefato `bench-log`.

### 2. Fuzzing guiado por cobertura (implementado)

- `docs/ADRs/ADR-0020-FUZZING-GUIADO-POR-COBERTURA.md` (**Aceito
  experimentalmente**): fixa `libfuzzer-sys` **0.4.13** (licença
  `(MIT OR Apache-2.0) AND NCSA`) e `cargo-fuzz` **0.13.2** (licença
  `MIT OR Apache-2.0`), uso de **nightly**, isolamento em `fuzz/`.
- Implementado: `fuzz/` como workspace separado (`exclude = ["fuzz"]` na raiz),
  alvos `texto`/`decoder`/`verificador`, `fuzz/Cargo.lock` fixado, e job
  `.github/workflows/fuzz.yml` (nightly, Linux).
- Verificado: `cargo +nightly fuzz build` **compila**; `cargo test --workspace`
  segue verde **sem** compilar `fuzz/`. Execução local no Windows falha por falta
  da DLL de runtime do ASan (`STATUS_DLL_NOT_FOUND`) — o caminho de execução é o
  job de CI em Linux. **Job `fuzz.yml` no CI (run `41430066954`)**: `texto`
  4 499 017, `decoder` 8 440 896, `verificador` 4 267 947 execuções; **0 crashes**
  em 120 s por alvo (≈17,2 M execuções).

### 3. Licença de Rio e Lift (declarada)

- `docs/RELATORIOS/LICENSEAMENTO-RIO-LIFT.md`: os três projetos têm o **mesmo
  autor**, então a licença é decisão do titular. O mantenedor **declarou
  `MIT OR Apache-2.0`** em 2026-11-02.
- **Lift**: `LICENSE-MIT`, `LICENSE-APACHE` e campo `license` em todos os
  manifestos; commit `5e59af5` publicado em `main`.
- **Rio**: commit `fb1a310` **publicado em `main`** (`git.prodam.am.gov.br`).
- ADR-0015/ADR-0017 atualizados: o bloqueio de **licença** caiu; os demais
  bloqueios técnicos (modelo não-VM, deps externas, subset) permanecem.

### Verificação (Resultado)

| Comando | Resultado |
|---|---|
| `cargo bench -p rvm-interpreter --bench chamada` | dev: dispersão 65% (inconclusiva); runner dedicado: H06 **confirmada** |
| `cargo +nightly fuzz build` | exit 0 (compila os três alvos) |
| `cargo build --workspace --all-targets` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |

## Adaptador Lift→RIR (ADR-0021)

Primeiro frontend real do repositório: `frontends/lift-rir`, workspace **separado**
(`exclude = ["frontends"]`), que consome a **HIR pública do Lift** (commit
`5e59af5`) e emite a RIR do EXP-0 para funções puras `int`/`boolean`/`void`.

### Entregue

- `frontends/lift-rir/src/lib.rs` — `lower_source`/`lower` + `AdapterError`;
  lowering CFG/SSA: `let`/atribuição, `if`/`else if`/`else` (com merge), `while`
  (phi no cabeçalho), `+ - * / %`/comparações/lógica curto-circuitada/chamadas.
- `frontends/lift-rir/tests/diferencial.rs` — 18 testes rodando o pipeline completo
  Lift→RIR→verificação→execução (inclui `while`, merge, `else if`, `&&`/`||`/`!` e
  `/`/`%`).
- Binário **`lift-run`** (`frontends/lift-rir/src/bin/lift_run.rs`): roda um
  `.lift` ponta a ponta na RVM, imprime o valor no formato do `rsuvm` e usa os
  mesmos códigos de exit (0/2/3/4/1). Testes em `tests/cli.rs` (8).
- Núcleo: **ADR-0022** adiciona `div_i64_checked`/`rem_i64_checked` e o trap
  `division_by_zero` (spec + bytecode + intérprete + corpus C-020/C-021/N-025/N-026).
- ADR-0021, READMEs e relatório `docs/RELATORIOS/ADAPTADOR-LIFT-RIR.md`.

### Teste diferencial (Resultado)

Oráculo do Lift: `cargo run -p lift_cli -- run examples/rsuvm-oracle.lift` →
`30 5 7 9 3 true false 25 12 55 0`. A RSUVM, sobre a RIR gerada, produz os mesmos
11 valores (`i64`/`bool`). Detalhes em `ADAPTADOR-LIFT-RIR.md`.

| Comando | Resultado |
|---|---|
| `cargo test --manifest-path frontends/lift-rir/Cargo.toml` | 31 passed, 0 failed (22 diferencial + 9 CLI) |
| `cargo run --bin lift-run -- fib.lift fib --arg i64:10` | `i64:55` (exit 0) |
| oráculo do Lift (`rsuvm-oracle*.lift`) | `30 5 7 9 3 true false 25 12 55 0` / `15 0 7 0 1 2 10 3 6 0` / `3 -3 2 -2` / `-1 0 1 true false false true false true false true true true` / `ola true false true` / `ola mundo` / `1.5 true false` / `20` / `9 14 6` / `7 ana 42` / `hi missing 7 9 5 true false` / `5` / `true false true false` / `11 3 bia` |
| `cargo test --workspace` (núcleo) | verde; **não** compila `frontends/` |

Limites: o subconjunto do adaptador ainda **não** cobre strings/arrays/objetos,
closures, `console.log`/`import` nem `coalesce`; dependência git fixada do Lift no
adaptador; sem codegen reverso.

## Ampliações pós-adaptador (ADRs 0022–0024)

### `else if`, lógica, curto-circuito e `string` (ADR-0021)

Adaptador refatorado para um construtor **CFG/SSA stateful**: `if`/`else if`/`else`,
`&&`/`||` (curto-circuito) e `!`. Oráculo do Lift (`rsuvm-oracle4.lift`):
`-1 0 1 true false false true false true false true true true` — bate com a RSUVM
(inclui curto-circuito sobre divisão).

`string` do Lift é mapeado (literal, `==`/`!=` por conteúdo, `+` → `concat_str`,
parâmetros/retorno). Oráculos (`rsuvm-oracle5.lift`/`oracle6.lift`):
`ola true false true` / `ola mundo` — batem.

### Divisão, resto e bit a bit (ADRs 0022–0023)

Núcleo ganhou `div/rem_i64_checked` (+ trap `division_by_zero`),
`shl/shr_i64_checked` (+ trap `invalid_shift`) e `and/or/xor_i64`; corpus
`C-020..C-026` e `N-025..N-027`. O adaptador mapeia `/` e `%`; oráculo
(`rsuvm-oracle3.lift`): `3 -3 2 -2`.

### Tipo `string` (ADR-0024) e concatenação (ADR-0025)

Núcleo ganhou `string` (literais `"..."` com escapes), `const_str`, `str_eq`,
`str_ne`, `str_len`, `concat_str`, limite `max_string_bytes` e corpus
`C-027..C-031` + `N-028`. A concatenação cobra os bytes na quota de memória
(cumulativo → `out_of_memory`) e limita o resultado (`string_too_long`). `Value`
deixou de ser `Copy`.

### Tipo `f64` (ADR-0026)

Núcleo ganhou `f64` (IEEE-754, sem trap): `const_f64`, `add/sub/mul/div_f64`,
`eq/ne/lt/le/gt/ge_f64`; corpus `C-032`/`C-033`. O adaptador mapeia `number` do
Lift (literal, aritmética, comparações, negação). Oráculo (`rsuvm-oracle7.lift`):
`1.5 true false` — bate. `Value` deixou de implementar `Eq` (NaN).

### Arrays imutáveis (ADR-0027)

Núcleo ganhou `array` (literal não-vazio e homogêneo), `array_len` e `array_get`
com **bounds check** (trap `index_out_of_bounds`); quota cumulativa `len × 24` e
limite `max_array_len`. Corpus `C-034`/`C-035`/`N-029`. O adaptador mapeia
literais `[...]` e indexação `xs[i]`; oráculo (`rsuvm-oracle8.lift`): `20` — bate.

### Mutação de array por atualização funcional (ADR-0028)

Núcleo ganhou `array_set` (atualização **funcional**: devolve array novo, origem
inalterada; sem referências nem aliasing), com bounds check (`index_out_of_bounds`)
e quota cumulativa `len × 24`. Corpus `C-036`/`C-037`/`C-038`/`N-030`/`N-031`. O
adaptador mapeia `xs[i] = v` quando `xs` é um local não-aliasado (rejeita alias);
oráculo (`rsuvm-oracle9.lift`): `9 14 6` — bate.

### Objetos/structs (ADR-0029)

Núcleo ganhou `Type::Struct` + tabela de structs no módulo, `Value::Struct` e as
operações `struct_new`/`field_get` (campos **escalares**; sem `field_set` nem
aninhamento); quota cumulativa `campos × 24`. Corpus
`C-039`/`C-040`/`C-041`/`N-032`/`N-033`/`N-034`. O adaptador mapeia `interface`
Lift, o literal `{ campo: valor }` e `obj.campo`; oráculo (`rsuvm-oracle10.lift`):
`7 ana 42` — bate.

### `Option`/`coalesce` (ADR-0030)

Núcleo ganhou `Type::Optional` (payload escalar) e as operações `option_none`,
`option_some`, `option_is_some`, `option_unwrap` (trap `unwrap_none`) e
`coalesce`. Corpus `C-042`..`C-046`/`N-035`..`N-037`. O adaptador mapeia `null`,
`option.some`/`is_some`/`unwrap` e baixa `??` com **fallback preguiçoso** via fluxo
de controle; oráculos (`rsuvm-oracle11.lift` = `hi missing 7 9 5 true false`,
`rsuvm-oracle12.lift` = `5`) — batem.

### Igualdade de `option` (ADR-0031)

Núcleo ganhou `option_eq`/`option_ne` (mesmo payload; `none == none` é `true`,
`some` compara payloads). Corpus `C-047`..`C-049`/`N-038`. O adaptador mapeia
`x == null`/`x != null` (e `==`/`!=` entre opcionais); oráculo
(`rsuvm-oracle13.lift`): `true false true false` — bate.

### Mutação de campo por atualização funcional (ADR-0032)

Núcleo ganhou `field_set` (atualização **funcional**: devolve struct nova, origem
inalterada), com verificação de campo/tipo e quota cumulativa `campos × 24`. Corpus
`C-050`..`C-052`/`N-039`/`N-040`. O adaptador mapeia `obj.campo = v` quando `obj` é
uma struct local não-aliasada (análise de aliasing generalizada); oráculo
(`rsuvm-oracle14.lift`): `11 3 bia` — bate.

### `Result` no núcleo (ADR-0033)

Núcleo ganhou `Type::Result` (payloads escalares) e as operações `result_ok`,
`result_err`, `result_is_ok`, `result_unwrap`, `result_unwrap_err` (trap
`unwrap_error`); o enum escalar foi unificado em `ScalarKind` (`ArrayElem`/
`OptionalElem` viraram aliases). Corpus `C-053`..`C-055`/`N-041`/`N-042`. O
adaptador **não** constrói `Result` (não há fonte no subconjunto Lift: só
`result.is_ok`/`unwrap`/`unwrap_err`, que exigem um valor de host); o **`Option`
aninhado** fica adiado (exige payload não-escalar em `Type`, isto é, uma
representação de tipos não-`Copy` ou uma tabela de tipos no módulo).

### Representação de tipos por id — ADR-0034 (proposta) e `?` (bloqueado)

A ADR-0034 registra a decisão (não implementada) de trocar o enum `Type: Copy` por
uma **tabela de tipos** no módulo (`TypeId`), com `array`/`struct`/`option`/
`result` recursivos por id, validação de ciclos e migração faseada. É o caminho
para destravar `Option`/`Result` de agregados e aninhados.

A propagação `?` no adaptador está **bloqueada na origem**: o Lift não tem o
operador `?` (só `T?` de tipo; `UnaryOperator` = `Negate|Not`; a documentação do
Lift declara `?` não implementado). Não se inventou sintaxe. O padrão de `?` é
expressável na RIR e está demonstrado no corpus: `C-056` (`Ok` → `ok:i64:12`) e
`C-057` (`Err` → `err:str:e`). Detalhes e evidências em
`docs/RELATORIOS/PROPAGACAO-INTERROGACAO.md`.

### Tipos por id — ADR-0034 (implementada)

O enum `Type: Copy` foi substituído por uma **tabela de tipos** no módulo
(`Module.types`), com `TypeId`/`TypeDef` e builtins canônicos (`0..=4`); tipos
compostos são recursivos por id e o verificador rejeita **ciclos**. `Value` carrega
`TypeId`, e texto/bytecode/adaptador foram convertidos. Isso destrava `option`
aninhado, `option` de array/struct e structs com campos agregados. Corpus
`C-058` (option aninhado), `C-059` (option de array), `C-060` (option de struct),
`N-043` (ciclo). O adaptador pré-interna tipos e cobre `option` de array/struct
(oráculo `rsuvm-oracle15.lift` = `2 7`); o Lift não expressa opção aninhada.

A nota sobre `?` continua válida: a propagação não existe no Lift
(`docs/RELATORIOS/PROPAGACAO-INTERROGACAO.md`); o núcleo a expressa no CFG
(`C-056`/`C-057`).

### Quota de memória por valor retido (ADR-0035)

O modelo cumulativo de arrays/strings foi substituído por bytes **retidos por
slot**: `Value::retained_bytes` (string = bytes; array/struct = `itens × 24` +
filhos; option/result = `24` + payload); a gravação de um slot cobra o valor novo
e libera o anterior, e o retorno do frame libera os slots. Laços com alocação
passam a ser limitados pela memória viva. Corpus `C-061` (100 iterações, 8 bytes
por iteração, `memory: 700`); `N-028` continua trapando. `memory_peak` passa a ser
pico de memória viva.

### Benchmark pré-registrado — reexecução (item 2)

O benchmark de chamada (`cargo bench -p rvm-interpreter --bench chamada`) foi
reexecutado no commit `89c21dc` (ADR-0034/0035), com o método e limiares
inalterados. Nesta máquina (Windows, i7-9700, rustc 1.94.1), duas execuções deram
dispersão de 57,8% e 31,8% no ponto classificador: **H06 inconclusiva**; custo de
ligação ≈24–26 µs. A confirmação em runner dedicado veio dos runs `41429076866`
(dispersão 1,33%) e `41520845695` (dispersão 0,45%, overhead relativo −0,0047,
commit `85af2e7`): **H06 CONFIRMADA**. Relatório:
`docs/RELATORIOS/BENCHMARK-REEXECUCAO-2026-11-02.md`.

### ABI de host com capabilities (item 3)

A ABI mínima fechada é `read_file(path: string, max_bytes: i64) -> result string
string`, com efeito `filesystem_read`, escopo no argumento 0 e código `0`/opcode
`54` no bytecode (ADR-0036). `execute` sem host nega toda chamada; o intérprete
confere a capability **antes** de chamar o backend e revalida o tipo do retorno.
O CLI concede com `--allow-fs-read <dir>` (absoluto, existente, canonicalizado;
leitura limitada a 1 MiB) e sai com 5 sem a flag, fora do escopo, com `..` ou
symlink para fora (ADR-0037). Testes: 5 em `rvm-interpreter/tests/host.rs`, 10 no
CLI (9 no Windows; o de symlink é `cfg(unix)`), round-trip de texto e o caso de
corpus `C-062` (verifica + nega sem host). A leitura concedida não é executada
pelo corpus; fica nos testes dedicados.

### Verificação (Resultado)

| Comando | Resultado |
|---|---|
| `cargo test --workspace` | **170 passed, 0 failed** no Windows (o teste de symlink é `cfg(unix)`; 171 no Linux); corpus 62 válidos + 42 inválidos |
| `cargo test --manifest-path frontends/lift-rir/Cargo.toml` | 43 passed, 0 failed (34 diferencial + 9 CLI) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
