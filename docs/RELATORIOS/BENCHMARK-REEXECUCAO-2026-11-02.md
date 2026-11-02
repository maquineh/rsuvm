# Reexecução do benchmark pré-registrado — pós ADR-0034/0035

- **Data:** 2026-11-02
- **Commit medido:** `89c21dc` (inclui ADR-0034 e ADR-0035)
- **Método:** exatamente o de
  [`BENCHMARKS-PREREGISTRADOS.md`](./BENCHMARKS-PREREGISTRADOS.md) §3: perfil
  `bench` (release), aquecimento descartado, 11 amostras × 20 000 execuções por
  ponto, medianas, mesmo comando (`cargo bench -p rvm-interpreter --bench
  chamada`). **Nenhum limiar foi alterado.**

## 1. Ambiente

| Item | Valor |
|---|---|
| SO | Microsoft Windows NT 10.0.26300.0 |
| CPU | Intel(R) Core(TM) i7-9700 CPU @ 3.00GHz |
| `rustc` | 1.94.1 (e408947bf 2026-03-25) |
| `cargo` | 1.94.1 (29ea6fb6a 2026-03-24) |
| Perfil | `bench` (release, `overflow-checks = true`) |
| Isolamento | nenhum (máquina de desenvolvimento) |

## 2. Execução 1

| k | fuel | intra ns | inter ns | H06 rel | dispersão |
|---:|---:|---:|---:|---:|---:|
| 1 | 6 | 565,9 | 583,9 | — | 0,2531 |
| 4 | 18 | 1 709,8 | 1 679,8 | −0,0419 | 0,7466 |
| 16 | 66 | 5 567,7 | 5 880,9 | 0,0889 | 0,3482 |
| 64 | 258 | 23 507,7 | 23 894,5 | 0,0041 | 0,3830 |
| 256 | 1 026 | 98 607,8 | 99 679,7 | 0,0091 | 0,5779 |

- `fuel` idêntico entre variantes (controle satisfeito: `4k + 2`).
- Custo de ligação (k = 64): **26 247,0 ns** por ligação.
- Dispersão no ponto classificador (k = 256): **0,5779 (57,8%)**.
- **Classificação: H06 INCONCLUSIVA** (dispersão > 10%, regra pré-registrada).

## 3. Execução 2

- Repetição do mesmo binário, mesmos parâmetros.
- Dispersão no ponto classificador: **0,3176 (31,8%)**.
- Custo de ligação (k = 64): **24 246,0 ns**.
- **Classificação: H06 INCONCLUSIVA** (mesma regra).

## 4. Leitura

- **H03 continua confirmada estruturalmente** (ADR-0018): a ligação resolve
  `call_static` para um índice global antes da verificação; não há serialização
  por chamada.
- **H06 não é classificável nesta máquina** com os limiares pré-registrados: as
  duas execuções tiveram dispersão muito acima de 10% no ponto classificador. Isso
  **não refuta** H06; apenas repete o comportamento já registrado no runner de
  desenvolvimento da medição original. A confirmação de H06 registrada em
  `BENCHMARKS-PREREGISTRADOS.md` §5.2 veio do runner dedicado do GitHub Actions
  (`bench.yml`, run `41429076866`), com dispersão de 1,33%.
- **Não há alegação de desempenho.** Os números absolutos desta máquina não são
  comparáveis aos do runner; a comparação válida é intra × inter no mesmo ponto.

## 5. Observação (não conclusiva)

No ponto k = 256, o tempo intra desta reexecução (≈98,6–102,2 µs) é cerca de 2,3×
o valor da tabela de desenvolvimento original (43,5 µs). Como a máquina, a carga e
o build não foram controlados, **não se atribui a causa** a ADR-0034/0035 sem
perfilamento. O número do runner (26,5 µs) continua sendo a referência de
classificação de H06.

## 6. Reprodução

```bash
cargo bench -p rvm-interpreter --bench chamada
```

O binário imprime a tabela, o custo de ligação e a classificação. O workflow
`.github/workflows/bench.yml` continua manual; para classificar H06 de forma
estável, rode-o em runner dedicado e registre o run.
