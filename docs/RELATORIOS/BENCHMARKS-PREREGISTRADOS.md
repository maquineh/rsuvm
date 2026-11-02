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

Medianas de 7 amostras × 5 000 execuções por ponto:

| k | fuel | intra ns | inter ns | H06 rel | dispersão |
|---:|---:|---:|---:|---:|---:|
| 1 | 6 | 301,4 | 300,3 | — | 0,1460 |
| 4 | 18 | 799,2 | 806,0 | 0,0159 | 0,0487 |
| 16 | 66 | 2 967,8 | 2 775,3 | −0,0919 | 0,1834 |
| 64 | 258 | 10 966,8 | 10 983,9 | 0,0262 | 0,3884 |

- `fuel` idêntico entre variantes (controle satisfeito: `4k + 2`).
- Dispersão máxima observada: **0,3884 (38,8%)**.
- Custo de ligação (k = 64): **≈ 46 420 ns por ligação**, pago uma vez, fora da
  execução.

Classificação pela regra pré-registrada (§3):

- **H06 — inconclusiva.** A dispersão excede 10% do valor mediano, então a regra
  pré-registrada declara inconclusiva, e **não** confirmada nem refutada. O
  overhead relativo oscila dentro do ruído; como a ligação estática torna o código
  executado idêntico ao intra-módulo, a diferença esperada é ~0, mas a medição
  nesta máquina não é estável o bastante para decidir dentro de 10%.
- **H03 — confirmada estruturalmente.** A ligação resolve `call_static` para um
  índice global antes da verificação; não há passo de serialização por chamada
  (ADR-0018). A evidência de medição de H03 fica subordinada à estabilidade de
  H06.

## 6. Riscos e limitações

1. Medição de tempo depende da máquina e do SO; a dispersão observada (até 38%)
   impediu classificar H06 nesta rodada.
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
