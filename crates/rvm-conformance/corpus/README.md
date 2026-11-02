# Corpus de conformidade — EXP-0

O corpus é **dado**, não código: cada arquivo `.rvm` é um módulo no formato
textual experimental (`docs/CONTRATOS/TEXTO-EXP-0.md`) com a expectativa declarada
em comentários. Os testes em `../tests/` percorrem este diretório, executam o
pipeline `texto → estrutura → verificação → execução` e comparam o resultado.

## Formato do cabeçalho

~~~text
; id: C-001
; expect: value i64:30
; args: i64:10, i64:20
; entry: soma
; fuel: 1000
; max-depth: 8
~~~

- `id:` obrigatório e único no corpus. Os IDs C-001..C-006 e N-001..N-012 vêm de
  `docs/fundacao/TESTES/CONFORMIDADE.md`; IDs acima de N-012 são casos
  adicionais deste repositório e estão identificados como tal.
- `expect:` obrigatório, exatamente um:
  - `value <display>` — execução termina e o valor é exatamente `<display>`
    (`unit`, `bool:true`, `i64:-3`);
  - `error <código>` — o módulo é rejeitado **antes** da execução
    (`ErrorClass::Module`);
  - `trap <código>` — a execução termina em trap semântico (`ErrorClass::Trap`);
  - `limit <código>` — a execução é interrompida por quota
    (`ErrorClass::ResourceLimit`).
- `args:` opcional, lista separada por vírgula de `<tipo>:<valor>`; ausente
  significa "função de entrada sem parâmetros".
- `entry:` opcional; o padrão é a entrada declarada no módulo.
- `fuel:`, `max-depth:` e `memory:` opcionais; o padrão é `ExecConfig::default()`
  (16 MiB de memória modelada).

## Invariantes verificados pelos testes

- Todo `value` também é executado duas vezes: valor e fuel precisam ser idênticos
  (C-006 generalizado).
- Para todo `value` de caso válido, `encode` → `decode` → `verify` → `execute`
  produz o mesmo valor e o mesmo fuel (diferencial RIR↔bytecode).
- N-012 (truncamento/`length` excessivo) não é expressável em texto: ele vive em
  `crates/rvm-bytecode/tests/hostile.rs` e nos testes de propriedade.
- Nenhum arquivo pode ser copiado de um teste de referência para "fazer passar";
  divergência vira relatório em `docs/RELATORIOS/`.
