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
fn construto_fora_do_subconjunto_e_rejeitado() {
    let source = "function conta(n: int): int { while n > 0 { return n; } return 0; }";
    let error = lower_source(source, "conta").expect_err("`while` está fora do subconjunto");
    assert!(
        matches!(error, AdapterError::Unsupported(_)),
        "erro inesperado: {error:?}"
    );
}
