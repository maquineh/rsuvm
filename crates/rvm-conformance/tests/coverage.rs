#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! Meta de cobertura do corpus por operação (item 3 do §12).
//!
//! O corpus é **dado**; este alvo verifica que cada operação do subset EXP-0
//! (`docs/CONTRATOS/CORPUS-COBERTURA.md`) aparece em pelo menos um caso válido e
//! que cada código de erro central aparece em pelo menos um caso inválido. Não
//! mede cobertura de código (linhas/branches): mede presença de operação nos
//! casos declarados. Uma operação ausente é lacuna declarada, não sucesso.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use rvm_ir::{Module, Op, Terminator};
use rvm_spec::Limits;

fn corpus_dir(directory: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join(directory)
}

fn case_files(directory: &str) -> Vec<PathBuf> {
    let dir = corpus_dir(directory);
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("falha ao ler {}: {error}", dir.display()))
        .map(|entry| entry.expect("entrada de diretório").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rvm"))
        .collect();
    files.sort();
    files
}

fn metadata_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(';')?.trim_start();
    let (found_key, value) = rest.split_once(':')?;
    if found_key.trim() != key {
        return None;
    }
    Some(value.trim())
}

/// Coleta os nomes estáveis de todas as operações e terminadores de um módulo.
fn collect_ops(module: &Module, ops: &mut BTreeSet<String>, terms: &mut BTreeSet<String>) {
    for function in &module.functions {
        for block in &function.blocks {
            for instr in &block.instrs {
                let name = match &instr.op {
                    Op::ConstI64(_) => "const_i64".to_string(),
                    Op::ConstBool(_) => "const_bool".to_string(),
                    Op::Bin { op, .. } => op.name().to_string(),
                    Op::Cmp { op, .. } => op.name().to_string(),
                    Op::Call { .. } => "call_static".to_string(),
                };
                ops.insert(name);
            }
            let term = match &block.term {
                Terminator::Return(_) => "return",
                Terminator::Br(_) => "br",
                Terminator::CondBr { .. } => "cond_br",
            };
            terms.insert(term.to_string());
        }
    }
}

#[test]
fn corpus_valido_cobre_todas_as_operacoes_do_subset() {
    let limits = Limits::default();
    let mut ops = BTreeSet::new();
    let mut terms = BTreeSet::new();
    for path in case_files("valid") {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("falha ao ler {}: {error}", path.display()));
        let module = rvm_text::parse(&source, &limits)
            .unwrap_or_else(|error| panic!("{}: parse falhou: {error}", path.display()));
        collect_ops(&module, &mut ops, &mut terms);
    }

    let required_ops = [
        "const_i64",
        "const_bool",
        "add_i64_checked",
        "sub_i64_checked",
        "mul_i64_checked",
        "eq_i64",
        "ne_i64",
        "lt_i64",
        "le_i64",
        "gt_i64",
        "ge_i64",
        "call_static",
    ];
    let missing: Vec<&str> = required_ops
        .iter()
        .copied()
        .filter(|name| !ops.contains(*name))
        .collect();
    assert!(
        missing.is_empty(),
        "operações sem caso válido no corpus: {missing:?} (cobertas: {ops:?})"
    );

    let required_terms = ["return", "br", "cond_br"];
    let missing_terms: Vec<&str> = required_terms
        .iter()
        .copied()
        .filter(|name| !terms.contains(*name))
        .collect();
    assert!(
        missing_terms.is_empty(),
        "terminadores sem caso válido no corpus: {missing_terms:?} (cobertos: {terms:?})"
    );
}

#[test]
fn corpus_invalido_cobre_os_codigos_de_erro_centrais() {
    let mut codes = BTreeSet::new();
    for path in case_files("invalid") {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("falha ao ler {}: {error}", path.display()));
        for line in source.lines() {
            if let Some(value) = metadata_value(line, "expect") {
                let (kind, rest) = value.split_once(' ').unwrap_or_else(|| {
                    panic!("{}: expect `{value}` sem argumento", path.display())
                });
                if kind != "value" {
                    codes.insert(rest.trim().to_string());
                }
            }
        }
    }

    let required = [
        "invalid_type",
        "invalid_module",
        "invalid_control_flow",
        "unknown_opcode",
        "unsupported_version",
        "integer_overflow",
        "out_of_fuel",
        "call_depth_exceeded",
        "out_of_memory",
    ];
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|name| !codes.contains(*name))
        .collect();
    assert!(
        missing.is_empty(),
        "códigos de erro sem caso inválido no corpus: {missing:?} (cobertos: {codes:?})"
    );
}
