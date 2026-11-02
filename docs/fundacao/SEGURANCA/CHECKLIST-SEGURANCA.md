# Checklist de segurança de implementação

## Antes de começar

- [ ] Escopo da tarefa e ADR confirmado.
- [ ] AGENTS.md, contratos, threat model e instruções locais lidos.
- [ ] Entradas hostis, ativos, capabilities e limites identificados.
- [ ] Dependências novas justificadas e licença/versão verificadas.

## Decode e verificação

- [ ] Limites de bytes e contagens checados antes de alocação.
- [ ] Soma/multiplicação de length/index usa operações checked.
- [ ] UTF-8/strings, quando houver, validam limites sem truncar silenciosamente.
- [ ] IDs, offsets, branches, assinaturas, tipos e versão validados.
- [ ] Opções desconhecidas são rejeitadas; sem default permissivo.
- [ ] Erro inválido não causa panic nem chega ao executor.

## Execução

- [ ] Fuel debitado antes de cada operação despachada.
- [ ] Depth e quotas não resetam em chamadas.
- [ ] Não há loop, recursão ou alocação sem política de limite.
- [ ] Nenhum acesso implícito ao ambiente.
- [ ] Trap, erro de verificador e limite têm categorias distintas.
- [ ] Estado de uma execução não vaza para a próxima.

## Rust e host

- [ ] Nenhum unsafe novo sem ADR e aprovação.
- [ ] Tipos de módulo não carregam ponteiros/endereço de processo.
- [ ] Capability tem nome, escopo, limite, erro e teste de negação.
- [ ] Logs não contêm segredo ou conteúdo sensível por padrão.

## Antes de declarar concluído

- [ ] Casos/regressões executados; resultados citam comando.
- [ ] Fuzz/property test informa orçamento e corpus, se executado.
- [ ] Riscos residuais e cenários não testados listados.
- [ ] Sem alegação ampla de sandbox ou ausência de vulnerabilidade.
- [ ] Documentos e ADR refletem o estado real.
