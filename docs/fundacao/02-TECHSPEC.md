# TechSpec — RVM Foundation 0.1

**Estado:** desenho experimental, sujeito aos ADRs e resultados da pesquisa.  
**Data:** 2026-11-01

## 1. Princípios

- **Núcleo sem linguagem hospedeira:** runtime conhece tipos/operações RVM, não sintaxe de Lift ou Rio.
- **Fronteira explícita:** frontends convertem semântica suportada para RIR; tipos sem mapeamento não são aproximados silenciosamente.
- **Verificação prévia:** nenhuma execução ocorre antes da verificação completa e da checagem dos limites estruturais.
- **Interpretação primeiro:** validar semântica de RIR com executor pequeno antes de congelar bytecode ou adicionar JIT.
- **Sem efeito ambiente:** efeitos futuros entram por capacidades explicitamente concedidas pelo host.
- **Evidência antes de consolidação:** interfaces, serialização e ownership permanecem propostas até passar experimentos e revisão.

## 2. Pipeline proposto

~~~text
fonte Lift ─┐
            ├─ frontend/adaptador por linguagem ─ RIR experimental
fonte Rio ──┘                                      │
                                                   ├─ verificador
                                                   ├─ intérprete de RIR (oráculo)
                                                   └─ lowering experimental → bytecode
                                                                      │
                                                               verificador BC
                                                                      │
                                                              executor Rust
~~~

Nomes e pontos de integração dos frontends são desconhecidos até auditoria. O diagrama é meta de pesquisa, não arquitetura já encontrada nos projetos.

## 3. Componentes de referência

Separação conceitual; nomes de crates são ilustrativos:

- rvm-spec: semântica, tipos, códigos de erro e versionamento.
- rvm-ir: estruturas e validação estrutural da RIR.
- rvm-verify: tipos, fluxo de controle, alvos e referências.
- rvm-interpreter: execução de referência e semântica dos traps.
- rvm-bytecode: codificação experimental e decodificador limitado.
- rvm-runtime: quotas, chamadas, valores e capabilities do host.
- rvm-cli: ferramenta de desenvolvimento; não concede capacidades por padrão.
- frontends/lift e frontends/rio: adaptadores isolados, habilitados após baseline.
- conformance: corpus especificado independentemente da implementação.

A estrutura do repositório pode ser diferente se preservar os limites de dependência. A divisão em crates não está aprovada como necessária.

## 4. Núcleo semântico candidato

Tipos iniciais: unit, bool e i64 assinado. Sem coerção implícita. string, f64, arrays, registros, referências, funções como valores e tipos estruturais ficam para decisão posterior.

Operações candidatas: constantes, comparações, soma/subtração/multiplicação com overflow verificado, desvio condicional, chamada estática e retorno. Divisão, conversão numérica e string aguardam definição de edge cases.

Soma, subtração e multiplicação fora de [-2^63, 2^63-1] produzem trap integer_overflow. O executor não pode depender do modo debug/release do Rust. Esta semântica é proposta e exige testes de borda.

## 5. Verificador

O verificador experimental valida:

- limites de módulos, funções, blocos, parâmetros e instruções antes de grandes alocações;
- IDs únicos e referências existentes;
- um terminador por bloco e destinos de branch existentes;
- assinatura, aridade e tipos de argumentos/retorno;
- condição de branch booleana;
- definição única e uso válido de cada valor, respeitando dominância/escopo;
- retorno compatível com assinatura;
- imports e entry point explícitos, sem resolução dinâmica;
- operação desconhecida e versão/codificação não suportada.

Falhas do verificador são erros de módulo, não traps de execução. Verificação não substitui quotas de execução.

## 6. Runtime e limites

Executor de referência sugerido: interpretador single-thread determinístico, sem JIT e sem acesso ambiente. Limites explícitos por execução incluem tamanho do módulo, quantidade de instruções/valores, profundidade de chamada e fuel.

Regra preliminar de fuel: débito antes de cada instrução/terminador despachado; se saldo for insuficiente, retorna out_of_fuel sem despachar. Chamada não pode resetar o saldo. O contrato final precisa definir a contagem exata.

Rust seguro reduz classes de erro de memória, mas não basta para estabelecer isolamento de segurança. Bugs lógicos, DoS, efeitos do host, parser e dependências continuam no modelo de ameaça.

## 7. Categorias de erro

1. **Decode:** bytes/texto malformado, versão desconhecida, tamanho excedido.
2. **Verificação:** RIR/bytecode estrutural ou tipadamente inválido.
3. **Trap semântico:** overflow ou outro comportamento definido que impede resultado normal.
4. **Limite de recurso:** fuel, depth ou contagem excedidos.
5. **Capability:** tentativa futura de usar recurso ausente/negado.

Mensagens humanas podem mudar; códigos/categorias de conformidade devem ser estáveis na revisão experimental.

## 8. Interoperabilidade

No primeiro protótipo, interoperabilidade significa módulos distintos chamando funções com tipos primitivos idênticos da RVM, por valor, sem ponteiro nativo. Isso ainda não resolve objetos, strings, closures, ownership ou estado compartilhado. ABI candidata limita-se a unit, bool e i64 e mantém chamadas no mesmo runtime.

## 9. Memória e concorrência

O primeiro runtime é single-thread e não oferece heap de objetos ao programa convidado. Isso reduz o escopo, mas não demonstra isolamento multilinguagem completo. Pesquisa futura compara cópia, move/transferência exclusiva, compartilhamento imutável e capabilities/handles. Compartilhamento mutável sem sincronização explícita é proibido nesta proposta.

## 10. Segurança e releases

O MVP não executa bytecode sem validação, fornece APIs ambiente por padrão nem converte automaticamente módulos para executáveis nativos. Para cargas hostis em produção, a documentação ainda não estabelece sandbox completa. Uma eventual oferta exigirá modelo de ameaças atualizado, quotas, revisão independente, fuzzing e avaliação do isolamento do processo hospedeiro.

## 11. Questões em aberto

- Representação SSA ou não no RIR final? Ver ADR-0002.
- Bytecode em registradores, pilha ou forma que espelhe RIR? Ver ADR-0004.
- Definição/custo de operações de memória e chamada.
- Mapeamento de tipos Lift/Rio e preservação de erros.
- Local do verificador e proteção contra parser adversarial.
- Quotas de memória em allocator controlado sem prometer tempo real.
- Estratégia ABI para compostos, callbacks e capabilities.
