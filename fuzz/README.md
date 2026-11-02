# fuzz — workspace isolado de fuzzing guiado por cobertura

Workspace Cargo **separado** do principal (`exclude = ["fuzz"]` em `../Cargo.toml`),
criado sob o **ADR-0020**. Não é compilado por `cargo test --workspace`,
`cargo build --workspace --all-targets`, clippy nem fmt do CI estável.

## Ferramenta e licenças (fixadas)

| Item | Versão | Licença |
|---|---|---|
| `cargo-fuzz` (ferramenta) | 0.13.2 | MIT OR Apache-2.0 |
| `libfuzzer-sys` (crate) | =0.4.13 | `(MIT OR Apache-2.0) AND NCSA` |

Requer toolchain **nightly**.

## Alvos

- `texto` — `rvm_text::parse`
- `decoder` — `rvm_bytecode::decode`
- `verificador` — `rvm_text::parse` + `rvm_verify::VerifiedModule::verify`

Contrato observado: qualquer entrada **não** causa pânico; o resultado pode ser
`Ok` ou `Err`. "Sem crash em X" não prova ausência de vulnerabilidade.

## Como rodar

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
cargo +nightly fuzz run texto       -- -max_total_time=120
cargo +nightly fuzz run decoder     -- -max_total_time=120
cargo +nightly fuzz run verificador -- -max_total_time=120
```

Corpus inicial em `corpus/<alvo>/`; crashes em `artifacts/<alvo>/`. Versões
fixadas em `Cargo.lock` (versionado).

Verificação do workspace principal (não afetada): `cargo test --workspace`.
