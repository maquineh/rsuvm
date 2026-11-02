# Roadmap Foundation 0.1

**Horizonte indicativo:** 24 semanas. Estimativas relativas, dependentes de acesso ao código, auditorias e resultados. Nenhum marco implica aceitar uma hipótese.

| Fase | Janela estimada | Trabalho | Gate de saída |
|---|---:|---|---|
| M0 — Baselines e ameaças | 1–3 | Inventário Lift/Rio, threat model, objetivos e workloads | Fatos documentados; lacunas, licença e subset registrados |
| M1 — Semântica mínima | 4–6 | Tipos, operações, traps e corpus inicial | Regras sem ambiguidades nos casos cobertos |
| M2 — RIR e intérprete de referência | 7–10 | RIR experimental, validação estrutural, execução pura | Casos válidos/inválidos reproduzíveis |
| M3 — Verificador e quotas | 11–14 | Tipos, CFG, chamadas, fuel/depth e fuzzing inicial | Nenhum inválido do corpus chega ao executor; orçamento documentado |
| M4 — Bytecode experimental | 15–17 | Prototipar representação/codificação e decoder limitado | Comparação registrada; sem freeze obrigatório |
| M5 — Integração frontend | 18–21 | Subset de Rio ou Lift conforme baseline/acesso | Programa real e diferencial; ou relatório de bloqueio técnico |
| M6 — Revisão Foundation | 22–24 | Benchmarks, revisão externa, ADRs e relatório | H01–H08 classificadas; próximos riscos publicados |

## Pausas de segurança/qualidade

- **Pausa semântica:** divergência entre frontend e RIR sem regra definida; não “corrigir” frontend por conveniência.
- **Pausa de segurança:** módulo malformado ou acima de quota causa panic/acesso fora dos limites/execução imprevista; não avançar para carga hostil.
- **Pausa de integração:** licença/API/arquitetura impede adaptação limpa; registrar e revisar rota antes de modificar projeto existente.
- **Pausa de desempenho:** resultado excede limiar pré-registrado; publicar resultado e alternativas, sem ajuste pós-hoc.

## Entregáveis por marco

Especificação versionada, código, testes, comandos e resultados, dados brutos quando medidos, hashes dos commits, ADRs atualizados e limitações. A estimativa não é promessa calendarizada.
