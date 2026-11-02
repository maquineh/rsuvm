# Relatório das etapas A — RSUVM

Este relatório acompanha a execução dos itens do "Próximo passo sugerido" (§12) do
`RELATORIO-IMPLEMENTACAO.md`, agrupados nas etapas A1–A4 e publicados por push em
`https://github.com/maquineh/rsuvm.git`.

Mapeamento aprovado pelo mantenedor:

| Etapa | Itens do §12 | Tema |
|---|---|---|
| A1 | 1–2 | Remoto/CI + licença do repositório e política de reuso do Rio |
| A2 | 3–4 | Ampliação do corpus/cobertura + quota de memória por frame |
| A3 | 5–6 | Campanha de fuzzing + benchmarks pré-registrados (H06/H03) |
| A4 | 7 | Auditoria do Lift ou fechamento do ADR-0007 como bloqueado |

Convenção: **Resultado** (executado, com comando) é separado de **Proposta** e
**Desconhecido**. Nada aqui foi auditado por terceiros.

## A1 — Remoto/CI, licença e política de reuso do Rio (itens 1–2)

### Entregue

- `LICENSE-MIT` e `LICENSE-APACHE` (Apache License 2.0 integral) na raiz.
- `license = "MIT OR Apache-2.0"` em `[workspace.package]` (`Cargo.toml`) e
  `license.workspace = true` nas nove crates.
- `docs/ADRs/ADR-0014-LICENCA-DO-REPOSITORIO.md` — decisão de licença dupla.
- `docs/ADRs/ADR-0015-POLITICA-DE-REUSO-DO-RIO.md` — Rio apenas como referência
  comportamental; proibido copiar código/AST; sem linkagem.
- Índice `docs/ADRs/README.md` atualizado; seção "Licença" no `README.md`.
- CI ganhou o passo de build do gate documentado em `AGENTS.md §4`
  (`cargo build --workspace --all-targets`): `.github/workflows/ci.yml` e
  `.gitlab-ci.yml`.

### Item 1 (remoto e CI verde)

O item 1 é de processo: manter o remoto atualizado e o CI verde a cada mudança.
Ele é atendido por este fluxo (commit + push por etapa) e pela verificação dos
gates antes de cada push. Os comandos e resultados estão em "Verificação" abaixo e
são repetidos em cada etapa.

### Item 2 (licença e reuso do Rio)

Decisões do mantenedor nesta rodada: licença **dupla MIT OR Apache-2.0**; reuso do
Rio **apenas como referência comportamental**. Registradas nos ADRs 0014 e 0015.

### Verificação (Resultado)

| Comando | Resultado |
|---|---|
| `cargo metadata --no-deps --format-version 1` | `license = "MIT OR Apache-2.0"` em cada crate |
| `cargo build --workspace --all-targets` | `Finished`, exit 0 |
| `cargo test --workspace` | exit 0 (120 testes verdes) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, sem warnings |
| `cargo fmt --all -- --check` | exit 0, sem diferenças |

## A2 — Corpus/cobertura e quota de memória (itens 3–4)

A preencher na etapa A2.

## A3 — Fuzzing e benchmarks (itens 5–6)

A preencher na etapa A3.

## A4 — Lift (item 7)

A preencher na etapa A4.
