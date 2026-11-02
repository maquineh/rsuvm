# Propagação `?` no adaptador Lift→RIR — investigação

- **Data:** 2026-11-02
- **Pergunta:** implementar `?` (propagação de `Option`/`Result`) no adaptador
  `frontends/lift-rir`.
- **Resultado:** **bloqueado na origem** — o Lift **não tem** o operador `?` de
  propagação. Não há construto na linguagem/HIR para baixar; implementá-lo exigiria
  alterar o Lift ou inventar sintaxe, o que as regras do projeto proíbem.

## 1. Evidência

1. No parser do Lift, `Token::Question` só aparece **na gramática de tipo**
   (`T?`), dentro de `parse_type`:
   `crates/lift_parser/src/parser.rs:316` (§ `else if let Some(question) =
   self.consume(|token| matches!(token, Token::Question))` monta
   `TypeExpressionKind::Optional`). Não há uso em posição de expressão.
2. Os operadores unários do Lift são apenas `Negate` e `Not`:
   `crates/lift_ast/src/lib.rs:319-323` (`enum UnaryOperator { Negate, Not }`).
3. A HIR tem `OptionSome`, `OptionNone`, `Coalesce` (`??`) e as inspeções
   `option.is_some`/`option.unwrap`/`result.is_ok`/`result.unwrap`/
   `result.unwrap_err`; **não** há variante de "try"/"propagate".
4. A documentação do próprio Lift declara: *"Pattern matching, the propagation
   operator `?`, `try/catch`, unions, and general generics remain
   unimplemented"* e *"Result still has no constructors: values come from imports
   or Lift functions"*.

## 2. Consequência para o adaptador

O adaptador consome a HIR pública do Lift (`lift_sema::analyze`) e traduz o que
existe. Como `?` não existe:

- não há `HirExprKind` correspondente para casar;
- não há como um programa `.lift` válido disparar a propagação;
- qualquer "suporte a `?`" no adaptador seria código morto ou uma extensão
  inventada da linguagem — rejeitado por `AGENTS.md` (não inventar API/medição).

**Nenhum módulo incorreto é produzido**: o adaptador simplesmente nunca encontra
esse caso porque a fonte não o expressa.

## 3. O que já existe e o que o núcleo consegue expressar

- `??` (coalesce) está implementado e testado (ADR-0030/0031): `x ?? y` com
  fallback preguiçoso.
- O **padrão** de `?` é expressável na RIR com o que já existe:
  1. avaliar o `Option`/`Result`;
  2. `option_is_some`/`result_is_ok`;
  3. `cond_br` para um bloco de sucesso e um de propagação;
  4. sucesso: `option_unwrap`/`result_unwrap` e seguir;
  5. propagação: devolver o valor original (no caso de `Result`) ou o `none`
     (no caso de `Option`) imediatamente.

Prova executável no corpus (sem novo opcode):

| Caso | Caminho | Resultado |
|---|---|---|
| `C-056` | `?` com `Ok` | `ok:i64:12` |
| `C-057` | `?` com `Err` | `err:str:e` |

Ambos passam pelo diferencial `texto → encode → decode → verify → execute`
(`cargo test --workspace`, `rvm-conformance`).

## 4. O que desbloquearia

1. **Mudança no Lift**: adicionar um nó de AST/HIR de propagação (`?`),
   com semântica e diagnósticos; depois o adaptador baixa para o CFG acima.
2. **Outro frontend** que já tenha `?` (ex.: um subconjunto Rust), mantendo o
   núcleo independente de frontend (ADR-0001).
3. **Não fazer**: manter `?` fora do subconjunto, como hoje, e usar `??`/`unwrap`
   explicitamente.

## 5. Estado

- `?` no adaptador: **não implementado** (bloqueado na linguagem de origem).
- Núcleo: pronto para expressar o padrão (ops existentes; casos C-056/C-057).
- Sem alteração de código nesta rodada além do corpus de demonstração.
