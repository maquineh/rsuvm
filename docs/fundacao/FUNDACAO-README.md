# RVM Foundation 0.1

**Pacote inicial de pesquisa e engenharia**  
Idioma: português brasileiro  
Data de preparação: 2026-11-01  
Estado: proposta experimental; nenhuma implementação da RVM foi auditada ou validada nesta entrega.

## Objetivo

Este pacote orienta a concepção e a futura implementação de uma máquina virtual/runtime em Rust que possa receber frontends independentes, começando por Rio e investigando Lift. Foi preparado para servir como contexto de implementação por outra LLM, incluindo DeepSeek. Não é especificação final nem autorização para modificar repositórios de terceiros.

## Leia nesta ordem

1. [PRD](docs/01-PRD.md) — problema, escopo e critérios de aceitação.
2. [TechSpec](docs/02-TECHSPEC.md) — arquitetura e limites experimentais.
3. [Plano de pesquisa](docs/03-PESQUISA.md) e [roadmap](docs/04-ROADMAP.md).
4. [Guardrails para DeepSeek](docs/SEGURANCA/GUARDRAILS-DEEPSEEK.md) e [modelo de ameaças](docs/SEGURANCA/MODELO-AMEACAS.md).
5. [Contratos RIR](docs/CONTRATOS/RIR-EXP-0.md), [bytecode](docs/CONTRATOS/BYTECODE-EXP-0.md), [ABI](docs/CONTRATOS/ABI-EXP-0.md) e [conformidade](docs/TESTES/CONFORMIDADE.md).
6. [Integração Lift](docs/INTEGRACAO/LIFT.md), [baseline Rio](docs/INTEGRACAO/RIO.md) e ADRs em docs/ADRs/.
7. Use o [prompt de implementação](docs/PROMPTS/PROMPT-DEEPSEEK.md) como instrução inicial para a LLM.

## Mapa

~~~text
RVM-Foundation-0.1/
├── README.md
├── AGENTS.md
└── docs/
    ├── 01-PRD.md
    ├── 02-TECHSPEC.md
    ├── 03-PESQUISA.md
    ├── 04-ROADMAP.md
    ├── 05-MATRIZ-COMPARATIVA.md
    ├── ADRs/
    ├── CONTRATOS/
    ├── INTEGRACAO/
    ├── PROMPTS/
    ├── SEGURANCA/
    └── TESTES/
~~~

## Convenção de evidência

- **Evidência documental:** afirmação encontrada em fonte identificada; pode não ter sido confirmada executando código.
- **Proposta:** desenho para experimento, sujeito a revisão.
- **Hipótese:** afirmação testável que pode ser refutada.
- **Desconhecido:** informação que precisa de inspeção, medição ou decisão humana.
- **Resultado:** só pode ser preenchido após execução reproduzível, com commit, ambiente e artefatos registrados.

A página pública do Lift afirma que ele é uma linguagem pequena, tipada e familiar a usuários de um subconjunto de TypeScript, que compila para Rust via Cargo. Isso é evidência documental, não auditoria do código, confirmação de execução ou comprovação de API interna para emitir RIR. Consulte [Integração Lift](docs/INTEGRACAO/LIFT.md).

O código do Rio não foi incluído nesta entrega nem encontrado no workspace desta conversa. Seus recursos e baseline permanecem desconhecidos até auditoria.

## Princípios

- RVM não importa regras de uma linguagem hospedada para dentro do runtime.
- A RIR é uma fronteira experimental; seu formato ainda não é estável.
- O primeiro executor sugerido é interpretativo e determinístico; JIT, AOT e GC geral ficam fora do MVP.
- Bytecode e entradas são dados não confiáveis: validar, impor limites e falhar de forma fechada.
- Nenhuma alegação de sandbox forte, isolamento de produção ou superioridade de desempenho é feita.
- Toda decisão que feche uma questão aberta deve atualizar ou substituir um ADR com evidência.

## Critério para implementação

A LLM deve começar por inventário do repositório real, registrar fatos com caminhos e linhas, confirmar dependências e executar apenas tarefas pequenas e reversíveis. Se o código de Rio não estiver disponível, pode preparar o trabalho independente da RVM e deixar o adaptador Rio pendente; não deve criar uma linguagem Rio fictícia.
