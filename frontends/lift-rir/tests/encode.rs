//! Teste do binário `lift-encode`: Lift → RIR → verificação → bytecode RVM.
//!
//! O bytecode emitido é decodificado, verificado e executado pelo pipeline RVM,
//! provando que o artefato é consumível pelo `rsuvm run-bc`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rvm_runtime::{CallDepthLimit, ExecConfig, Value};
use rvm_spec::Limits;

const BIN: &str = env!("CARGO_BIN_EXE_lift-encode");

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lift-encode-{}-{name}", std::process::id()))
}

#[test]
fn gera_bytecode_executavel_na_rvm() {
    let source = temp_path("soma.lift");
    let output = temp_path("soma.rsvmbc");
    fs::write(
        &source,
        "function soma(a: int, b: int): int { return a + b; }",
    )
    .expect("escrever fonte Lift");
    let result = Command::new(BIN)
        .arg(&source)
        .arg("soma")
        .arg(&output)
        .output()
        .expect("executar lift-encode");
    assert_eq!(
        result.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("bytes em"));
    let bytes = fs::read(&output).expect("ler bytecode");
    let limits = Limits::default();
    let module = rvm_bytecode::decode(&bytes, &limits).expect("decodificar bytecode");
    let verified = rvm_verify::VerifiedModule::verify(module, &limits).expect("verificar bytecode");
    let config = ExecConfig::new(10_000, CallDepthLimit::default());
    let outcome = rvm_interpreter::execute(
        &verified,
        verified.entry(),
        &[Value::I64(10), Value::I64(20)],
        &config,
    )
    .expect("executar bytecode");
    assert_eq!(outcome.value, Value::I64(30));
    let _ = fs::remove_file(source);
    let _ = fs::remove_file(output);
}

#[test]
fn sem_argumentos_e_erro_de_uso() {
    let result = Command::new(BIN)
        .output()
        .expect("executar lift-encode sem argumentos");
    assert_eq!(result.status.code(), Some(1));
}

#[test]
fn recurso_fora_do_subconjunto_falha_fechado() {
    let source = temp_path("enum.lift");
    let output = temp_path("enum.rsvmbc");
    fs::write(
        &source,
        "enum Direction { North, East }\nfunction f(): int { return 1; }",
    )
    .expect("escrever fonte Lift com enum");
    let result = Command::new(BIN)
        .arg(&source)
        .arg("f")
        .arg(&output)
        .output()
        .expect("executar lift-encode");
    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&output);
    assert_eq!(result.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("fora do subconjunto"),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn encode_example(stem: &str) -> Vec<u8> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/lift")
        .join(format!("{stem}.lift"));
    let output = temp_path(&format!("{stem}-example.rsvmbc"));
    let result = Command::new(BIN)
        .arg(&source)
        .arg(stem)
        .arg(&output)
        .output()
        .expect("executar lift-encode");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}: {}",
        source.display(),
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = fs::read(&output).expect("ler bytecode do exemplo");
    let _ = fs::remove_file(&output);
    bytes
}

#[test]
fn examples_lift_executam_na_rvm() {
    let limits = Limits::default();
    let config = ExecConfig::new(10_000, CallDepthLimit::default());
    let cases: [(&str, &[Value], Value); 6] = [
        ("soma", &[Value::I64(10), Value::I64(20)], Value::I64(30)),
        ("fib", &[Value::I64(10)], Value::I64(55)),
        ("segundo", &[], Value::I64(20)),
        ("soma_ponto", &[], Value::I64(7)),
        ("resolve", &[], Value::I64(42)),
        ("saudacao", &[], Value::Str("ola".to_string())),
    ];
    for (stem, args, expected) in cases {
        let bytes = encode_example(stem);
        let module = rvm_bytecode::decode(&bytes, &limits)
            .unwrap_or_else(|error| panic!("{stem}: decode falhou: {error}"));
        let verified = rvm_verify::VerifiedModule::verify(module, &limits)
            .unwrap_or_else(|error| panic!("{stem}: verify falhou: {error}"));
        let outcome = rvm_interpreter::execute(&verified, verified.entry(), args, &config)
            .unwrap_or_else(|error| panic!("{stem}: execute falhou: {error}"));
        assert_eq!(outcome.value, expected, "{stem}");
    }
}
