#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! Fronteiras da quota de memória modelada (ADR-0013).

use rvm_interpreter::execute;
use rvm_ir::testing;
use rvm_ir::Module;
use rvm_runtime::{frame_cost, CallDepthLimit, ExecConfig, MemoryLimit, Value};
use rvm_spec::{ErrorClass, ErrorCode, Limits};
use rvm_verify::VerifiedModule;

fn verificar(module: Module) -> VerifiedModule {
    VerifiedModule::verify(module, &Limits::default()).expect("módulo deveria verificar")
}

fn config(memory: u64) -> ExecConfig {
    ExecConfig::new(10_000, CallDepthLimit::new(256).expect("limite válido"))
        .with_memory(MemoryLimit::new(memory))
}

#[test]
fn frame_exato_cabe_e_registra_pico() {
    let module = verificar(testing::constante(42));
    let custo = frame_cost(1); // uma definição, sem parâmetros
    let outcome = execute(&module, module.entry(), &[], &config(custo)).expect("cabe");
    assert_eq!(outcome.value, Value::I64(42));
    assert_eq!(outcome.memory_peak, custo);
}

#[test]
fn um_byte_menos_falha_antes_de_alocar() {
    let module = verificar(testing::constante(42));
    let custo = frame_cost(1);
    let error = execute(&module, module.entry(), &[], &config(custo - 1)).expect_err("não cabe");
    assert_eq!(error.code(), ErrorCode::OutOfMemory);
    assert_eq!(error.class(), ErrorClass::ResourceLimit);
    assert!(error.location().is_some(), "erro precisa apontar a função");
}

#[test]
fn soma_ocupa_dois_frames_simultaneos() {
    let module = verificar(testing::soma());
    // main: 3 valores; soma: 2 parâmetros + 1 definição.
    let total = frame_cost(3) + frame_cost(3);
    let outcome = execute(&module, module.entry(), &[], &config(total)).expect("cabe");
    assert_eq!(outcome.value, Value::I64(30));
    assert_eq!(outcome.memory_peak, total);
    let error = execute(&module, module.entry(), &[], &config(total - 1)).expect_err("não cabe");
    assert_eq!(error.code(), ErrorCode::OutOfMemory);
}

#[test]
fn memoria_zero_falha_antes_da_primeira_instrucao() {
    let module = verificar(testing::constante(1));
    let error = execute(&module, module.entry(), &[], &config(0)).expect_err("zero memória");
    assert_eq!(
        error.code(),
        ErrorCode::OutOfMemory,
        "não pode ser out_of_fuel"
    );
}
