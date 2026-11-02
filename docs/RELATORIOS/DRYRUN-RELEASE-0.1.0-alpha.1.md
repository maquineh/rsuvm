# Dry-run de release — 0.1.0-alpha.1

- **Data:** 2026-11-02
- **Commit:** `49c8cbe` (árvore de preparação do alpha)
- **Objetivo:** validar empacotamento, checksums, instalação limpa e imagem Docker
  **sem** publicar release ou criar tag.
- **Ambiente:** Windows 10.0.26300, rustc/cargo 1.94.1, Docker 27.3.1
  (Linux containers), imagem de build `rust:1.94-slim-bookworm`, runtime
  `debian:bookworm-slim`.

## 1. Artefatos gerados localmente

| Arquivo | Conteúdo |
|---|---|
| `rsuvm-windows-x86_64.zip` | `rsuvm.exe`, README, LICENSE-MIT, LICENSE-APACHE, CHANGELOG |
| `rsuvm-linux-x86_64.tar.gz` | `rsuvm` (ELF Linux x86_64), README, licenças, CHANGELOG |
| `SHA256SUMS` | hashes dos dois arquivos |

O binário Linux foi extraído da imagem Docker construída a partir do mesmo
commit; o `tar.gz` foi criado dentro do container para preservar o bit de
execução. O binário Windows veio de `cargo build --release --locked -p rvm-cli`.

### Hashes

```text
6714b6402391468c8236b07502dcda75e031167651ec4dda8c262b1967d4ec1f  rsuvm-windows-x86_64.zip
7db4fa92470400f21b7314da1d6a40742216df2d83da1a4b83bfd9ef9ed4bd43  rsuvm-linux-x86_64.tar.gz
```

Verificação por `Get-FileHash -Algorithm SHA256`: **OK** para ambos.

## 2. Smoke tests

| Teste | Resultado |
|---|---|
| `rsuvm --version` (Windows, extraído do zip) | `rsuvm 0.1.0-alpha.1` |
| `rsuvm run examples/soma.rvm --arg i64:10 --arg i64:20` (Windows) | `i64:30` |
| `rsuvm --version` (Linux, extraído do tar em `debian:bookworm-slim`) | `rsuvm 0.1.0-alpha.1` |
| `rsuvm run examples/soma.rvm ...` (Linux) | `i64:30` |
| `cargo install --path crates/rvm-cli --locked --root <temp>` | instalado `rvm-cli v0.1.0-alpha.1` |
| binário instalado `rsuvm --version` | `rsuvm 0.1.0-alpha.1` |

## 3. Docker

```bash
docker build -t rsuvm:alpha .
docker run --rm rsuvm:alpha --version
docker run --rm -v "$PWD/examples:/work:ro" rsuvm:alpha \
  run /work/soma.rvm --arg i64:10 --arg i64:20
```

Resultados: build exit 0; `--version` → `rsuvm 0.1.0-alpha.1`; execução do exemplo
→ `i64:30`. A imagem roda como UID `10001`.

## 4. Dry-run do workflow de release (GitHub Actions)

- **Run:** `41543773653` (`workflow_dispatch`, commit `49c8cbe`)
- **Resultado:** `success`
- **Jobs:** `build (ubuntu-latest)` success; `build (windows-latest)` success;
  `publish release` **skipped** (por desenho: publicação só em push de tag).
- Nenhum release foi criado.

Artefatos baixados do run e verificados:

| Artefato | SHA-256 |
|---|---|
| `rsuvm-linux-x86_64.tar.gz` | `f52a96528750725e8c68f6cc33e3d053326df67f014b49f3309d74bb47bb297f` |
| `rsuvm-windows-x86_64.zip` | `f1d397d671c65b33affae7cefda11a2807602f6e846784f1fac68fd660f03444` |

- Checksums por arquivo (`.sha256`) publicados pelo workflow: **OK**.
- O binário Linux nativo do runner foi extraído e executado em
  `debian:bookworm-slim`: `rsuvm 0.1.0-alpha.1` e `i64:30` no exemplo.
- O binário Windows do artefato foi extraído e executado no host: versão e
  exemplo `i64:30`.
- O passo de `SHA256SUMS` agregado do job de publicação foi validado manualmente
  sobre os artefatos baixados (comando `sha256sum *.tar.gz *.zip > SHA256SUMS`);
  o resultado confere. O job de publicação em si só roda na tag.

## 5. Limitações deste dry-run

- Não é uma release: nenhuma tag foi criada e nada foi publicado.
- Os artefatos não são assinados; apenas checksums SHA-256.
- macOS não foi testado nem empacotado.
- O binário Linux foi validado em `debian:bookworm-slim`; outras distribuições
  não foram testadas.
- O binário Windows foi executado no host local, não em VM Windows limpa.
- O workflow de publicação (`publish`) não foi executado; só será validado no
  primeiro push de tag. O dry-run valida build, testes, empacotamento e artefatos.
- A campanha de fuzzing é curta (3 × 120 s) e está registrada em
  `docs/RELATORIOS/FUZZING-EXP-0.md` §7; não é auditoria.

## 6. Gates locais antes do empacotamento

- `cargo test --workspace --locked`: 176 passed, 0 failed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0.
- `cargo fmt --all -- --check`: exit 0.
- `cargo +1.75.0 check --workspace --all-targets --locked`: exit 0.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`: exit 0.
- `docker build -t rsuvm:alpha .`: exit 0; `docker run --rm rsuvm:alpha
  --version`: `rsuvm 0.1.0-alpha.1`.
