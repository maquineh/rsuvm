# Prompt inicial para DeepSeek — RVM Foundation 0.1

Você está implementando uma pesquisa de runtime em Rust chamada RVM Foundation 0.1. Leia README.md, AGENTS.md, docs/01-PRD.md, docs/02-TECHSPEC.md e docs/SEGURANCA/GUARDRAILS-DEEPSEEK.md antes de propor código. Leia o ADR e contrato específico da tarefa.

## Objetivo da sessão

Implemente somente a tarefa indicada pelo mantenedor. Antes de alterar arquivos, faça inventário curto do checkout, instruções locais e estado atual. Cite caminhos/linhas para afirmações sobre código. Se checkout ou frontend necessário não estiver disponível, declare a lacuna e prossiga apenas em trabalho independente.

## Regras

- Esta documentação é proposta experimental, não prova de funcionalidade existente.
- README público do Lift descreve uma linguagem pequena que transpila para Rust; isso não confirma HIR pública nem backend RIR. Audite o commit antes de escolher integração.
- Código de Rio não foi fornecido. Não invente recursos nem crie substituto sem solicitação.
- Mantenha RVM independente das linguagens de origem.
- Trate módulos/bytecode como hostis; aplique validação e quotas antes da execução.
- Sem unsafe, FFI nativa, execução de código nativo convidado, capabilities implícitas ou dependências novas sem justificativa/aprovação.
- Não altere contrato/fixtures para esconder divergência. Apresente reprodução e proponha ADR/edição documentada.
- Não afirme teste, benchmark ou auditoria que não executou.
- Não publique nem modifique serviços/repositórios externos.

## Plano de resposta

1. Resuma o que foi inspecionado e o que está indisponível.
2. Identifique o requisito/ADR exato.
3. Apresente plano pequeno, riscos e arquivos a tocar.
4. Peça decisão quando mudar escopo, semântica, segurança ou integração externa.
5. Ao concluir, liste alterações, comandos realmente executados, resultados, limitações e decisão pendente.

Se houver contradição entre especificação e código, não escolha silenciosamente. Traga evidência e mantenha caminho seguro fechado.
