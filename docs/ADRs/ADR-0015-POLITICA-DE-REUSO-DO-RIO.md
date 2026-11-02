# ADR-0015 — Política de reuso do Rio

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** item 2 do "Próximo passo sugerido" (§12) do
  `RELATORIO-IMPLEMENTACAO.md` ("definir a política de reuso do Rio"); decisão do
  mantenedor nesta rodada.

## Contexto

`docs/RELATORIOS/BASELINE-RIO.md` (RIO-BASELINE-001) estabelece, por inspeção
estática, que o repositório Rio:

- não tem `LICENSE`/`COPYING` na raiz e não declara campo `license` no
  `Cargo.toml` (licença **desconhecida**);
- expõe apenas `compile(source) -> Result<String, Diagnostic>`: o lexer, o parser,
  a AST (`pub(super)`), o analisador e o emissor são privados;
- usa modelo de execução de compilação nativa + processo, com dez dependências
  externas (rede, TLS, SQLite nativo, Postgres, Tokio), incompatíveis com a
  política de zero dependências da RSUVM.

Copiar código, criar obra derivada ou distribuir exige decisão do mantenedor. A
política precisa dizer o que é permitido **sem** presumir autorização.

## Decisão

1. O Rio pode ser usado **apenas como referência comportamental**: observar
   documentação, entradas e saídas, e executá-lo localmente como oráculo *caixa
   preta* para testes diferenciais, sem copiar código.
2. É **proibido** copiar, adaptar, traduzir ou redistribuir código, AST, tipos,
   mensagens internas ou qualquer obra derivada do Rio dentro deste repositório.
   Nenhum arquivo do Rio entra em `crates/` nem em `docs/fundacao/`.
3. O Rio **não** é linkado como dependência: a política de zero dependências
   (`AGENTS.md`, regra 2) permanece. Um adaptador futuro, se existir, será um
   processo externo ou um artefato separado, nunca uma crate do núcleo.
4. Expectativas diferenciais registradas aqui descrevem apenas comportamento
   observado (entrada → saída/trap) e não reproduzem código-fonte do Rio.
5. Qualquer reuso de código, ligação ou redistribuição exige, antes, esclarecimento
   da licença com o mantenedor do Rio e um novo ADR que substitua este. O ADR-0008
   da fundação (integração Rio) permanece aberto e inalterado.

## Alternativas

1. **Bloquear todo contato com o Rio:** mais conservador, mas descarta a única
   rota de teste diferencial disponível sem licença.
2. **Permitir reuso de código:** rejeitado enquanto a licença for desconhecida;
   criar obra derivada sem licença é risco jurídico.
3. **Duplicar lexer/parser do Rio por conta própria:** cria uma linguagem "Rio
   fictícia" e diverge da fonte; rejeitado por `FUNDACAO-README.md`.

## Consequências

- Positivas: viabiliza comparação comportamental sem presumir licença; mantém o
  núcleo independente de linguagem (ADR-0001) e sem dependências externas.
- Negativas: sem acesso à AST/HIR pública, o adaptador não pode ser construído por
  consumo direto do crate; o oráculo é um processo separado e mais lento.
- Residual: a licença do Rio continua desconhecida; a política é uma contenção,
  não uma autorização. Executar o Rio como oráculo ainda depende de a licença
  permitir o uso local, o que não foi confirmado.

## Validação

- Nenhum arquivo ou trecho de código do Rio existe no repositório: busca por
  identificadores do Rio (`rio_lang`, `ExpressionKind`, `StatementKind`) em
  `crates/` e `docs/` fora de `docs/RELATORIOS/BASELINE-RIO.md` e deste ADR retorna
  apenas citações de evidência.
- A política está registrada e o índice de ADRs a referencia.
- ADR-0008 da fundação permanece byte a byte inalterado.
