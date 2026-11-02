#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! Teste de conformidade orientado por dados (EXP-0).
//!
//! Percorre `corpus/valid` e `corpus/invalid`, lê a expectativa declarada em
//! comentários (`corpus/README.md`) e executa o pipeline
//! `texto → estrutura → verificação → execução`, comparando o resultado.
//!
//! Regras de falha:
//! - `error <código>` precisa falhar **antes** da execução e com
//!   `ErrorClass::Module`;
//! - `trap <código>` precisa verificar e falhar na execução com
//!   `ErrorClass::Trap`;
//! - `limit <código>` precisa verificar e falhar na execução com
//!   `ErrorClass::ResourceLimit`;
//! - `value <display>` precisa verificar e executar, e o valor precisa ser
//!   exatamente `<display>`; o caso também é reexecutado e passa pelo diferencial
//!   `encode → decode → verify → execute`.
//!
//! Nenhuma expectativa é ajustada aqui: divergência exige mudança de contrato ou
//! relatório de divergência.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use rvm_ir::{FnId, Module};
use rvm_runtime::{CallDepthLimit, ExecConfig, MemoryLimit, Value};
use rvm_spec::{ErrorClass, ErrorCode, Limits, RvmError};

const DEFAULT_FUEL_FALLBACK: u64 = 100_000;

#[derive(Debug)]
enum Expect {
    Value(String),
    Error(ErrorCode),
    Trap(ErrorCode),
    Limit(ErrorCode),
}

#[derive(Debug)]
struct Case {
    id: String,
    expect: Expect,
    args: Vec<Value>,
    entry: Option<String>,
    fuel: Option<u64>,
    max_depth: Option<u16>,
    memory: Option<u64>,
    source: String,
    path: PathBuf,
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus")
}

fn case_files(directory: &str) -> Vec<PathBuf> {
    let dir = corpus_root().join(directory);
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

fn parse_value(text: &str) -> Value {
    let (kind, raw) = text
        .split_once(':')
        .unwrap_or_else(|| panic!("argumento `{text}` sem `tipo:valor`"));
    match kind.trim() {
        "i64" => Value::I64(
            raw.trim()
                .parse::<i64>()
                .unwrap_or_else(|error| panic!("i64 inválido `{raw}`: {error}")),
        ),
        "bool" => match raw.trim() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            other => panic!("bool inválido `{other}`"),
        },
        "f64" => Value::F64(
            raw.trim()
                .parse::<f64>()
                .unwrap_or_else(|error| panic!("f64 inválido `{raw}`: {error}")),
        ),
        other => panic!("tipo de argumento não suportado no corpus: `{other}`"),
    }
}

fn parse_case(path: &Path) -> Case {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("falha ao ler {}: {error}", path.display()));

    let mut id: Option<String> = None;
    let mut expect: Option<Expect> = None;
    let mut args: Vec<Value> = Vec::new();
    let mut entry: Option<String> = None;
    let mut fuel: Option<u64> = None;
    let mut max_depth: Option<u16> = None;
    let mut memory: Option<u64> = None;

    for line in source.lines() {
        if let Some(value) = metadata_value(line, "id") {
            id = Some(value.to_string());
        } else if let Some(value) = metadata_value(line, "expect") {
            let (kind, rest) = value
                .split_once(' ')
                .unwrap_or_else(|| panic!("{}: expect `{value}` sem argumento", path.display()));
            let code = |name: &str| {
                ErrorCode::from_stable_name(name)
                    .unwrap_or_else(|| panic!("{}: código desconhecido `{name}`", path.display()))
            };
            let parsed = match kind {
                "value" => Expect::Value(rest.trim().to_string()),
                "error" => Expect::Error(code(rest.trim())),
                "trap" => Expect::Trap(code(rest.trim())),
                "limit" => Expect::Limit(code(rest.trim())),
                other => panic!("{}: tipo de expectativa `{other}`", path.display()),
            };
            assert!(
                expect.is_none(),
                "{}: mais de uma linha `expect`",
                path.display()
            );
            expect = Some(parsed);
        } else if let Some(value) = metadata_value(line, "args") {
            for item in value.split(',') {
                let item = item.trim();
                if !item.is_empty() {
                    args.push(parse_value(item));
                }
            }
        } else if let Some(value) = metadata_value(line, "entry") {
            entry = Some(value.to_string());
        } else if let Some(value) = metadata_value(line, "fuel") {
            fuel = Some(
                value
                    .parse()
                    .unwrap_or_else(|error| panic!("{}: fuel inválido: {error}", path.display())),
            );
        } else if let Some(value) = metadata_value(line, "memory") {
            memory =
                Some(value.parse().unwrap_or_else(|error| {
                    panic!("{}: memory inválido: {error}", path.display())
                }));
        } else if let Some(value) = metadata_value(line, "max-depth") {
            max_depth =
                Some(value.parse().unwrap_or_else(|error| {
                    panic!("{}: max-depth inválido: {error}", path.display())
                }));
        }
    }

    Case {
        id: id.unwrap_or_else(|| panic!("{}: falta `id:`", path.display())),
        expect: expect.unwrap_or_else(|| panic!("{}: falta `expect:`", path.display())),
        args,
        entry,
        fuel,
        max_depth,
        memory,
        source,
        path: path.to_path_buf(),
    }
}

fn entry_id(module: &Module, requested: Option<&str>) -> FnId {
    match requested {
        None => module.entry,
        Some(name) => {
            module
                .function_by_name(name)
                .unwrap_or_else(|| panic!("função de entrada `{name}` não existe no módulo"))
                .id
        }
    }
}

fn config_for(case: &Case) -> ExecConfig {
    let depth = case
        .max_depth
        .map(|value| {
            CallDepthLimit::new(value).unwrap_or_else(|| {
                panic!("{}: max-depth {value} fora de 1..=256", case.path.display())
            })
        })
        .unwrap_or_default();
    let fuel = case.fuel.unwrap_or(DEFAULT_FUEL_FALLBACK);
    let mut config = ExecConfig::new(fuel, depth);
    if let Some(bytes) = case.memory {
        config = config.with_memory(MemoryLimit::new(bytes));
    }
    config
}

/// Executa o pipeline completo e devolve `Ok((valor, fuel))` ou o erro do
/// estágio em que o pipeline parou.
fn run_pipeline(case: &Case) -> Result<(Value, u64), RvmError> {
    let limits = Limits::default();
    let module = rvm_text::parse(&case.source, &limits)?;
    let entry = entry_id(&module, case.entry.as_deref());
    let verified = rvm_verify::VerifiedModule::verify(module, &limits)?;
    let outcome = rvm_interpreter::execute(&verified, entry, &case.args, &config_for(case))?;
    Ok((outcome.value, outcome.fuel_used))
}

fn check_value_case(case: &Case, expected: &str) {
    let (value, fuel_used) = run_pipeline(case)
        .unwrap_or_else(|error| panic!("{}: esperava valor, obteve erro: {error}", case.id));
    assert_eq!(
        value.to_string(),
        expected,
        "{}: valor divergente ({})",
        case.id,
        case.path.display()
    );

    // C-006 generalizado: reexecutar módulo puro reproduz valor e fuel.
    let (again, fuel_again) = run_pipeline(case)
        .unwrap_or_else(|error| panic!("{}: reexecução falhou: {error}", case.id));
    assert_eq!(again, value, "{}: reexecução produziu outro valor", case.id);
    assert_eq!(
        fuel_again, fuel_used,
        "{}: reexecução consumiu outro fuel",
        case.id
    );

    // Diferencial RIR ↔ bytecode: mesmo valor e mesmo fuel.
    let limits = Limits::default();
    let module = rvm_text::parse(&case.source, &limits)
        .unwrap_or_else(|error| panic!("{}: parse falhou no diferencial: {error}", case.id));
    let entry = entry_id(&module, case.entry.as_deref());
    let bytes = rvm_bytecode::encode(&module);
    let decoded = rvm_bytecode::decode(&bytes, &limits)
        .unwrap_or_else(|error| panic!("{}: decode falhou: {error}", case.id));
    let verified = rvm_verify::VerifiedModule::verify(decoded, &limits)
        .unwrap_or_else(|error| panic!("{}: verify do bytecode falhou: {error}", case.id));
    let outcome = rvm_interpreter::execute(&verified, entry, &case.args, &config_for(case))
        .unwrap_or_else(|error| panic!("{}: execução do bytecode falhou: {error}", case.id));
    assert_eq!(
        outcome.value.to_string(),
        expected,
        "{}: bytecode divergiu do caminho textual",
        case.id
    );
    assert_eq!(
        outcome.fuel_used, fuel_used,
        "{}: bytecode consumiu fuel diferente",
        case.id
    );
}

fn check_error_case(case: &Case, expected: ErrorCode) {
    let limits = Limits::default();
    let error = match rvm_text::parse(&case.source, &limits) {
        Err(error) => error,
        Ok(module) => match rvm_verify::VerifiedModule::verify(module, &limits) {
            Err(error) => error,
            Ok(_) => panic!(
                "{}: esperava erro de módulo `{}`, mas o módulo verificou",
                case.id,
                expected.as_str()
            ),
        },
    };
    assert_eq!(
        error.code(),
        expected,
        "{}: código divergente (detalhe: {})",
        case.id,
        error
    );
    assert_eq!(
        error.class(),
        ErrorClass::Module,
        "{}: erro de módulo precisa acontecer antes da execução",
        case.id
    );
}

fn check_execution_failure(case: &Case, expected: ErrorCode, class: ErrorClass) {
    let limits = Limits::default();
    let module = rvm_text::parse(&case.source, &limits)
        .unwrap_or_else(|error| panic!("{}: parse falhou: {error}", case.id));
    let entry = entry_id(&module, case.entry.as_deref());
    let verified = rvm_verify::VerifiedModule::verify(module, &limits)
        .unwrap_or_else(|error| panic!("{}: verify falhou: {error}", case.id));
    let error = rvm_interpreter::execute(&verified, entry, &case.args, &config_for(case))
        .err()
        .unwrap_or_else(|| {
            panic!(
                "{}: esperava falha `{}`, mas executou",
                case.id,
                expected.as_str()
            )
        });
    assert_eq!(error.code(), expected, "{}: código divergente", case.id);
    assert_eq!(
        error.class(),
        class,
        "{}: classe divergente para `{}`",
        case.id,
        expected.as_str()
    );
}

fn run_corpus(directory: &str) -> (BTreeSet<String>, usize) {
    let mut ids = BTreeSet::new();
    let mut count = 0;
    for path in case_files(directory) {
        let case = parse_case(&path);
        assert!(
            ids.insert(case.id.clone()),
            "{}: id duplicado `{}`",
            case.path.display(),
            case.id
        );
        match &case.expect {
            Expect::Value(expected) => check_value_case(&case, expected),
            Expect::Error(expected) => check_error_case(&case, *expected),
            Expect::Trap(expected) => check_execution_failure(&case, *expected, ErrorClass::Trap),
            Expect::Limit(expected) => {
                check_execution_failure(&case, *expected, ErrorClass::ResourceLimit)
            }
        }
        count += 1;
    }
    (ids, count)
}

#[test]
fn corpus_valido_executa_e_reproduz() {
    let (ids, count) = run_corpus("valid");
    assert!(
        count >= 16,
        "corpus válido inesperadamente pequeno: {count}"
    );
    for required in [
        "C-001", "C-006", "C-010", "C-011", "C-016", "C-017", "C-018", "C-019", "C-020", "C-021",
        "C-022", "C-023", "C-024", "C-025", "C-026", "C-027", "C-028", "C-029", "C-030", "C-031",
        "C-032", "C-033", "C-034", "C-035", "C-036", "C-037", "C-038", "C-039", "C-040", "C-041",
        "C-042", "C-043", "C-044", "C-045", "C-046", "C-047", "C-048", "C-049", "C-050", "C-051",
        "C-052",
    ] {
        assert!(
            ids.contains(required),
            "caso obrigatório `{required}` ausente"
        );
    }
}

#[test]
fn corpus_invalido_e_rejeitado_antes_da_execucao() {
    let (ids, count) = run_corpus("invalid");
    assert!(
        count >= 20,
        "corpus inválido inesperadamente pequeno: {count}"
    );
    for required in [
        "N-001", "N-007", "N-009", "N-010", "N-011", "N-019", "N-020", "N-021", "N-022", "N-023",
        "N-024", "N-025", "N-026", "N-027", "N-028", "N-029", "N-030", "N-031", "N-032", "N-033",
        "N-034", "N-035", "N-036", "N-037", "N-038", "N-039", "N-040",
    ] {
        assert!(
            ids.contains(required),
            "caso obrigatório `{required}` ausente"
        );
    }
}

#[test]
fn ids_do_corpus_sao_unicos_entre_diretorios() {
    let mut todos = BTreeSet::new();
    for directory in ["valid", "invalid"] {
        for path in case_files(directory) {
            let case = parse_case(&path);
            assert!(
                todos.insert(case.id.clone()),
                "{}: id `{}` repetido entre valid/ e invalid/",
                case.path.display(),
                case.id
            );
        }
    }
}
