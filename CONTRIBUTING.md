# Contribuindo com a RSUVM

Obrigado pelo interesse. A RSUVM é um protótipo de pesquisa: mudanças devem
preservar os invariantes de segurança, determinismo e ausência de dependências
do núcleo.

## Antes de abrir um PR

Leia, nesta ordem:

1. `AGENTS.md`;
2. `README.md` e `docs/ARQUITETURA.md`;
3. os contratos em `docs/CONTRATOS/`;
4. o ADR da área que você vai tocar;
5. `SECURITY.md` se o tema for segurança.

## Regras do núcleo

- `unsafe`, FFI, `transmute`, ponteiros crus e carregamento dinâmico são
  proibidos. Exceção exige ADR, modelo de ameaça, teste e aprovação.
- Apenas `std` nos crates do núcleo. Dependência nova exige justificativa,
  licença verificada, versão fixada e aprovação.
- Entrada é hostil: valide limites antes de alocar, indexar, recursar ou
  despachar.
- Biblioteca não pode `panic!`, `unwrap`, `expect` nem indexar direto; propague
  `RvmError`.
- Determinismo: sem `HashMap`, relógio, ambiente, ordem de arquivos ou
  `debug_assertions` no caminho de execução.
- Nenhum efeito implícito. `read_file` é a única função de host e exige
  `filesystem_read` com escopo.
- Não relaxe teste nem limite para mascarar falha; abra relatório de divergência.

## Build e testes

```bash
cargo build --workspace --locked
cargo test --workspace --locked
cargo test --release --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

MSRV declarada: Rust `1.75`. Verifique também:

```bash
cargo +1.75.0 check --workspace --all-targets --locked
```

O adaptador Lift tem workspace separado:

```bash
cargo test --manifest-path frontends/lift-rir/Cargo.toml
```

## Processo de decisão

Decisões que fecham questão aberta exigem ADR em `docs/ADRs/`, com status,
alternativas, consequências e critério de validação. Contratos em
`docs/CONTRATOS/` descrevem o que o código **deve** fazer; se código e contrato
divergirem, abra relatório em `docs/RELATORIOS/`, não “conserte” o teste.

## PRs

- Um tema por PR, com testes e documentação afetada.
- Descreva o requisito/ADR atendido, a decisão, os comandos executados e as
  lacunas restantes.
- Não inclua artefatos de build, binários, `target/` ou segredos.
- Para mudanças de segurança, siga `SECURITY.md` em vez de abrir issue pública.

## Licença

Ao contribuir, você concorda que a contribuição é licenciada sob `MIT OR
Apache-2.0`, conforme `LICENSE-MIT` e `LICENSE-APACHE`.
