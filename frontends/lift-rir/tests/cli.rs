//! Teste de CLI do binário `lift-run` (ADR-0021).
//!
//! Roda o binário de verdade sobre arquivos `.lift` temporários e confere a
//! saída e o código de exit, espelhando o contrato do `rsuvm`
//! (`docs/CONTRATOS/ERROS-EXP-0.md`): 0 ok, 2 módulo, 3 trap, 4 limite.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_lift-run");

fn temp_lift(name: &str, source: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("lift-run-{}-{name}.lift", std::process::id()));
    fs::write(&path, source).expect("escrever arquivo .lift");
    path
}

struct Run {
    code: Option<i32>,
    stdout: String,
}

fn run(args: &[&str]) -> Run {
    let output = Command::new(BIN)
        .args(args)
        .output()
        .expect("executar lift-run");
    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
    }
}

#[test]
fn executa_fibonacci_e_imprime_o_valor() {
    let file = temp_lift(
        "fib",
        "function fib(n: int): int { if n <= 1 { return n; } else { return fib(n - 1) + fib(n - 2); } }",
    );
    let result = run(&[file.to_str().unwrap(), "fib", "--arg", "i64:10"]);
    assert_eq!(result.code, Some(0));
    assert_eq!(result.stdout.trim(), "i64:55");
    let _ = fs::remove_file(file);
}

#[test]
fn comparacao_imprime_boolean() {
    let file = temp_lift(
        "positivo",
        "function positivo(a: int): boolean { return a > 0; }",
    );
    let result = run(&[file.to_str().unwrap(), "positivo", "--arg", "i64:-3"]);
    assert_eq!(result.code, Some(0));
    assert_eq!(result.stdout.trim(), "bool:false");
    let _ = fs::remove_file(file);
}

#[test]
fn construto_fora_do_subconjunto_sai_com_codigo_de_modulo() {
    let file = temp_lift(
        "while",
        "function conta(n: int): int { while n > 0 { return n; } return 0; }",
    );
    let result = run(&[file.to_str().unwrap(), "conta", "--arg", "i64:3"]);
    assert_eq!(result.code, Some(2), "esperava exit 2 (módulo)");
    let _ = fs::remove_file(file);
}

#[test]
fn overflow_sai_com_codigo_de_trap() {
    let file = temp_lift(
        "soma",
        "function soma(a: int, b: int): int { return a + b; }",
    );
    let result = run(&[
        file.to_str().unwrap(),
        "soma",
        "--arg",
        "i64:9223372036854775807",
        "--arg",
        "i64:1",
    ]);
    assert_eq!(result.code, Some(3), "esperava exit 3 (trap)");
    let _ = fs::remove_file(file);
}

#[test]
fn fuel_insuficiente_sai_com_codigo_de_limite() {
    let file = temp_lift(
        "fib2",
        "function fib(n: int): int { if n <= 1 { return n; } else { return fib(n - 1) + fib(n - 2); } }",
    );
    let result = run(&[
        file.to_str().unwrap(),
        "fib",
        "--arg",
        "i64:10",
        "--fuel",
        "1",
    ]);
    assert_eq!(result.code, Some(4), "esperava exit 4 (limite)");
    let _ = fs::remove_file(file);
}
