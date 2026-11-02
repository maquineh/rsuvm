#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! `--memory` no CLI: quota de memória modelada.

use std::path::{Path, PathBuf};
use std::process::Command;

fn exemplo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/soma.rvm")
}

fn executar(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rsuvm"))
        .args(args)
        .output()
        .expect("executar rsuvm")
}

fn caminho() -> &'static str {
    Box::leak(exemplo().to_string_lossy().into_owned().into_boxed_str())
}

#[test]
fn memoria_insuficiente_sai_com_codigo_de_recurso() {
    let output = executar(&[
        "run",
        caminho(),
        "--arg",
        "i64:1",
        "--arg",
        "i64:2",
        "--memory",
        "1",
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(4), "stderr: {stderr}");
    assert!(stderr.contains("out_of_memory"), "stderr: {stderr}");
}

#[test]
fn memoria_suficiente_executa() {
    let output = executar(&[
        "run",
        caminho(),
        "--arg",
        "i64:10",
        "--arg",
        "i64:20",
        "--memory",
        "100000",
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "i64:30");
}

#[test]
fn memoria_malformada_e_erro_de_uso() {
    let output = executar(&["run", caminho(), "--memory", "muita"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--memory"));
}
