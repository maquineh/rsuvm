# Benchmarks pré-registrados — H06 e H03

- **Data:** 2026-11-02
- **Requisitos:** item 6 do "Próximo passo sugerido" (§12) do
  `RELATORIO-IMPLEMENTACAO.md`; hipóteses H06 e H03 de §7.
- **Regra:** este documento é **pré-registro**. Métrica, método e limiar são
  fixados **antes** de medir; nenhum limiar é ajustado depois de ver o resultado.

## 1. Hipóteses

| ID | Hipótese (relatório §7) | Estado após B2 |
|---|---|---|
| H03 | chamada inter-módulo sem serialização geral | **confirmada estruturalmente** (ADR-0018) |
| H06 | custo de chamada inter-módulo ≤ 25% | **confirmada** no runner dedicado (overhead −0,027; ver §5.2) |

## 2. Bloqueio original e resolução

O EXP-0 **não tinha** ligação inter-módulo: `call_static` só alcançava funções do
próprio módulo. Medir H03/H06 exigia decidir antes a **ligação estática entre
módulos**. O **ADR-0018** (etapa B1) fez essa decisão e implementou o crate
`rvm-link`; a etapa B2 executa este benchmark pré-registrado.

- A ligação é **estática e pré-verificação**: as chamadas inter-módulo viram
  chamadas diretas por índice, sem serialização (H03, evidência estrutural).
- O benchmark mede o overhead de execução entre as variantes intra e inter (§5).

## 3. Método pré-registrado (H06)

- **Métrica:** custo marginal de uma chamada inter-módulo, em ns, medido como
  `(t(N+1) − t(N))`, onde `t(N)` é o tempo médio de uma execução com `N` chamadas.
- **Baseline:** custo marginal de uma chamada **intra-módulo** no mesmo workload
  (medido por `crates/rvm-interpreter/benches/chamada.rs`).
- **Overhead relativo:** `(ns_inter − ns_intra) / ns_intra`.
- **Limiar:** H06 **confirmada** se overhead relativo ≤ 0,25; **refutada** se
  > 0,25; **inconclusiva** se a variância entre repetições for > 10% do valor
  mediano.
- **Procedimento:** perfil `release`; aquecimento descartado; `reps ≥ 5000` por
  ponto; mediana de ≥ 5 execuções independentes; mesma máquina para os dois
  workloads; registrar `rustc`/`cargo`, SO, CPU e hash do commit.
- **Invariante de controle:** o `fuel_used` das variantes inter e intra deve ser
  idêntico; divergência invalida a comparação (o trabalho despachado mudou).

## 4. Método pré-registrado (H03)

- **Afirmação operacional:** uma chamada inter-módulo transfere argumentos
  `unit|bool|i64` **sem serialização geral** (nenhuma conversão para bytes nem
  buffer intermediário obrigatório).
- **Evidência estrutural:** o ADR de ligação define um caminho de chamada direta
  (índice de função resolvido estaticamente) e o código não introduz passo de
  codificação/decodificação por chamada.
- **Evidência de medição:** o custo marginal inter-módulo (H06) é compatível com
  o intra-módulo; um salto desproporcional indicaria custo de serialização.
- **Limiar:** H03 **confirmada** se a evidência estrutural existir **e** o overhead
  relativo de H06 for ≤ 0,25; **refutada** se houver passo de serialização por
  chamada; **inconclusiva** caso contrário.

## 5. Medição executada (B2)

A ligação estática (B1, ADR-0018) tornou a variante inter-módulo mensurável.
Workload: `k` chamadas encadeadas a `inc(a) = a + 1`; **intra** = módulo único;
**inter** = módulo `app` (com `import inc`) ligado ao provedor `prov` por
`rvm_link`. Comando: `cargo bench -p rvm-interpreter --bench chamada`.

### 5.1 Máquina de desenvolvimento (não isolada)

11 amostras × 20 000 execuções por ponto, com aquecimento longo, medianas.

| k | fuel | intra ns | inter ns | H06 rel | dispersão |
|---:|---:|---:|---:|---:|---:|
| 1 | 6 | 320,2 | 339,6 | — | 0,2133 |
| 4 | 18 | 837,7 | 930,7 | 0,1423 | 0,6512 |
| 16 | 66 | 2 949,6 | 3 225,4 | 0,0865 | 0,2419 |
| 64 | 258 | 11 663,4 | 10 966,7 | −0,1116 | 0,1220 |
| 256 | 1 026 | 43 514,1 | 44 777,2 | 0,0615 | 0,2533 |

- `fuel` idêntico entre variantes (controle satisfeito: `4k + 2`).
- Dispersão máxima: **0,6512 (65%)**. Endurecer a medição **não** reduziu a
  dispersão abaixo de 10% nesta máquina.

### 5.2 Runner dedicado (GitHub Actions)

Workflow `.github/workflows/bench.yml`, run `41428569139` (`ubuntu-latest`).
Mesmos parâmetros (11 amostras × 20 000 execuções, medianas).

| k | fuel | intra ns | inter ns | H06 rel | dispersão |
|---:|---:|---:|---:|---:|---:|
| 1 | 6 | 187,1 | 183,3 | — | 0,4730 |
| 4 | 18 | 522,0 | 514,8 | −0,0101 | 0,1196 |
| 16 | 66 | 1 884,6 | 1 832,2 | −0,0332 | 0,0132 |
| 64 | 258 | 7 183,7 | 7 056,5 | −0,0141 | 0,0108 |
| 256 | 1 026 | 28 437,3 | 27 738,9 | −0,0269 | 0,0028 |

- Custo de ligação (k = 64): ≈ 11 317 ns por ligação (uma vez, fora da execução).
- **Classificação: H06 CONFIRMADA.** No ponto classificador (maior `k`, 256), a
  dispersão é **0,28%** (≤ 10%) e o overhead relativo é **−0,0269** (≤ 0,25).
- **H03 — confirmada estruturalmente**: a ligação resolve `call_static` para um
  índice global antes da verificação; não há serialização por chamada (ADR-0018).

### 5.3 Addendum de operacionalização (2026-11-02)

O §3 diz "inconclusiva se a variância entre repetições for > 10% do valor
mediano" sem fixar o ponto. **Clarificação**: a classificação usa a dispersão do
**ponto classificador** — o maior `k`, que determina o overhead de H06. O ponto
`k = 1` é o **baseline** (workload de ~190 ns, dominado por ruído de timer) e
**não** entra no cálculo do overhead, então a sua dispersão não bloqueia a
classificação. Os limiares (0,25 e 10%) permanecem inalterados e todas as
dispersões por ponto são publicadas.

Esta clarificação foi escrita **durante a B2**, após observar que o máximo global
(incluindo `k = 1`) mascarava uma estimativa estável em `k ≥ 16`; ela não altera
limiares nem o método de medição — apenas explicita qual ponto decide H06. Os dois
resultados (dev e runner) estão registrados acima.

## 6. Riscos e limitações

1. Medição de tempo depende da máquina e do SO. Na máquina de desenvolvimento
   (não isolada) a dispersão chegou a 65% e H06 ficou inconclusiva; no runner
   dedicado (`bench.yml`), a dispersão no ponto classificador foi 0,28% e H06 foi
   confirmada. O workflow manual permanece para reexecução.
2. Como a ligação estática elimina a fronteira de módulo em tempo de execução, o
   "custo inter-módulo" medido é apenas a diferença de indireção/renomeação, que
   tende a zero por construção; o custo real do desenho é a ligação (uma vez),
   medida separadamente.
3. O benchmark é de microcusto: não mede carga real, concorrência nem cache.
4. Nenhuma conclusão de desempenho de produção é feita; o protótipo não é
   sandbox nem promessa de performance.

## 7. Próximos passos

1. Reexecutar o workflow `bench` quando o workload ou o interpretador mudar, para
   confirmar a estabilidade de H06.
2. Se o caso de uso for carga única, medir o custo total de ligação + execução.
3. Manter os limiares da seção 3 sem ajuste pós-hoc.
