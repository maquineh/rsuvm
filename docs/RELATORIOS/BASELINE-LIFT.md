# Baseline Lift — LIFT-BASELINE-001

- **Data da auditoria:** 2026-11-02
- **Método:** clone raso e **inspeção estática** do fonte; **nenhuma build, teste
  ou execução** do Lift foi realizada
- **Fonte:** `https://github.com/maquineh/lift` (repositório público indicado em
  `docs/fundacao/INTEGRACAO/LIFT.md`)
- **Commit auditado:** `691c344880ac85056bc4bc85bf1e513654769bb1`
  (`refs/heads/main`, `HEAD` no momento do clone; `git ls-remote` confirmou o
  mesmo hash)
- **Licença:** **`MIT OR Apache-2.0`** — na auditoria original (commit `691c344`)
  era **desconhecida**; foi **declarada pelo mantenedor em 2026-11-02** (commit
  `5e59af5`), com `LICENSE-MIT`, `LICENSE-APACHE` e campo `license` em todos os
  manifestos
- **Veredito:** disponível para leitura, mapeamento semântico e **reuso sob
  `MIT OR Apache-2.0`**; **não integrado** nesta entrega (ver §5, demais bloqueios)

> Todos os itens abaixo são **evidência documental** (declaração do arquivo) ou
> fato de inspeção estática. Nenhum é resultado de execução do Lift.

## 1. Identidade e governança

| Item | Evidência |
|---|---|
| Workspace Cargo com 7 crates + 3 pacotes em `stdlib/` | `Cargo.toml:1-10`, `stdlib/` |
| Versão `0.1.0`, `edition = "2021"`, `rust-version = "1.94.1"` | `Cargo.toml:[workspace.package]` |
| Último commit: `test: add Lift acceptance and release gates` | `git log -1` |
| 52 arquivos `.rs`, ~17 254 linhas (sem `target/`) | contagem por `Get-ChildItem`/`Get-Content` |
| Nenhum `unsafe` de verdade: 1 ocorrência, a string `"unsafe"` em lista de palavras reservadas | `crates/lift_sema/src/interop.rs:261` |
| CI GitHub Actions: `frontend-fuzz`, `notes-contract`, `release-acceptance`, `vscode-extension` | `.github/workflows/` |
| `fuzz/` com alvo `frontend` e corpus `core.lift`/`malformed.lift`/`template.lift` | `fuzz/fuzz_targets/frontend.rs`, `fuzz/corpus/frontend/` |

**Licença:** declarada pelo mantenedor em 2026-11-02 (`MIT OR Apache-2.0`,
commit `5e59af5`). Na auditoria original não havia `LICENSE` nem campo `license`;
o bloqueio de licença foi **resolvido** pelo próprio titular dos direitos.

## 2. Estrutura e dependências

Crates: `lift_ast`, `lift_lexer`, `lift_parser`, `lift_sema`, `lift_codegen`,
`lift_project`, `lift_cli`; pacotes `stdlib/lift_core`, `stdlib/lift_db`,
`stdlib/lift_web`.

Dependências externas declaradas (`Cargo.toml:[workspace.dependencies]`):
`ariadne 0.6.0` (diagnóstico), `logos 0.16.1` (lexer), `clap 4.6.7` (CLI),
`serde`/`serde_json` (serialização), `toml 0.8.23` (manifesto de projeto).

Observação de escopo: são dependências de ferramenta/CLI, mais leves que as do
Rio (sem TLS, SQLite nativo, Postgres ou Tokio), mas ainda **incompatíveis** com
a política de zero dependências da RSUVM (`AGENTS.md`, regra 2).

## 3. Pipeline e API pública

| Etapa | Arquivo | Superfície pública |
|---|---|---|
| Lexer | `crates/lift_lexer/src/lib.rs` | `tokenize(source) -> Result<Vec<(Token, Span)>, LexError>`; `Token`, `TemplatePart`, `LexError` |
| Parser | `crates/lift_parser/src/lib.rs` | `parse(tokens) -> Result<ProgramAst, Vec<ParseError>>`; `ParseError` |
| AST | `crates/lift_ast/src/lib.rs` | `ProgramAst`, `Declaration`, `Expression`, `ExpressionKind`, `Statement`, `TypeExpression`, `ProjectAst`, `PackageAst` — **públicos** |
| Análise semântica | `crates/lift_sema/src/lib.rs` | `analyze(...)`, `analyze_project(...)`; **HIR público**: `HirProgram`, `HirFunction`, `HirExpr`, `HirStatement`, `ResolvedType` |
| Codegen | `crates/lift_codegen/src/lib.rs` | `emit_rust(&HirProgram) -> String`; `emit_cargo_manifest(...)` |
| Projeto/CLI/LSP | `lift_project`, `lift_cli` | `forklift`, `lint`, `modules`, `sources`, `lsp` |

Diferença importante em relação ao Rio: o Lift **expõe AST e HIR públicas**
(`ProgramAst`, `HirProgram`) e uma função `emit_rust` — ou seja, é consumível
externamente em princípio. A saída de codegen, porém, é **código-fonte Rust +
Cargo**, não RIR nem bytecode.

## 4. Modelo de execução e semântica observável

| Recurso | Evidência | Observação |
|---|---|---|
| Compila para Rust e usa Cargo | `emit_rust`, `emit_cargo_manifest`; README | não é VM nem trap de VM |
| Testes em CI | `release-acceptance.yml` roda `cargo test --workspace` + stdlib + `verify_notes.py` | reprodutível se a licença permitir |
| Fuzzing de frontend | `fuzz/fuzz_targets/frontend.rs` + corpus | já existe no projeto |
| Tipos/recursos além do subset | HIR expõe `ResolvedType`, `ResourceKind`, `ParameterPassMode`; AST tem arrays, objetos, closures, interfaces, imports | fora de `unit|bool|i64` do EXP-0 |
| Palavra reservada `unsafe` reconhecida | `interop.rs:261` | FFI/unsafe tratados pelo frontend, não pelo runtime |

## 5. Bloqueios e riscos para um adaptador Lift→RIR

1. **Licença — resolvida.** Declarada `MIT OR Apache-2.0` pelo mantenedor
   (commit `5e59af5`); o reuso passa a ser permitido sob esses termos.
2. **Modelo é compilação nativa + Cargo.** `emit_rust` produz Rust; a execução
   usa `rustc`/Cargo. Não é um backend de trap/VM nem permite impostar
   fuel/profundidade como o intérprete da RSUVM.
3. **HIR é rico demais para o subset.** `HirProgram` cobre recursos (arrays,
   objetos, interfaces, opcionais/`Result`, closures, imports, recursos/ownership)
   que o EXP-0 não define. Mapear exige recorte explícito, não tradução direta.
4. **Dependências externas.** `logos`, `clap`, `ariadne`, `serde`, `toml` contra a
   política de zero dependências; um adaptador teria de ser processo separado.
5. **Reutilização via API é plausível, mas condicionada.** Ao contrário do Rio, o
   Lift tem HIR pública; ainda assim, consumir `lift_sema` importaria o crate e
   suas dependências, o que colide com o núcleo independente de linguagem
   (ADR-0001).

## 6. O que permanece desconhecido

- Se o mantenedor autoriza reuso/derivação sob alguma licença.
- Se o build e a suíte passam no ambiente desta máquina (não executados).
- Estabilidade do `HirProgram` como fronteira pública (sem versionamento de API
  observado).
- Semântica exata de números, nulidade, erros, arrays e ownership no analisador
  (não analisada linha a linha).
- Desempenho e comportamento sob entradas adversariais.

## 7. Conclusão

A auditoria do item 7 foi **executada** (commit imutável e licença verificados).
A **licença foi resolvida** (o mantenedor declarou `MIT OR Apache-2.0`, commit
`5e59af5`), mas a integração Lift permanece **fora de escopo** por modelo de
execução não-VM, dependências externas e descompasso de subset. Decisão em
`docs/ADRs/ADR-0017-INTEGRACAO-LIFT-BLOQUEADA.md`. Nenhum código do Lift foi
copiado para este repositório.
