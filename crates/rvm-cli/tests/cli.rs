#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! Testes ponta a ponta do binário `rsuvm` (códigos de saída e contrato do CLI
//! em `docs/CONTRATOS/ERROS-EXP-0.md` §5).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rsuvm"))
}

fn caminho_exemplo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/soma.rvm")
}

fn caminho_corpus(relativo: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rvm-conformance/corpus")
        .join(relativo)
}

fn saida(comando: &mut Command) -> Output {
    comando.output().expect("executar binário rsuvm")
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

#[test]
fn run_soma_imprime_valor_e_sai_com_zero() {
    let output = saida(
        bin()
            .arg("run")
            .arg(caminho_exemplo())
            .args(["--arg", "i64:10", "--arg", "i64:20"]),
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert_eq!(texto(&output.stdout).trim(), "i64:30");
    assert!(texto(&output.stderr).is_empty());
}

#[test]
fn check_aceita_modulo_valido() {
    let output = saida(bin().arg("check").arg(caminho_exemplo()));
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stdout).contains("ok:"));
}

#[test]
fn fmt_imprime_forma_canonica() {
    let output = saida(bin().arg("fmt").arg(caminho_exemplo()));
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        texto(&output.stderr)
    );
    let texto_fmt = texto(&output.stdout);
    assert!(
        texto_fmt.contains("module exemplo version 1"),
        "{texto_fmt}"
    );
    assert!(texto_fmt.contains("add_i64_checked"), "{texto_fmt}");
}

#[test]
fn tipo_invalido_sai_com_codigo_de_modulo() {
    let output = saida(
        bin()
            .arg("check")
            .arg(caminho_corpus("invalid/N-001-tipo-de-argumento.rvm")),
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("invalid_type"));
}

#[test]
fn overflow_sai_com_codigo_de_trap() {
    let output = saida(
        bin()
            .arg("run")
            .arg(caminho_corpus("invalid/N-009-overflow.rvm")),
    );
    assert_eq!(
        output.status.code(),
        Some(3),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("integer_overflow"));
}

#[test]
fn fuel_insuficiente_sai_com_codigo_de_recurso() {
    let output = saida(
        bin()
            .arg("run")
            .arg(caminho_exemplo())
            .args(["--arg", "i64:1", "--arg", "i64:2", "--fuel", "1"]),
    );
    assert_eq!(
        output.status.code(),
        Some(4),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("out_of_fuel"));
}

#[test]
fn profundidade_invalida_e_erro_de_uso() {
    for depth in ["0", "999"] {
        let output = saida(bin().arg("run").arg(caminho_exemplo()).args([
            "--arg",
            "i64:1",
            "--arg",
            "i64:2",
            "--max-depth",
            depth,
        ]));
        assert_eq!(output.status.code(), Some(1), "depth {depth}");
        assert!(texto(&output.stderr).contains("max-depth"));
    }
}

#[test]
fn argumento_malformado_e_erro_de_uso() {
    let output = saida(
        bin()
            .arg("run")
            .arg(caminho_exemplo())
            .args(["--arg", "10", "--arg", "20"]),
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("tipo"));
}

#[test]
fn opcao_de_capability_nao_existe() {
    let output = saida(
        bin()
            .arg("run")
            .arg(caminho_exemplo())
            .args(["--allow-filesystem", "true"]),
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("desconhecida"));
}

#[test]
fn arquivo_inexistente_e_erro_de_io() {
    let output = saida(bin().arg("run").arg("nao-existe-rsuvm.rvm"));
    assert_eq!(output.status.code(), Some(1));
    assert!(texto(&output.stderr).contains("não foi possível ler"));
}

#[test]
fn encode_e_run_bc_preservam_o_resultado() {
    let destino = std::env::temp_dir().join(format!("rsuvm-cli-{}-encode.bin", std::process::id()));
    let encode = saida(bin().arg("encode").arg(caminho_exemplo()).arg(&destino));
    assert_eq!(
        encode.status.code(),
        Some(0),
        "stderr: {}",
        texto(&encode.stderr)
    );
    assert!(destino.exists(), "arquivo binário não foi criado");

    let run = saida(
        bin()
            .arg("run-bc")
            .arg(&destino)
            .args(["--arg", "i64:10", "--arg", "i64:20"]),
    );
    let _ = std::fs::remove_file(&destino);
    assert_eq!(run.status.code(), Some(0), "stderr: {}", texto(&run.stderr));
    assert_eq!(texto(&run.stdout).trim(), "i64:30");
}

#[test]
fn run_bc_truncado_sai_com_codigo_de_modulo() {
    let destino =
        std::env::temp_dir().join(format!("rsuvm-cli-{}-truncado.bin", std::process::id()));
    std::fs::write(&destino, b"RSVMB1\x01\x00").expect("escrever arquivo de teste");
    let output = saida(bin().arg("run-bc").arg(&destino));
    let _ = std::fs::remove_file(&destino);
    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("truncated_input"));
}

#[test]
fn version_e_help_saem_com_zero() {
    for flag in ["version", "--help"] {
        let output = saida(bin().arg(flag));
        assert_eq!(output.status.code(), Some(0));
        assert!(!texto(&output.stdout).is_empty());
    }
}
