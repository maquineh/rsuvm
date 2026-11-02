# ADR-0009 — Estrutura do repositório em workspace de crates

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-01
- **Decisão do mantenedor nesta sessão:** workspace de crates (opção escolhida
  entre as alternativas abaixo).
- **Requisitos:** RF-06 (frontends independentes), ADR-0001, TechSpec §3.

## Contexto

`docs/fundacao/02-TECHSPEC.md` §3 lista componentes de referência e afirma
explicitamente: "A estrutura do repositório pode ser diferente se preservar os
limites de dependência. A divisão em crates não está aprovada como necessária."
Era, portanto, uma questão aberta (`docs/fundacao/ADR` não a cobria). O RF-06
exige que o runtime não dependa de crates de Lift ou Rio, e o modelo de ameaça
exige que o executor só receba módulos verificados.

## Decisão

Adotar um workspace Cargo com nove crates e dependências acíclicas:

~~~text
rvm-spec ── rvm-ir ── rvm-verify ── rvm-interpreter ── rvm-cli
      │        │           │              │
      │        ├── rvm-text ──────────────┤
      │        └── rvm-bytecode ──────────┤
      └── rvm-runtime ────────────────────┘
                     rvm-conformance (dev-dependencies)
~~~

- Zero dependências externas: apenas `std`. Nenhum crate novo sem ADR.
- `rvm-verify` é o único produtor de `VerifiedModule` (campo privado, sem
  `From<Module>`).
- Adaptadores de frontend (`frontends/lift`, `frontends/rio`) não existem neste
  repositório e, quando existirem, ficarão **fora** do núcleo.

## Alternativas

1. **Crate único `rsuvm` com módulos.** Menos boilerplate, revisão mais simples;
   os limites de dependência passariam a ser convenção sem bloqueio do compilador,
   e o RF-06 só seria verificável por revisão.
2. **Dois crates (núcleo + CLI).** Meio-termo; falha o objetivo de isolar o
   verificador do intérprete e o texto do binário, e não impede que o núcleo
   importe um adaptador de linguagem.
3. **Não implementar; manter apenas documentação.** Compatível com a fundação,
   mas não entrega o protótipo experimental que o PRD §8 exige.

## Consequências

- Positivas: o compilador impõe a separação; trabalho paralelo em crates
  disjuntas; cada contrato EGM corresponde a um diretório com código real;
  testabilidade isolada (`cargo test -p rvm-verify`).
- Negativas: nove `Cargo.toml` e mais cerimônia de manutenção; compilação
  incremental de mais unidades (custo baixo, sem dependências externas);
  duplicação de `cfg_attr` de lints por crate.
- Risco: a divisão pode se revelar fina demais (ex.: `rvm-runtime` muito pequeno).
  Reabrir este ADR se dois crates passarem a ter acoplamento constante ou se um
  crate tiver menos de 100 linhas de responsabilidade própria.

## Validação

- `cargo build --workspace` e `cargo test --workspace` verdes (registrados no
  relatório de implementação).
- O grafo de dependências é o da tabela de `docs/ARQUITETURA.md`; qualquer seta
  nova exige atualização deste ADR.
- A separação só é considerada "validada no escopo" quando existir ao menos um
  frontend externo consumindo `rvm-text`/`rvm-ir` sem alterar o núcleo — hoje
  isso é desconhecido.
