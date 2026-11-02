# Índice de ADRs — RSUVM

ADRs registram decisões e pendências deste repositório. Status **não** é sinônimo
de aprovação para produção.

Os ADRs 0001..0008 são da fundação e vivem, sem modificação, em
`../fundacao/ADRs/` (RVM independente de linguagem, RIR como fronteira,
intérprete antes de JIT, bytecode não congelado, capabilities deny-by-default,
memória/transferência, integração Lift e integração Rio).

| ADR | Tema | Status |
|---|---|---|
| [0009](ADR-0009-ESTRUTURA-DE-CRATES.md) | Workspace de crates e grafo de dependências | Aceito experimentalmente |
| [0010](ADR-0010-FORMATO-TEXTUAL-EXPERIMENTAL.md) | Formato textual EXP-0-TEXT para corpus e CLI | Aceito experimentalmente |
| [0011](ADR-0011-BYTECODE-ESPELHO-LINEAR.md) | Protótipo de bytecode espelho da RIR | Proposto (ADR-0004 aberto) |
| [0012](ADR-0012-TETO-DE-PROFUNDIDADE.md) | Teto de profundidade e frames explícitos | Aceito experimentalmente (revisado após teste) |
| [0013](ADR-0013-QUOTA-DE-MEMORIA.md) | Quota de memória modelada para frames | Aceito experimentalmente |
| [0014](ADR-0014-LICENCA-DO-REPOSITORIO.md) | Licença dupla `MIT OR Apache-2.0` do repositório | Aceito experimentalmente |
| [0015](ADR-0015-POLITICA-DE-REUSO-DO-RIO.md) | Política de reuso do Rio (licença declarada `MIT OR Apache-2.0`) | Aceito experimentalmente |
| [0016](ADR-0016-MEMORIA-DE-BUFFERS-DE-ARGUMENTOS.md) | Buffers transitórios de argumentos na quota de memória | Aceito experimentalmente |
| [0017](ADR-0017-INTEGRACAO-LIFT-BLOQUEADA.md) | Integração Lift fora de escopo (licença resolvida; bloqueios técnicos) | Aceito experimentalmente |
| [0018](ADR-0018-LIGACAO-ESTATICA-INTER-MODULO.md) | Ligação estática inter-módulo (crate `rvm-link`) | Aceito experimentalmente |
| [0019](ADR-0019-ESTRATEGIA-DE-FUZZING.md) | Estratégia de fuzzing (determinística; cobertura adiada) | Aceito experimentalmente |
| [0020](ADR-0020-FUZZING-GUIADO-POR-COBERTURA.md) | Fuzzing guiado por cobertura (dependência de dev) | Aceito experimentalmente |
| [0021](ADR-0021-ADAPTADOR-LIFT-RIR.md) | Adaptador protótipo Lift→RIR (`frontends/lift-rir`) | Aceito experimentalmente |
| [0022](ADR-0022-DIVISAO-E-RESTO-INTEIROS.md) | Divisão e resto inteiros verificados no núcleo | Aceito experimentalmente |
| [0023](ADR-0023-DESLOCAMENTOS-E-BIT-A-BIT.md) | Deslocamentos e bit a bit inteiros no núcleo | Aceito experimentalmente |

Estados usados: `Proposto`, `Aceito experimentalmente`, `Validado no escopo`,
`Rejeitado`, `Substituído`. "Validado no escopo" exige critério e evidência
registrados em relatório; nenhum ADR deste repositório atingiu esse estado.
