# PRD — RVM Foundation 0.1

**Estado:** proposta para pesquisa  
**Público:** mantenedor, implementadores e revisores de segurança  
**Data:** 2026-11-01

## 1. Resumo

A RVM Foundation 0.1 investiga um runtime em Rust capaz de executar programas produzidos por frontends de linguagens diferentes por meio de uma representação e contrato compartilhados. O primeiro foco é correção semântica, validação de módulos, limites de recursos e mensuração do custo de chamadas entre linguagens. Rio é candidata a primeiro frontend cujo código precisa ser auditado. Lift é uma linguagem indicada pelo mantenedor e será avaliada a partir do repositório maquineh/lift; nenhuma integração interna foi confirmada.

O resultado desejado é um protótipo experimental pequeno, documentação semântica, suíte de conformidade e relatório de hipóteses. Não é um runtime de produção multi-tenant.

## 2. Problema e motivação

Cada linguagem costuma trazer regras próprias de tipos, memória, chamadas e bibliotecas. Pontes ad hoc podem tornar valores, ownership, erros e capacidades do host ambíguos. A RVM pesquisa se uma fronteira comum verificável torna essas interações explícitas sem forçar cada linguagem a mudar sua semântica.

A existência de bytecode comum, tipos compartilhados ou interpretador não prova por si só interoperabilidade segura. Correção, isolamento e custo precisam de contratos e experimentos separados.

## 3. Usuários e casos de uso

- **Implementador de frontend:** traduz semântica suportada para uma representação documentada.
- **Desenvolvedor de runtime:** valida um módulo e executa-o sob limites explícitos.
- **Pesquisador/revisor:** reproduz testes, compara alternativas e registra evidência.
- **Mantenedor:** avalia se a RVM deve evoluir para mais tipos, recursos ou modelos de concorrência.

Casos iniciais: funções puras entre módulos, operações de inteiros verificadas, fluxo de controle, rejeição de módulos inválidos, execução limitada por fuel e teste diferencial em subset definido nos frontends.

## 4. Objetivos

1. Definir um núcleo semântico pequeno, independente da linguagem de origem.
2. Criar um modelo experimental de RIR e verificador que rejeite módulos estrutural ou tipadamente inválidos.
3. Executar módulos válidos num interpretador Rust sob limites de computação e profundidade.
4. Definir pesquisa Lift e Rio apoiada por auditoria, sem comprometer seus compiladores.
5. Construir testes positivos, negativos, diferenciais e de propriedades.
6. Medir interoperabilidade contra baselines apropriados e publicar resultados reproduzíveis.
7. Manter o projeto incapaz de acessar recursos do host sem concessão explícita.

## 5. Fora de escopo no 0.1

- JIT, AOT próprio ou emissão de código nativo para programas convidados.
- Compatibilidade integral com TypeScript, Rust, Lift ou qualquer outra linguagem.
- Modificar o repositório Lift ou presumir APIs internas nele.
- Declarar features do Rio sem ler o código e seus testes.
- GC geral, objetos arbitrários, herança, closures, reflexão, async/await ou atores em produção.
- FFI unsafe, carregamento de plugins dinâmicos ou acesso implícito a arquivos/rede/ambiente.
- Formato binário estável, compatibilidade futura garantida ou versão 1.0.
- Alegar sandbox forte ou uso seguro em produção multi-tenant.
- Superar JVM, .NET, BEAM, GraalVM ou Wasmtime em desempenho geral.

## 6. Requisitos funcionais

| ID | Requisito | Critério de aceitação inicial |
|---|---|---|
| RF-01 | Representar função, parâmetros, retorno, blocos e fluxo de controle | Corpus RIR válido passa e é executável no intérprete de referência |
| RF-02 | Verificar estrutura e tipos antes da execução | Corpus inválido é rejeitado com código estável; nenhum inválido chega ao executor |
| RF-03 | Executar operações puras do subset | Resultado ou trap coincide com semântica documentada |
| RF-04 | Limitar trabalho | Fuel e profundidade interrompem o programa no limite especificado |
| RF-05 | Reportar falhas reproduzíveis | Erros distinguem módulo inválido, trap semântico e limite de recurso |
| RF-06 | Permitir frontends independentes | Dependências do runtime não importam crates específicos de Lift ou Rio |
| RF-07 | Executar testes diferenciais | Corpus registra versão do frontend, entrada, resultado, erro e artefatos |
| RF-08 | Não conceder efeitos implicitamente | MVP puro não lê/escreve arquivos, rede, env ou subprocessos |
| RF-09 | Registrar decisões de design | Questões arquiteturais têm ADR separado, status e plano de validação |

## 7. Requisitos não funcionais

- **Determinismo:** mesma versão, módulo, limites e configuração produzem o mesmo valor/trap, fora de tempos de parede e falhas externas.
- **Defesa em profundidade:** validação prévia, Rust seguro, verificações em runtime e quotas explícitas.
- **Observabilidade:** erro com código, função/bloco/instrução quando disponível, sem expor segredos.
- **Reprodutibilidade:** commits, versões, hardware, comando, dados brutos e ferramenta registrados.
- **Portabilidade:** declarar matriz de sistemas suportada somente depois de CI real.
- **Evolução:** mudanças incompatíveis exigem proposta versionada e regressão da suíte.

## 8. Aceitação do protótipo

A Foundation 0.1 é aceita como protótipo de pesquisa quando: (a) semântica está escrita; (b) módulos inválidos são rejeitados antes da execução; (c) recursos são limitados de forma observável; (d) corpus é automatizável; (e) ao menos uma rota frontend→RIR é demonstrada ou bloqueada com evidência; (f) hipóteses são classificadas como corroboradas no escopo, refutadas ou inconclusivas; e (g) riscos e lacunas são publicados. Isso não equivale a autorização para produção.

## 9. Métricas propostas

- Classificação correta no corpus válido/inválido.
- Número de divergências semânticas entre oráculo e RVM.
- Custo de chamada interna e interlinguagem, latência e alocações.
- Precisão de fuel e memória máxima observada.
- Bugs por classe encontrados por fuzzing, com corpus e orçamento.
- Cobertura do subset Lift/Rio com suporte documentado e executado.

Metas quantitativas são hipóteses e devem ser pré-registradas; não são resultados.
