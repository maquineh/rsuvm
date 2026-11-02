# Instruções para agentes de implementação — RSUVM

Estas regras valem para qualquer agente (humano ou LLM) que altere este repositório.
Elas derivam de `docs/fundacao/FUNDACAO-AGENTS.md` (pacote RVM-Foundation-0.1) e são
mais restritivas onde este repositório já tem código.

## 1. Estado do projeto

RSUVM é um **protótipo experimental de pesquisa**. Nada aqui é formato estável,
sandbox validado ou autorização para produção. Leia antes de editar:

1. `README.md`
2. `docs/ARQUITETURA.md`
3. `docs/CONTRATOS/ERROS-EXP-0.md`, `RIR-EXP-0-IMPLEMENTACAO.md`, `TEXTO-EXP-0.md`
4. `docs/fundacao/SEGURANCA/GUARDRAILS-DEEPSEEK.md` e `MODELO-AMEACAS.md`
5. O ADR da decisão que você vai tocar (`docs/ADRs/`).

## 2. Inventário antes de editar

- Leia manifests, testes e CI existentes antes de propor mudanças.
- Relate fatos com caminho e linha. Não invente módulo, tipo, API, teste ou medição.
- Desconhecido é uma resposta válida; chute não é.

## 3. Invariantes de código (obrigatórias)

1. **Rust seguro**: `unsafe`, FFI, `transmute`, ponteiro cru ou carregamento dinâmico
   são proibidos (o workspace aplica `unsafe_code = "forbid"`). Exceção exige ADR,
   modelo de ameaça, teste e aprovação do mantenedor.
2. **Sem dependências externas**: apenas `std`. Crate nova exige justificativa,
   licença, versão fixada e aprovação (regra 12 da Foundation).
3. **Entrada não confiável**: texto, bytecode, nomes, tamanhos, índices e offsets são
   hostis. Valide limites **antes** de alocar, indexar, recursar ou despachar.
4. **Falha fechada**: código de biblioteca não pode `panic!`, `unwrap`, `expect` nem
   indexar direto; use `get`, `checked_*`, `try_into` e propague `RvmError`.
   `cfg_attr(test, allow(...))` já existe para testes unitários.
5. **Sem efeitos implícitos**: nenhuma operação de SO (arquivo, rede, ambiente,
   relógio, processo) é oferecida ao convidado por padrão. A única exceção é
   `filesystem_read` via `host_read_file`, concedida explicitamente pelo host com
   escopo (ADR-0036); `execute` sem host nega tudo.
6. **Determinismo**: mesma versão + módulo + limites + configuração (+ host, quando
   houver efeito explícito) ⇒ mesmo valor/trap. Não dependa de `HashMap`, tempo de
   parede, ordem de arquivos ou `debug_assertions` no caminho puro.
7. **Independência de frontend**: nenhum crate do runtime pode importar ou citar
   Lift/Rio. Adaptadores vivem fora do núcleo (`frontends/`, ainda inexistente).
8. **Sem `unsafe` em testes** e sem teste que dependa de rede, porta ou relógio.

## 4. Fluxo de verificação

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Um teste só conta como executado se o comando e o resultado aparecerem no relatório.
Nunca declare conformidade, fuzzing ou auditoria sem execução registrada.

## 5. Contratos e documentação

- `docs/CONTRATOS/` descreve o que o código **deve** fazer. Se divergirem, abra
  relatório de divergência em `docs/RELATORIOS/`, não "conserte" o teste.
- Não edite teste de referência nem relaxe limite para mascarar falha.
- Toda decisão que feche questão aberta exige ADR em `docs/ADRs/` com status,
  alternativas, consequências e critério de validação.
- Nomes de código em inglês/snake_case; documentação em português do Brasil.
- `README.md` por diretório de código é gerado/verificado pelo skill `egm-bootstrap`;
  não o substitua por comentário solto.

## 6. Relatório final de tarefa

Inclua: arquivos alterados; requisito/ADR atendido; decisão tomada; comandos
executados com resultado exato; testes não executados; risco/lacuna restante;
próximo passo sugerido. Não escreva "seguro", "isolado" ou "compatível" sem
delimitar a propriedade medida.
