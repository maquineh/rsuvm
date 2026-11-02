# ADR-0014 — Licença do repositório

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** item 2 do "Próximo passo sugerido" (§12) do
  `RELATORIO-IMPLEMENTACAO.md`; lacuna 6 de §9 ("Licença do projeto não definida —
  pendência do mantenedor").

## Contexto

O repositório foi publicado em `https://github.com/maquineh/rsuvm.git` sem arquivo
de licença e sem campo `license` em nenhum manifesto. Sem licença, o padrão legal é
"todos os direitos reservados": terceiros não podem copiar, modificar nem
redistribuir, mesmo que o código esteja público. Isso bloqueia qualquer reuso,
revisão independente ou integração futura (incluindo um eventual adaptador de
frontend) e é incompatível com o objetivo de pesquisa declarado no `README.md`.

A decisão de licença é do mantenedor (`RELATORIO-IMPLEMENTACAO.md` §9.6). Nenhum
agente deve escolher uma licença sozinho.

## Decisão

1. O repositório adota licença **dupla `MIT OR Apache-2.0`**, o padrão da
   comunidade Rust, permitindo que quem reutiliza escolha a licença que lhe
   convém.
2. A raiz recebe `LICENSE-MIT` e `LICENSE-APACHE` com os textos integrais
   (Apache License 2.0, incluindo o apêndice de aplicação). O titular de copyright
   é `Raphael Maquiné`.
3. `Cargo.toml` declara `license = "MIT OR Apache-2.0"` em `[workspace.package]`;
   cada crate herda o campo com `license.workspace = true`.
4. `publish = false` permanece em todas as crates: a licença autoriza reuso do
   código, mas nenhuma crate é publicada no crates.io por este repositório.
5. O pacote `docs/fundacao/` é mantido intacto como evidência documental
   (`docs/fundacao/ORIGEM.md`); esta decisão cobre o código e a documentação
   próprios deste repositório, não relicencia material de origem que já tenha
   termos próprios.

## Alternativas

1. **MIT isolada:** mais curta, sem concessão explícita de patentes.
2. **Apache-2.0 isolada:** concessão de patentes, porém mais longa e menos comum
   isolada em crates pequenas.
3. **Proprietária / todos os direitos reservados:** manteria o bloqueio de reuso;
   contrário ao propósito de protótipo de pesquisa público.
4. **Adiar a decisão:** mantém a lacuna 6 aberta e o repositório inutilizável por
   terceiros; rejeitado.

## Consequências

- Positivas: reuso e revisão independente deixam de estar bloqueados por ausência
  de licença; a opção Apache-2.0 oferece concessão de patentes; a opção MIT é
  simples para integração.
- Negativas: exige incluir o aviso de copyright ao redistribuir; a licença não
  substitui assessoria jurídica nem avalia patentes de terceiros.
- Residual: a licença de material de terceiros referenciado (Rio, Lift) continua
  desconhecida e é tratada em `ADR-0015` e no baseline do Lift. Licença de código
  não implica segurança do protótipo.

## Validação

- `LICENSE-MIT` e `LICENSE-APACHE` existem na raiz.
- `cargo metadata --no-deps --format-version 1` reporta
  `license = "MIT OR Apache-2.0"` em cada crate.
- `cargo build --workspace`, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings` e
  `cargo fmt --all -- --check` continuam verdes após a mudança de manifesto.
- `README.md` ganha uma seção de licença apontando para os dois arquivos.
