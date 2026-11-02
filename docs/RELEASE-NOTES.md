# RSUVM — release experimental

> **Este é um alpha de pesquisa.** Single-tenant, não é sandbox, não isola
> processo/OS e não suporta multi-tenant. Formatos IR/texto/bytecode e API Rust
> são experimentais e podem mudar sem compatibilidade.

## Destaques

- Verificador de RIR, intérprete determinístico e quotas de fuel/profundidade/
  memória modelada.
- Tipos `unit`, `bool`, `i64`, `string`, `f64`, `array`, `struct`, `option` e
  `result`.
- Formato textual e bytecode experimentais, com round-trip.
- Ligação estática inter-módulo.
- ABI de host `read_file`/`filesystem_read` com capability, escopo canonicalizado
  e teto de 1 MiB por leitura.
- CLI `rsuvm` (`run`, `run-bc`, `check`, `fmt`, `encode`).

## Segurança e limites

- Default deny; `execute` sem host nega efeitos.
- Escopo de arquivo absoluto, sem `..`, canonicalizado; symlink/junção para fora
  é negado; apenas arquivo regular.
- Reserva de quota antes do IO; teto de 1 MiB por leitura.
- **Não** há quota agregada de IO/tempo/taxa, isolamento de SO ou auditoria
  independente.
- TOCTOU e hard links permanecem residuais.

Leia `SECURITY.md`, `docs/GUIA-USUARIO.md` e `docs/COMPATIBILIDADE.md`.

## Verificação

- `cargo test --workspace --locked`
- `cargo test --release --workspace --locked`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo fmt --all -- --check`
- CI e resultados: link do run desta tag.

## Checksums

Baixe `SHA256SUMS` junto dos artefatos e verifique antes de executar.

## Reporte de segurança

Não abra issue pública. Use
<https://github.com/maquineh/rsuvm/security/advisories/new>.
