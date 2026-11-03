# Adaptador Lift→RIR — protótipo e teste diferencial

- **Data:** 2026-11-02
- **Adaptador:** `frontends/lift-rir` (workspace separado; ADR-0021)
- **Lift de origem:** commit `5e59af55baba7af53f8763140d35c972399d2851`
  (`https://github.com/maquineh/lift`, licença `MIT OR Apache-2.0`)
- **Escopo:** subconjunto de funções `int`/`boolean`/`string`/`number`/`void`,
  arrays de escalares, structs (`interface`) com campos escalares e opcionais
  (`T?`), com `let`/`const`, **atribuição** (a binding e a elemento de array),
  `return`, `if`/`else if`/`else` (com merge), **`while`** e lógica `!`/`&&`/`||`
  com curto-circuito (ver ADR-0021)

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

### 2.2 Divisão e resto

Terceiro programa de referência (`rsuvm-oracle3.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle3.lift
```

Saída do Lift (oráculo): `3 -3 2 -2` (truncamento para zero).

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `divide` | `divide(20, 6)` | 3 | `i64:3` | sim |
| `divide` | `divide(-20, 6)` | -3 | `i64:-3` | sim |
| `resto` | `resto(20, 6)` | 2 | `i64:2` | sim |
| `resto` | `resto(-20, 6)` | -2 | `i64:-2` | sim |

Divisão/resto por zero produz o trap `division_by_zero` (ADR-0022); o `lift-run`
sai com código 3.

### 2.3 `else if`, lógica e curto-circuito

Quarto programa de referência (`rsuvm-oracle4.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle4.lift
```

Saída do Lift (oráculo):
`-1 0 1 true false false true false true false true true true`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `faixa` (`else if`) | `faixa(-5)` / `faixa(0)` / `faixa(5)` | -1 / 0 / 1 | `i64:-1` / `i64:0` / `i64:1` | sim |
| `ambos` (`&&`) | `(true,true)` / `(true,false)` | true / false | `bool:true` / `bool:false` | sim |
| `algum` (`\|\|`) | `(false,false)` / `(false,true)` | false / true | `bool:false` / `bool:true` | sim |
| `nega` (`!`) | `(true)` / `(false)` | false / true | `bool:false` / `bool:true` | sim |
| `seguro_and` (**curto-circuito**) | `(4,0)` | false | `bool:false` | sim |
| `seguro_and` | `(4,2)` | true | `bool:true` | sim |
| `seguro_or` (**curto-circuito**) | `(4,0)` | true | `bool:true` | sim |
| `seguro_or` | `(4,2)` | true | `bool:true` | sim |

O caso `seguro_and(4,0)` = `b != 0 && (a / b) > 0` prova o curto-circuito: se o
Lift avaliasse `4 / 0`, o programa terminaria em erro; ambos os lados produzem
`false` sem trap.

Os testes automatizados vivem em `frontends/lift-rir/tests/diferencial.rs`
(19 testes; cada um roda o pipeline completo Lift→RIR→verificação→execução).

### 2.4 `string`

Quinto programa de referência (`rsuvm-oracle5.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle5.lift
```

Saída do Lift (oráculo): `ola true false true`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `saudacao` (literal `string`) | `saudacao()` | ola | `str:ola` | sim |
| `igual` (`==` de string) | `("x","x")` | true | `bool:true` | sim |
| `igual` | `("x","y")` | false | `bool:false` | sim |
| `diferente` (`!=` de string) | `("x","y")` | true | `bool:true` | sim |
| `junta` (`+` de string) | `("ola"," mundo")` | ola mundo | `str:ola mundo` | sim |

O caso `junta` usa o oráculo `rsuvm-oracle6.lift` (o `+` de `string` do Lift é
concatenação; ver ADR-0025).

### 2.5 `f64` (`number`)

Sexto programa de referência (`rsuvm-oracle7.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle7.lift
```

Saída do Lift (oráculo): `1.5 true false`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `media` (`number` aritmética) | `media(1.0, 2.0)` | 1.5 | `f64:1.5` | sim |
| `menor` (`<` de number) | `(1.0, 2.0)` | true | `bool:true` | sim |
| `menor` | `(2.0, 1.0)` | false | `bool:false` | sim |

Comando de verificação:

```bash
cargo test --manifest-path frontends/lift-rir/Cargo.toml
```

### 2.6 Arrays

Sétimo programa de referência (`rsuvm-oracle8.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle8.lift
```

Saída do Lift (oráculo): `20`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `segundo` (literal + índice) | `segundo()` | 20 | `i64:20` | sim |

### 2.7 `array_set` (`xs[i] = v`)

Oitavo programa de referência (`rsuvm-oracle9.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle9.lift
```

Saída do Lift (oráculo): `9 14 6`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `troca` (`xs[0] = 9`) | `troca()` | 9 | `i64:9` | sim |
| `soma` (`xs[1] = 10`) | `soma()` | 14 | `i64:14` | sim |
| `acumula` (`while` + `xs[i] = i*2`) | `acumula()` | 6 | `i64:6` | sim |

`xs[i] = v` vira `array_set` + **religação** do binding ao array novo (atualização
funcional, ADR-0028). Como o Lift tem semântica de **referência**, o adaptador só
baixa a atribuição quando `xs` é um **local não-aliasado** (declarado por literal e
usado só como objeto de indexação); arrays possivelmente aliasados devolvem
`AdapterError::Unsupported` em vez de um módulo incorreto.

Resultado: `25 passed` (diferencial) + `9 passed` (CLI) = **34 passed; 0 failed**.

### 2.8 Structs (`interface`)

Nono programa de referência (`rsuvm-oracle10.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle10.lift
```

Saída do Lift (oráculo): `7 ana 42`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `soma` (literal + `p.x`) | `soma()` | 7 | `i64:7` | sim |
| `nome` (campos `string`/`boolean`) | `nome()` | ana | `str:ana` | sim |
| `usa` (struct como parâmetro) | `usa()` | 42 | `i64:42` | sim |

O adaptador mapeia `interface Nome { … }` para a tabela de structs, o literal
`{ campo: valor }` para `struct_new` (na ordem declarada) e `obj.campo` para
`field_get` (ADR-0029). Campos não escalares (arrays, structs) são rejeitados.

Resultado: `27 passed` (diferencial) + `9 passed` (CLI) = **36 passed; 0 failed**.

### 2.9 `Option`/`??`

Décimo e décimo-primeiro programas de referência (`rsuvm-oracle11.lift` e
`rsuvm-oracle12.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle11.lift
cargo run -p lift_cli -- run examples/rsuvm-oracle12.lift
```

Saídas do Lift (oráculo): `hi missing 7 9 5 true false` e `5`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `resolve` (`??` com fallback) | `("hi")` | hi | `str:hi` | sim |
| `resolve` | `(null)` | missing | `str:missing` | sim |
| `teste` (`??` sobre função) | `(true)` / `(false)` | 7 / 9 | `i64:7` / `i64:9` | sim |
| `some_val` (`option.unwrap`) | `some_val()` | 5 | `i64:5` | sim |
| `is_some_val`/`is_none_val` | — | true / false | `bool:true` / `bool:false` | sim |
| `seguro` (**fallback preguiçoso**) | `seguro()` | 5 | `i64:5` | sim |

`??` é baixado com **fluxo de controle** (`option_is_some` + ramos), preservando o
fallback preguiçoso: `option.some(5) ?? (1 / 0)` devolve `5` sem trap; já
`null ?? (1 / 0)` trap `division_by_zero` (ADR-0030). `x == null`/`x != null` ficam
fora (`Unsupported`).

Resultado: `31 passed` (diferencial) + `9 passed` (CLI) = **40 passed; 0 failed**.

### 2.10 `== null`/`!= null`

Décimo-segundo programa de referência (`rsuvm-oracle13.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle13.lift
```

Saída do Lift (oráculo): `true false true false`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `eh_nulo` (`x == null`) | `(null)` / `(some 1)` | true / false | `bool:true` / `bool:false` | sim |
| `nao_nulo` (`x != null`) | `(some 1)` / `(null)` | true / false | `bool:true` / `bool:false` | sim |

`==`/`!=` com qualquer operando `option` vira `option_eq`/`option_ne` (ADR-0031);
outros operadores sobre `option` continuam `Unsupported`.

### 2.11 `field_set` (`obj.campo = v`)

Décimo-terceiro programa de referência (`rsuvm-oracle14.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle14.lift
```

Saída do Lift (oráculo): `11 3 bia`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `muda` (`p.x = 9`) | `muda()` | 11 | `i64:11` | sim |
| `acumula` (`while` + `p.x = p.x + i`) | `acumula()` | 3 | `i64:3` | sim |
| `troca` (campo `string`) | `troca()` | bia | `str:bia` | sim |

`obj.campo = v` vira `field_set` + religação (atualização funcional, ADR-0032),
só quando `obj` é um local não-aliasado (análise generalizada); structs
possivelmente aliasadas devolvem `Unsupported`.

Resultado: `33 passed` (diferencial) + `9 passed` (CLI) = **42 passed; 0 failed**.

### 2.12 Tabela de tipos e `option` de agregados (ADR-0034)

Com a representação de tipos por id, o adaptador pré-interna os tipos do programa
e emite `TypeId`; `option` de array e de struct passam a ser suportados
(`rsuvm-oracle15.lift`):

```bash
cargo run -p lift_cli -- run examples/rsuvm-oracle15.lift
```

Saída do Lift (oráculo): `2 7`.

| Caso (Lift) | Chamada | Oráculo Lift | RSUVM (RIR) | Igual? |
|---|---|---|---|---|
| `opt_array` (`int[]?`) | `opt_array()` | 2 | `i64:2` | sim |
| `opt_struct` (`P?`) | `opt_struct()` | 7 | `i64:7` | sim |

O **`option` aninhado** é suportado pelo **núcleo** (`C-058`), mas o Lift o rejeita
na fonte ("opcional aninhado ... não é suportado"), então o adaptador não o
exercita.

Resultado: `34 passed` (diferencial) + `9 passed` (CLI) = **43 passed; 0 failed**.

### 2.13 Lift atualizado (`e04bc26`) e `lift-encode`

O adaptador foi atualizado para o Lift `e04bc26` (commit atual da `main`). A
linguagem ganhou enums, unions, generics, closures, `match`, `for-of`,
`try/catch`, `?` e FFI; o RIR do EXP-0 não representa esses recursos. O adaptador
**falha fechado** com `AdapterError::Unsupported` para eles, em vez de gerar um
módulo aproximado.

- Subconjunto antigo: **43 testes** continuam passando contra `e04bc26`.
- Novo binário **`lift-encode`**: `lift <arquivo.lift> <entry> <saida.rsvmbc>`;
  roda lexer/parser/analisador, baixa para RIR, verifica e codifica; não executa.
- Novos testes `tests/encode.rs`: decodificam o `.rsvmbc`, verificam e executam
  na RVM; também compilam os seis exemplos em `examples/lift/`.
- Total atual: **9 CLI + 34 diferencial + 4 encode = 47 passed; 0 failed**.

```bash
cargo run --manifest-path frontends/lift-rir/Cargo.toml --bin lift-encode -- \
    examples/lift/soma.lift soma /tmp/soma.rsvmbc
rsuvm run-bc /tmp/soma.rsvmbc --arg i64:10 --arg i64:20
# i64:30
```

Nota: o checkout local do Lift tem alterações não commitadas em
`lift_ast`/`lift_parser`/`lift_sema` (enums genéricos). O pin do adaptador usa
`e04bc26`, que está publicado; para incorporar essas alterações, é preciso
commit/push no Lift e atualizar o rev do adaptador.

A proposta de suporte a enums/unions no RIR está em
[ADR-0039](../ADRs/ADR-0039-ENUMS-UNIONS-RIR.md) e
[ENUMS-UNIONS-PROPOSTA](../CONTRATOS/ENUMS-UNIONS-PROPOSTA.md). Nada foi
implementado; o adaptador continua rejeitando esses recursos.

## 3. Limitações

1. Subconjunto ainda estreito: **sem** enums, unions, generics, closures/valores
   de função, métodos higher-order de array, `match`, `for-of`, `break`/
   `continue`, `throw`/`try-catch`/`?`, `console.log`/`import`/FFI. Fora disso,
   `AdapterError::Unsupported`. Divisão/resto, `string` (com concatenação),
   `f64` (`number`), arrays (literal, índice e `xs[i] = v`), structs (`interface`,
   literal, `obj.campo` e `obj.campo = v`) e opcionais (`null`,
   `option.some`/`is_some`/`unwrap`, `??`, `== null`/`!= null`), inclusive
   `option` de array/struct, são suportados; a atribuição a elemento/campo exige
   que o alvo seja um local não-aliasado (ADR-0028/0032); o Lift não expressa
   `option` aninhado (o núcleo expressa, ADR-0034).
2. O adaptador é um **workspace separado** e depende do Lift por git fixado; o
   núcleo RSUVM continua sem dependências externas.
3. O teste compara com o oráculo do Lift **registrado**; não invoca o compilador do
   Lift em tempo de teste (evita dependência de rede/rustc no teste).
4. Não há tradução reversa (RIR→Lift) nem cobertura de efeitos de host. `Result`
   existe no núcleo (ADR-0033) mas **não** no adaptador: o subconjunto Lift só
   inspeciona `result.is_ok`/`unwrap`/`unwrap_err`, cuja fonte é um valor de host.

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
