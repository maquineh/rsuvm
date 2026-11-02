#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! Ligação estática inter-módulo (B1, ADR-0018).
//!
//! Exercita o caminho `texto → link → estrutura → verificação → execução` com
//! dois módulos, sendo um deles com `import` de função externa. Também cobre os
//! modos de falha do linker (não resolvido, ambíguo, nome duplicado) e a
//! rejeição, pelo verificador, de um módulo ainda não linkado.

use rvm_ir::Module;
use rvm_runtime::{CallDepthLimit, ExecConfig, Value};
use rvm_spec::{ErrorClass, ErrorCode, Limits};
use rvm_verify::VerifiedModule;

const APP: &str = "module app version 1\nentry main\nimport soma\n\nfn main() -> i64 {\n  entry:\n    v0 = const_i64 10\n    v1 = const_i64 20\n    v2 = call_static soma(v0, v1)\n    return v2\n}\n";

const PROV: &str = "module prov version 1\nentry soma\n\nfn soma(a: i64, b: i64) -> i64 {\n  entry:\n    v2 = add_i64_checked a, b\n    return v2\n}\n";

fn parse_ok(source: &str) -> Module {
    rvm_text::parse(source, &Limits::default()).expect("módulo deveria parsear")
}

fn config() -> ExecConfig {
    ExecConfig::new(1_000, CallDepthLimit::new(16).expect("limite válido"))
}

#[test]
fn chamada_inter_modulo_e_linkada_verificada_e_executada() {
    let limits = Limits::default();
    let modules = vec![parse_ok(APP), parse_ok(PROV)];
    let linked = rvm_link::link(&modules, 0, 0, &limits).expect("ligação deveria resolver `soma`");

    // O módulo plano não tem externas e a chamada aponta para a função do
    // provedor.
    assert!(
        linked.functions.iter().all(|f| !f.is_external()),
        "resultado não pode conter externas"
    );
    let verified = VerifiedModule::verify(linked, &limits).expect("módulo linkado verifica");
    let outcome = rvm_interpreter::execute(&verified, verified.entry(), &[], &config())
        .expect("execução linkada");
    assert_eq!(outcome.value, Value::I64(30));
}

#[test]
fn modulo_linkado_sobrevive_ao_diferencial_de_bytecode() {
    let limits = Limits::default();
    let modules = vec![parse_ok(APP), parse_ok(PROV)];
    let linked = rvm_link::link(&modules, 0, 0, &limits).expect("ligação");
    let entry = linked.entry;

    let bytes = rvm_bytecode::encode(&linked);
    let decoded = rvm_bytecode::decode(&bytes, &limits).expect("decode do módulo linkado");
    let verified = VerifiedModule::verify(decoded, &limits).expect("verificação do bytecode");
    let outcome = rvm_interpreter::execute(&verified, entry, &[], &config()).expect("execução");
    assert_eq!(outcome.value, Value::I64(30));
}

#[test]
fn externa_sem_provedor_falha_com_invalid_module() {
    let limits = Limits::default();
    let error = rvm_link::link(&[parse_ok(APP)], 0, 0, &limits).expect_err("sem provedor");
    assert_eq!(error.code(), ErrorCode::InvalidModule);
    assert_eq!(error.class(), ErrorClass::Module);
}

#[test]
fn provedor_ambiguo_falha_com_invalid_module() {
    let limits = Limits::default();
    let prov2 = PROV.replace("module prov", "module prov2");
    let modules = vec![parse_ok(APP), parse_ok(PROV), parse_ok(&prov2)];
    let error = rvm_link::link(&modules, 0, 0, &limits).expect_err("dois provedores de `soma`");
    assert_eq!(error.code(), ErrorCode::InvalidModule);
}

#[test]
fn nome_de_modulo_duplicado_falha() {
    let limits = Limits::default();
    let modules = vec![parse_ok(APP), parse_ok(APP)];
    let error = rvm_link::link(&modules, 0, 0, &limits).expect_err("nomes iguais");
    assert_eq!(error.code(), ErrorCode::InvalidModule);
}

#[test]
fn verificador_rejeita_modulo_com_externa_nao_linkada() {
    let limits = Limits::default();
    let module = parse_ok(APP);
    let error = VerifiedModule::verify(module, &limits).expect_err("externa não linkada");
    assert_eq!(error.code(), ErrorCode::InvalidModule);
    assert_eq!(error.class(), ErrorClass::Module);
}
