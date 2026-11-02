//! Testes de integração do intérprete sobre módulos verificados.
//!
//! Assim como os testes unitários dos crates (`cfg_attr(test, allow(...))`),
//! este alvo libera os lints de pânico: asserção é o mecanismo de falha. Todo
//! módulo passa por [`VerifiedModule::verify`] antes de chegar a `execute`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rvm_interpreter::{execute, RunOutcome};
use rvm_ir::testing;
use rvm_ir::{FnId, Module};
use rvm_runtime::{CallDepthLimit, ExecConfig, Value};
use rvm_spec::{ErrorClass, ErrorCode, Limits, Location, RvmError};
use rvm_verify::VerifiedModule;

/// Verifica o módulo de teste com os limites padrão.
fn verificar(module: Module) -> VerifiedModule {
    VerifiedModule::verify(module, &Limits::default()).expect("módulo de teste deveria verificar")
}

/// Limite de profundidade de teste (sempre dentro de `1..=HARD_MAX`).
fn limite(valor: u16) -> CallDepthLimit {
    CallDepthLimit::new(valor).expect("limite de teste deveria ser válido")
}

/// Executa esperando sucesso.
fn executar(
    module: &VerifiedModule,
    entry: FnId,
    args: &[Value],
    config: &ExecConfig,
) -> RunOutcome {
    match execute(module, entry, args, config) {
        Ok(outcome) => outcome,
        Err(error) => panic!("execução deveria ter sucesso, mas falhou com {error}"),
    }
}

/// Executa esperando erro.
fn esperar_erro(
    module: &VerifiedModule,
    entry: FnId,
    args: &[Value],
    config: &ExecConfig,
) -> RvmError {
    match execute(module, entry, args, config) {
        Ok(outcome) => panic!("esperava erro, mas a execução devolveu {outcome:?}"),
        Err(error) => error,
    }
}

#[test]
fn constante_consome_duas_unidades_de_fuel() {
    let module = verificar(testing::constante(42));
    let outcome = executar(&module, 0, &[], &ExecConfig::default());
    assert_eq!(outcome.value, Value::I64(42));
    assert_eq!(outcome.fuel_used, 2, "1 instrução + 1 terminador");
    assert_eq!(outcome.max_depth, 1);
}

#[test]
fn soma_atravessa_chamada_e_soma_fuel() {
    let module = verificar(testing::soma());
    let outcome = executar(&module, 0, &[], &ExecConfig::default());
    assert_eq!(outcome.value, Value::I64(30));
    assert_eq!(outcome.fuel_used, 6, "4 no chamador + 2 no chamado");
    assert_eq!(outcome.max_depth, 2);
}

#[test]
fn comparacao_produz_bool() {
    let module = verificar(testing::comparacao());
    let outcome = executar(&module, 0, &[], &ExecConfig::default());
    assert_eq!(outcome.value, Value::Bool(true));
    assert_eq!(outcome.fuel_used, 4, "3 instruções + 1 terminador");
}

#[test]
fn desvio_verdadeiro_segue_apenas_o_bloco_entao() {
    let module = verificar(testing::desvio_verdadeiro());
    let outcome = executar(&module, 0, &[], &ExecConfig::default());
    assert_eq!(outcome.value, Value::I64(1), "o bloco `senao` não executa");
    assert_eq!(outcome.fuel_used, 4);
    assert_eq!(outcome.max_depth, 1);
}

#[test]
fn merge_copia_argumentos_para_os_parametros_do_bloco() {
    let module = verificar(testing::com_merge());

    let verdadeiro = executar(&module, 0, &[Value::Bool(true)], &ExecConfig::default());
    assert_eq!(verdadeiro.value, Value::I64(1));
    assert_eq!(verdadeiro.fuel_used, 5);

    let falso = executar(&module, 0, &[Value::Bool(false)], &ExecConfig::default());
    assert_eq!(falso.value, Value::I64(2));
    assert_eq!(falso.fuel_used, 5);
}

#[test]
fn unidade_retorna_unit_com_fuel_um() {
    let module = verificar(testing::unidade());
    let outcome = executar(&module, 0, &[], &ExecConfig::default());
    assert_eq!(outcome.value, Value::Unit);
    assert_eq!(outcome.fuel_used, 1, "apenas o terminador");
    assert_eq!(outcome.max_depth, 1);
}

#[test]
fn overflow_e_trap_com_localizacao_da_instrucao() {
    let module = verificar(testing::overflow());
    let error = esperar_erro(&module, 0, &[], &ExecConfig::default());
    assert_eq!(error.code(), ErrorCode::IntegerOverflow);
    assert_eq!(error.class(), ErrorClass::Trap);
    assert_eq!(
        error.location(),
        Some(&Location::Instruction {
            function: "main".to_string(),
            block: "entry".to_string(),
            index: 2,
        })
    );
}

#[test]
fn loop_infinito_esgota_fuel() {
    let module = verificar(testing::loop_infinito());
    let config = ExecConfig::new(1000, limite(64));
    let error = esperar_erro(&module, 0, &[], &config);
    assert_eq!(error.code(), ErrorCode::OutOfFuel);
    assert_eq!(error.class(), ErrorClass::ResourceLimit);
    assert!(
        matches!(error.location(), Some(Location::Block { .. })),
        "o esgotamento ocorre no terminador, dentro de um bloco"
    );
}

#[test]
fn recursao_excede_profundidade_e_fuel_cresce_por_frame() {
    let module = verificar(testing::recursao());
    let error = esperar_erro(&module, 0, &[], &ExecConfig::new(1000, limite(8)));
    assert_eq!(error.code(), ErrorCode::CallDepthExceeded);
    assert_eq!(error.class(), ErrorClass::ResourceLimit);
    assert_eq!(
        error.location(),
        Some(&Location::Instruction {
            function: "main".to_string(),
            block: "entry".to_string(),
            index: 0,
        }),
        "a falha de profundidade é atribuída à chamada que a excederia"
    );

    // Cada frame recursivo consome exatamente 1 unidade de fuel antes de tentar
    // a próxima chamada: com 7 o fuel acaba antes do limite de 8; com 8 o limite
    // é alcançado. Isso fixa a monotonicidade do consumo por frame.
    assert_eq!(
        esperar_erro(&module, 0, &[], &ExecConfig::new(7, limite(8))).code(),
        ErrorCode::OutOfFuel
    );
    assert_eq!(
        esperar_erro(&module, 0, &[], &ExecConfig::new(8, limite(8))).code(),
        ErrorCode::CallDepthExceeded
    );

    // No teto absoluto (HARD_MAX = 256) a profundidade limita antes de 1000
    // unidades de fuel; o mesmo limiar aparece em 255/256.
    let teto = limite(CallDepthLimit::HARD_MAX);
    assert_eq!(
        esperar_erro(&module, 0, &[], &ExecConfig::new(255, teto)).code(),
        ErrorCode::OutOfFuel
    );
    assert_eq!(
        esperar_erro(&module, 0, &[], &ExecConfig::new(256, teto)).code(),
        ErrorCode::CallDepthExceeded
    );
    assert_eq!(
        esperar_erro(&module, 0, &[], &ExecConfig::new(1000, teto)).code(),
        ErrorCode::CallDepthExceeded
    );
}

#[test]
fn argumentos_invalidos_sao_rejeitados_antes_do_fuel() {
    let constante = verificar(testing::constante(1));
    // Fuel 0: se a validação do host ocorresse depois do débito, o código seria
    // `out_of_fuel`; ele precisa ser `invalid_type`.
    let sem_fuel = ExecConfig::new(0, limite(8));
    let aridade = esperar_erro(&constante, 0, &[Value::I64(7)], &sem_fuel);
    assert_eq!(aridade.code(), ErrorCode::InvalidType);
    assert_eq!(aridade.class(), ErrorClass::Module);

    let soma = verificar(testing::soma());
    let tipo = esperar_erro(
        &soma,
        1,
        &[Value::I64(1), Value::Bool(true)],
        &ExecConfig::default(),
    );
    assert_eq!(tipo.code(), ErrorCode::InvalidType);
    assert_eq!(tipo.class(), ErrorClass::Module);
}

#[test]
fn execucao_repetida_e_deterministica() {
    let module = verificar(testing::soma());
    let referencia = executar(&module, 0, &[], &ExecConfig::default());
    for _ in 0..100 {
        let outcome = executar(&module, 0, &[], &ExecConfig::default());
        assert_eq!(outcome, referencia);
    }
}

#[test]
fn fuel_zero_falha_na_primeira_instrucao() {
    let module = verificar(testing::constante(42));
    let config = ExecConfig::new(0, limite(8));
    let error = esperar_erro(&module, 0, &[], &config);
    assert_eq!(error.code(), ErrorCode::OutOfFuel);
    assert_eq!(error.class(), ErrorClass::ResourceLimit);
    assert_eq!(
        error.location(),
        Some(&Location::Instruction {
            function: "main".to_string(),
            block: "entry".to_string(),
            index: 0,
        }),
        "saldo 0 falha antes de despachar a instrução 0"
    );
}

#[test]
fn fuel_zero_falha_no_terminador_de_funcao_sem_instrucoes() {
    let module = verificar(testing::unidade());
    let config = ExecConfig::new(0, limite(8));
    let error = esperar_erro(&module, 0, &[], &config);
    assert_eq!(error.code(), ErrorCode::OutOfFuel);
    assert_eq!(error.class(), ErrorClass::ResourceLimit);
    assert_eq!(
        error.location(),
        Some(&Location::Block {
            function: "main".to_string(),
            block: "entry".to_string(),
        })
    );
}

#[test]
fn entrada_inexistente_e_invalid_module() {
    let module = verificar(testing::constante(0));
    let error = esperar_erro(&module, 99, &[], &ExecConfig::default());
    assert_eq!(error.code(), ErrorCode::InvalidModule);
    assert_eq!(error.class(), ErrorClass::Module);
    assert_eq!(error.location(), Some(&Location::Module));
}
