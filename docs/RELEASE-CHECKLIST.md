# Checklist de release — alpha

Use este checklist antes de criar a tag. Nenhum item é opcional para um release
público; itens marcados “bloqueante” impedem a tag.

## 1. Escopo e promessa

- [ ] README declara: experimental, single-tenant, **não** sandbox, **não**
      produção (versões EN e PT-BR).
- [ ] `SECURITY.md`/`SECURITY.pt-BR.md` com escopo, limitações conhecidas e canal
      privado.
- [ ] `docs/COMPATIBILITY.md`/`docs/COMPATIBILIDADE.md` com versionamento,
      formatos e plataformas.
- [ ] `docs/IMPLEMENTATION.md` reflete o estado implementado.
- [ ] Nenhuma alegação de multi-tenant, isolamento de SO ou “seguro” sem
      qualificador.

## 2. Versão e documentação

- [ ] `Cargo.toml` com a versão do release (ex.: `0.1.0-alpha.1`).
- [ ] `CHANGELOG.md` e `CHANGELOG.pt-BR.md` atualizados com a data e os destaques.
- [ ] `docs/USER-GUIDE.md` e `docs/GUIA-USUARIO.md` cobrem instalação, comandos,
      opções, códigos de saída, `--allow-fs-read` e limites.
- [ ] `docs/EMBEDDING.md` e `docs/INCORPORACAO.md` cobrem
      `execute`/`execute_with_host`.
- [ ] `docs/RELEASE-NOTES.md` atualizado para a versão.
- [ ] Exemplos (`examples/*.rvm`) verificam e os que não usam host executam com
      a saída esperada.
- [ ] Links internos e badges do README não estão quebrados.

## 3. CI e qualidade

- [ ] `cargo fmt --all -- --check`.
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- [ ] `cargo test --workspace --locked`.
- [ ] `cargo test --release --workspace --locked`.
- [ ] `cargo +1.75.0 check --workspace --all-targets --locked`.
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`.
- [ ] CI verde em Linux e Windows.
- [ ] Adaptador Lift: `cargo test --manifest-path frontends/lift-rir/Cargo.toml`.

## 4. Segurança

- [ ] Revisão interna guiada por `docs/fundacao/SEGURANCA/CHECKLIST-SEGURANCA.md`.
- [ ] Limitações residuais declaradas: TOCTOU, hard links, IO agregado,
      ausência de isolamento multi-tenant.
- [ ] Fuzzing executado por orçamento declarado; resultados registrados.
- [ ] Auditoria independente: executada ou explicitamente ausente.
- [ ] Nenhum segredo, token, caminho pessoal ou dado sensível nos artefatos.
- [ ] `SECURITY.md`/`SECURITY.pt-BR.md` com canal de reporte ativo.

## 5. Artefatos

- [ ] `cargo build --release --locked -p rvm-cli`.
- [ ] `docker build -t rsuvm:alpha .` e `docker run --rm rsuvm:alpha --version`.
- [ ] Smoke test em container/VM limpo:
      `rsuvm --version` e `rsuvm run examples/soma.rvm ...`.
- [ ] Pacotes Linux e Windows com README, licenças e CHANGELOG.
- [ ] `SHA256SUMS` gerado e verificado.
- [ ] Release notes com limitações, benchmark H06 e link para segurança.
- [ ] Tag anotada `vX.Y.Z-alpha.N` criada a partir do commit validado.

## 6. Pós-release

- [ ] GitHub Release publicado com artefatos e checksums.
- [ ] Anúncio com escopo e limitações.
- [ ] Issues/security advisories monitorados.
- [ ] Plano de rollback/yank documentado (se crates.io for usado).
- [ ] Próximo patch/minor registrado no `CHANGELOG.md`.

## Comandos de referência

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --release --workspace --locked
cargo +1.75.0 check --workspace --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo build --release --locked -p rvm-cli
```
