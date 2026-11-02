#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! ABI de host e capabilities (ADR-0036): verificação, negação e escopo.

use rvm_interpreter::{execute, execute_with_host};
use rvm_ir::{Block, Function, Instr, Module, Op, Param, Terminator};
use rvm_runtime::{CallDepthLimit, Capabilities, Effect, ExecConfig, Host, Value};
use rvm_spec::{
    builtin_types, BuiltinHost, ErrorClass, ErrorCode, Limits, TypeDef, TypeId, IR_VERSION,
    TYPE_I64, TYPE_STR,
};
use rvm_verify::VerifiedModule;

const RESULT_TY: TypeId = 5;

/// `fn le(path: string, max: i64) -> result string string { return host_read_file(path, max) }`
fn modulo_host() -> Module {
    let mut types = builtin_types();
    types.push(TypeDef::Result(TYPE_STR, TYPE_STR));
    Module {
        name: "host".to_string(),
        version: IR_VERSION,
        entry: 0,
        types,
        structs: Vec::new(),
        functions: vec![Function {
            id: 0,
            name: "le".to_string(),
            params: vec![
                Param {
                    name: "path".to_string(),
                    ty: TYPE_STR,
                },
                Param {
                    name: "max".to_string(),
                    ty: TYPE_I64,
                },
            ],
            ret: RESULT_TY,
            entry: 0,
            blocks: vec![Block {
                id: 0,
                label: "entry".to_string(),
                params: Vec::new(),
                instrs: vec![Instr {
                    def: 2,
                    op: Op::HostCall {
                        function: BuiltinHost::ReadFile,
                        args: vec![0, 1],
                    },
                }],
                term: Terminator::Return(Some(2)),
            }],
        }],
    }
}

fn config() -> ExecConfig {
    ExecConfig::new(10_000, CallDepthLimit::new(16).expect("limite"))
}

/// Host de teste que devolve conteúdo fixo e registra a última chamada.
#[derive(Default)]
struct MockHost {
    capabilities: Capabilities,
    content: String,
    calls: usize,
}

impl Host for MockHost {
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn call(
        &mut self,
        _function: BuiltinHost,
        _args: &[Value],
        ret: TypeId,
    ) -> Result<Value, rvm_spec::RvmError> {
        self.calls += 1;
        Ok(Value::Result {
            ty: ret,
            value: Ok(Box::new(Value::Str(self.content.clone()))),
        })
    }
}

#[test]
fn host_sem_capability_e_negado_antes_do_backend() {
    let verified = VerifiedModule::verify(modulo_host(), &Limits::default()).expect("verifica");
    let mut host = MockHost {
        capabilities: Capabilities::none(),
        content: "conteudo".to_string(),
        calls: 0,
    };
    let args = [Value::Str("/tmp/x".to_string()), Value::I64(100)];
    let error = execute_with_host(&verified, verified.entry(), &args, &config(), &mut host)
        .expect_err("sem concessão");
    assert_eq!(error.code(), ErrorCode::CapabilityDenied);
    assert_eq!(error.class(), ErrorClass::Capability);
    assert_eq!(host.calls, 0, "o backend não pode ser chamado");
}

#[test]
fn host_com_capability_e_escopo_correto_executa() {
    let verified = VerifiedModule::verify(modulo_host(), &Limits::default()).expect("verifica");
    let mut host = MockHost {
        capabilities: Capabilities::grant(Effect::FilesystemRead, "/tmp"),
        content: "conteudo".to_string(),
        calls: 0,
    };
    let args = [Value::Str("/tmp/x".to_string()), Value::I64(100)];
    let outcome = execute_with_host(&verified, verified.entry(), &args, &config(), &mut host)
        .expect("com concessão");
    assert_eq!(
        outcome.value,
        Value::Result {
            ty: RESULT_TY,
            value: Ok(Box::new(Value::Str("conteudo".to_string()))),
        }
    );
    assert_eq!(host.calls, 1);
}

#[test]
fn host_com_escopo_fora_da_concessao_e_negado() {
    let verified = VerifiedModule::verify(modulo_host(), &Limits::default()).expect("verifica");
    let mut host = MockHost {
        capabilities: Capabilities::grant(Effect::FilesystemRead, "/tmp/a"),
        content: "conteudo".to_string(),
        calls: 0,
    };
    let args = [Value::Str("/tmp/b".to_string()), Value::I64(100)];
    let error = execute_with_host(&verified, verified.entry(), &args, &config(), &mut host)
        .expect_err("escopo fora");
    assert_eq!(error.code(), ErrorCode::CapabilityDenied);
    assert_eq!(host.calls, 0);
}

#[test]
fn execute_sem_host_nega_qualquer_chamada() {
    let verified = VerifiedModule::verify(modulo_host(), &Limits::default()).expect("verifica");
    let args = [Value::Str("/tmp/x".to_string()), Value::I64(100)];
    let error = execute(&verified, verified.entry(), &args, &config())
        .expect_err("execute não tem host de efeitos");
    assert_eq!(error.code(), ErrorCode::CapabilityDenied);
    assert_eq!(error.class(), ErrorClass::Capability);
}

#[test]
fn host_com_aridade_errada_e_invalid_type() {
    let mut module = modulo_host();
    let function = module.functions.get_mut(0).expect("função");
    let block = function.blocks.get_mut(0).expect("bloco");
    block.instrs[0] = Instr {
        def: 2,
        op: Op::HostCall {
            function: BuiltinHost::ReadFile,
            args: vec![0],
        },
    };
    let error = VerifiedModule::verify(module, &Limits::default()).expect_err("aridade");
    assert_eq!(error.code(), ErrorCode::InvalidType);
}
