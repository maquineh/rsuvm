# Integração experimental com Lift

**Estado da evidência:** consulta documental ao repositório público em 2026-11-01; sem clone, auditoria de fonte ou execução nesta entrega.  
**Repositório indicado:** <https://github.com/maquineh/lift>

## O que a documentação declara

O README atual descreve Lift como linguagem pequena, estaticamente tipada, familiar a um subconjunto de usuários de TypeScript (não implementação completa de TypeScript), que compila entradas .lift e módulos explícitos para Rust e usa Cargo para verificar/compilar/executar. O README lista tipos primitivos, arrays, interfaces/literais de objeto, fluxo de controle e padrões de ownership cobertos por exemplos/testes. Também declara dependência de Rust/Cargo/linker, CI Windows/Ubuntu e limites de FFI. São declarações documentais do projeto Lift, não resultados independentes desta Foundation. Fontes: [README](https://github.com/maquineh/lift) e [README em português](https://github.com/maquineh/lift/blob/main/README.pt-BR.md).

A documentação apresenta geração de Rust/Cargo. Ela **não confirma** API pública de HIR, crate semântico consumível externamente nem backend Lift→RIR. Não transportar para esta documentação a alegação anterior de que um ponto lift_sema/HIR estável foi confirmado.

## Proposta de integração

1. Fixar commit e registrar hash, licença, manifests, CI, dependências e limites da versão.
2. Inspecionar fonte, testes e documentação de parser/análise/codegen; mapear estruturas públicas e internas.
3. Escolher rota em ADR: API/crate reutilizável se adequada; integração isolada no projeto RVM; ou frontend mínimo independente para subset. Não presumir que qualquer rota esteja pronta.
4. Selecionar programas simples com semântica documentada. Registrar compilador, commit e resultado de referência. Backend nativo pode servir como oráculo em fixture confiável; código nativo não é sandbox.
5. Implementar adaptador de subset e comparar resultado/traps das operações definidas.
6. Não alterar upstream, não trocar backend padrão e não assumir compatibilidade com a linguagem inteira.

## Subset inicial candidato

Funções estáticas, parâmetros/retorno primitivos e condicionais, nos casos em que Lift e RIR coincidirem explicitamente. int/number, arrays, interfaces, opcionais, Result, closures, I/O, FFI e ownership precisam de mapeamento e testes próprios. “Parecido com TypeScript” não implica semântica TypeScript.

## Checklist de auditoria

- [ ] Commit/hash e licença verificados.
- [ ] Build e matriz CI reproduzidos ou limitações anotadas.
- [ ] Estrutura de frontend/codegen observada no código, sem nomes presumidos.
- [ ] API pública versus privada identificada.
- [ ] Semântica de números, nulidade, erros, arrays, objetos e funções anotada.
- [ ] unsafe, subprocessos, geração de arquivos e chamadas Cargo revisados.
- [ ] Exemplos/testes relevantes executados com comandos registrados.
- [ ] Opção de integração, riscos, manutenção e divergências registrados em ADR.

## Limite temporal

Documentação consultada em 2026-11-01. Branch main é mutável; implementação deve apontar para commit imutável e revalidar o material.
