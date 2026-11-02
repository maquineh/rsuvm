# Pedido de licença — Rio e Lift

- **Data:** 2026-11-02
- **Objetivo:** destravar qualquer reuso de Rio e Lift obtendo **declaração de
  licença** dos mantenedores. É uma **ação humana** (decisão de terceiros); este
  documento prepara o pedido e registra o que está bloqueado.
- **Estado:** pendente de resposta. Nada foi copiado de Rio ou Lift.

## 1. Por que a licença importa

Sem `LICENSE` e sem campo `license` no manifesto, o padrão é "todos os direitos
reservados": não é lícito copiar, derivar nem redistribuir, mesmo com o código
público. As duas auditorias registraram a ausência:

| Projeto | Repositório | Commit auditado | Licença |
|---|---|---|---|
| Rio | `https://git.prodam.am.gov.br/gsist/darqq/rio-lang` | `5566f4f2e3993b7ba6d5e54bf7d25bf2f2095d01` | **desconhecida** (`RIO-BASELINE-001`) |
| Lift | `https://github.com/maquineh/lift` | `691c344880ac85056bc4bc85bf1e513654769bb1` | **desconhecida** (`LIFT-BASELINE-001`) |

Observação factual: os autores dos dois repositórios são
`raphael.maquine@prodam.am.gov.br` (Rio) e `Raphael Maquiné
<raphael.maquine@prodam.am.gov.br>` (Lift) — os mesmos do RSUVM. Ainda assim, a
decisão de licença deve ser registrada formalmente.

## 2. O que está bloqueado hoje

- **Rio** (ADR-0015): permitido apenas como **referência comportamental** — observar
  documentação e comportamento e executá-lo como oráculo caixa-preta para testes
  diferenciais; **proibido** copiar código/AST/derivados e linkar.
- **Lift** (ADR-0017): integração **bloqueada** — sem licença, modelo de execução
  por compilação nativa + Cargo, dependências externas e subset incompatível. A
  HIR é **pública** (`HirProgram`), o que abriria a rota "consumir como oráculo",
  condicionada à licença.

## 3. Perguntas para os mantenedores

1. Qual licença SPDX se aplica a cada repositório? Podem adicionar um arquivo
   `LICENSE` e o campo `license` no `Cargo.toml`?
2. Autorizam o uso como **referência comportamental** (executar como oráculo
   caixa-preta, sem copiar código) para testes diferenciais?
3. (Lift) Autorizam **consumir a API pública** (`HirProgram`, `emit_rust`) de um
   crate externo, com atribuição, sem alterar o upstream?
4. Autorizam **derivar/adaptar** código sob alguma licença? Se sim, qual e com que
   condições (atribuição, patentes, uso interno)?
5. Há restrição institucional (Prodam) que impeça publicação, derivação ou uso
   fora do órgão?

## 4. Usos contemplados (para decisão informada)

| Uso | Rio | Lift | Precisa de licença? |
|---|---|---|---|
| Observar documentação/comportamento | sim | sim | não (leitura) |
| Executar como oráculo caixa-preta em teste diferencial | sim | sim | depende (execução local) |
| Consumir API pública (HIR) em crate externo | não (AST privada) | possível | **sim** |
| Copiar/adaptar código no núcleo RSUVM | **proibido** | **proibido** | **sim** |
| Linkar como dependência do núcleo | **proibido** | **proibido** | **sim** |

## 5. Ação

- **Responsável:** mantenedor do RSUVM.
- **Passos:** enviar as perguntas da §3 aos mantenedores de Rio e Lift; ao obter
  resposta, registrar a licença nos baselines (`RIO-BASELINE-001`,
  `LIFT-BASELINE-001`) e reabrir/substituir ADR-0015 e ADR-0017 conforme o caso.
- **Enquanto não houver resposta:** permanece o estado atual (nenhum reuso de
  código; Lift bloqueado; Rio só como referência comportamental).
