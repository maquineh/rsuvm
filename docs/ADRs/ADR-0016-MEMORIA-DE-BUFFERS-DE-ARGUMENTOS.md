# ADR-0016 — Buffers transitórios de argumentos na quota de memória

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** item 4 do "Próximo passo sugerido" (§12) do
  `RELATORIO-IMPLEMENTACAO.md` ("implementar quota de memória para frames e medir
  consumo por frame"); lacuna residual declarada no ADR-0013.

## Contexto

O ADR-0013 modelou a quota de memória apenas como custo de **frame**
(`frame_cost(slots) = 256 + slots × 24`). Na seção "Consequências" ficou
registrado que "vetores transitórios de argumentos de chamada/desvio não são
contabilizados". Na prática, o intérprete aloca um `Vec<Value>` para copiar os
argumentos antes de gravá-los nos slots do destino: em `Op::Call`
(`collect_args`) e em `br`/`cond_br` (`jump`). Essa memória é **viva** durante a
operação e não era cobrada, subestimando o pico — justamente o caso de uma
chamada de aridade alta em profundidade máxima.

Medir RSS continua fora do escopo: não é determinístico, depende do alocador e do
SO, e não é portável em Rust estável (ADR-0013, "Alternativas").

## Decisão

1. Novo custo modelado em `rvm-runtime`:
   `arg_buffer_cost(args) = args × SLOT_BYTES` (`SLOT_BYTES = 24`), com saturação;
   um buffer vazio custa zero.
2. `Op::Call`: o buffer de argumentos é cobrado **antes** de `collect_args`
   (falha fechada antes de alocar) e liberado depois que o frame do callee é
   empilhado.
3. `br`/`cond_br`: `jump` cobra o buffer antes de reservá-lo e o libera depois de
   gravar os parâmetros de bloco. O buffer **deixou de ser reutilizado** entre
   desvios: cada desvio tem seu buffer transitório, para que o modelo corresponda
   à memória viva descrita no ADR-0013.
4. `RunOutcome::memory_peak` passa a incluir frames **e** buffers transitórios.
5. O modelo continua determinístico e **não** mede RSS do processo.

## Alternativas

1. **Manter a lacuna do ADR-0013:** subestima o pico de chamadas de aridade alta;
   rejeitado.
2. **Medir RSS/`/proc`:** não determinístico, não portátil, não testável em CI;
   já rejeitado no ADR-0013.
3. **Reutilizar o buffer e cobrar a capacidade retida:** mais fiel ao alocador,
   mas acopla a quota à estratégia de alocação do host e dificulta o teste de
   fronteira exata; rejeitado nesta rodada.

## Consequências

- Positivas: o pico modelado se aproxima do consumo real; a fronteira exata passa
  a ser testável com buffer; a lacuna declarada no ADR-0013 é fechada.
- Negativas: remover a reutilização do buffer de desvio implica uma alocação por
  desvio (custo de tempo medido na etapa A3); o modelo ainda não mede RSS,
  capacidade reservada do `Vec` nem overhead do alocador.
- Residual: `Vec::try_reserve_exact` pode reservar mais que o pedido; o modelo
  conta `len × 24`, não `capacity × 24`. Continua sendo quota de crescimento, não
  medição de consumo.

## Validação

- `crates/rvm-runtime`: teste `custo_de_buffer_de_argumentos_e_modelado_e_satura`.
- `crates/rvm-interpreter/tests/memoria.rs`: pico de `soma` inclui o buffer de
  chamada (704 = 328 + 48 + 328); `com_merge` inclui o buffer de desvio
  (424 = 400 + 24); um byte a menos falha com `out_of_memory`.
- Corpus `N-024` (`memory: 703`) exige `limit out_of_memory` — só falha porque o
  buffer de 2 argumentos (48) é cobrado.
- Total do workspace e log em `docs/RELATORIOS/RELATORIO-ETAPAS-A.md`.
- Não medido: consumo real de memória (RSS) por frame — permanece pergunta aberta.
