#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! Testes de propriedades do EXP-0.
//!
//! Cada teste declara o orçamento que usa (semente, número de mutações) e
//! verifica apenas o que executa. "Nenhum pânico observado" descreve a rodada,
//! não ausência de vulnerabilidade (ver `docs/fundacao/TESTES/CONFORMIDADE.md` §6).

use std::fs;
use std::path::{Path, PathBuf};

use rvm_ir::Module;
use rvm_runtime::{CallDepthLimit, Effect, ExecConfig, HostApi, Value};
use rvm_spec::{ErrorClass, ErrorCode, Limits, Type};

fn config(fuel: u64, depth: u16) -> ExecConfig {
    ExecConfig::new(
        fuel,
        CallDepthLimit::new(depth).expect("profundidade de teste válida"),
    )
}

fn verified(module: Module) -> rvm_verify::VerifiedModule {
    rvm_verify::VerifiedModule::verify(module, &Limits::default())
        .expect("módulo de teste deveria verificar")
}

fn corpus_sources(directory: &str) -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join(directory);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("diretório do corpus")
        .map(|entry| entry.expect("entrada").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rvm"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path).expect("leitura do caso");
            (path.display().to_string(), source)
        })
        .collect()
}

#[test]
fn fuel_e_debitado_antes_do_despacho_e_nao_reseta_em_chamada() {
    // `testing::soma()` custa 6 unidades: 4 no chamador + 2 no chamado.
    let module = verified(rvm_ir::testing::soma());
    let entrada = module.entry();
    let sucesso = rvm_interpreter::execute(&module, entrada, &[], &config(6, 8))
        .expect("fuel exato deve bastar");
    assert_eq!(sucesso.value, Value::I64(30));
    assert_eq!(sucesso.fuel_used, 6, "custo contratado de `soma` é 6");

    let erro = rvm_interpreter::execute(&module, entrada, &[], &config(5, 8))
        .expect_err("um fuel a menos precisa falhar");
    assert_eq!(erro.code(), ErrorCode::OutOfFuel);
    assert_eq!(erro.class(), ErrorClass::ResourceLimit);

    let zero = rvm_interpreter::execute(&module, entrada, &[], &config(0, 8))
        .expect_err("fuel 0 precisa falhar");
    assert_eq!(zero.code(), ErrorCode::OutOfFuel);
}

#[test]
fn reexecucao_repetida_nao_compartilha_estado() {
    let module = verified(rvm_ir::testing::soma());
    let entrada = module.entry();
    let primeira = rvm_interpreter::execute(&module, entrada, &[], &config(1000, 8))
        .expect("primeira execução");
    for _ in 0..100 {
        let atual =
            rvm_interpreter::execute(&module, entrada, &[], &config(1000, 8)).expect("reexecução");
        assert_eq!(atual.value, primeira.value);
        assert_eq!(atual.fuel_used, primeira.fuel_used);
        assert_eq!(atual.max_depth, primeira.max_depth);
    }
}

#[test]
fn bytecode_truncado_em_qualquer_prefixo_falha_sem_panico() {
    let limits = Limits::default();
    for module in [
        rvm_ir::testing::soma(),
        rvm_ir::testing::constante(7),
        rvm_ir::testing::com_merge(),
    ] {
        let bytes = rvm_bytecode::encode(&module);
        assert!(!bytes.is_empty(), "encode devolveu vetor vazio");
        for prefix in 0..bytes.len() {
            let truncated = &bytes[..prefix];
            assert!(
                rvm_bytecode::decode(truncated, &limits).is_err(),
                "prefixo de {prefix} bytes deveria ser rejeitado"
            );
        }
        let decoded = rvm_bytecode::decode(&bytes, &limits).expect("módulo completo decodifica");
        assert_eq!(
            decoded, module,
            "decode(encode(m)) deve reproduzir o módulo"
        );
    }
}

#[test]
fn mutacao_de_bytes_nao_causa_panico() {
    // Orçamento declarado: semente fixa, 10 módulos, 200 mutações por módulo,
    // 1..=3 bytes alterados por mutação. Não é prova de ausência de bugs.
    let limits = Limits::default();
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let modules = [
        rvm_ir::testing::soma(),
        rvm_ir::testing::constante(42),
        rvm_ir::testing::desvio_verdadeiro(),
        rvm_ir::testing::com_merge(),
        rvm_ir::testing::unidade(),
        rvm_ir::testing::loop_infinito(),
        rvm_ir::testing::recursao(),
        rvm_ir::testing::overflow(),
        rvm_ir::testing::comparacao(),
    ];
    let mut total = 0_u32;
    for module in modules {
        let original = rvm_bytecode::encode(&module);
        assert!(!original.is_empty());
        for _ in 0..200 {
            let mut mutated = original.clone();
            let changes = (next() % 3) as usize + 1;
            for _ in 0..changes {
                let index = (next() as usize) % mutated.len();
                let byte = mutated[index];
                mutated[index] = byte ^ ((next() % 255) as u8 + 1);
            }
            // O contrato aqui é só "não entra em pânico e termina"; resultado
            // pode ser Ok ou Err.
            let _ = rvm_bytecode::decode(&mutated, &limits);
            total += 1;
        }
    }
    assert_eq!(total, 1800, "orçamento de mutações precisa ser cumprido");
}

#[test]
fn mutacao_textual_nao_causa_panico() {
    // Orçamento: cada caso válido do corpus, 200 mutações de bytes ASCII,
    // 1..=3 posições alteradas. Entradas não-UTF-8 são descartadas.
    let limits = Limits::default();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut attempts = 0_u32;
    for (path, source) in corpus_sources("valid") {
        let original = source.into_bytes();
        assert!(!original.is_empty(), "{path} não deveria estar vazio");
        for _ in 0..200 {
            let mut mutated = original.clone();
            let changes = (next() % 3) as usize + 1;
            for _ in 0..changes {
                let index = (next() as usize) % mutated.len();
                mutated[index] = 0x20 + ((next() % 90) as u8);
            }
            if let Ok(text) = String::from_utf8(mutated) {
                let _ = rvm_text::parse(&text, &limits);
                attempts += 1;
            }
        }
    }
    assert!(attempts > 0, "nenhuma mutação textual foi exercitada");
}

#[test]
fn capabilities_sao_negadas_em_todos_os_efeitos() {
    let host = HostApi::standalone();
    let effects = [
        Effect::FilesystemRead,
        Effect::FilesystemWrite,
        Effect::Network,
        Effect::Environment,
        Effect::Clock,
        Effect::Process,
    ];
    for effect in effects {
        assert!(!host.capabilities().is_granted(effect));
        let error = host
            .request(effect, "escopo-de-teste")
            .expect_err("nenhum efeito pode ser concedido no MVP");
        assert_eq!(error.code(), ErrorCode::CapabilityDenied);
        assert_eq!(error.class(), ErrorClass::Capability);
    }
}

#[test]
fn limite_de_entrada_e_aplicado_antes_do_parse() {
    let limits = Limits {
        max_input_bytes: 32,
        ..Limits::default()
    };
    let source = "module grande version 1\nentry main\nfn main() -> i64 {\n  entry:\n    v0 = const_i64 1\n    return v0\n}\n";
    assert!(source.len() > 32);
    let error = rvm_text::parse(source, &limits).expect_err("entrada acima do limite");
    assert_eq!(error.code(), ErrorCode::InputTooLarge);
    assert_eq!(error.class(), ErrorClass::Module);
}

#[test]
fn unidade_e_comparacao_tem_tipos_estaveis() {
    let unidade = verified(rvm_ir::testing::unidade());
    let outcome =
        rvm_interpreter::execute(&unidade, unidade.entry(), &[], &config(10, 4)).expect("unit");
    assert_eq!(outcome.value, Value::Unit);
    assert_eq!(outcome.value.ty(), Type::Unit);

    let comparacao = verified(rvm_ir::testing::comparacao());
    let outcome = rvm_interpreter::execute(&comparacao, comparacao.entry(), &[], &config(10, 4))
        .expect("bool");
    assert_eq!(outcome.value, Value::Bool(true));
    assert_eq!(outcome.value.ty(), Type::Bool);
}
