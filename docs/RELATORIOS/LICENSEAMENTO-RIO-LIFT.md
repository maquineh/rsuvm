# Licenciamento de Rio e Lift — decisão do mantenedor

- **Data:** 2026-11-02
- **Objetivo:** registrar que a definição de licença de Rio e Lift **não** é um
  pedido a terceiros, mas uma decisão do próprio mantenedor, e o que isso destrava.
- **Estado:** pendente de decisão do mantenedor. Nada foi copiado de Rio ou Lift.

## 1. Fato: os três projetos têm o mesmo autor

As auditorias registraram o mesmo autor nos três repositórios:

| Projeto | Repositório | Commit auditado | Autor | Licença |
|---|---|---|---|---|
| RSUVM | `https://github.com/maquineh/rsuvm` | — | Raphael Maquiné `<raphael.maquine@prodam.am.gov.br>` | `MIT OR Apache-2.0` (ADR-0014) |
| Rio | `https://git.prodam.am.gov.br/gsist/darqq/rio-lang` | `5566f4f2e3993b7ba6d5e54bf7d25bf2f2095d01` | `raphael.maquine@prodam.am.gov.br` | **desconhecida** (`RIO-BASELINE-001`) |
| Lift | `https://github.com/maquineh/lift` | `691c344880ac85056bc4bc85bf1e513654769bb1` | Raphael Maquiné `<raphael.maquine@prodam.am.gov.br>` | **desconhecida** (`LIFT-BASELINE-001`) |

Consequência: **não há mantenedor de terceiros a consultar**. A licença de Rio e
Lift é uma decisão do titular dos direitos — o próprio mantenedor do RSUVM. O
bloqueio registrado em ADR-0015/ADR-0017 é **formal** (falta declarar a licença),
não uma barreira de terceiros.

## 2. O que isso muda

- **Não** é preciso enviar pedido de licença a ninguém; a decisão é interna.
- O mantenedor pode, se quiser, **declarar** a licença de Rio e Lift (por exemplo,
  a mesma `MIT OR Apache-2.0`) e então destravar reuso/derivação conforme o caso.
- Enquanto não declarar, vale o estado atual: Lift **bloqueado** (ADR-0017); Rio
  permitido só como **referência comportamental** (ADR-0015).

## 3. Opções de decisão

| Opção | Efeito | Observação |
|---|---|---|
| Declarar `MIT OR Apache-2.0` em Rio e Lift | destrava reuso/derivação conforme ADR-0008/0007 | exige adicionar `LICENSE` e campo `license` nos repositórios de origem |
| Declarar licença só para referência comportamental | permite oráculo caixa-preta sem copiar código | mantém ADR-0015 |
| Manter sem licença | permanece o bloqueio formal | nenhuma mudança |
| Licença proprietária | impede reuso externo | coerente com uso interno |

## 4. Ação

- **Responsável:** mantenedor (titular dos direitos).
- **Se decidir declarar licença:** adicionar `LICENSE`/campo `license` em Rio e
  Lift; atualizar `RIO-BASELINE-001`/`LIFT-BASELINE-001`; reabrir/substituir
  ADR-0015 e ADR-0017.
- **Se decidir manter:** nenhuma ação; o estado atual permanece.
- Este documento substitui o antigo `PEDIDO-LICENCA-RIO-LIFT.md`, que pressupunha
  terceiros.
