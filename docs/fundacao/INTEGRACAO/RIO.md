# Integração com Rio — baseline pendente

**Estado:** desconhecido até obter e auditar o código-fonte de Rio.

A conversa anterior descreveu Rio como linguagem em evolução e citou um possível projeto local C:\projetos\rio-lang, mas o código não foi fornecido nesta entrega e não foi encontrado no workspace atual. A menção não confirma que o projeto exista hoje nem quais recursos estejam implementados.

## Procedimento antes de integrar

1. Obter repositório/arquivo de origem autorizado e registrar commit/hash e licença.
2. Inventariar lexer, parser, AST, semântica, valores, interpretador, erros e testes.
3. Separar recursos implementados/testados de planejados e protótipos.
4. Executar a suíte existente e criar corpus de regressão do comportamento real.
5. Identificar acoplamentos ao interpretador sem refatorar ainda.
6. Produzir RIO-BASELINE-001 com tabela recurso | código/teste | comportamento | lacuna.
7. Só então propor adaptador Rio→RIR em ADR.

## Princípios

- RVM não depende de crate, sintaxe, tipo ou erro exclusivo do Rio.
- Interpretador original, se existir, pode servir de oráculo para programas definidos, mas não substitui especificação.
- Divergência entre Rio e RVM vira registro de caso, não ajuste oculto.
- Não recriar linguagem fictícia para preencher falta do projeto.
- Não substituir executor atual antes de comparação e regressão.

## Primeiro experimento após baseline

Selecionar função pura já coberta por teste de Rio. Definir resultado esperado independentemente; adaptar para RIR; comparar valor, erro e comportamento observável. Aprovação vale só para subset executado.
