# ADR-0020 — Fuzzing guiado por cobertura (dependência de desenvolvimento)

- **Status:** Proposto (aguarda aprovação explícita do mantenedor)
- **Data:** 2026-11-02
- **Requisitos:** etapa "próximos passos" 2 (decisão em ADR); lacuna 3 do §9
  ("fuzzing limitado e determinístico"); `AGENTS.md` regras 2 e 12 (dependência
  externa exige justificativa, licença, versão fixada e aprovação);
  ADR-0019 (estratégia de fuzzing atual).

## Contexto

O ADR-0019 manteve a campanha de fuzzing **determinística e em processo** e
adiou o fuzzing guiado por cobertura por exigir dependência externa e toolchain
nightly. Este ADR decide, com fatos verificados em 2026-11-02, **o que** seria
adotado caso o mantenedor aprove, sem ainda alterar o repositório.

Fatos verificados (crates.io/GitHub em 2026-11-02):

| Item | Versão | Licença | Fonte |
|---|---|---|---|
| `libfuzzer-sys` | **0.4.13** | `(MIT OR Apache-2.0) AND NCSA` | crates.io; repo `rust-fuzz/libfuzzer` |
| `cargo-fuzz` (ferramenta, `cargo install`) | **0.13.2** | `MIT OR Apache-2.0` | repo `rust-fuzz/cargo-fuzz` |

- `libfuzzer-sys` embute o runtime libFuzzer do LLVM (C/C++); a licença inclui
  **NCSA** (University of Illinois), que passa a valer para esse componente.
- `cargo-fuzz` arrasta dependências transitivas (clap, tempfile, toml,
  cargo_metadata, rayon, num_cpus, …).
- Fuzzing com libFuzzer exige **nightly** e flags `-Z`/sanitizer (coverage).

## Decisão proposta

1. Adotar `cargo-fuzz` **0.13.2** e `libfuzzer-sys` **0.4.13** como ferramenta de
   **desenvolvimento**, em diretório **separado** `fuzz/` que é um **workspace
   próprio** e fica **excluído** do workspace principal (`[workspace] exclude =
   ["fuzz"]` em `Cargo.toml`). `cargo test --workspace`, `cargo build --workspace
   --all-targets`, clippy e fmt **não** passam a compilar nem a depender dessas
   crates; o invariante de zero dependências do núcleo é preservado.
2. O alvo de fuzz cobre as três superfícies de entrada não confiável — parser
   textual (`rvm_text::parse`), decoder binário (`rvm_bytecode::decode`) e
   verificador (`structure::check` + `verify`) — com orçamento por tempo e
   sementes/corpus declarados.
3. O job de fuzzing roda em **nightly**, separado do CI estável, apenas por
   disparo manual/agendado, com **limite de tempo** e publicação dos artefatos
   (crashes, corpus) como artefato de CI. Falha de fuzzing não é gate do build
   estável.
4. `Cargo.lock` do `fuzz/` é versionado; versões fixadas (`=0.4.13`, `=0.13.2`).
5. A campanha determinística do ADR-0019 **permanece** como verificação rápida e
   reprodutível no CI estável; o fuzzing guiado por cobertura é complementar.

## Alternativas

1. **Não adotar fuzzing guiado por cobertura** (manter só ADR-0019): preserva o
   invariante sem exceções; perde feedback de cobertura.
2. **AFL++/cargo-afl:** também dependência externa e toolchain; sem vantagem
   clara sobre libFuzzer aqui; rejeitado.
3. **Fuzzer caseiro guiado por cobertura:** exigiria instrumentação que o Rust
   estável não expõe; rejeitado.
4. **Adotar no workspace principal:** quebraria o build estável e o invariante;
   rejeitado.

## Consequências

- Positivas: feedback de cobertura real; corpus persistente; cobertura das três
  superfícies hostis; isolado do build estável.
- Negativas: exceção pontual à regra de zero dependências (só em `fuzz/`); nova
  licença (**NCSA**) a rastrear; exige nightly no job de fuzzing; superfície de
  manutenção do `Cargo.lock` do `fuzz/`.
- Residual: nenhuma medição de cobertura foi feita ainda; "sem crash em X" não
  prova ausência de vulnerabilidade.

## Validação (a executar se aprovado)

- `fuzz/Cargo.toml` com versões fixadas e `Cargo.lock` versionado.
- `cargo test --workspace` continua verde **sem** compilar `fuzz/` (workspace
  separado e excluído).
- Job de fuzzing em nightly roda por orçamento declarado e publica artefatos.
- `docs/RELATORIOS/FUZZING-EXP-0.md` registra ferramenta, versão, sementes,
  corpus, tempo, crashes e minimizações.

## Pendência

Tornar este ADR **Aceito experimentalmente** e implementar requer **aprovação
explícita do mantenedor** para a dependência de desenvolvimento, a licença NCSA e
o uso de nightly (regra 12 de `AGENTS.md`). Enquanto não aprovado, o repositório
permanece com a campanha determinística do ADR-0019.
