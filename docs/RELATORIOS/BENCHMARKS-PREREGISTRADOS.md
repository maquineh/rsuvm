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
| H06 | custo de chamada inter-módulo ≤ 25% | **inconclusiva** (dispersão > 10%; ver §5) |

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

Execução de referência (máquina de desenvolvimento **não isolada**): 11 amostras ×
20 000 execuções por ponto, com aquecimento longo, medianas.

| k | fuel | intra ns | inter ns | H06 rel | dispersão |
|---:|---:|---:|---:|---:|---:|
| 1 | 6 | 320,2 | 339,6 | — | 0,2133 |
| 4 | 18 | 837,7 | 930,7 | 0,1423 | 0,6512 |
| 16 | 66 | 2 949,6 | 3 225,4 | 0,0865 | 0,2419 |
| 64 | 258 | 11 663,4 | 10 966,7 | −0,1116 | 0,1220 |
| 256 | 1 026 | 43 514,1 | 44 777,2 | 0,0615 | 0,2533 |

- `fuel` idêntico entre variantes (controle satisfeito: `4k + 2`).
- Dispersão máxima observada: **0,6512 (65%)**.
- Custo de ligação (k = 64): ≈ 24 000 ns por ligação, pago uma vez, fora da
  execução.
- Uma execução anterior (7 amostras × 5 000 execuções) teve o mesmo desfecho;
  endurecer a medição (mais amostras/repetições e aquecimento) **não** reduziu a
  dispersão abaixo de 10% nesta máquina.

Classificação pela regra pré-registrada (§3):

- **H06 — inconclusiva.** A dispersão excede 10% do valor mediano em todos os
  pontos, então a regra pré-registrada declara inconclusiva — **não** confirmada
  nem refutada. Nenhum limiar foi ajustado. Como a ligação estática torna o código
  executado idêntico ao intra-módulo, a diferença esperada é ~0; o ambiente
  compartilhado é ruidoso demais para resolver essa diferença.
- **H03 — confirmada estruturalmente.** A ligação resolve `call_static` para um
  índice global antes da verificação; não há passo de serialização por chamada
  (ADR-0018). A evidência de medição de H03 fica subordinada à estabilidade de
  H06.

### Classificação em runner dedicado

Para reduzir o ruído, há um workflow **manual** em
`.github/workflows/bench.yml` (GitHub-hosted, disparo por `workflow_dispatch`):
ele roda `cargo bench -p rvm-interpreter --bench chamada` e publica o log como
artefato `bench-log`. A classificação de H06 deve ser lida desse log; enquanto o
resultado vier com dispersão > 10%, permanece inconclusiva.

## 6. Riscos e limitações

1. Medição de tempo depende da máquina e do SO; a dispersão observada (até 65%)
   impediu classificar H06 nesta rodada. Há workflow manual para runner dedicado
   (`.github/workflows/bench.yml`).
2. Como a ligação estática elimina a fronteira de módulo em tempo de execução, o
   "custo inter-módulo" medido é apenas a diferença de indireção/renomeação, que
   tende a zero por construção; o custo real do desenho é a ligação (uma vez),
   medida separadamente.
3. O benchmark é de microcusto: não mede carga real, concorrência nem cache.
4. Nenhuma conclusão de desempenho de produção é feita; o protótipo não é
   sandbox nem promessa de performance.

## 7. Próximos passos

1. Repetir em runner dedicado/isolado, com mais amostras e mediana, para reduzir
   a dispersão e classificar H06.
2. Se o caso de uso for carga única, medir o custo total de ligação + execução.
3. Manter os limiares da seção 3 sem ajuste pós-hoc.
