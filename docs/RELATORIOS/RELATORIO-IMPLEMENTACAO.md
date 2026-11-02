# Relatório de implementação — RSUVM EXP-0

- **Data:** 2026-11-01
- **Checkout:** `H:\projetos\rsuvm` (sem repositório git inicializado; ver lacuna 5)
- **Ambiente:** Windows, rustc 1.94.1, cargo 1.94.1, rustfmt 1.8.0, clippy 0.1.94
- **Base documental:** RVM-Foundation-0.1, copiada intacta em `docs/fundacao/`
  (24 arquivos, SHA-256 conferidos — `docs/fundacao/ORIGEM.md`)
- **Escopo desta rodada:** núcleo da máquina (RIR, texto, verificador, intérprete,
  runtime, bytecode, CLI) + corpus de conformidade + decisões documentadas; sem
  adaptadores de frontend.

> Convenção: este relatório separa **Resultado** (executado, com comando e saída em
> `docs/RELATORIOS/EVIDENCIAS-EXECUCAO.md`) de **Proposta** e **Desconhecido**.
> Nada aqui foi auditado por terceiros.

## 1. O que foi entregue

| Crate | Responsabilidade | Arquivos | Testes |
|---|---|---|---|
| `rvm-spec` | tipos, operações verificadas, códigos/classes de erro, limites | `src/lib.rs` | 6 |
| `rvm-ir` | modelo RIR, invariantes de IDs/nomes, `structure::check`, fixtures | `src/lib.rs`, `structure.rs`, `testing.rs` | 18 |
| `rvm-runtime` | `Value`, `ExecConfig`, `CallDepthLimit`, `MemoryLimit` e capabilities deny-by-default | `src/lib.rs` | 6 |
| `rvm-verify` | CFG, alcançabilidade, dominância, tipos, `VerifiedModule` | `src/lib.rs` | 17 |
| `rvm-text` | parser e impressão canônica do formato textual EXP-0 | `src/lib.rs` | 10 |
| `rvm-bytecode` | codificação experimental + decoder limitado | `src/lib.rs`, `tests/hostile.rs` | 4 + 14 |
| `rvm-interpreter` | execução determinística, fuel, profundidade, memória e traps | `src/lib.rs`, `tests/execucao.rs`, `tests/memoria.rs` | 18 |
| `rvm-cli` | binário `rsuvm` (run, run-bc, check, fmt, encode) | `src/main.rs`, `tests/cli.rs`, `tests/memoria.rs` | 16 |
| `rvm-conformance` | corpus em dados + conformidade + propriedades | `src/lib.rs`, `tests/*.rs`, `corpus/` | 3 + 8 |

Total: **120 testes verdes** (soma dos alvos; doc-tests inclusos, 0 neles),
**8.112 linhas** de Rust em **18 arquivos**, **37 arquivos `.rvm`** (36 no corpus +
1 exemplo) e **37 documentos Markdown** fora de `docs/fundacao/`, dos quais **20
são contratos EGM** (raiz, blueprint de `crates/` e um por crate/src);
`corpus/README.md` documenta o formato do corpus.

Documentos novos: `docs/ARQUITETURA.md`; contratos `RIR-EXP-0-IMPLEMENTACAO`,
`TEXTO-EXP-0`, `ERROS-EXP-0`, `BYTECODE-EXP-0-IMPLEMENTACAO`,
`CAPACIDADES-EXP-0`; ADR-0009 (workspace de crates), ADR-0010 (formato textual),
ADR-0011 (bytecode protótipo), ADR-0012 (profundidade/frames explícitos) e
ADR-0013 (quota de memória modelada);
`docs/RELATORIOS/BASELINE-RIO.md` (RIO-BASELINE-001), `EVIDENCIAS-EXECUCAO.md`,
este relatório; `.gitlab-ci.yml` e `.github/workflows/ci.yml` (gates de CI); `AGENTS.md` do repositório e
`scripts/gerar-evidencias.ps1`.

## 2. Requisitos atendidos (PRD §6)

| ID | Situação | Evidência |
|---|---|---|
| RF-01 representar funções/blocos/fluxo | atendido | `rvm-ir` (`Module`/`Function`/`Block`/`Terminator`), corpus C-001..C-016 |
| RF-02 verificar antes de executar | atendido no corpus | `VerifiedModule` só por `verify`; testes de conformidade exigem `ErrorClass::Module` antes da execução |
| RF-03 operações puras do subset | atendido no subset | C-001..C-016; traps N-009/N-019/N-020 |
| RF-04 fuel e profundidade | atendido no escopo | testes de fronteira exata (fuel 5/6; profundidade 8/255/256) |
| RF-05 erros distinguem módulo/trap/limite | atendido | `ErrorClass` + códigos de saída 2/3/4 no CLI |
| RF-06 frontends independentes | atendido estruturalmente | zero dependências externas; nenhum crate cita Lift/Rio; ADR-0009 |
| RF-07 testes diferenciais registrados | **parcial** | diferencial RIR↔bytecode e corpus com entrada/expectativa; **não** há frontend real nem versão de frontend a registrar |
| RF-08 sem efeitos implícitos | atendido no escopo | `HostApi::request` nega 6 efeitos; `host_read_file` exige capability explícita e é negado sem host (ADR-0036) |
| RF-09 decisões em ADR | atendido | ADR-0009..0012 + ADRs 0001..0008 da fundação preservados |

## 3. Verificação executada (Resultado)

Log bruto: `docs/RELATORIOS/EVIDENCIAS-EXECUCAO.md` (gerado por
`scripts/gerar-evidencias.ps1`).

| Comando | Resultado |
|---|---|
| `cargo build --workspace --all-targets` | `Finished dev profile`, exit 0 |
| `cargo test --workspace` | **120 passed, 0 failed** (14 alvos) |
| `cargo test --release --workspace` | **120 passed, 0 failed** (semântica independente do perfil) |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished`, **0 warnings** |
| `cargo fmt --all -- --check` | sem saída (formatado) |
| `rsuvm run examples/soma.rvm --arg i64:10 --arg i64:20` | `i64:30`, exit 0 |
| `rsuvm run` N-001 (tipo inválido) | `invalid_type … [função main, bloco entry, instrução 1]`, exit 2 |
| `rsuvm run` N-009 (overflow) | `integer_overflow …`, exit 3 |
| `rsuvm run` N-010 `--fuel 1000` | `out_of_fuel … saldo inicial 1000`, exit 4 |
| `rsuvm run` N-011 `--max-depth 8` | `call_depth_exceeded: profundidade 9 excede o limite de 8`, exit 4 |
| `rsuvm encode` + `run-bc` | 93 bytes; `i64:42`, exit 0 |

### 3.1 Verificação dos contratos EGM (Step 5.4)

Script de verificação sobre os 21 READMEs de contrato (raiz + `crates/`):

- **links relativos**: todos resolvem para arquivos existentes; nenhum aponta para
  diretório sem `README.md`;
- **seções obrigatórias**: presentes por tipo de contrato (raiz: System Overview,
  Subdirectories & System Boundaries, Global Invariants, Global Side Effects,
  System Verification; peanut: Overview, Interface, Invariants, Side Effects,
  Verification; blueprint intermediário: Overview, Subdirectories & Boundaries);
- **code fences**: balanceadas em todos os arquivos;
- **comando de verificação**: `cargo test --workspace` presente em todos os
  contratos de pacote e na raiz.

Falso positivo registrado: o script cobrava `## Verification` também dos
blueprints intermediários, que pelo template do EGM têm apenas Overview e
Subdirectories. A verificação foi conferida manualmente: os contratos de pacote
(`crates/*/src/README.md`) têm a seção.

Um dos 21 contratos (`crates/rvm-verify/src/README.md`) foi escrito por um worker
EGM antes de ele ser interrompido; foi lido e suas citações de linha conferidas
contra o código (`VerifiedModule` em 64, `structure::check` em 82, `Def` em 116,
`verify_function` em 408, `compute_dominators` em 525, `invalid` em 792). Os demais
foram escritos pelo orquestrador.

## 4. Corpus de conformidade

`crates/rvm-conformance/corpus/`: 16 válidos (C-001..C-016) e 20 inválidos
(N-001..N-011, N-013..N-021). Mapeamento verificado por teste:

| Caso | Código/classe | Estágio |
|---|---|---|
| N-001 tipo de argumento | `invalid_type` / Module | verificação |
| N-002 aridade | `invalid_type` / Module | verificação |
| N-003 valor não definido | `invalid_module` / Module | resolução de nomes |
| N-004 definição duplicada | `invalid_module` / Module | resolução de nomes |
| N-005 terminador duplicado | `invalid_control_flow` / Module | parser |
| N-006 alvo de desvio | `invalid_control_flow` / Module | resolução de nomes |
| N-007 retorno incompatível | `invalid_type` / Module | verificação |
| N-008 opcode desconhecido | `unknown_opcode` / Module | parser |
| N-009 overflow | `integer_overflow` / Trap | execução |
| N-010 loop com fuel finito | `out_of_fuel` / ResourceLimit | execução |
| N-011 recursão profunda | `call_depth_exceeded` / ResourceLimit | execução |
| N-012 truncamento/**length** | `truncated_input`/`input_too_large` | decoder binário (`tests/hostile.rs`) |
| N-013 bloco sem terminador | `invalid_control_flow` | parser |
| N-014 versão inexistente | `unsupported_version` | parser |
| N-015 argumento de bloco | `invalid_type` | verificação |
| N-016 dominância | `invalid_control_flow` | verificação |
| N-017 condição não booleana | `invalid_type` | verificação |
| N-018 bloco inalcançável | `invalid_control_flow` | verificação |
| N-019 multiplicação com overflow | `integer_overflow` / Trap | execução |
| N-020 subtração com overflow | `integer_overflow` / Trap | execução |
| N-021 quota de memória | `out_of_memory` / ResourceLimit | execução |

Casos válidos também são reexecutados (mesmo valor e fuel) e passam pelo
diferencial `encode → decode → verify → execute` com valor e fuel idênticos.

## 5. Descobertas, decisões e divergências

1. **Estouro de pilha no intérprete recursivo (defeito real).** Com
   `CallDepthLimit::HARD_MAX = 256`, a versão recursiva abortou o processo de teste
   com `STATUS_STACK_OVERFLOW` antes de aplicar o limite. A decisão anterior
   (ADR-0012 "Proposto") foi refutada na prática; a execução passou a usar pilha de
   frames explícita na heap, mantendo `HARD_MAX = 256`. ADR-0012 foi reescrito com
   o resultado; `docs/ARQUITETURA.md` e o contrato da RIR foram corrigidos.
2. **Formato textual (ADR-0010).** O documento da fundação dizia que a notação
   legível não é gramática; foi necessário fechar uma gramática para o corpus ser
   dado. Ela é explicitamente não normativa e não promete compatibilidade.
3. **Ambiguidade de `return` no texto.** `return <nome>` versus `return` seguido
   de novo bloco `<nome>:` é desambiguado pelo token seguinte (`:`/`(`); a regra
   está documentada no crate e no contrato.
4. **Quota de memória (ADR-0013).** A lacuna 1 foi fechada com um modelo
   determinístico de bytes por frame (256 + slots × 24), cobrado antes de alocar,
   com liberação no retorno e pico observável. Medir RSS real continua fora do
   escopo e o modelo não substitui medição.
5. **Bytecode (ADR-0011).** Protótipo implementado para exercitar decoder hostil e
   diferencial; o ADR-0004 continua aberto e nenhum formato é estável.
6. **Não houve ajuste de expectativa para "fazer passar".** Nenhum caso do corpus
   foi editado durante a integração; a única correção de teste foi interna ao
   `rvm-ir` (marcação de parâmetros implícitos no teste de densidade).

## 6. Segurança: o que foi demonstrado e o que não foi

Demonstrado no escopo executado:

- Nenhum módulo inválido do corpus chega ao executor (teste de classe `Module`
  antes da execução).
- Overflow aritmético produz trap com localização, em vez de wrap silencioso.
- Fuel e profundidade interrompem execução com códigos distintos e determinísticos.
- O caminho de concessão de capability é explícito e escopado (ADR-0036): sem
  concessão, seis efeitos e `host_read_file` são negados; a leitura só ocorre com
  `filesystem_read` concedido para o caminho.
- Decoder binário e parser textual rejeitam truncamento, opcode/tag/versão
  desconhecidos e contagens gigantes sem pânico observado nos orçamentos
  declarados.

**Não** demonstrado: isolamento de processo/OS, ausência de DoS (a quota é modelada, não RSS), resistência a fuzzing de campanha, revisão
independente, comportamento sob ASLR/side channels, nem segurança de capabilities
futuras. O protótipo não é sandbox multi-tenant.

## 7. Hipóteses da fundação (H01–H08)

| ID | Classificação | Base |
|---|---|---|
| H01 núcleo comum a dois frontends | **inconclusiva** | nenhum frontend integrado; AST do Rio é privada e a licença é desconhecida |
| H02 verificador rejeita inválidos antes da execução | **corroborada no escopo do corpus** | 19 casos inválidos, todos `ErrorClass::Module` antes da execução |
| H03 chamada inter-módulo sem serialização geral | **confirmada estruturalmente** | ADR-0018; ver `BENCHMARKS-PREREGISTRADOS.md` |
| H04 transferência exclusiva/alias | **não testada** | sem objetos, referências ou aliasing no EXP-0 |
| H05 capabilities deny-by-default | **corroborada no escopo** | default deny; concessão explícita só para `filesystem_read` com escopo; testes de negação (ADR-0036) |
| H06 custo de chamada inter-módulo ≤ 25% | **confirmada no runner dedicado; inconclusiva na reexecução local** | dispersão 1,33% no runner; 57,8%/31,8% local — ver `BENCHMARK-REEXECUCAO-2026-11-02.md` |
| H07 fuel/profundidade determinísticos | **corroborada no escopo** | fronteiras exatas 5/6, 8/255/256 e testes repetidos |
| H08 integração Lift sem alterar semântica | **bloqueada** | sem clone/auditoria do Lift; ADR-0007 inalterado |

Nenhuma classificação é "provada"; todas valem apenas para o corpus e ambiente
executados.

## 8. Critérios de aceitação do PRD §8

| Critério | Situação |
|---|---|
| (a) semântica escrita | atendido (contratos + código) |
| (b) inválidos rejeitados antes da execução | atendido no corpus |
| (c) recursos limitados observáveis | atendido no escopo | fuel, profundidade e memória modelada (ADR-0013) |
| (d) corpus automatizável | atendido (`cargo test --workspace`) |
| (e) rota frontend→RIR demonstrada **ou** bloqueada com evidência | bloqueada com evidência: `BASELINE-RIO.md` (AST privada, sem `LICENSE`) e ADR-0007/0008 inalterados |
| (f) hipóteses classificadas | atendido (§7) |
| (g) riscos publicados | atendido (§6, §9) |

## 9. Riscos, lacunas e desconhecidos

1. **Memória de frames**: quota modelada implementada e testada (ADR-0013).
   Resíduo: o modelo não mede RSS real e vetores transitórios de argumentos de
   chamada/desvio não são contados; a quota limita crescimento, não prova consumo.
   **Atualização (ADR-0035):** o modelo passou a contabilizar bytes retidos por
   slot e a liberá-los ao sobrescrever/retornar; buffers transitórios continuam
   contados (ADR-0016).
2. **Sem benchmark**: H06 e qualquer alegação de desempenho continuam sem medição.
   **Atualização:** o benchmark pré-registrado foi executado depois (runner
   dedicado e reexecução local); ver `BENCHMARKS-PREREGISTRADOS.md` §5/§8 e
   `BENCHMARK-REEXECUCAO-2026-11-02.md`. A reexecução local pós ADR-0034/0035
   ficou inconclusiva por dispersão; a confirmação de H06 é a do runner.
3. **Fuzzing limitado e determinístico**: 10 módulos × 200 mutações (bytecode) e
   10 × 200 (texto); não é campanha e não prova ausência de bugs.
4. **Sem revisão independente ou auditoria** de segurança.
5. **Versionamento e CI**: repositório git inicializado, publicado no GitHub e
   com gates em `.gitlab-ci.yml` e `.github/workflows/ci.yml` (fmt, clippy, test,
   test-release). O push para o remoto GitHub e a
   proteção da branch principal dependem de credencial do mantenedor.
6. **Licença do projeto não definida** (pendência do mantenedor).
7. **Corpus parcial**: cobre `add/sub/mul`, `eq/ne/lt/le/gt/ge`, chamada com `unit` e overflow de `add/sub/mul`; ainda não cobre limite estrutural, encadeamento de merges nem `call_static` com aridade máxima.
8. **Parser textual é superfície de ataque** e não passou por campanha de fuzzing.
9. **Limites estruturais** colapsam em `invalid_module`, sem código granular.
10. **RIO-BASELINE-001 é inspeção estática**: nada do Rio foi compilado/executado;
    licença desconhecida bloqueia reutilização.
11. **Lift não foi auditado** nesta rodada (sem clone; somente evidência documental).
12. ADR-0004 e ADR-0006 (formato e memória) continuam abertos.

## 10. Testes não executados

- Suíte do repositório Rio (`scripts/validate.py`, `cargo test`) — por decisão de
  escopo e para não alterar o checkout de terceiros.
- Qualquer coisa do Lift (não clonado).
- `cargo bench` (não há bench configurado), `cargo miri`, fuzzers externos.
- CI de outras plataformas; execução apenas local em Windows.
- Auditoria de dependências (não há dependências externas a auditar).

## 11. Processo e falhas de subagentes (transparência)

- `rvm-verify`: subagente terminou com 17 testes e clippy limpo.
- `rvm-bytecode`: subagente escreveu implementação e testes hostis, mas falhou
  antes do relatório; o código foi verificado pelo orquestrador (4+14 testes).
- `rvm-interpreter`: subagente escreveu a primeira versão, que estourou a pilha no
  teste de profundidade e fez o agente falhar; o orquestrador reescreveu o motor de
  execução como máquina de frames explícita.
- `rvm-text`: subagente falhou antes de escrever código; o orquestrador implementou
  parser, impressão e 10 testes.
- CLI, corpus, testes de conformidade/propriedades e documentação: orquestrador.
- EGM: dos 4 workers de README, 1 falhou sem saída e 3 foram interrompidos após mais
  de 20 minutos sem arquivos; apenas `crates/rvm-verify/src/README.md` foi
  produzido por worker (mantido e conferido contra o código). Os outros 19
  contratos foram escritos pelo orquestrador e verificados por script de links.

Essas falhas são de orquestração, não de especificação; nenhuma expectativa de
conformidade foi relaxada por causa delas.

## 12. Próximo passo sugerido

1. Manter o remoto GitHub atualizado e o pipeline de CI verde a cada mudança.
2. Definir licença do repositório e a política de reuso do Rio.
3. Ampliar o corpus (limite estrutural, merges encadeados, aridade máxima) e fixar meta de cobertura por operação.
4. Implementar quota de memória para frames e medir consumo por frame.
5. Campanha de fuzzing (parser, decoder, verificador) com corpus e orçamento.
6. Benchmarks pré-registrados para H06 e H03 (chamada inter-módulo exige decidir
   linking estático antes).
7. Auditar Lift (commit imutável, licença) ou fechar ADR-0007 como bloqueado.
