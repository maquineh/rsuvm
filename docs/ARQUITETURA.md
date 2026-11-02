# Arquitetura da RSUVM — EXP-0

**Estado:** desenho experimental em implementação. Este documento descreve o que
o código **deve** fazer; o estado real de cada crate aparece em
`docs/RELATORIOS/RELATORIO-IMPLEMENTACAO.md` e no `README.md` de cada crate.
**Base:** `docs/fundacao/02-TECHSPEC.md` (componentes de referência) e decisões em
`docs/ADRs/`.

## 1. Pipeline

~~~text
texto .rvm ── rvm-text::parse ──┐
                                ├── rvm_ir::Module ── rvm-ir::structure::check ──┐
bytes .rsvmbc ─ rvm-bytecode ───┘                                              │
                                                                                ▼
                                                     rvm-verify::VerifiedModule::verify
                                                                                │
                                                                                ▼
                                        rvm-interpreter::execute(VerifiedModule, …)
                                                                                │
                                                                                ▼
                                    RunOutcome { valor, fuel_used, max_depth }   │
                                    ou RvmError { código estável, categoria }    │
~~~

Regras estruturais:

1. Nenhum crate do núcleo conhece sintaxe ou AST de Lift/Rio (ADR-0001).
2. O executor recebe **somente** `rvm_verify::VerifiedModule`, cujo único
   construtor público é `verify`. Não existe `From<Module>`.
3. O verificador chama `rvm_ir::structure::check` antes de olhar tipos; o decoder
   binário também chama `structure::check` antes de devolver o módulo.
4. O texto e o bytecode produzem `Module`; ambos são entradas **não confiáveis**.
5. A ligação inter-módulo é **opcional e pré-verificação**: `rvm-link::link`
   combina módulos com declarações externas (`import`) em um único `Module` plano,
   sem externas, que segue o pipeline normal (ADR-0018).

## 2. Crates e dependências

| Crate | Depende de | Responsabilidade |
|---|---|---|
| `rvm-spec` | — | tipos, operações, códigos de erro, limites, semântica de overflow |
| `rvm-ir` | spec | modelo RIR, invariantes de IDs, `structure::check`, fixtures de teste |
| `rvm-runtime` | spec | `Value`, quotas (`ExecConfig`, `CallDepthLimit`), hosts e capabilities (deny-by-default) |
| `rvm-verify` | spec, ir | tipos, CFG, dominância, assinatura de chamada e de host, `VerifiedModule` |
| `rvm-text` | spec, ir | parser e impressão canônica do formato textual experimental |
| `rvm-bytecode` | spec, ir | codificação linear experimental e decoder limitado |
| `rvm-interpreter` | spec, ir, runtime, verify | execução determinística com fuel, profundidade, traps e host |
| `rvm-link` | spec, ir | ligação estática inter-módulo: resolve externas em um módulo plano |
| `rvm-cli` | todos acima | binário `rsuvm`; nega efeitos por padrão e concede `filesystem_read` com `--allow-fs-read` (ADR-0036) |
| `rvm-conformance` | todos (dev) | corpus em dados + testes de conformidade e propriedades |

O grafo é acíclico e o compilador impõe a separação: `rvm-interpreter` não pode
importar `rvm-text`, e nenhum crate de runtime pode importar um adaptador de
linguagem (ADR-0009).

## 3. Subset semântico (EXP-0)

- Tipos: `unit`, `bool`, `i64`, `string`, `f64` mais os compostos `array T`,
  `struct Nome`, `option T` e `result T T`, todos recursivos pela **tabela de
  tipos** do módulo (ADR-0034). Sem coerção implícita.
- Operações: `const_i64`, `const_bool`, `const_str`, `const_f64`, `add_i64_checked`,
  `sub_i64_checked`, `mul_i64_checked`, `div_i64_checked`, `rem_i64_checked`,
  `shl_i64_checked`, `shr_i64_checked`, `and_i64`, `or_i64`, `xor_i64`, `eq_i64`,
  `ne_i64`, `lt_i64`, `le_i64`, `gt_i64`, `ge_i64`, `str_eq`, `str_ne`, `str_len`,
  `concat_str`, `add_f64`, `sub_f64`, `mul_f64`, `div_f64`, `eq_f64`, `ne_f64`,
  `lt_f64`, `le_f64`, `gt_f64`, `ge_f64`, `array`, `array_len`, `array_get`,
  `array_set`, `struct_new`, `field_get`, `field_set`, `option_none`, `option_some`,
  `option_is_some`, `option_unwrap`, `coalesce`, `option_eq`, `option_ne`,
  `result_ok`, `result_err`, `result_is_ok`, `result_unwrap`, `result_unwrap_err`,
  `call_static`, `host_read_file` (ABI fechada, ADR-0036).
- Terminadores: `return`, `br`, `cond_br`.
- Merge por parâmetros de bloco (phi explícito).
- Fora do EXP-0: referências, closures, `unsafe`, FFI,
  exceções recuperáveis, threads, async. Arrays existem **imutáveis** (ADR-0027),
  com atualização **funcional** por `array_set` (ADR-0028, sem referências), sem
  mutação in-place nem slicing; structs existem com qualquer tipo de campo
  (ADR-0029/0034), com atualização **funcional** por `field_set` (ADR-0032), sem
  mutação in-place; `option`/`result` existem com **qualquer payload**, inclusive
  agregados e aninhados (ADR-0030/0033/0034), com igualdade
  `option_eq`/`option_ne` (ADR-0031); `f64` existe (ADR-0026) sem
  `%`/`pow`/conversões; concatenação de `string` existe (ADR-0025), sem slicing.

Semântica aritmética: `i64::checked_*`; resultado fora de `[-2^63, 2^63-1]`
produz o trap `integer_overflow`, divisão/resto por zero produz
`division_by_zero` (ADR-0022) e deslocamento com quantidade fora de `0..=63`
produz `invalid_shift` (ADR-0023). `f64` usa **IEEE-754 sem trap** (divisão por
zero → `inf`/`NaN`, ADR-0026). O comportamento é idêntico em debug e release
(`overflow-checks = true` também no perfil release, como defesa extra).

## 4. Quotas

- **Fuel:** 1 unidade por instrução despachada + 1 por terminador despachado.
  Débito antes do despacho; saldo 0 ⇒ `out_of_fuel` sem executar a operação.
  Chamadas não resetam o saldo.
- **Profundidade:** frame de entrada = 1; entrar no frame `d+1` com
  `d+1 > max_call_depth` ⇒ `call_depth_exceeded`.
- **Limites estruturais:** contagens máximas por módulo/função/bloco e bytes de
  entrada ([`rvm_spec::Limits`]) aplicados antes de alocar.
- **Teto de profundidade:** `CallDepthLimit::HARD_MAX = 256`, validado por tipo e
  aplicado sobre uma **pilha explícita de frames na heap**. A primeira
  implementação era recursiva e estourou a pilha nativa de uma thread de teste
  com 256 frames (ADR-0012 registra o resultado e a correção); profundidade de
  chamadas não pode decidir o uso de pilha nativa do host.
- **Memória modelada:** custo por frame = `256 + slots × 24` bytes e custo de
  buffer transitório de argumentos = `args × 24` bytes; a cobrança acontece antes
  de alocar, conta memória viva e libera ao fim. O **conteúdo** dos slots também é
  contabilizado: `string` custa os bytes UTF-8, `array`/`struct` custam
  `itens × 24` + filhos, `option`/`result` custam `24` + payload; a gravação de um
  slot cobra o valor novo e libera o anterior, e o retorno do frame libera os
  slots (ADR-0035). Teto padrão de 16 MiB e `out_of_memory` ao exceder
  (ADR-0013, ADR-0016, ADR-0035). É modelo determinístico, não RSS.
- **Quota por efeito (host):** `read_file` exige `max_bytes` e o host pode impor
  teto menor (1 MiB no CLI); a string devolvida entra na quota de memória ao ser
  gravada. A capability é conferida **antes** do efeito; sem concessão ⇒
  `capability_denied` (ADR-0036).

## 5. Erros

Uma única estrutura `RvmError` com código estável, categoria de pipeline e
localização opcional. Tabela completa em `docs/CONTRATOS/ERROS-EXP-0.md`. As três
respostas exigidas pelo PRD (módulo inválido, trap, limite) são distinguidas por
`ErrorClass`, não por texto humano.

## 6. Formatos de entrada

- Texto experimental: `docs/CONTRATOS/TEXTO-EXP-0.md` (ADR-0010).
- Bytecode experimental: `docs/CONTRATOS/BYTECODE-EXP-0-IMPLEMENTACAO.md`
  (ADR-0011; ADR-0004 continua aberto, nenhum formato é estável).

## 7. Testes

| Camada | Onde |
|---|---|
| semântica de operações | `crates/rvm-spec` |
| invariantes estruturais | `crates/rvm-ir` |
| tipos/CFG/dominância | `crates/rvm-verify` |
| parser/impressão | `crates/rvm-text` |
| decoder hostil | `crates/rvm-bytecode` |
| fuel/depth/traps | `crates/rvm-interpreter` |
| corpus C-001..C-006 / N-001..N-012 | `crates/rvm-conformance` |
| CLI (códigos de saída) | `crates/rvm-cli/tests` |

Comando de verificação do projeto: `cargo test --workspace`.

## 8. Limites declarados (não alegações de segurança)

- O protótipo **não** é sandbox de processo/OS: quotas e verificação são
  in-process. Uso multi-tenant hostil exige isolamento externo revisado.
- O corpus demonstra o comportamento no conjunto testado, não ausência de
  vulnerabilidades.
- Nenhuma capability é concedida por padrão: `execute` sem host e `HostApi`
  negam tudo. A exceção explícita é `read_file` com `filesystem_read`, concedida
  programaticamente por `Capabilities::grant`/`execute_with_host` ou pelo CLI com
  `--allow-fs-read <dir>` (ADR-0036). Isso é uma propriedade do código atual, não
  uma promessa de que efeitos futuros serão seguros.
- Adaptadores Lift/Rio não existem neste repositório; ver
  `docs/RELATORIOS/BASELINE-RIO.md` e `docs/fundacao/INTEGRACAO/`.
