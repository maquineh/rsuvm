# ADR-0012 — Teto de profundidade e máquina de frames explícita

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-01 (revisado após resultado de teste)
- **Requisitos:** RF-04 (limitar trabalho), modelo de ameaças (DoS por recursão),
  ADR-0003 (intérprete primeiro).
- **Substitui a versão "Proposto"** deste mesmo ADR, que previa intérprete
  recursivo com teto de 256 frames.

## Contexto

A profundidade de chamadas é configurável pelo host e, se não tiver teto, pode
esgotar a pilha do processo hospedeiro — um DoS do próprio hospedeiro,
independente do convidado. A primeira implementação do intérprete era uma função
Rust recursiva: cada frame de RIR consumia um frame nativo.

## Resultado observado (não hipótese)

Em 2026-11-01, com a implementação recursiva e `CallDepthLimit::HARD_MAX = 256`
(já validado por tipo, `ExecConfig` recusando valores acima), a execução:

```text
cargo test -p rvm-interpreter
...
test recursao_excede_profundidade_e_fuel_cresce_por_frame ... thread has overflowed its stack
error: test failed
Caused by: process didn't exit successfully ... (exit code: 0xc00000fd, STATUS_STACK_OVERFLOW)
```

O processo abortou antes de o limite ser aplicado: os 256 frames nativos em
compilação *debug* excederam a pilha da thread de teste. Ou seja, o teto não
garantia falha fechada; garantia apenas que o estouro seria limitado a 256
frames. Isso refuta a suficiência da mitigação proposta anteriormente.

## Decisão

1. A execução usa uma **pilha de frames na heap** (`Vec<Frame>`), sem recursão na
   linguagem hospedeira para chamadas do convidado.
2. `CallDepthLimit` continua sendo um tipo validado em `1..=HARD_MAX`, com
   `HARD_MAX = 256` e padrão `64`; o limite agora é aplicado sobre a pilha
   explícita, independente da pilha nativa.
3. O verificador aceita recursão; o limite é de execução, não de módulo.

## Alternativas

1. **Reduzir `HARD_MAX`** para 32–64: reduz a chance, mas não elimina a
   dependência do tamanho de frame nativo, da plataforma e do perfil de
   compilação; o modo de falha continua sendo abort.
2. **Executar em thread com pilha maior:** esconde o problema, adiciona
   concorrência/limpeza e mantém o abort como modo de falha.
3. **Recursivo com medição de pilha:** não portável (profundidade de pilha não é
   observável de forma padrão em Rust) e não verificável por teste.

## Consequências

- Positivas: profundidade deixa de ser risco de estouro de pilha nativa;
  `call_depth_exceeded` é observável e testado nas fronteiras (255/256);
  comportamento igual em debug e release.
- Negativas: memória por frame é alocada na heap (`slots` × profundidade), então
  um host que aceite módulos grandes e profundidade 256 pode reservar memória
  proporcional — limitada, mas não gratuita. A alocação usa
  `try_reserve_exact` e falha como `invalid_module`, não como abort.
- Residual: a memória total de frames não é contabilizada em bytes contra uma
  quota explícita; isso continua pendente (ver PRD §7, quotum de memória).

## Validação executada

- `cargo test -p rvm-interpreter`: 14 testes verdes, incluindo
  `recursao_excede_profundidade_e_fuel_cresce_por_frame`, que exercita
  profundidade 8, 255 e 256 e exige `call_depth_exceeded` sem abortar.
- `cargo test --workspace`: 111 testes verdes (relatório de implementação).
- Não foi medido consumo de memória por frame; pergunta em aberto.
