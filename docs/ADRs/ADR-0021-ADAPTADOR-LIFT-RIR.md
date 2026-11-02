# ADR-0021 — Adaptador protótipo Lift→RIR

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** pedido do mantenedor; ADR-0001 (independência de frontend);
  ADR-0007/ADR-0017 (integração Lift); `AGENTS.md` regra 2 (núcleo sem
  dependências externas).

## Contexto

Com a licença do Lift resolvida (`MIT OR Apache-2.0`, commit `5e59af5`), o
bloqueio jurídico caiu. O Lift expõe **HIR pública** (`lift_sema::analyze` →
`HirProgram`) e o RSUVM só executa a RIR do EXP-0. Para "usar Lift na RSUVM" falta
um **frontend** que traduza programas Lift para a RIR. Pela ADR-0001, o núcleo não
pode importar Lift: adaptadores ficam fora de `crates/`.

## Decisão

1. Criar o adaptador protótipo **`frontends/lift-rir`**, um **workspace Cargo
   separado** (`exclude = ["frontends"]` no workspace principal). O CI estável e
   `cargo test --workspace` **não** o compilam.
2. O adaptador depende das crates do Lift por **git fixado** no commit
   `5e59af55baba7af53f8763140d35c972399d2851` (`lift_ast`, `lift_lexer`,
   `lift_parser`, `lift_sema`) e de `rvm-spec`/`rvm-ir` por path. O **núcleo**
   permanece sem dependências externas.
3. **Subconjunto** suportado: funções com parâmetros/retorno `int` (`i64`),
   `boolean` (`bool`), `string` (`string`) ou `void` (`unit`); `let`/`const`,
   **atribuição**, `return`, `if`/`else if`/`else` (inclusive **merge** e `if` sem
   `else`) e **`while`**; expressões `int`/`boolean`/`string` literais, locais,
   `+ - * / %`, comparações `== != < <= > >=` (`==`/`!=` de `string` por conteúdo),
   negação `-x`, lógica `!`/`&&`/`||` (**curto-circuito**) e chamadas a funções do
   próprio programa.
4. **Lowering CFG/SSA**: cada binding Lift (`BindingId`) é rastreado para o
   `ValueId` corrente; `if/else` e `while` viram blocos com **parâmetros de bloco**
   que carregam o ambiente (phi explícito) no merge do `if` e no cabeçalho do
   `while`; `+ - *` usam `*_i64_checked` (overflow vira trap), como no EXP-0.
5. Fora do subconjunto (`coalesce`, `f64`, arrays, objetos, closures,
   `console.log`, `import`, `struct`, statements de topo) devolve
   [`AdapterError::Unsupported`], nunca um módulo incorreto. Divisão/resto foram
   habilitadas pela **ADR-0022** e `string` pela **ADR-0024** (tipos/ops novos no
   núcleo).
6. **Teste diferencial**: o valor executado na RSUVM é comparado com o **oráculo do
   Lift** (mesmo programa rodado pelo compilador do Lift), registrado em
   `docs/RELATORIOS/ADAPTADOR-LIFT-RIR.md`.
7. O adaptador inclui o binário **`lift-run`** (`src/bin/lift_run.rs`) para rodar
   um `.lift` ponta a ponta na RVM:
   `cargo run --manifest-path frontends/lift-rir/Cargo.toml --bin lift-run --
   <arquivo.lift> <entry> [--arg i64:10]... [--fuel N] [--max-depth N] [--memory N]`.
   Ele imprime o valor no formato do `rsuvm` e usa os mesmos códigos de exit
   (0 ok, 2 módulo, 3 trap, 4 limite, 1 uso/IO).

## Alternativas

1. **Serializar a HIR do Lift (JSON) e traduzir fora:** a HIR do Lift **não**
   deriva `serde`; exigiria alterar o Lift. Rejeitado nesta rodada.
2. **Duplicar lexer/parser do Lift no adaptador:** cria uma "linguagem fictícia" e
   diverge da fonte; rejeitado (`FUNDACAO-README.md`).
3. **Adaptador por processo externo** (rodar o binário Lift e capturar saída):
   possível para oráculo, mas não produz RIR executável na RSUVM; rejeitado como
   rota principal.
4. **Integrar o adaptador no núcleo:** quebraria a ADR-0001 e o invariante de zero
   dependências; rejeitado.

## Consequências

- Positivas: demonstra a rota frontend→RIR com um frontend real; roda um
  subconjunto Lift na RSUVM; o núcleo permanece puro e sem dependências.
- Negativas: cobre só `int`/`boolean`/`void` (sem efeitos, strings, arrays,
  objetos); o adaptador carrega as dependências do Lift (`logos`, `serde`, …) no
  **seu** workspace; depende de um commit fixado do Lift.
- Residual: a HIR do Lift é rica; ampliar o subconjunto (divisão, `while`,
  strings) exige decisões e testes próprios. Não há codegen reverso (RIR→Lift).

## Validação

- `cargo test --manifest-path frontends/lift-rir/Cargo.toml`: **28 testes**
  (`tests/diferencial.rs`: 19; `tests/cli.rs`: 9) — inclui `while` + atribuição,
  `if`/`else if`/`else` com merge, `&&`/`||`/`!` com curto-circuito, recursão
  `fib`, `/` e `%`, `string` (literal e `==`/`!=`), o trap de divisão por zero,
  rejeição de `number` (f64) e os códigos de exit do `lift-run` (0/2/3/4).
- **Diferencial**: `cargo run -p lift_cli -- run examples/rsuvm-oracle.lift` no
  Lift produz `30 5 7 9 3 true false 25 12 55 0`; `rsuvm-oracle2.lift` produz
  `15 0 7 0 1 2 10 3 6 0` — idênticos ao executado na RSUVM
  (`docs/RELATORIOS/ADAPTADOR-LIFT-RIR.md`).
- `cargo test --workspace` e `cargo build --workspace --all-targets` (núcleo)
  seguem verdes e **não** compilam `frontends/`.
