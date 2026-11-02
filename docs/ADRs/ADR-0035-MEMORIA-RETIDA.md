# ADR-0035 — Quota de memória por valor retido (liberação em slots)

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** lacuna declarada em §9 do `RELATORIO-IMPLEMENTACAO.md` e no
  ADR-0013; pedido do mantenedor ("fechar a quota de memória").

## Contexto

A partir da ADR-0025, arrays e strings passaram a ser cobrados de forma
**cumulativa**: cada literal/concatenação/atualização somava bytes à quota e nada
era liberado até o fim da execução. Isso fechava o crescimento descontrolado, mas
tornava qualquer laço com alocação inviável: 100 iterações de uma string de 8
bytes consumiam 800 bytes para sempre, mesmo com apenas uma string viva.

O modelo de frames já liberava o custo do frame no retorno (ADR-0013/0016), mas
não havia contabilização do **conteúdo** dos slots.

## Decisão

1. Adicionar [`rvm_runtime::Value::retained_bytes`], o custo modelado retido por um
   valor:
   - `unit`/`bool`/`i64`/`f64`: `0`;
   - `string`: tamanho UTF-8;
   - `array`/`struct`: `itens × SLOT_BYTES (24)` + custo recursivo dos filhos;
   - `option`/`result`: `SLOT_BYTES` quando carregam payload + custo do payload.
   Somas saturam em `u64::MAX` e são determinísticas.
2. O intérprete passa a manter, **por slot**, os bytes retidos. `write_current`
   cobra o custo do valor novo e **libera** o custo do valor anterior do mesmo
   slot. No retorno do frame, libera o custo do frame **e** a soma dos slots.
3. Antes de alocar valores grandes, o intérprete faz uma **pré-checagem**
   (`ensure_memory`) com o custo previsto; a cobrança exata acontece na gravação do
   slot. Falha continua fechada (`out_of_memory`).
4. O valor de retorno é lido do frame chamado e gravado no slot do chamador, que
   cobra a cópia; o frame chamado libera sua cópia ao sair. Assim não há
   transferência invisível de memória.
5. `RunOutcome::memory_peak` passa a ser o **pico de memória viva** (frames +
   valores retidos), não o acumulado histórico.
6. A cobrança de buffer transitório de argumentos/chamadas (ADR-0016) permanece:
   ela é liberada ao fim do desvio/chamada.

## Alternativas

1. **Análise de vivacidade por `ValueId`** (liberar após o último uso): mais
   precisa, mas exige um passe novo no verificador e um mapa de últimos usos no
   `VerifiedModule`. Adiada.
2. **Arena por frame:** libera tudo no retorno, mas não resolve laços dentro de um
   frame. Rejeitada.
3. **GC:** incompatível com o modelo determinístico e com o escopo atual.
4. **Manter cumulativo:** simples, mas inviabiliza laços com alocação. Rejeitada.

## Consequências

- Positivas: laços com alocação passam a ser limitados pela memória **viva**, não
  pelo total histórico; o pico reportado reflete memória viva; sem `unsafe` nem
  dependências.
- Negativas: valores mortos que nunca são sobrescritos ficam contabilizados até o
  fim do frame (sem vivacidade); clones temporários de operandos não são cobrados
  (o pico pode subestimar em ~1 cópia por operação); o modelo continua sendo
  memória modelada, **não RSS**.
- Residual: sem liberação por último uso, sem GC, sem medição de memória do host.

## Validação

- `crates/rvm-runtime`: teste de `retained_bytes`.
- `crates/rvm-interpreter`: `write_slot_charged`/`ensure_memory`; liberação no
  retorno; `memoria.rs` segue verde.
- Corpus `C-061`: laço de 100 iterações concatena 8 bytes por iteração com
  `memory: 700` e conclui — o modelo cumulativo exigiria ~800 bytes só de strings e
  falharia. `N-028` continua trapando (`out_of_memory`) porque a string viva cresce.
- `cargo test --workspace` (149 testes, 61 válidos + 42 inválidos), `clippy` e
  `fmt` verdes.
