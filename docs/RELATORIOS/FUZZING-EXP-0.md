# Campanha de fuzzing — EXP-0

- **Data:** 2026-11-02
- **Base do repositório:** commit `3688795` (etapa A2) + árvore de trabalho da
  etapa A3
- **Ambiente:** Windows 10.0.26300, rustc 1.94.1, cargo 1.94.1, Intel Core
  i7-9700 @ 3.00GHz
- **Ferramenta:** harness **em processo, determinístico e orçado** (sem
  dependências externas). Não é fuzzer guiado por cobertura.
- **Reprodução:**
  `cargo test -p rvm-conformance --test fuzzing -- --nocapture`

> "Nenhum pânico no orçamento declarado" descreve **apenas esta rodada**
> (`docs/fundacao/TESTES/CONFORMIDADE.md` §6). Não prova ausência de
> vulnerabilidade.

## 1. Por que não um fuzzer externo

A política do repositório é **zero dependências externas** (`AGENTS.md`, regra 2).
Fuzzers guiados por cobertura usuais (`cargo-fuzz`/libFuzzer, AFL) exigem
dependências e, em geral, toolchain nightly. A campanha usa, então, mutação
determinística com sementes fixas — reprodutível, sem relógio e sem rede. O custo
aceito é não haver feedback de cobertura: a campanha **não** explora caminhos
novos por si só.

## 2. Alvos, sementes e orçamento

| Alvo | Base | Mutações | Sementes | Total de operações |
|---|---|---:|---:|---:|
| Parser textual (`rvm_text::parse`) | 19 casos válidos do corpus | 100 | 4 | 7 600 |
| Decoder binário (`rvm_bytecode::decode`) | 28 módulos (`encode`) | 100 | 4 | 11 200 |
| Verificador (`structure::check` + `verify` + `execute` limitado) | 28 módulos | 100 | 4 | 11 200 |

- Bases do decoder/verificador: 19 casos válidos do corpus + 9 módulos de
  `rvm_ir::testing`.
- Sementes (xorshift64, fixas):
  `0x2545F4914F6CDD1D`, `0x9E3779B97F4A7C15`, `0xD1B54A32D192ED03`,
  `0x0123456789ABCDEF`.
- Mutações: textual troca 1–4 bytes por ASCII imprimível (descarta não-UTF-8);
  bytecode faz XOR de 1–3 bytes; verificador aplica mutação estrutural (versão,
  entry, tipos, IDs de bloco, terminadores, operações, remoção/inserção de
  instrução).
- Módulo mutado que ainda verifica é executado com orçamento pequeno
  (`fuel = 500`, profundidade `32`); o contrato observado é "não entra em pânico",
  não sucesso.

## 3. Resultado (Resultado)

| Alvo | Operações | Pânico observado | Observação |
|---|---:|---|---|
| Texto | 7 600 | não | — |
| Bytecode | 11 200 | não | — |
| Verificador | 11 200 | não | 2 842 mutações ainda verificaram e foram executadas |

- Nenhum `panic`/abort observado nos três alvos.
- Nenhuma minimização foi necessária: não houve falha a reduzir.
- Tempo de execução da campanha no ambiente acima: ~0,14 s (sem relógio no
  caminho de execução; o tempo é informativo, não orçamento).

## 4. Limitações declaradas

1. Não é guiado por cobertura: não mede caminhos exercitados e não substitui
   libFuzzer/AFL.
2. Sementes e orçamento fixos: reprodutível, porém finito e enviesado.
3. Só a superfície de entrada (texto/bytes) e o verificador estrutural/tipado
   foram mutados; não há mutação dirigida a invariantes específicos.
4. "Zero pânico" é uma observação da rodada, não uma propriedade provada.
5. Não houve campanha por tempo, corpus externo nem minimização automática.

## 5. Próximos passos sugeridos

- Repetir a campanha por tempo (ex.: 30 min) em runner dedicado, se um fuzzer
  guiado por cobertura for aceito como dependência de desenvolvimento com ADR.
- Adicionar mutação de aridade/assinatura e de fronteiras de limite estrutural
  como alvos dedicados.
- Registrar o tempo de parede da campanha quando houver runner com orçamento
  fixo (hoje o critério é número de operações, não tempo).
