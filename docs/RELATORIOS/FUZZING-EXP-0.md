# Campanha de fuzzing — EXP-0

- **Data:** 2026-11-02
- **Base do repositório:** commit `13a1b0a` (etapa B1) + árvore de trabalho da
  etapa B2
- **Ambiente:** Windows 10.0.26300, rustc 1.94.1, cargo 1.94.1, Intel Core
  i7-9700 @ 3.00GHz
- **Ferramenta:** harness **em processo, determinístico e orçado** (sem
  dependências externas). Não é fuzzer guiado por cobertura (ADR-0019).
- **Reprodução:**
  `cargo test -p rvm-conformance --test fuzzing -- --nocapture`

> "Nenhum pânico no orçamento declarado" descreve **apenas esta rodada**
> (`docs/fundacao/TESTES/CONFORMIDADE.md` §6). Não prova ausência de
> vulnerabilidade.

## 1. Por que não um fuzzer externo

A política do repositório é **zero dependências externas** (`AGENTS.md`, regra 2),
inclusive de desenvolvimento, e o CI roda em toolchain estável. Fuzzers guiados
por cobertura (`cargo-fuzz`/libFuzzer, AFL) exigem dependência externa e, em
geral, nightly. A decisão está no **ADR-0019**: a campanha permanece determinística
e a adoção de um fuzzer guiado por cobertura exige ADR próprio e aprovação do
mantenedor. O custo aceito é não haver feedback de cobertura.

## 2. Alvos, sementes e orçamento

| Alvo | Base | Mutações | Sementes | Total de operações |
|---|---|---:|---:|---:|
| Parser textual (`rvm_text::parse`) | 19 casos válidos do corpus | 200 | 6 | 22 800 |
| Decoder binário (`rvm_bytecode::decode`) | 28 módulos (`encode`) | 200 | 6 | 33 600 |
| Verificador (`structure::check` + `verify` + `execute` limitado) | 28 módulos | 200 | 6 | 33 600 |
| **Linker** (`rvm_link::link` + `verify` + `execute`) | 29 módulos | 200 | 6 | 34 800 |

- Bases do decoder/verificador: 19 casos válidos do corpus + 9 módulos de
  `rvm_ir::testing`. O linker acrescenta um módulo com `import` (externa) e pareia
  cada base com um provedor mutado.
- Sementes (xorshift64, fixas):
  `0x2545F4914F6CDD1D`, `0x9E3779B97F4A7C15`, `0xD1B54A32D192ED03`,
  `0x0123456789ABCDEF`, `0xA0761D6478BD642F`, `0xE7037ED1A0B428DB`.
- Mutações: textual troca 1–4 bytes por ASCII imprimível (descarta não-UTF-8);
  bytecode faz XOR de 1–3 bytes; verificador aplica mutação estrutural (versão,
  entry, tipos, IDs de bloco, terminadores, operações, remoção/inserção de
  instrução); linker muta os dois módulos do par.
- Módulo mutado que ainda verifica (ou ligação que resolve) é executado com
  orçamento pequeno (`fuel = 500`/`300`, profundidade `32`/`16`); o contrato
  observado é "não entra em pânico", não sucesso.

## 3. Resultado (Resultado)

| Alvo | Operações | Pânico observado | Observação |
|---|---:|---|---|
| Texto | 22 800 | não | — |
| Bytecode | 33 600 | não | — |
| Verificador | 33 600 | não | 8 599 mutações ainda verificaram e foram executadas |
| Linker | 34 800 | não | 5 160 ligações resolveram e foram executadas |

- Nenhum `panic`/abort observado nos quatro alvos.
- Nenhuma minimização foi necessária: não houve falha a reduzir.
- Tempo de execução da campanha no ambiente acima: ~0,8 s (informativo, não
  orçamento).

## 4. Limitações declaradas

1. Não é guiado por cobertura: não mede caminhos exercitados e não substitui
   libFuzzer/AFL (ADR-0019).
2. Sementes e orçamento fixos: reprodutível, porém finito e enviesado.
3. As mutações são genéricas; não há mutação dirigida a invariantes específicos
   além das estruturas cobertas.
4. "Zero pânico" é uma observação da rodada, não uma propriedade provada.
5. Não houve campanha por tempo, corpus externo nem minimização automática.

## 5. Próximos passos sugeridos

- Propor ADR para fuzzer guiado por cobertura (dependência de desenvolvimento,
  licença e versão fixadas) e rodar por tempo em runner dedicado.
- Adicionar mutação de aridade/assinatura e de fronteiras de limite estrutural
  como alvos dedicados.
- Registrar o tempo de parede quando houver runner com orçamento fixo (hoje o
  critério é número de operações).
