# ADR-0018 — Ligação estática inter-módulo

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** etapa B1 (lacuna remanescente do §9 do
  `RELATORIO-IMPLEMENTACAO.md`); H03 e H06 de §7; item 6 do §12 ("chamada
  inter-módulo exige decidir linking estático antes"); ADR-0001 (núcleo
  independente de linguagem).

## Contexto

No EXP-0, a RIR representa **um** módulo e `call_static` só alcança funções do
próprio módulo. As hipóteses H03 ("chamada inter-módulo sem serialização geral") e
H06 ("custo de chamada inter-módulo ≤ 25%") estavam **não medidas** por falta de
ligação entre módulos. O benchmark pré-registrado (etapa A3) depende de uma decisão
de ligação estática.

Alternativas consideradas para representar a dependência entre módulos: (a) um
campo `imports` no `Module`, com `call_static` referenciando importações por
deslocamento de `FnId`; (b) ligação em tempo de execução, com resolução por nome;
(c) serialização de argumentos entre módulos; (d) ligação estática por
**declarações externas** resolvidas antes da verificação.

## Decisão

1. A ligação é **estática e pré-verificação**. O tipo `rvm_ir::Module` **não
   muda**: uma **declaração externa** é uma `Function` sem blocos
   (`Function::is_external`). Ela nomeia uma função a ser fornecida por outro
   módulo.
2. O formato textual ganha `import <nome>` no nível do módulo
   (`docs/CONTRATOS/TEXTO-EXP-0.md`), que cria a declaração externa. A impressão
   canônica reemite `import <nome>`.
3. `rvm_ir::structure::check` aceita declarações externas, mas a **função de
   entrada não pode ser externa**. `rvm_verify::VerifiedModule::verify` **rejeita**
   módulos que ainda contenham externas: o executor nunca vê uma declaração
   externa.
4. Novo crate **`rvm-link`** com
   `link(modules, entry_module, entry_function, limits) -> Result<Module, RvmError>`:
   - valida nomes de módulo únicos;
   - atribui IDs globais densos às funções concretas (ordem dos módulos);
   - resolve cada externa para a função concreta de **mesmo nome** em outro
     módulo; ausência de provedor ou dois provedores ⇒ `invalid_module`;
   - reescreve `call_static` para IDs globais e renomeia funções para unicidade
     (`<modulo>__<funcao>`), preservando determinismo;
   - produz um `Module` plano, sem externas, que segue o pipeline normal
     (verificar → executar; `encode`/`decode` continuam válidos).
5. **Sem serialização**: a chamada inter-módulo é uma chamada direta por índice
   após a ligação — nenhum argumento é convertido para bytes (H03).

## Alternativas

1. **Campo `imports` no `Module`:** mais expressivo (namespaces, qualificação),
   porém invade a RIR, o bytecode e o verificador; adiado.
2. **Ligação em runtime (resolução por nome na execução):** introduz custo e
   não-determinismo de descoberta, e um caminho de falha novo no executor;
   rejeitado.
3. **Serialização de argumentos:** contraria H03 e adiciona custo; rejeitado.
4. **Duplicar a função provedora em cada módulo:** duplica código e quebra a
   identidade da função; rejeitado.

## Consequências

- Positivas: H03/H06 deixam de estar bloqueadas; o núcleo (RIR, verificador,
  intérprete, bytecode) permanece sem imports e sem dependências; a ligação é
  determinística e testável.
- Negativas: a resolução é **por nome único** — sem namespaces qualificados, dois
  provedores com o mesmo nome são ambíguos; os nomes das funções mudam no módulo
  plano (`<modulo>__<funcao>`), alterando diagnósticos; não há versionamento de
  ABI entre módulos.
- Residual: ligação **dinâmica**, assinaturas de import e resolução qualificada
  ficam para ADR futuro; a medição de H06 ainda depende de executar o benchmark
  pré-registrado (etapa B2).

## Validação

- `crates/rvm-conformance/tests/linking.rs`: liga dois módulos, verifica e executa
  (`i64:30`); diferencial `encode → decode → verify → execute`; falhas de externa
  sem provedor, provedor ambíguo, nome de módulo duplicado e verificação de módulo
  não linkado.
- `crates/rvm-ir/src/structure.rs`: aceita externa sem corpo e rejeita entrada
  externa.
- `crates/rvm-text`: `import` parseia, imprime e sobrevive ao round-trip.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`
  e `cargo fmt --all -- --check` verdes.
