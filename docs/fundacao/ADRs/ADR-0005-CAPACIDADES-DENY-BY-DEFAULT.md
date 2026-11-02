# ADR-0005 — Capabilities do host deny-by-default

- **Status:** Proposto
- **Data:** 2026-11-01
- **Contexto:** Interoperabilidade que acessa ambiente implicitamente amplia o escopo de segurança e torna o programa dependente do host.

## Decisão proposta

Nenhum acesso a filesystem, rede, ambiente, relógio ou subprocesso é implícito. MVP não disponibiliza essas capabilities. APIs futuras exigem concessão explícita do host, escopo/limites, erros documentados e testes de negação.

## Alternativas

- Biblioteca padrão com acesso geral ao SO.
- Allowlist global de funções por nome.
- Capabilities explícitas por execução.

## Consequências

- Host controla autoridade e os testes podem demonstrar negação.
- APIs são mais verbosas; valores do host talvez precisem de handles.
- Capability não protege contra bug em sua implementação no host.

## Validação

Testes sem capability para cada efeito e auditoria de adaptadores. Isolamento in-process continua hipótese e não autoriza uso multi-tenant.
