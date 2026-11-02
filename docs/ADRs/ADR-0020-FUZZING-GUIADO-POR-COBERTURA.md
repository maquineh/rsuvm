# ADR-0020 — Fuzzing guiado por cobertura (dependência de desenvolvimento)

- **Status:** Aceito experimentalmente (aprovado pelo mantenedor em 2026-11-02)
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

## Implementação (2026-11-02)

Aprovado e implementado:

- `fuzz/` criado como **workspace separado** (`fuzz/Cargo.toml` com
  `[workspace] members = ["."]`), com `libfuzzer-sys = "=0.4.13"` e versões
  fixadas em `fuzz/Cargo.lock`.
- `Cargo.toml` da raiz ganhou `exclude = ["fuzz"]`: `cargo test --workspace` e
  `cargo build --workspace --all-targets` **não** compilam `fuzz/` (verificado).
- Alvos: `texto`, `decoder`, `verificador` (`fuzz/fuzz_targets/`), com corpus
  inicial em `fuzz/corpus/` e `fuzz/README.md`.
- Job de CI: `.github/workflows/fuzz.yml` (nightly, `ubuntu-latest`, disparo
  manual/semanal, publica artefatos). Não gateia o CI estável.

### Validação executada

| Comando | Resultado |
|---|---|
| `cargo +nightly fuzz build` | **sucesso** (compila os três alvos) |
| `cargo test --workspace` | exit 0 (workspace principal **não** compila `fuzz/`) |
| `cargo +nightly fuzz run texto` | **falha no ambiente local**: `STATUS_DLL_NOT_FOUND` (runtime ASan ausente no toolchain nightly Windows); execução fica no job de CI em Linux |
| `.github/workflows/fuzz.yml` (run `41430066954`) | **sucesso**: `texto` 4 499 017 execuções (cov 1231), `decoder` 8 440 896 (cov 501), `verificador` 4 267 947 (cov 1507); **0 crashes** em 120 s por alvo |

Limite: a execução local do fuzzer no Windows não é suportada sem a DLL de runtime
do ASan; o caminho de execução é o job `fuzz.yml` (Linux nightly). A campanha de
cobertura rodou (~17,2 M execuções, 0 crashes no orçamento), mas "sem crash" nesta
rodada não prova ausência de vulnerabilidade.
