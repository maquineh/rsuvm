# RSUVM Blueprint — Rust Universal Virtual Machine

## System Overview

RSUVM é um **protótipo experimental de pesquisa** (EXP-0): uma máquina virtual em
Rust que executa uma representação intermediária comum (RIR) produzida por
frontends independentes. O núcleo não conhece Lift, Rio ou qualquer linguagem
hospedeira: aceita `unit`/`bool`/`i64`, funções puras, fluxo explícito por blocos
e chamadas estáticas, verifica o módulo inteiro **antes** de executar e roda sob
quotas de fuel e profundidade. Nenhum formato é estável; nada aqui é sandbox
validado nem autorização para produção.

**Base documental:** pacote RVM-Foundation-0.1, copiado sem modificação em
[docs/fundacao](./docs/fundacao/ORIGEM.md) (24 arquivos verificados por SHA-256).

Pipeline:

~~~text
texto .rvm ── rvm-text::parse ──┐
                               ├── Module ── structure::check ── verify
bytes .rsvmbc ─ rvm-bytecode ──┘                                   │
                                                                   ▼
                     rvm-interpreter::execute(VerifiedModule)  →  valor | trap | limite
~~~

## Início rápido

```bash
cargo build --workspace
cargo test --workspace
cargo run -p rvm-cli -- run examples/soma.rvm --arg i64:10 --arg i64:20   # i64:30
cargo run -p rvm-cli -- check examples/soma.rvm
```

## Subdirectories & System Boundaries

- [crates](./crates/README.md) — workspace de dez crates acíclicos, sem
  dependências externas: spec, ir, runtime, verify, text, bytecode, interpreter,
  link, cli e conformance. Os limites de dependência são impostos pelo compilador.
- [docs](./docs/ARQUITETURA.md) — arquitetura, contratos (`docs/CONTRATOS/`),
  ADRs deste repositório (`docs/ADRs/`), ADRs da fundação
  (`docs/fundacao/ADRs/`) e relatórios (`docs/RELATORIOS/`).
- [examples](./examples/soma.rvm) — módulos `.rvm` de demonstração em formato
  textual experimental; não são contrato de código.
- [scripts](./scripts/gerar-evidencias.ps1) — geração do log de evidências de
  execução; não é artefato de runtime.
- [fuzz](./fuzz/README.md) — workspace **separado** de fuzzing guiado por cobertura
  (nightly, dependência de dev; não faz parte do workspace principal; ADR-0020).
- [frontends](./frontends/README.md) — adaptadores de linguagem fora do núcleo;
  protótipo Lift→RIR (ADR-0021), também workspace separado.

## Workspace Invariants

- Zero dependências externas: apenas `std`; crate nova exige ADR e aprovação.
- `unsafe_code = "forbid"` e `missing_docs = "deny"` em todos os alvos
  (`Cargo.toml:[workspace.lints.rust]`).
- `overflow-checks = true` também no perfil release: a semântica de overflow vem de
  `checked_*`, mas aritmética acidental não pode virar wrap silencioso
  (`Cargo.toml:[profile.release]`).
- Verificação completa precede execução; o executor recebe somente
  `rvm_verify::VerifiedModule`.
- ADR-0004 permanece aberto: nenhum formato wire é estável; o bytecode é
  experimental (ADR-0011).

## Global Invariants

- **Independência de frontend** (ADR-0001): nenhum crate do núcleo importa ou cita
  Lift/Rio; adaptadores, quando existirem, ficarão fora de `crates/`.
- **Falha fechada**: entradas textuais, binárias, nomes, índices e contagens são
  não confiáveis; limites são checados antes de alocar, indexar, recursar ou
  despachar.
- **Sem efeitos implícitos** (ADR-0005): nenhuma capability é concedida; não há
  caminho de concessão no EXP-0.
- **Determinismo**: mesmo módulo + limites + configuração + argumentos ⇒ mesmo
  valor ou mesmo código de erro; sem `HashMap`, relógio, ambiente ou sistema de
  arquivos no caminho de execução.
- **Diagnóstico estável**: códigos e classes de `rvm-spec` são contrato; mensagens
  humanas não são (`docs/CONTRATOS/ERROS-EXP-0.md`).
- **Teto de profundidade** aplicado sobre pilha de frames na heap (ADR-0012): a
  versão recursiva estourou a pilha nativa com 256 frames.

## Global Side Effects

- O núcleo (`rvm-spec`, `rvm-ir`, `rvm-runtime`, `rvm-verify`, `rvm-text`,
  `rvm-bytecode`, `rvm-interpreter`) é puro: sem filesystem, rede, ambiente,
  relógio, subprocesso ou `unsafe`.
- `rvm-cli` lê o arquivo de entrada indicado, escreve apenas o arquivo de saída de
  `encode` e usa `stdout`/`stderr` do processo.
- A suíte de testes lê `crates/rvm-conformance/corpus` e cria arquivos temporários
  em testes do CLI; não escreve no repositório.
- `scripts/gerar-evidencias.ps1` escreve `docs/RELATORIOS/EVIDENCIAS-EXECUCAO.md`
  (log de execução), nunca código.

## System Verification

- Run: `cargo test --workspace`
- Gate adicional usado no relatório: `cargo clippy --workspace --all-targets -- -D
  warnings` e `cargo fmt --all -- --check`.
- CI: `.gitlab-ci.yml` (GitLab) e `.github/workflows/ci.yml` (GitHub Actions)
  rodam `fmt --check`, `clippy -D warnings`, `test` e `test --release`; Windows é
  opcional por regra. Workflows auxiliares **manuais**: `bench.yml` (benchmark
  pré-registrado) e `fuzz.yml` (fuzzing guiado por cobertura, nightly).
- Resultados atuais e log bruto: [RELATORIO-IMPLEMENTACAO](./docs/RELATORIOS/RELATORIO-IMPLEMENTACAO.md)
  e [EVIDENCIAS-EXECUCAO](./docs/RELATORIOS/EVIDENCIAS-EXECUCAO.md): 148 testes
  verdes, 52 casos válidos + 39 inválidos no corpus.

## Segurança (limites declarados)

- Rust seguro reduz classes de erro de memória, mas o protótipo **não** é
  isolamento de processo/OS; uso multi-tenant hostil exigiria isolamento externo
  revisado.
- "Zero panic no corpus" descreve a rodada de testes, não ausência de
  vulnerabilidade. Fuzzing de campanha, benchmark e auditoria independente **não**
  foram executados.
- Integração Lift/Rio **não** foi feita: bloqueios técnicos registrados em
  [BASELINE-RIO](./docs/RELATORIOS/BASELINE-RIO.md) e
  [BASELINE-LIFT](./docs/RELATORIOS/BASELINE-LIFT.md). As licenças de Rio e Lift
  foram declaradas `MIT OR Apache-2.0` pelo mantenedor.

## Licença

Dupla **`MIT OR Apache-2.0`** ([ADR-0014](./docs/ADRs/ADR-0014-LICENCA-DO-REPOSITORIO.md)):
o reuso pode escolher uma das duas. Textos integrais em
[LICENSE-MIT](./LICENSE-MIT) e [LICENSE-APACHE](./LICENSE-APACHE). A política de
reuso do Rio está em
[ADR-0015](./docs/ADRs/ADR-0015-POLITICA-DE-REUSO-DO-RIO.md). A licença autoriza
reuso de código; não é alegação de segurança nem substitui assessoria jurídica.
