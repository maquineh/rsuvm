//! Teste diferencial do adaptador Lift→RIR (ADR-0021).
//!
//! Cada caso parte de **código-fonte Lift real**, roda o pipeline do Lift
//! (`tokenize → parse → analyze`), baixa a HIR para a RIR, verifica e executa na
//! RSUVM. O valor esperado é o **oráculo do Lift** para o mesmo programa
//! (registrado em `docs/RELATORIOS/ADAPTADOR-LIFT-RIR.md`).
//!
//! O teste roda no workspace separado do adaptador; não faz parte do
//! `cargo test --workspace` do núcleo.

use lift_rir::{lower_source, AdapterError};
use rvm_runtime::{CallDepthLimit, ExecConfig, Value};
use rvm_spec::Limits;

/// Roda Lift→RIR→verificação→execução e devolve o valor.
fn executar(source: &str, entry: &str, args: &[Value]) -> Value {
    let module = lower_source(source, entry).expect("lowering Lift→RIR");
    let verified = rvm_verify::VerifiedModule::verify(module, &Limits::default())
        .expect("módulo gerado deveria verificar");
    let config = ExecConfig::new(1_000_000, CallDepthLimit::new(64).expect("limite válido"));
    rvm_interpreter::execute(&verified, verified.entry(), args, &config)
        .expect("execução do módulo gerado")
        .value
}

#[test]
fn soma_de_inteiros() {
    let source = "function soma(a: int, b: int): int { return a + b; }";
    assert_eq!(
        executar(source, "soma", &[Value::I64(10), Value::I64(20)]),
        Value::I64(30)
    );
}

#[test]
fn if_else_terminal_com_negacao() {
    let source = "function absoluto(a: int): int { if a < 0 { return -a; } else { return a; } }";
    assert_eq!(
        executar(source, "absoluto", &[Value::I64(-5)]),
        Value::I64(5)
    );
    assert_eq!(
        executar(source, "absoluto", &[Value::I64(7)]),
        Value::I64(7)
    );
}

#[test]
fn if_else_escolhe_maior() {
    let source =
        "function maior(a: int, b: int): int { if a > b { return a; } else { return b; } }";
    assert_eq!(
        executar(source, "maior", &[Value::I64(3), Value::I64(9)]),
        Value::I64(9)
    );
    assert_eq!(
        executar(source, "maior", &[Value::I64(3), Value::I64(-9)]),
        Value::I64(3)
    );
}

#[test]
fn comparacao_retorna_boolean() {
    let source = "function positivo(a: int): boolean { return a > 0; }";
    assert_eq!(
        executar(source, "positivo", &[Value::I64(5)]),
        Value::Bool(true)
    );
    assert_eq!(
        executar(source, "positivo", &[Value::I64(-5)]),
        Value::Bool(false)
    );
}

#[test]
fn let_e_multiplicacao() {
    let source = "function quadrado(a: int): int { let x = a + 1; return x * x; }";
    assert_eq!(
        executar(source, "quadrado", &[Value::I64(4)]),
        Value::I64(25)
    );
}

#[test]
fn chamadas_encadeadas_entre_funcoes() {
    let source = "function dobro(a: int): int { return a * 2; } function quadruplo(a: int): int { return dobro(dobro(a)); }";
    assert_eq!(
        executar(source, "quadruplo", &[Value::I64(3)]),
        Value::I64(12)
    );
}

#[test]
fn recursao_com_if_terminal_fibonacci() {
    let source = "function fib(n: int): int { if n <= 1 { return n; } else { return fib(n - 1) + fib(n - 2); } }";
    assert_eq!(executar(source, "fib", &[Value::I64(10)]), Value::I64(55));
    assert_eq!(executar(source, "fib", &[Value::I64(0)]), Value::I64(0));
}

#[test]
fn while_com_atribuicao_soma_ate_n() {
    let source = "function soma_ate(n: int): int { let acc = 0; let i = 1; while i <= n { acc = acc + i; i = i + 1; } return acc; }";
    assert_eq!(
        executar(source, "soma_ate", &[Value::I64(5)]),
        Value::I64(15)
    );
    assert_eq!(
        executar(source, "soma_ate", &[Value::I64(0)]),
        Value::I64(0)
    );
}

#[test]
fn while_conta_ate_n() {
    let source = "function conta(n: int): int { let i = 0; while i < n { i = i + 1; } return i; }";
    assert_eq!(executar(source, "conta", &[Value::I64(7)]), Value::I64(7));
    assert_eq!(executar(source, "conta", &[Value::I64(0)]), Value::I64(0));
}

#[test]
fn if_com_merge_de_variavel() {
    let source = "function classifica(a: int): int { let r = 0; if a > 0 { r = 1; } else { r = 2; } return r; }";
    assert_eq!(
        executar(source, "classifica", &[Value::I64(5)]),
        Value::I64(1)
    );
    assert_eq!(
        executar(source, "classifica", &[Value::I64(-5)]),
        Value::I64(2)
    );
}

#[test]
fn if_sem_else_usa_valor_anterior() {
    let source = "function limita(a: int): int { let r = a; if a > 10 { r = 10; } return r; }";
    assert_eq!(
        executar(source, "limita", &[Value::I64(20)]),
        Value::I64(10)
    );
    assert_eq!(executar(source, "limita", &[Value::I64(3)]), Value::I64(3));
}

#[test]
fn while_aninhado_em_if() {
    let source = "function soma_condicional(n: int, liga: boolean): int { let acc = 0; if liga { let i = 0; while i < n { acc = acc + i; i = i + 1; } } return acc; }";
    assert_eq!(
        executar(
            source,
            "soma_condicional",
            &[Value::I64(4), Value::Bool(true)]
        ),
        Value::I64(6)
    );
    assert_eq!(
        executar(
            source,
            "soma_condicional",
            &[Value::I64(4), Value::Bool(false)]
        ),
        Value::I64(0)
    );
}

#[test]
fn divisao_e_resto() {
    let source = "function divide(a: int, b: int): int { return a / b; } function resto(a: int, b: int): int { return a % b; }";
    assert_eq!(
        executar(source, "divide", &[Value::I64(20), Value::I64(6)]),
        Value::I64(3)
    );
    assert_eq!(
        executar(source, "divide", &[Value::I64(-20), Value::I64(6)]),
        Value::I64(-3)
    );
    assert_eq!(
        executar(source, "resto", &[Value::I64(20), Value::I64(6)]),
        Value::I64(2)
    );
    assert_eq!(
        executar(source, "resto", &[Value::I64(-20), Value::I64(6)]),
        Value::I64(-2)
    );
}

#[test]
fn else_if_encadeado() {
    let source = "function faixa(n: int): int { if n < 0 { return -1; } else if n == 0 { return 0; } else { return 1; } }";
    assert_eq!(executar(source, "faixa", &[Value::I64(-5)]), Value::I64(-1));
    assert_eq!(executar(source, "faixa", &[Value::I64(0)]), Value::I64(0));
    assert_eq!(executar(source, "faixa", &[Value::I64(5)]), Value::I64(1));
}

#[test]
fn logica_and_or_not() {
    let source = "function ambos(a: boolean, b: boolean): boolean { return a && b; } function algum(a: boolean, b: boolean): boolean { return a || b; } function nega(a: boolean): boolean { return !a; }";
    assert_eq!(
        executar(source, "ambos", &[Value::Bool(true), Value::Bool(true)]),
        Value::Bool(true)
    );
    assert_eq!(
        executar(source, "ambos", &[Value::Bool(true), Value::Bool(false)]),
        Value::Bool(false)
    );
    assert_eq!(
        executar(source, "algum", &[Value::Bool(false), Value::Bool(false)]),
        Value::Bool(false)
    );
    assert_eq!(
        executar(source, "algum", &[Value::Bool(false), Value::Bool(true)]),
        Value::Bool(true)
    );
    assert_eq!(
        executar(source, "nega", &[Value::Bool(true)]),
        Value::Bool(false)
    );
    assert_eq!(
        executar(source, "nega", &[Value::Bool(false)]),
        Value::Bool(true)
    );
}

#[test]
fn and_tem_curto_circuito_sobre_divisao() {
    // Se `&&` não fosse curto-circuito, `a / b` com b = 0 trapava.
    let source = "function seguro(a: int, b: int): boolean { return b != 0 && (a / b) > 0; }";
    assert_eq!(
        executar(source, "seguro", &[Value::I64(4), Value::I64(0)]),
        Value::Bool(false)
    );
    assert_eq!(
        executar(source, "seguro", &[Value::I64(4), Value::I64(2)]),
        Value::Bool(true)
    );
}

#[test]
fn or_tem_curto_circuito_sobre_divisao() {
    let source = "function seguro(a: int, b: int): boolean { return b == 0 || (a / b) > 0; }";
    assert_eq!(
        executar(source, "seguro", &[Value::I64(4), Value::I64(0)]),
        Value::Bool(true)
    );
    assert_eq!(
        executar(source, "seguro", &[Value::I64(4), Value::I64(2)]),
        Value::Bool(true)
    );
}

#[test]
fn string_literal_e_igualdade() {
    let source = "function saudacao(): string { return \"ola\"; } function igual(a: string, b: string): boolean { return a == b; } function diferente(a: string, b: string): boolean { return a != b; }";
    assert_eq!(
        executar(source, "saudacao", &[]),
        Value::Str("ola".to_string())
    );
    assert_eq!(
        executar(
            source,
            "igual",
            &[Value::Str("x".into()), Value::Str("x".into())]
        ),
        Value::Bool(true)
    );
    assert_eq!(
        executar(
            source,
            "igual",
            &[Value::Str("x".into()), Value::Str("y".into())]
        ),
        Value::Bool(false)
    );
    assert_eq!(
        executar(
            source,
            "diferente",
            &[Value::Str("x".into()), Value::Str("y".into())]
        ),
        Value::Bool(true)
    );
}

#[test]
fn concatenacao_de_strings() {
    let source = "function junta(a: string, b: string): string { return a + b; }";
    assert_eq!(
        executar(
            source,
            "junta",
            &[Value::Str("ola".into()), Value::Str(" mundo".into())]
        ),
        Value::Str("ola mundo".to_string())
    );
}

#[test]
fn f64_aritmetica_e_comparacoes() {
    let source = "function media(a: number, b: number): number { return (a + b) / 2.0; } function menor(a: number, b: number): boolean { return a < b; } function nega(a: number): number { return -a; }";
    assert_eq!(
        executar(source, "media", &[Value::F64(1.0), Value::F64(2.0)]),
        Value::F64(1.5)
    );
    assert_eq!(
        executar(source, "menor", &[Value::F64(1.0), Value::F64(2.0)]),
        Value::Bool(true)
    );
    assert_eq!(
        executar(source, "menor", &[Value::F64(2.0), Value::F64(1.0)]),
        Value::Bool(false)
    );
    assert_eq!(
        executar(source, "nega", &[Value::F64(2.5)]),
        Value::F64(-2.5)
    );
}

#[test]
fn arrays_literal_e_indice() {
    let source = "function segundo(): int { const xs: int[] = [10, 20, 30]; return xs[1]; } function tamanho(): int { const xs: int[] = [1, 2, 3, 4]; return 0; }";
    assert_eq!(executar(source, "segundo", &[]), Value::I64(20));
    assert_eq!(executar(source, "tamanho", &[]), Value::I64(0));
}

#[test]
fn array_set_em_local_nao_aliasado() {
    // Oráculo do Lift (`rsuvm-oracle9.lift`): `9`, `14`, `6`.
    let troca = "function troca(): int { let xs: int[] = [1, 2, 3]; xs[0] = 9; return xs[0]; }";
    assert_eq!(executar(troca, "troca", &[]), Value::I64(9));

    let soma = "function soma(): int { let xs: int[] = [1, 2, 3]; xs[1] = 10; return xs[0] + xs[1] + xs[2]; }";
    assert_eq!(executar(soma, "soma", &[]), Value::I64(14));

    let acumula = "function acumula(): int { let xs: int[] = [0, 0, 0]; let i = 0; while i < 3 { xs[i] = i * 2; i = i + 1; } return xs[0] + xs[1] + xs[2]; }";
    assert_eq!(executar(acumula, "acumula", &[]), Value::I64(6));
}

#[test]
fn array_set_aliasado_e_rejeitado() {
    // `ys` recebe `xs`; mutar `ys` mudaria `xs` no Lift (referência), mas não na
    // RIR (valor). O adaptador recusa em vez de gerar módulo incorreto.
    let source =
        "function f(): int { let xs: int[] = [1, 2, 3]; let ys: int[] = xs; ys[0] = 9; return ys[0]; }";
    let error = lower_source(source, "f").expect_err("array aliasado");
    assert!(
        matches!(error, AdapterError::Unsupported(_)),
        "erro inesperado: {error:?}"
    );
}

#[test]
fn array_set_fora_dos_limites_trapa() {
    let source = "function f(): int { let xs: int[] = [1, 2, 3]; xs[5] = 9; return xs[0]; }";
    let module = lower_source(source, "f").expect("lowering Lift→RIR");
    let verified = rvm_verify::VerifiedModule::verify(module, &Limits::default())
        .expect("módulo gerado deveria verificar");
    let config = ExecConfig::new(1_000_000, CallDepthLimit::new(64).expect("limite válido"));
    let error = rvm_interpreter::execute(&verified, verified.entry(), &[], &config)
        .expect_err("índice fora dos limites deveria trapar");
    assert_eq!(error.code(), rvm_spec::ErrorCode::IndexOutOfBounds);
}

#[test]
fn struct_literal_e_acesso() {
    // Oráculo do Lift (`rsuvm-oracle10.lift`): `7`, `ana`, `42`.
    let source = "interface Ponto { x: int, y: int } \
        interface Usuario { nome: string, ativo: boolean } \
        interface Caixa { valor: int } \
        function soma(): int { const p: Ponto = { x: 3, y: 4 }; return p.x + p.y; } \
        function nome(): string { const u: Usuario = { nome: \"ana\", ativo: true }; return u.nome; } \
        function soma_caixa(c: Caixa): int { return c.valor + 1; } \
        function usa(): int { const c: Caixa = { valor: 41 }; return soma_caixa(c); }";
    assert_eq!(executar(source, "soma", &[]), Value::I64(7));
    assert_eq!(executar(source, "nome", &[]), Value::Str("ana".to_string()));
    assert_eq!(executar(source, "usa", &[]), Value::I64(42));
}

#[test]
fn struct_com_campo_nao_escalar_e_rejeitado() {
    let source = "interface Bag { xs: int[] } function f(): int { return 0; }";
    let error = lower_source(source, "f").expect_err("campo não escalar");
    assert!(
        matches!(error, AdapterError::Unsupported(_)),
        "erro inesperado: {error:?}"
    );
}

#[test]
fn option_some_none_coalesce_e_builtins() {
    // Oráculo do Lift (`rsuvm-oracle11.lift`): `hi missing 7 9 5 true false`.
    let source = "function resolve(value: string?): string { return value ?? \"missing\"; } \
        function maybe(present: boolean): int? { if present { return option.some(7); } return null; } \
        function teste(present: boolean): int { return maybe(present) ?? 9; } \
        function some_val(): int { const x: int? = option.some(5); return option.unwrap(x); } \
        function is_some_val(): boolean { const x: int? = option.some(5); return option.is_some(x); } \
        function is_none_val(): boolean { const x: int? = null; return option.is_some(x); }";
    assert_eq!(
        executar(
            source,
            "resolve",
            &[Value::Option {
                elem: rvm_spec::OptionalElem::Str,
                value: Some(Box::new(Value::Str("hi".to_string()))),
            }]
        ),
        Value::Str("hi".to_string())
    );
    assert_eq!(
        executar(
            source,
            "resolve",
            &[Value::Option {
                elem: rvm_spec::OptionalElem::Str,
                value: None,
            }]
        ),
        Value::Str("missing".to_string())
    );
    assert_eq!(
        executar(source, "teste", &[Value::Bool(true)]),
        Value::I64(7)
    );
    assert_eq!(
        executar(source, "teste", &[Value::Bool(false)]),
        Value::I64(9)
    );
    assert_eq!(executar(source, "some_val", &[]), Value::I64(5));
    assert_eq!(executar(source, "is_some_val", &[]), Value::Bool(true));
    assert_eq!(executar(source, "is_none_val", &[]), Value::Bool(false));
}

#[test]
fn option_fallback_preguicoso() {
    // Oráculo do Lift (`rsuvm-oracle12.lift`): `5` — `1 / 0` não é avaliado
    // porque o `option` é `some`.
    let source = "function seguro(): int { return option.some(5) ?? (1 / 0); }";
    assert_eq!(executar(source, "seguro", &[]), Value::I64(5));
}

#[test]
fn option_fallback_avaliado_quando_none_trapa() {
    let source = "function f(): int { const x: int? = null; return x ?? (1 / 0); }";
    let module = lower_source(source, "f").expect("lowering Lift→RIR");
    let verified = rvm_verify::VerifiedModule::verify(module, &Limits::default())
        .expect("módulo gerado deveria verificar");
    let config = ExecConfig::new(1_000_000, CallDepthLimit::new(64).expect("limite válido"));
    let error = rvm_interpreter::execute(&verified, verified.entry(), &[], &config)
        .expect_err("fallback avaliado deveria trapar");
    assert_eq!(error.code(), rvm_spec::ErrorCode::DivisionByZero);
}

#[test]
fn comparacao_com_null_e_rejeitada() {
    let source = "function f(): boolean { const x: int? = null; return x == null; }";
    let error = lower_source(source, "f").expect_err("== null está fora do subconjunto");
    assert!(
        matches!(error, AdapterError::Unsupported(_)),
        "erro inesperado: {error:?}"
    );
}

#[test]
fn construto_fora_do_subconjunto_e_rejeitado() {
    // `console.log` (efeito de host) está fora do subconjunto.
    let source = "function f(): int { console.log(\"x\"); return 0; }";
    let error = lower_source(source, "f").expect_err("console.log está fora do subconjunto");
    assert!(
        matches!(error, AdapterError::Unsupported(_)),
        "erro inesperado: {error:?}"
    );
}
