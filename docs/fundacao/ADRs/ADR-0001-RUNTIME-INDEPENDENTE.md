# ADR-0001 — Runtime independente das linguagens

- **Status:** Proposto
- **Data:** 2026-11-01
- **Contexto:** RVM pesquisa mais de um frontend (Rio e Lift); embutir semântica de um deles no executor tornaria a máquina dependente de linguagem.

## Decisão proposta

Manter núcleo, tipos, verificador e executor independentes de sintaxe e estruturas internas de Lift/Rio. Cada linguagem deve ter seu adaptador para representação comum.

## Alternativas

1. Runtime específico do Rio primeiro e extração posterior.
2. Runtime compartilhado com branches por linguagem.
3. Frontends independentes para contrato comum.

## Consequências

- A fronteira comum exige semântica precisa e pode limitar recursos do subset.
- Adaptadores preservam diferenças, mas criam trabalho de integração.
- Benefício só é confirmado quando dois frontends reais convergirem num corpus definido.

## Validação

Corroborar se ao menos dois frontends gerarem o mesmo subset e executor não consultar identidade da linguagem. Revisar se a representação perder semântica essencial ou exigir regras específicas.
