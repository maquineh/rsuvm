# Modelo de ameaças inicial

**Estado:** hipótese de escopo para o protótipo; revisar antes de uso com dados reais.

## Ativos

- memória/estado do host e de outros módulos;
- integridade do processo runtime;
- capabilities do host: arquivos, rede, env, relógio e subprocessos;
- disponibilidade, uso de CPU/memória e possibilidade de interromper execução;
- confidencialidade de dados fornecidos ao módulo;
- reprodutibilidade de erro e auditoria.

## Adversários considerados no teste

- módulo/bytecode malformado ou deliberadamente inválido;
- programa válido com loop, recursão ou consumo de recurso adversarial;
- frontend que emite tipo/op incompatível;
- dependência ou módulo host que falha dentro dos cenários ensaiados.

Todos os dados vindos de módulo são não confiáveis. Não confiar em tipo, length, offset ou assinatura declarados pelo emissor.

## Não coberto pelo MVP

- comprometimento de kernel, hypervisor, CPU ou ferramenta de build;
- side channels de timing/cache e confidencialidade contra observador privilegiado;
- host ou plugin unsafe malicioso;
- proteção robusta multi-tenant em produção;
- vulnerabilidades desconhecidas em Rust, Cargo ou dependências;
- execução de código nativo gerado pelo compilador de linguagem.

## Fronteiras e propriedades desejadas

- parser, decoder, verifier e interpreter recebem dados hostis e falham sem panic; isso precisa ser testado.
- nenhuma operação de host está disponível por padrão.
- quotas limitam instruções, profundidade, tamanho do módulo e alocações previstas.
- verificação completa precede execução.
- módulo não cria ponteiro para memória Rust nem carrega bibliotecas.

Quotas/verificações in-process não equivalem a isolamento de processo/OS. Para serviço multi-tenant hostil, exigir isolamento externo revisado, por exemplo processo/container/sandbox apropriado, além do runtime, até que evidência sustente outra posição.

## Riscos e controles de pesquisa

| Risco | Controle inicial | Evidência que falta |
|---|---|---|
| Bypass do verificador | Executor recebe apenas VerifiedModule construído internamente; revalidar invariantes | Auditoria e fuzzing |
| DoS por loop/recursão | Fuel/depth debitados deterministicamente | Testes de fronteira e análise de alocação |
| DoS por tamanho | Limites antes de alocar e quotas por seção | Fuzzing do decoder |
| Escape via host API | Sem capability por padrão; allowlist futura | Revisão de cada adapter |
| Divergência de semântica | Especificação independente e corpus diferencial | Baselines executados |
| Confundir geração nativa com sandbox | Mensagem clara em CLI/docs | Revisão antes de release |

## Resposta a incidente

Parar execução/release afetada; preservar input reduzido, versão e logs sem segredos; classificar impacto; corrigir e adicionar regressão; documentar divulgação conforme política do projeto. Processo formal de divulgação e contato de segurança devem ser criados antes de release público.
