# ADR-0017 — Integração Lift bloqueada com evidência

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** item 7 do "Próximo passo sugerido" (§12) do
  `RELATORIO-IMPLEMENTACAO.md` ("auditar Lift (commit imutável, licença) ou fechar
  ADR-0007 como bloqueado"); lacuna 11 de §9; ADR-0007 da fundação (aberto);
  ADR-0001 (runtime independente de linguagem).

## Contexto

O ADR-0007 da fundação deixou a rota de integração com Lift **aberta, com
auditoria pendente**. O relatório de implementação registrou que o Lift não foi
auditado (lacuna 11). O item 7 pede auditar o Lift com commit imutável e licença
**ou** fechar a integração como bloqueada.

A auditoria foi executada (inspeção estática; ver
`docs/RELATORIOS/BASELINE-LIFT.md`, LIFT-BASELINE-001):

- repositório público `https://github.com/maquineh/lift`, commit
  `691c344880ac85056bc4bc85bf1e513654769bb1`;
- **licença desconhecida**: sem `LICENSE`/`COPYING`/`NOTICE` e sem campo
  `license` em nenhum manifesto;
- AST e HIR **públicas** (`ProgramAst`, `HirProgram`) e `emit_rust(&HirProgram)`,
  mas a execução é **compilação nativa + Cargo**, não VM/trap;
- 6 dependências externas (`ariadne`, `logos`, `clap`, `serde`, `serde_json`,
  `toml`);
- HIR cobre recursos fora do subset EXP-0 (arrays, objetos, interfaces,
  opcionais/`Result`, closures, imports, ownership).

## Decisão

1. Para **este repositório**, a integração Lift é **bloqueada** — não há
   adaptador, linkagem nem cópia de código nesta entrega.
2. O bloqueio se sustenta em quatro fatos: licença desconhecida; modelo de
   execução de compilação nativa (não oferece trap/quotas de VM); dependências
   externas incompatíveis com a política de zero dependências; e descompasso
   entre a HIR do Lift e o subset `unit|bool|i64` do EXP-0.
3. Qualquer rota futura (consumir a HIR pública como oráculo, adaptador externo
   como processo separado, ou frontend mínimo independente) exige **novo ADR**,
   esclarecimento de licença com o mantenedor do Lift e recorte de subset com
   testes diferenciais.
4. O ADR-0007 da fundação **não é modificado** (o pacote `docs/fundacao/` é
   preservado intacto); este ADR registra o estado no escopo do repositório.
5. Nenhum código do Lift é copiado, adaptado ou redistribuído aqui.

## Alternativas

1. **Manter ADR-0007 aberto sem auditar:** ignora o item 7 e a lacuna 11;
   rejeitado.
2. **Integrar via API pública do HIR agora:** rejeitado enquanto a licença for
   desconhecida e sem recorte de subset; importaria dependências externas ao
   núcleo (ADR-0001).
3. **Duplicar lexer/parser do Lift:** rejeitado — criaria uma linguagem fictícia e
   divergiria da fonte (`FUNDACAO-README.md`).

## Consequências

- Positivas: a lacuna 11 é fechada com evidência; H03/H06 e a integração de
  frontend deixam de depender de uma rota inexistente; o núcleo permanece
  independente de linguagem e sem dependências.
- Negativas: a RSUVM continua sem frontend real; o critério (e) do PRD (§8)
  permanece "bloqueado com evidência" para Lift e Rio.
- Residual: a licença do Lift é desconhecida e pode mudar; reabrir exige nova
  auditoria de commit.

## Validação

- `docs/RELATORIOS/BASELINE-LIFT.md` registra commit, licença, dependências, API
  pública e bloqueios; nenhum item é resultado de execução.
- Nenhum arquivo do Lift existe no repositório: busca por identificadores do Lift
  (`lift_sema`, `HirProgram`, `emit_rust`, `ProgramAst`) fora de
  `docs/RELATORIOS/BASELINE-LIFT.md` e deste ADR retorna apenas citações.
- ADR-0007 da fundação permanece byte a byte inalterado.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D
  warnings` e `cargo fmt --all -- --check` seguem verdes.

## Atualização (2026-11-02)

O **mantenedor declarou `MIT OR Apache-2.0`** para o Lift (commit `5e59af5`,
publicado em `https://github.com/maquineh/lift`). O **bloqueio de licença** caiu;
os outros três motivos **permanecem**: modelo de execução por compilação nativa +
Cargo (não VM/trap), dependências externas incompatíveis com a política de zero
dependências e descompasso de subset (HIR rica × `unit|bool|i64`). A decisão de
**não integrar** segue válida por esses motivos; um adaptador futuro continua
exigindo ADR próprio.
