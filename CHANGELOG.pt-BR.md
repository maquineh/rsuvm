# Changelog

[English](./CHANGELOG.md) | **Português (BR)**

Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/).
O projeto usa versionamento semântico `0.x`: mudanças incompatíveis podem ocorrer
em minor releases. Formatos IR/texto/bytecode são **experimentais** e não têm
compatibilidade entre versões.

## [0.1.0-alpha.1] - 2026-11-02

### Adicionado

- Workspace de crates `rvm-spec`, `rvm-ir`, `rvm-runtime`, `rvm-verify`,
  `rvm-text`, `rvm-bytecode`, `rvm-interpreter`, `rvm-link`, `rvm-cli` e
  `rvm-conformance`, sem dependências externas no núcleo.
- Verificador de RIR com checagem estrutural, CFG, dominância, tipos e
  assinaturas.
- Intérprete determinístico com fuel, profundidade, memória modelada e traps.
- Tipos `unit`, `bool`, `i64`, `string`, `f64`, `array`, `struct`, `option` e
  `result`, com tabela de tipos por id.
- Formato textual experimental e bytecode linear experimental.
- Ligação estática inter-módulo (`rvm-link`).
- ABI mínima de host `read_file`/`filesystem_read`, com `Capabilities`, `Host`,
  `NoHost` e `execute_with_host`.
- CLI `rsuvm` com `run`, `run-bc`, `check`, `fmt`, `encode` e `--allow-fs-read`.
- Corpus de conformidade com 62 casos válidos e 42 inválidos.
- 176 testes no workspace, CI Linux/Windows, MSRV e benchmark pré-registrado.

### Segurança

- Default deny: `execute` sem host nega qualquer efeito.
- Escopo de arquivo absoluto, sem `..`, com canonicalização de raiz e alvo,
  recusa de symlink/junção para fora e apenas arquivo regular.
- Teto de 1 MiB por leitura e reserva de quota antes do IO.
- `unsafe` proibido no workspace; nenhuma dependência externa no núcleo.
- Campanha curta de fuzzing guiada por cobertura (run `41543777109`, 3 × 120 s,
  **24 751 525 execuções, 0 crashes observados**); não é campanha de longa
  duração nem auditoria.

### Limitações conhecidas

- TOCTOU entre canonicalização e abertura; hard links não são detectados.
- Sem quota agregada de IO/tempo/taxa; teto de 1 MiB é por chamada.
- Sem isolamento de processo/OS e sem suporte multi-tenant.
- Sem auditoria independente e sem campanha de fuzzing de longa duração.
- Formatos e API experimentais, sem compatibilidade entre versões.

[0.1.0-alpha.1]: https://github.com/maquineh/rsuvm/releases/tag/v0.1.0-alpha.1
