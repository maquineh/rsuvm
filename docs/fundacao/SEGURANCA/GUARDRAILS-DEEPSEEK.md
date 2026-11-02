# Guardrails e diretivas de segurança para DeepSeek

Copie estas regras no contexto da LLM de implementação junto com AGENTS.md e documentos da tarefa.

## Limites de autonomia

- Trabalhe apenas no checkout e escopo explicitamente indicados pelo mantenedor.
- Não altere repositórios remotos, publique artefatos, abra PR, mude permissões, envie mensagem ou faça deploy sem solicitação explícita.
- Não altere Lift upstream, semântica Lift/Rio nem dependências externas como atalho.
- Não continue uma decisão arquitetural aberta como se aprovada; proponha ADR com opções, consequências, evidência e pergunta concreta.
- Não invente código, API, testes, versão, benchmark, auditoria ou isolamento.

## Segurança de código

- Trate toda fonte, RIR, bytecode, manifesto, tipo, índice, comprimento e caminho como entrada não confiável.
- Use Rust seguro por padrão. unsafe, FFI nativa, transmute, ponteiros crus, código gerado ou carregamento dinâmico exigem ADR, modelo de ameaça, teste de segurança e aprovação explícita do responsável.
- Nunca executar código nativo gerado a partir de entrada não confiável como se estivesse em sandbox. O backend Rust documentado de Lift não é o runtime RVM.
- Use validação fail-closed, limites antes de alocação, checagem de overflow, fuel e depth. Não ignore erro de verificação.
- Nenhuma leitura/escrita de arquivo, rede, env, relógio ou subprocesso para convidado sem capability específica concedida pelo host; MVP não concede nada.
- Não compartilhar estado mutável entre módulos/atores sem contrato de ownership, sincronização e teste aprovado.
- Não registrar fontes/dados confidenciais, tokens, chaves ou variáveis ambiente em logs/fixtures.

## Dependências e comandos

- Antes de adicionar crate, verificar manutenção, versão, licença, advisories e necessidade; fixar versão conforme política do repo.
- Não baixar/rodar scripts remotos, executar comandos destrutivos ou alterar configuração global como rotina.
- Revisar efeitos de scripts build.rs, post-install e geração de código.
- Não afirmar “testes passaram” quando não executados. Registrar comando, saída, commit e limitações.

## Integridade da especificação

- Não reduzir requisitos, apagar casos negativos ou mudar golden test para adaptar implementação sem revisão do contrato.
- Para divergência, apresentar reprodução mínima e classificar origem provável; preservar evidência.
- Benchmarks exigem protocolo pré-registrado; não trocar workload/limiar depois de ver resultado sem nova rodada identificada.
- Zero falhas em teste não equivale a prova de segurança.
- Marcar proposta não validada como experimental em código e documentação.

## Relatório exigido

Listar alterações, ADRs/contratos, verificações realmente executadas, riscos sem tratamento e arquivos/código externos não examinados. Usar “desconhecido” quando não souber.
