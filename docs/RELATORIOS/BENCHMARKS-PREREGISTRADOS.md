# Benchmarks pré-registrados — H06 e H03

- **Data:** 2026-11-02
- **Requisitos:** item 6 do "Próximo passo sugerido" (§12) do
  `RELATORIO-IMPLEMENTACAO.md`; hipóteses H06 e H03 de §7.
- **Regra:** este documento é **pré-registro**. Métrica, método e limiar são
  fixados **antes** de medir; nenhum limiar é ajustado depois de ver o resultado.

## 1. Hipóteses

| ID | Hipótese (relatório §7) | Estado atual |
|---|---|---|
| H03 | chamada inter-módulo sem serialização geral | **não testada** |
| H06 | custo de chamada inter-módulo ≤ 25% | **não medida** |

## 2. Bloqueio comum

O EXP-0 **não tem ligação inter-módulo**: `call_static` só alcança funções do
próprio módulo (`docs/ARQUITETURA.md` §3; contrato RIR). Medir H03/H06 exige
decidir antes a **ligação estática entre módulos** — decisão que ainda não existe
em ADR. Sem isso, um benchmark inter-módulo mediria um caminho que o protótipo não
tem.

O item 6 reconhece essa dependência ("chamada inter-módulo exige decidir linking
estático antes"). Portanto, nesta rodada:

- H03 e H06 permanecem **não medidas**;
- o método abaixo fica pré-registrado para quando a ligação existir;
- mede-se, como **substituto exploratório**, o custo de `call_static`
  **intra-módulo** (não classifica H03/H06).

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

## 5. Substituto exploratório medido nesta rodada

Workload: `k` chamadas encadeadas a `inc(a) = a + 1`, `k ∈ {1, 4, 16, 64}`.
Comando: `cargo bench -p rvm-interpreter --bench chamada`.

| k | ns/execução | fuel | ns/fuel |
|---:|---:|---:|---:|
| 1 | 296,6 | 6 | 49,433 |
| 4 | 911,1 | 18 | 50,618 |
| 16 | 3 086,3 | 66 | 46,763 |
| 64 | 10 941,6 | 258 | 42,409 |

Custo marginal observado: **≈ 205 ns** (k 1→4), **≈ 181 ns** (4→16),
**≈ 164 ns** (16→64) por chamada **intra-módulo**.

- Ambiente: Windows 10.0.26300, rustc 1.94.1, cargo 1.94.1, Intel Core i7-9700
  @ 3.00GHz.
- `fuel_used` é determinístico e cresce linearmente (`4k + 2`), como esperado.
- Estes números **não** classificam H03 nem H06: medem o custo de uma chamada
  dentro do mesmo módulo. Servem de baseline para o benchmark inter-módulo.

## 6. Riscos e limitações

1. Medição de tempo depende da máquina e do SO; só vale no ambiente registrado.
2. O benchmark é de microcusto: não mede carga real, concorrência nem cache.
3. H03 é parcialmente estrutural (presença/ausência de serialização); a parte
   estrutural depende do ADR de ligação ainda inexistente.
4. Nenhuma conclusão de desempenho de produção é feita; o protótipo não é
   sandbox nem promessa de performance.

## 7. Próximos passos

1. Decidir a ligação estática entre módulos em ADR (com requisitos e limites).
2. Implementar a chamada inter-módulo sob essa decisão.
3. Executar este benchmark pré-registrado e classificar H03 e H06 com os limiares
   fixados na seção 3, sem ajuste pós-hoc.
