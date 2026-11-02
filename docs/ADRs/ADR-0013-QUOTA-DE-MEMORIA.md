# ADR-0013 — Quota de memória modelada para frames

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-01
- **Requisitos:** RF-04 (limitar trabalho), PRD §7 (quota de memória), lacuna 1 do
  relatório de implementação, ADR-0005 (nenhum efeito implícito).

## Contexto

A primeira versão executava com quota de fuel e profundidade, mas **sem quota de
memória**: um módulo com `max_values_per_function` alto chamado em profundidade
máxima podia reservar `slots × frames` sem contrapartida. O relatório de
implementação registrou isso como risco residual (lacuna 1) e a validação de
`Limits` não cobre memória de execução.

Medir RSS real não serve: não é determinístico, depende do alocador e do SO, e
não é observável de forma portátil em Rust estável. Uma quota precisa ser
reprodutível e testável em qualquer plataforma.

## Decisão

1. Quota **modelada em bytes RSUVM**, definida em `rvm-runtime`:
   `FRAME_BASE_BYTES = 256`, `SLOT_BYTES = 24` e
   `frame_cost(slots) = 256 + slots × 24` (com saturação).
2. `MemoryLimit` é um tipo de valor (`u64`) com `DEFAULT = 16 MiB`; `0` é aceito
   e significa que nenhum frame cabe.
3. `ExecConfig` carrega o limite (`with_memory`/`memory_limit`); o padrão continua
   válido para todos os testes existentes.
4. O intérprete cobra `frame_cost(value_count)` **antes** de alocar os slots de um
   frame, conta memória **viva** (não cumulativa) e libera o custo no retorno;
   `RunOutcome::memory_peak` expõe o pico observado.
5. Estouro produz `out_of_memory` (`Resource`/`ResourceLimit`), com localização da
   função, e o CLI expõe `--memory <bytes>` (exit 4).

## Alternativas

1. **Medir RSS/`/proc`**: não determinístico, não portátil, não testável em CI.
2. **Alocador global instrumentado**: mede de verdade, mas exige estado global e
   atributo de alocador (efeito em todo o processo), fora da política de Rust
   seguro simples e não determinístico entre plataformas.
3. **Quota em número de slots** em vez de bytes: mais simples, mas não expressa
   "16 MiB" e mistura contagem com custo.
4. **Sem quota** (estado anterior): falha aberta para DoS de memória.

## Consequências

- Positivas: limite determinístico, reprodutível e testável; fronteira exata
  verificada (custo e custo−1); pico observável; sem dependências novas.
- Negativas: o modelo **não é** a memória real do processo (overhead de `Vec`,
  alocador e capacidade reservada não são medidos); a cobrança de buffers
  transitórios de argumentos foi adicionada depois pelo ADR-0016.
- Residual: `frame_cost` pode subestimar o consumo real; a quota protege contra
  crescimento descontrolado, não contra picos de alocador.

## Validação

- `crates/rvm-interpreter/tests/memoria.rs`: custo exato cabe; custo−1 falha antes
  de alocar; dois frames simultâneos (656 bytes) cabem e 655 falham; limite `0`
  falha com `out_of_memory` (não `out_of_fuel`).
- `crates/rvm-cli/tests/memoria.rs`: `--memory 1` ⇒ exit 4 com `out_of_memory`;
  `--memory 100000` executa; `--memory muita` ⇒ exit 1.
- Corpus `N-021` (`memory: 100`) exige `limit out_of_memory`.
- Total do workspace e log em `docs/RELATORIOS/EVIDENCIAS-EXECUCAO.md`.
- Não medido: consumo real de memória por frame (pergunta em aberto).
