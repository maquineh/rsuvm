# RSUVM 0.1.0-alpha.1 — uma VM experimental em Rust

[English](./ANNOUNCEMENT-0.1.0-alpha.1.md) | **Português (BR)**

Estou publicando o primeiro alpha da **RSUVM**, uma máquina virtual de pesquisa
escrita em Rust. Ela executa uma representação intermediária (RIR) verificada,
com intérprete determinístico, quotas de execução e uma ABI mínima de host.

## O que já funciona

- Verificador de RIR (estrutura, CFG, dominância, tipos e assinaturas).
- Intérprete com fuel, profundidade, memória modelada e traps estáveis.
- Tipos `unit`, `bool`, `i64`, `string`, `f64`, `array`, `struct`, `option` e
  `result`.
- Formato textual e bytecode experimentais, com round-trip.
- Ligação estática inter-módulo (`rvm-link`).
- CLI `rsuvm`: `run`, `run-bc`, `check`, `fmt`, `encode`.
- ABI `read_file`/`filesystem_read` com capability, escopo canonicalizado e teto
  de 1 MiB por leitura, com reserva de quota antes do IO.
- 176 testes em Linux/Windows (debug/release), MSRV 1.75, CI e benchmark
  pré-registrado.

## O que ela **não** é

- **Não é sandbox.** É single-tenant; não isola processo/OS e não suporta
  multi-tenant.
- Não tem auditoria independente nem campanha de fuzzing de longa duração.
- TOCTOU e hard links continuam residuais no acesso a arquivos.
- Não há quota agregada de IO/tempo/taxa.
- Formatos IR/texto/bytecode e API são experimentais, sem compatibilidade entre
  versões.
- Não há frontend real integrado; o uso atual é via `.rvm` ou pela biblioteca.

## Como experimentar

```bash
cargo install --path crates/rvm-cli --locked
rsuvm run examples/soma.rvm --arg i64:10 --arg i64:20   # i64:30
rsuvm check examples/soma.rvm
```

Ou baixe os artefatos do release e confira o `SHA256SUMS`.

## Segurança

Leia `SECURITY.pt-BR.md` e `docs/GUIA-USUARIO.md`. Para código não confiável, use
isolamento externo de processo/OS; o runtime não substitui containers, VMs,
seccomp ou cgroups.

Vulnerabilidades: GitHub Security Advisories, não issue pública.

## Feedback

O que mais ajudaria neste alpha:

- rodar seus próprios módulos e relatar fricção;
- revisar o verificador/decoder/intérprete;
- apontar lacunas de documentação;
- discutir o caso de uso e um frontend viável.

Obrigado por testar. Este é um artefato de pesquisa, não um produto pronto.
