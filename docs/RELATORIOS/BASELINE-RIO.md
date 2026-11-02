# Baseline Rio — RIO-BASELINE-001

- **Data da auditoria:** 2026-11-01
- **Método:** inspeção estática do clone local e `git ls-remote`; **nenhuma build,
  teste ou execução** do repositório Rio foi realizada
- **Fonte do pedido:** mantenedor indicou
  `https://git.prodam.am.gov.br/gsist/darqq/rio-lang` nesta sessão; o clone local
  já existia em `H:\projetos\rio-lang`
- **Commit auditado:** `5566f4f2e3993b7ba6d5e54bf7d25bf2f2095d01` (branch
  `codex/documentacao-linguagem`), que contém `d71da1a6ea5422235d5b7038d4c84a6726363fd1`
  (`refs/heads/main` remoto) mais três commits de documentação
- **Licença:** **`MIT OR Apache-2.0`** — na auditoria original era **desconhecida**
  (sem `LICENSE`/`COPYING` na raiz e sem campo `license`); foi **declarada pelo
  mantenedor em 2026-11-02** (commit `fb1a310`, branch `chore/licenca-mit-apache`)
- **Veredito do baseline:** disponível para leitura, mapeamento semântico e
  **reuso sob `MIT OR Apache-2.0`**; **não integrado** nesta entrega

> Todos os itens abaixo são **evidência documental** (declaração do arquivo) ou
> fato de inspeção estática. Nenhum é resultado de execução do Rio.

## 1. Identidade e governança

| Item | Evidência |
|---|---|
| Crate raiz `rio-lang-prototype` v0.21.0, `edition = "2021"`, `publish = false` | `Cargo.toml:1-7` |
| Biblioteca `rio_lang` (`src/lib.rs`) e binários `rio`, `rio-sqlite` | `Cargo.toml:9-19` |
| Histórico: 16 commits, autor único (`raphael.maquine@prodam.am.gov.br`) | `git rev-list --count HEAD`, `git log --format='%ae'` |
| Árvore de trabalho suja: 17 arquivos modificados + 4 não rastreados (docs/medições) | `git status --porcelain` |
| `Cargo.lock` versionado; sem `rust-toolchain.toml` | raiz do repositório |
| CI GitLab: estágio `validate`, imagem `rust:1.94.1-slim-bookworm`, timeout 45m, artefatos em `out/validation/` | `.gitlab-ci.yml:1-30` |
| Windows e macOS existem mas só rodam com `RIO_WINDOWS_CI=1` / `RIO_MACOS_CI=1` | `.gitlab-ci.yml:32-60` |
| `AGENTS.md` próprio do projeto (instruções para agentes) | `AGENTS.md` (5.376 bytes) |
| Documentação extensa: `README.md` 94.870 B, `DESIGN.md` 65.093 B, `VALIDATION.md` 123.268 B, `CHANGELOG.md` 34.410 B | raiz |
| Nenhum `unsafe` em `src/*.rs` (0 ocorrências) | contagem por `Select-String -Pattern '\bunsafe\b'` |

**Licença:** declarada pelo mantenedor em 2026-11-02 (`MIT OR Apache-2.0`) e
**publicada** em `main` (commit `fb1a310`). Na auditoria original a licença era
desconhecida; o bloqueio de licença foi **resolvido** pelo titular dos direitos.

## 2. Estrutura e dependências

- 89 arquivos `.rs` e ~33.235 linhas (excluindo `target/`, `third_party/`, `out/`).
- Diretórios: `src/`, `tests/` (~50 arquivos), `benches/` (`pipeline`, `memory`,
  `frontend` — `harness = false`), `examples/`, `docs/` (40+ arquivos),
  `packages/rio_app`, `packages/rio_web`, `editors/`, `deploy/`, `scripts/`,
  `third_party/`.
- Dependências externas declaradas (`Cargo.toml:21-31`): `mio 1.2.4` (net/poll),
  `rustls 0.23.46` (ring/std/tls12), `webpki-roots 1.0.9`, `rusqlite 0.40.2`
  (`bundled` — compila SQLite em C), `serde`, `serde_json`, `futures-util 0.3`,
  `tokio 1` (io-util/net/rt/time), `tokio-postgres 0.7.18`,
  `tokio-postgres-rustls 0.14.0`.
- Perfil release com `overflow-checks = true` (`Cargo.toml:41-42`).

Observação de escopo: são 10 dependências externas com rede, TLS, SQLite nativo e
Postgres. A RSUVM tem política de zero dependências; qualquer adaptador Rio
executado no host RSUVM importaria essa superfície (ou seria um processo externo).

## 3. Pipeline da linguagem

| Etapa | Arquivo (tamanho) | Superfície |
|---|---|---|
| Lexer | `src/lexer.rs` (13.338 B) | privada (`mod lexer;`) |
| Parser | `src/parser.rs` (32.412 B) | privada (`mod parser;`) |
| AST | `src/ast.rs` (6.419 B) | **`pub(super)` apenas** — não reutilizável fora do crate |
| Tipos/anotações | `src/types.rs` (32.766 B) | privada (`mod types;`) |
| Análise semântica | `src/analyzer.rs` (71.001 B) | privada (`mod analyzer;`) |
| Emissão de código | `src/emitter.rs` (49.752 B) | privada; emite **Rust** |
| Runtime auxiliar | `src/runtime.rs` (4.567 B) | privada; helpers para o código emitido |
| Builtins nativos | `src/native.rs` (52.632 B) | privada |
| Runtime assíncrono | `src/execution.rs` (21.036 B) | `pub mod execution` (futures próprios: `block_on`, `spawn_local`, `sleep`, `Scope`) |
| Módulos | `src/modules.rs` (27.839 B) | privada; `pub use modules::{...}` em `lib.rs:38-41` |
| CLI | `src/main.rs` (14.652 B) | binário `rio` |
| LSP | `src/lsp.rs` (29.570 B) | `pub use lsp::run_stdio as run_language_server` (`lib.rs:46`) |

Declarações em `src/lib.rs`: módulos de linguagem (`lexer`, `ast`, `parser`,
`types`, `analyzer`, `emitter`) são privados (`lib.rs:7-25`). A única função
pública de compilação é:

~~~rust
pub fn compile(source: &str) -> Result<String, Diagnostic>   // lib.rs:68
~~~

Isto é: **a saída pública é código-fonte Rust, não AST, HIR, IR ou bytecode.**

Execução: `src/main.rs` localiza `rustc` (`RUSTC` ou PATH, `main.rs:115-118`),
invoca `rustc` (`main.rs:118-125`), e o subcomando `run` roda o binário gerado
(`main.rs:226-271`). O `--help` declara que `run/build/test` precisam de `rustc`
(`main.rs:60-61`). Portanto o modelo é **compilação nativa + execução de processo**,
não uma VM.

## 4. Semântica observável no código e nos testes

| Recurso | Evidência | Observação |
|---|---|---|
| Inteiro | `ExpressionKind::Integer(i64)` (`ast.rs:67`) | `i64`, não `f64` |
| Ponto flutuante | `ExpressionKind::Float(f64)` (`ast.rs:68`) | fora do subset RSUVM |
| Booleano | `ExpressionKind::Boolean(bool)` (`ast.rs:70`) | compatível com `bool` |
| Texto | `ExpressionKind::Text(String)` (`ast.rs:71`) | fora do subset |
| Operadores binários | `Add`, `Subtract`, `Multiply`, `FloorDivide`, `Divide`, `Equal`, `NotEqual`, `Less`, `LessEqual`, `Greater`, `GreaterEqual`, `And`, `Or` (`ast.rs:16-30`) | divisão existe; EXP-0 não a define |
| Overflow de `+ - *` | emitter escolhe `checked_add`/`checked_sub`/`checked_mul` e falha com `"integer overflow in '<op>'"` (`emitter.rs:769-775`) | semântica equivalente a trap; mecanismo é `panic!` no processo |
| Overflow de `//` | `panic!("Rio runtime error ... integer overflow in '//'")` (`runtime.rs:24`) | trap por pânico |
| Unário `-` | `"integer overflow in unary '-'"` (`emitter.rs:738`) | **semântica documental prévia não cobria**: o RIR-EXP-0 não tem negação unária |
| Checagem de overflow no código gerado | `-C overflow-checks=yes` (`cache.rs:19`) | reforça o comportamento em release |
| `if/else` | `StatementKind::If` (`ast.rs:167-171`) | — |
| `return` | `StatementKind::Return(Option<ExpressionId>)` (`ast.rs:177`) | opcional |
| Função | `struct Function { is_async, name, parameters, return_annotation, body }` (`ast.rs:193-200`) | `async` existe |
| Chamada | `ExpressionKind::Call { target, arguments }` (`ast.rs:92-95`) | com `CallTarget::Qualified` |
| `print` | `StatementKind::Print` (`ast.rs:165`) | efeito de host; proibido no MVP RSUVM |
| Imports/módulos | `struct Import`, `Program.imports` (`ast.rs:208-214, 231-242`) | fora do EXP-0 |
| Tipos declarativos | `Option<TypeAnnotation>` em parâmetros/retorno (`ast.rs:181-199`) | anotações opcionais |
| Registros | `struct Record { fields }` (`ast.rs:202-206`) | fora do subset |
| Recursão | não localizada evidência explícita nesta inspeção | **desconhecido** |

## 5. Testes e comandos declarados

`scripts/validate.py` executa, nesta ordem (`scripts/validate.py:32-40`):
`rustc -vV`, `python3 scripts/check-encoding.py`, `cargo fmt --all -- --check`,
`cargo check --offline --locked --all-targets`, `cargo test --offline --locked`,
`cargo run --offline --locked --example native_runtime` e
`cargo run --offline --locked --bin rio -- test examples --timeout-ms 10000`,
gravando log por etapa em `out/validation/`.

Nada disso foi executado nesta auditoria. O `Cargo.lock` está versionado e o CI
usa `--locked`, o que permite reprodução futura; `out/` e `target/` já existem no
clone local.

## 6. Bloqueios e riscos para um adaptador Rio→RIR

1. **Sem API de AST/HIR pública.** Todos os módulos da linguagem são privados e os
   tipos de `ast.rs` são `pub(super)` (`lib.rs:7-25`, `ast.rs:1-6`). A única
   entrada pública é `compile()` → `String` (Rust). Um adaptador dentro do crate
   `rsuvm` não consegue consumir a AST sem alterar o Rio ou duplicar lexer/parser.
2. **Licença — resolvida e publicada.** Declarada `MIT OR Apache-2.0` pelo
   mantenedor (commit `fb1a310` em `main`); o reuso passa a ser permitido sob
   esses termos.
3. **Modelo de execução é compilação nativa + processo.** `run` compila com
   `rustc` e executa o binário (`main.rs:115-125, 226-271`). Isso não é sandbox, não
   é trap de VM e não permite impostar fuel/profundidade: um adaptador não pode
   reutilizar o backend como executor verificado.
4. **Erros são pânicos de processo.** Overflow vira `panic!` no código emitido ou
   no runtime auxiliar (`emitter.rs:769-775`, `runtime.rs:24,52-53`). Mapear isso
   para `RvmError` exige controle do interpretador, não do binário nativo.
5. **Dependências pesadas** (TLS, HTTP, SQLite bundled, Postgres, Tokio) contra a
   política de zero dependências da RSUVM; um adaptador externo teria de ser
   processo separado ou aceitar essa superfície.
6. **Reprodutibilidade:** árvore suja (17 M + 4 untracked) e branch local com
   commits de documentação sobre `main`. Qualquer auditoria futura precisa fixar
   `d71da1a6` (main) ou o commit exato usado, e registrar o diff não commitado.
7. **Subset não é TypeScript:** `FloorDivide`/`Divide` e `f64` existem; anotações
   são opcionais; `async`, registros, listas, mapas e comprehensions estão na AST.
   Mapear para `unit|bool|i64` exige recorte explícito, com testes — não é
   tradução direta.

## 7. O que permanece desconhecido

- Se o analisador constrói uma representação tipada intermediária reutilizável
  internamente (não há evidência de HIR pública; `types.rs` não foi analisado linha
  a linha nesta auditoria).
- Semântica exata de recursão, escaping e ordem de avaliação no analisador.
- Contagem e nomes exatos de testes (não executados).
- Número de dependências efetivamente linkadas no binário `rio`.
- Se há permissão/licença para reutilizar código — **resolvido**: `MIT OR
  Apache-2.0` declarada pelo mantenedor (commit `fb1a310`).
- Desempenho, alocação e comportamento em entradas adversariais.

## 8. Próximos passos recomendados (sem implementar)

1. Licença **declarada** `MIT OR Apache-2.0` pelo mantenedor (commit `fb1a310`);
   publicar a branch no remoto interno.
2. Solicitar ao Rio uma API pública mínima — `parse(source) -> AST` serializada ou
   um dump JSON estável — em vez de duplicar o parser.
3. Fixar commit imutável (`d71da1a6` ou outro) para o adaptador e registrar hash.
4. Definir o subset Rio: funções puras com parâmetros/retorno `i64`/`bool`,
   `+ - *` com overflow, comparações e `if/else`; excluir `print`, imports,
   `f64`, `async`, listas/mapas/registros.
5. Só então propor ADR de adaptador (ADR-0008 continua aberto) e um caso mínimo de
   teste diferencial.
