# Compatibilidade e versionamento

## Versão do projeto

O workspace usa `0.1.0-alpha.1` (primeiro alpha público). Enquanto estiver em `0.x`:

- **minor** pode conter mudanças incompatíveis;
- **patch** deve conter correções compatíveis;
- mudanças de formato/API relevantes são registradas no `CHANGELOG.md` e, quando
  fecham decisão, em ADR.

Não há LTS. O suporte cobre a última versão publicada, na melhor capacidade.

## Formatos IR, texto e bytecode

- A RIR é experimental; `IR_VERSION = 1` identifica a forma atual, não uma
  promessa de estabilidade.
- O formato textual (`EXP-0-TEXT`) e o bytecode (`RSVMB1`) são **experimentais e
  não normativos**. Não há compatibilidade entre revisões.
- O CLI pode rejeitar bytecode de outra revisão com `unsupported_version`,
  `unknown_opcode` ou `invalid_module`.
- Para armazenar módulos, guarde o texto-fonte e a versão do binário usado; não
  trate o bytecode como formato de arquivamento estável.

## API Rust

- Os crates são `0.x`; a API pública pode mudar em minor.
- `rvm-spec` fixa códigos de erro estáveis; renomear/reclassificar exige
  atualização de `docs/CONTRATOS/ERROS-EXP-0.md` e ADR.
- `VerifiedModule` é a única porta para execução; não há promessa de estabilidade
  para detalhes internos.
- `Host`, `Capabilities` e `BuiltinHost` são ABI experimental; extensões exigem
  ADR e modelo de ameaça.

## CLI

- Nomes de subcomando, flags e códigos de saída são documentados em
  `docs/GUIA-USUARIO.md` e `docs/CONTRATOS/ERROS-EXP-0.md`.
- Mensagens de erro, formatação de `Display` e ordem de diagnóstico podem mudar.
- O formato canônico de `fmt` é reparseável pela mesma versão; não é garantido
  entre versões.

## Plataformas

| Plataforma | Estado |
|---|---|
| Linux x86_64 | CI e desenvolvimento |
| Windows x86_64 | CI e desenvolvimento |
| macOS | não testado; não distribuir sem validação |
| outras arquiteturas | não testadas |

MSRV: Rust `1.75` (verificada por `cargo +1.75.0 check --workspace --locked`).

## Canais de distribuição

- Código-fonte no GitHub (`main`).
- Binários em GitHub Releases, com `SHA256SUMS`.
- Crates.io: **não publicado** neste alpha (`publish = false`). Uma publicação
  futura exigirá estabilidade de API, metadados, docs.rs e política de semver.

## Mudanças de segurança

Correções de segurança podem ser lançadas sem aviso prévio detalhado, com nota
no `CHANGELOG.md` e, quando apropriado, GitHub Security Advisory. Ver
`SECURITY.md`.
