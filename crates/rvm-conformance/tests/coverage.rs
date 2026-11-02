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
                    Op::ConstStr(_) => "const_str".to_string(),
                    Op::Bin { op, .. } => op.name().to_string(),
                    Op::Cmp { op, .. } => op.name().to_string(),
                    Op::StrEq { .. } => "str_eq".to_string(),
                    Op::StrNe { .. } => "str_ne".to_string(),
                    Op::StrLen { .. } => "str_len".to_string(),
                    Op::ConcatStr { .. } => "concat_str".to_string(),
                    Op::ConstF64(_) => "const_f64".to_string(),
                    Op::Float { op, .. } => op.name().to_string(),
                    Op::ArrayLiteral { .. } => "array".to_string(),
                    Op::ArrayLen { .. } => "array_len".to_string(),
                    Op::ArrayGet { .. } => "array_get".to_string(),
                    Op::ArraySet { .. } => "array_set".to_string(),
                    Op::StructNew { .. } => "struct_new".to_string(),
                    Op::FieldGet { .. } => "field_get".to_string(),
                    Op::FieldSet { .. } => "field_set".to_string(),
                    Op::OptionNone(_) => "option_none".to_string(),
                    Op::OptionSome { .. } => "option_some".to_string(),
                    Op::OptionIsSome { .. } => "option_is_some".to_string(),
                    Op::OptionUnwrap { .. } => "option_unwrap".to_string(),
                    Op::Coalesce { .. } => "coalesce".to_string(),
                    Op::OptionEq { .. } => "option_eq".to_string(),
                    Op::OptionNe { .. } => "option_ne".to_string(),
                    Op::ResultOk { .. } => "result_ok".to_string(),
                    Op::ResultErr { .. } => "result_err".to_string(),
                    Op::ResultIsOk { .. } => "result_is_ok".to_string(),
                    Op::ResultUnwrap { .. } => "result_unwrap".to_string(),
                    Op::ResultUnwrapErr { .. } => "result_unwrap_err".to_string(),
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
        "div_i64_checked",
        "rem_i64_checked",
        "shl_i64_checked",
        "shr_i64_checked",
        "and_i64",
        "or_i64",
        "xor_i64",
        "const_str",
        "str_eq",
        "str_ne",
        "str_len",
        "concat_str",
        "const_f64",
        "add_f64",
        "sub_f64",
        "mul_f64",
        "div_f64",
        "eq_f64",
        "ne_f64",
        "lt_f64",
        "le_f64",
        "gt_f64",
        "ge_f64",
        "array",
        "array_len",
        "array_get",
        "array_set",
        "struct_new",
        "field_get",
        "field_set",
        "option_none",
        "option_some",
        "option_is_some",
        "option_unwrap",
        "coalesce",
        "option_eq",
        "option_ne",
        "result_ok",
        "result_err",
        "result_is_ok",
        "result_unwrap",
        "result_unwrap_err",
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
        "division_by_zero",
        "invalid_shift",
        "index_out_of_bounds",
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
