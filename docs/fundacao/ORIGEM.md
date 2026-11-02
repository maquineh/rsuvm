# Origem e integridade dos documentos da fundação

**Fonte:** `C:\Users\rapin\Documents\Codex\2026-11-01\referenced-chatgpt-conversation-this-is-an\outputs\RVM-Foundation-0.1`
**Cópia:** 2026-11-01, para `H:\projetos\rsuvm\docs\fundacao\`
**Estado:** os arquivos copiados **não foram modificados**. São evidência
documental de uma proposta experimental; não são especificação validada nem
prova de que algo foi implementado.

## O que foi copiado

- Todo o conteúdo de `docs/` do pacote original (24 arquivos Markdown, incluindo
  `ADRs/`, `CONTRATOS/`, `INTEGRACAO/`, `PROMPTS/`, `SEGURANCA/` e `TESTES/`).
- `README.md` do pacote como `FUNDACAO-README.md`.
- `AGENTS.md` do pacote como `FUNDACAO-AGENTS.md`.

O `AGENTS.md` e o `README.md` deste repositório (`H:\projetos\rsuvm\`) são
documentos **novos**, derivados das regras da fundação e mais restritivos onde já
existe código.

## Verificação de integridade

Executada em 2026-11-01 com `Get-FileHash -Algorithm SHA256` em cada arquivo
original e no correspondente copiado:

- 24 arquivos `.md` originais, 24 copiados;
- comparação por caminho relativo + hash: **nenhuma divergência**.
- Nenhuma build, teste ou execução do pacote de origem foi realizada; portanto não
  há "Resultado" de execução associado a ele.

## Convenção de leitura

- `Evidência documental`: afirmação feita por um destes arquivos.
- `Proposta`: desenho sujeito a revisão.
- `Hipótese`: afirmação testável e falsificável.
- `Desconhecido`: falta inspeção, medição ou decisão humana.
- `Resultado`: só existe após execução reproduzível registrada em
  `docs/RELATORIOS/`.

Os ADRs da fundação (0001..0008) permanecem em `docs/fundacao/ADRs/`; os ADRs
deste repositório começam em `docs/ADRs/ADR-0009-*`.
