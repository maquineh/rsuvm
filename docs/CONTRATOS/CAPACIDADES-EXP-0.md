# Capabilities — EXP-0 (deny-by-default)

**Estado:** proposta implementada de forma restritiva: o MVP **não tem caminho de
concessão**. ADR-0005 é a decisão de origem; este contrato descreve o que o código
atual garante e o que ele não garante.

## 1. Regra

Nenhuma capability é implícita. Arquivos, rede, ambiente, relógio e subprocessos
não são oferecidos ao convidado. No EXP-0 não existe instrução de chamada ao host:
o programa só executa operações puras do subset.

## 2. API

- `rvm_runtime::Effect` enumera `filesystem_read`, `filesystem_write`, `network`,
  `environment`, `clock` e `process`.
- `rvm_runtime::Capabilities` tem campo privado e **um único** construtor,
  `none()`. Não existe `grant`, `with`, `insert`, `Default` com conteúdo nem
  desserialização.
- `rvm_runtime::HostApi::request(effect, scope)` devolve sempre
  `Err(capability_denied)` no MVP. O ramo `Ok` só existiria se o conjunto
  contivesse o efeito, o que é inalcançável.
- Não há função no crate que abra arquivo, socket, variável de ambiente, relógio
  ou processo. A negação é estrutural, não um `if` que pode ser desligado por
  flag.

## 3. Obrigações de teste

1. Um teste por efeito (`Effect`), verificando `capability_denied` e
   `ErrorClass::Capability` (já em `crates/rvm-runtime/src/lib.rs`).
2. `Capabilities::none().granted()` vazio e `is_granted` falso para todos os
   efeitos.
3. Nenhum teste pode depender de sistema de arquivos, rede, ambiente, relógio ou
   subprocesso.
4. O CLI não aceita flags de concessão de capability; adicionar uma exige ADR,
   revisão de modelo de ameaça e testes de negação adversariais.

## 4. O que esta regra **não** prova

- Isolamento de processo/OS: quotas e verificação são in-process.
- Ausência de DoS: fuel e profundidade limitam trabalho, mas não tempo de parede
  nem memória do host de forma exaustiva.
- Segurança de uma capability futura: cada adaptador de efeito precisa de escopo,
  limite, erro documentado, teste de negação e revisão independente.
- Que um bug no verificador não permita efeitos: na ausência de instruções de
  host, não há efeito a alcançar; qualquer extensão futura reabre a análise.

## 5. Extensão futura (fora do EXP-0)

Conceder uma capability exigirá: ADR com justificativa de ameaça, tipo de handle
sem ponteiro nativo, escopo e quotas por efeito, erro específico, testes de
negação e de abuso, além de decisão explícita do mantenedor. Nada disso existe
hoje e nada deve ser inferido a partir deste documento.
