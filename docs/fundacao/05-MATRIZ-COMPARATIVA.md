# Matriz comparativa de runtimes

A matriz orienta pesquisa, não classifica vencedores. Cada plataforma tem escopo e compromissos próprios. Revalidar versões antes de decidir.

| Plataforma | O que estudar | Pergunta útil para RVM | Fonte primária |
|---|---|---|---|
| JVM | Formato/validação de bytecode, compatibilidade e hospedagem | Que contratos compartilhados permitem coexistência sem apagar semânticas de origem? | [Java Virtual Machine Specification](https://docs.oracle.com/javase/specs/jvms/se25/html/) |
| .NET CLR | Common Type System e integração entre linguagens | Que tipos comuns bastam e que semântica continua específica da linguagem? | [Microsoft: Common Type System](https://learn.microsoft.com/en-us/dotnet/standard/base-types/common-type-system) |
| BEAM/OTP | Processos, heaps, mensagens e supervisão | Qual isolamento vem de processos e que custos de mensagem medir? | [Erlang processes](https://www.erlang.org/doc/system/eff_guide_processes.html) |
| GraalVM Polyglot | Embedding, valores guest, acesso ao host e isolates | Como expor capabilities e valores entre linguagens? Quais limites dependem da configuração? | [GraalVM: Embedding Languages](https://www.graalvm.org/latest/reference-manual/embed-languages/) |
| WebAssembly + Wasmtime | Validação, memória de instâncias, sandbox e API do host | Qual proteção existe no runtime e quais efeitos ainda dependem do host? | [Wasmtime security](https://docs.wasmtime.dev/security.html) |
| WebAssembly Component Model | Tipos de interface, recursos/handles, linking e concorrência | Que parte da ABI multilinguagem já tem especificação e testes? | [Repositório oficial Component Model](https://github.com/WebAssembly/component-model) |
| RVM proposta | RIR, verificador, runtime e capabilities ainda experimentais | Há necessidade demonstrada não atendida por soluções existentes? | Esta Foundation, sem resultados independentes |

## Como comparar

- Compare mesmo problema e mesma semântica, não só nomes de features.
- Diferencie isolamento de linguagem, processo, capability e segurança do host.
- Separe custo de compilação, inicialização, chamada, cópia, execução e troca de mensagens.
- Não atribua pontuação geral sem pesos pré-registrados e cenários explícitos.
- Feature documentada não prova segurança nem desempenho no workload RVM.

## Reavaliação de novidade

Antes de alegar combinação inédita, revisar especificações, implementações e trabalhos primários sobre WASI/Component Model, JVM/CLR, BEAM e GraalVM. Registrar data, versão e diferenças concretas. Nenhuma alegação de novidade está aprovada nesta Foundation.
