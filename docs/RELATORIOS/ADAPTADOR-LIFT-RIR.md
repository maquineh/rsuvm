# Adaptador Lift→RIR — protótipo e teste diferencial

- **Data:** 2026-11-02
- **Adaptador:** `frontends/lift-rir` (workspace separado; ADR-0021)
- **Lift de origem:** commit `5e59af55baba7af53f8763140d35c972399d2851`
  (`https://github.com/maquineh/lift`, licença `MIT OR Apache-2.0`)
- **Escopo:** subconjunto de funções `int`/`boolean`/`void` com `let`/`const`,
  **atribuição**, `return`, `if/else` (com merge) e **`while`** (ver ADR-0021)

## 1. Pipeline

~~~text
código .lift ── lift_lexer::tokenize ── lift_parser::parse ── lift_sema::analyze
                                                                      │
                                                              HirProgram (HIR pública)
                                                                      │
                                                    lift-rir::lower_source(…, entry)
                                                                      ▼
                                    rvm_ir::Module ── rvm_verify ── rvm_interpreter::execute
~~~

## 2. Teste diferencial (Resultado)

Programa Lift de referência (`rsuvm-oracle.lift`) executado pelo **compilador do
Lift**:

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle.lift
```

Saída do Lift (oráculo):

~~~text
30
5
7
9
3
true
false
25
12
55
0
~~~

O adaptador baixa o **mesmo** programa para a RIR e executa na RSUVM. Comparação:

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `soma` | `soma(10, 20)` | 30 | `i64:30` | sim |
| `absoluto` | `absoluto(-5)` | 5 | `i64:5` | sim |
| `absoluto` | `absoluto(7)` | 7 | `i64:7` | sim |
| `maior` | `maior(3, 9)` | 9 | `i64:9` | sim |
| `maior` | `maior(3, -9)` | 3 | `i64:3` | sim |
| `positivo` | `positivo(5)` | true | `bool:true` | sim |
| `positivo` | `positivo(-5)` | false | `bool:false` | sim |
| `quadrado` (`let`) | `quadrado(4)` | 25 | `i64:25` | sim |
| `quadruplo` (chamadas) | `quadruplo(3)` | 12 | `i64:12` | sim |
| `fib` (recursão) | `fib(10)` | 55 | `i64:55` | sim |
| `fib` | `fib(0)` | 0 | `i64:0` | sim |

**Todos os 11 pontos coincidem.**

### 2.1 `while`, atribuição e merge de `if`

Segundo programa de referência (`rsuvm-oracle2.lift`) executado pelo compilador do
Lift:

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle2.lift
```

Saída do Lift (oráculo): `15 0 7 0 1 2 10 3 6 0`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `soma_ate` (`while`+atribuição) | `soma_ate(5)` | 15 | `i64:15` | sim |
| `soma_ate` | `soma_ate(0)` | 0 | `i64:0` | sim |
| `conta` (`while`) | `conta(7)` | 7 | `i64:7` | sim |
| `conta` | `conta(0)` | 0 | `i64:0` | sim |
| `classifica` (`if/else` com merge) | `classifica(5)` | 1 | `i64:1` | sim |
| `classifica` | `classifica(-5)` | 2 | `i64:2` | sim |
| `limita` (`if` sem `else`) | `limita(20)` | 10 | `i64:10` | sim |
| `limita` | `limita(3)` | 3 | `i64:3` | sim |
| `soma_condicional` (`while` aninhado em `if`) | `(4, true)` | 6 | `i64:6` | sim |
| `soma_condicional` | `(4, false)` | 0 | `i64:0` | sim |

**Todos os 10 pontos coincidem.**

Os testes automatizados vivem em `frontends/lift-rir/tests/diferencial.rs`
(13 testes; cada um roda o pipeline completo Lift→RIR→verificação→execução).

Comando de verificação:

```bash
cargo test --manifest-path frontends/lift-rir/Cargo.toml
```

Resultado: `13 passed` (diferencial) + `6 passed` (CLI) = **19 passed; 0 failed**.

## 3. Limitações

1. Subconjunto ainda estreito: **sem** `else if`, divisão/resto, `f64`, strings,
   arrays, objetos, closures, `console.log`/`import` nem `not`/`and`/`or`. Fora
   disso, `AdapterError::Unsupported`. Divisão/resto **não existem** no núcleo
   EXP-0 (exigiriam ADR com op nova); strings exigiriam um tipo novo no núcleo.
2. O adaptador é um **workspace separado** e depende do Lift por git fixado; o
   núcleo RSUVM continua sem dependências externas.
3. O teste compara com o oráculo do Lift **registrado**; não invoca o compilador do
   Lift em tempo de teste (evita dependência de rede/rustc no teste).
4. Não há tradução reversa (RIR→Lift) nem cobertura de efeitos de host.

## 4. Reprodução

```bash
# adaptador (workspace separado)
cargo test --manifest-path frontends/lift-rir/Cargo.toml

# oráculo do Lift (requer o clone do Lift)
cargo run -p lift_cli -- run examples/rsuvm-oracle.lift
```

## 5. Binário `lift-run`

Roda um `.lift` ponta a ponta na RVM (o `.lift` deve ser salvo **sem BOM**).

```bash
cargo run --manifest-path frontends/lift-rir/Cargo.toml --bin lift-run -- \
    fib.lift fib --arg i64:10
# i64:55
```

Contrato de saída e exit codes (espelha o `rsuvm`):

| Situação | Exit | Exemplo |
|---|---|---|
| Sucesso | 0 | `fib(10)` → `i64:55` |
| Módulo (fora do subconjunto / fonte inválida) | 2 | `while` → `AdapterError::Unsupported` |
| Trap (overflow) | 3 | `i64::MAX + 1` → `integer_overflow` |
| Limite (fuel/profundidade/memória) | 4 | `--fuel 1` → `out_of_fuel` |
| Uso/IO | 1 | arquivo inexistente, flag inválida |

Testes do binário: `frontends/lift-rir/tests/cli.rs` (5 testes, cobrindo 0/2/3/4).
