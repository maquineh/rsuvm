//! `lift-run` — executa um subconjunto de **Lift** na RVM (ADR-0021).
//!
//! Lê um arquivo `.lift`, roda o pipeline do Lift, baixa para a RIR, verifica e
//! executa, imprimindo o valor no mesmo formato do `rsuvm` (`i64:30`,
//! `bool:true`, `unit`). Não concede filesystem, rede, ambiente, relógio ou
//! subprocessos ao programa (o núcleo não tem capabilities).

#![forbid(unsafe_code)]

use std::fmt;
use std::process::ExitCode;

use lift_rir::lower_source;
use rvm_runtime::{CallDepthLimit, ExecConfig, MemoryLimit, Value};
use rvm_spec::{ErrorClass, Limits, RvmError};

const EXIT_USAGE: u8 = 1;
const EXIT_MODULE: u8 = 2;
const EXIT_TRAP: u8 = 3;
const EXIT_LIMIT: u8 = 4;
const EXIT_CAPABILITY: u8 = 5;

const DEFAULT_FUEL: u64 = 100_000;
const DEFAULT_MAX_DEPTH: u16 = 64;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("lift-run: {failure}");
            ExitCode::from(failure.exit_code())
        }
    }
}

fn usage() -> String {
    format!(
        "lift-run {version} — executa um subconjunto de Lift na RVM (ADR-0021)\n\
         \n\
         USO:\n    \
         lift-run <arquivo.lift> <entry> [--arg <tipo:valor>]... [--fuel N] [--max-depth N] [--memory N]\n\
         \n\
         Tipos de argumento: i64:<inteiro>, bool:true|false, unit.\n\
         Subconjunto: funções puras int/boolean/void, let/return, if/else terminal,\n\
         + - *, comparações, negação unária e chamadas internas.\n\
         Este binário não concede filesystem, rede, ambiente, relógio ou subprocessos.\n",
        version = env!("CARGO_PKG_VERSION")
    )
}

fn run() -> Result<(), Failure> {
    let mut file: Option<String> = None;
    let mut entry: Option<String> = None;
    let mut args: Vec<Value> = Vec::new();
    let mut fuel = DEFAULT_FUEL;
    let mut max_depth = DEFAULT_MAX_DEPTH;
    let mut memory = MemoryLimit::DEFAULT.get();

    let mut tokens = std::env::args().skip(1);
    while let Some(token) = tokens.next() {
        match token.as_str() {
            "-h" | "--help" => {
                println!("{}", usage());
                return Ok(());
            }
            "--arg" => {
                let raw = tokens
                    .next()
                    .ok_or_else(|| Failure::Usage("`--arg` sem valor".to_string()))?;
                args.push(parse_value(&raw)?);
            }
            "--fuel" => fuel = parse_u64(tokens.next(), "--fuel")?,
            "--max-depth" => max_depth = parse_u16(tokens.next(), "--max-depth")?,
            "--memory" => memory = parse_u64(tokens.next(), "--memory")?,
            other if other.starts_with("--") => {
                return Err(Failure::Usage(format!("opção desconhecida `{other}`")));
            }
            other => {
                if file.is_none() {
                    file = Some(other.to_string());
                } else if entry.is_none() {
                    entry = Some(other.to_string());
                } else {
                    return Err(Failure::Usage(format!(
                        "argumento posicional extra `{other}`"
                    )));
                }
            }
        }
    }

    let file = file.ok_or_else(|| Failure::Usage("faltou `<arquivo.lift>`".to_string()))?;
    let entry = entry.ok_or_else(|| Failure::Usage("faltou `<entry>`".to_string()))?;

    let source =
        std::fs::read_to_string(&file).map_err(|error| Failure::Io(format!("{file}: {error}")))?;

    let module =
        lower_source(&source, &entry).map_err(|error| Failure::Adapter(error.to_string()))?;
    let limits = Limits::default();
    let verified = rvm_verify::VerifiedModule::verify(module, &limits).map_err(Failure::Rvm)?;

    let depth = CallDepthLimit::new(max_depth)
        .ok_or_else(|| Failure::Usage(format!("--max-depth {max_depth} fora de 1..=256")))?;
    let config = ExecConfig::new(fuel, depth).with_memory(MemoryLimit::new(memory));

    let outcome = rvm_interpreter::execute(&verified, verified.entry(), &args, &config)
        .map_err(Failure::Rvm)?;
    println!("{}", outcome.value);
    Ok(())
}

fn parse_value(raw: &str) -> Result<Value, Failure> {
    let (kind, value) = raw
        .split_once(':')
        .ok_or_else(|| Failure::Usage(format!("argumento `{raw}` sem `tipo:valor`")))?;
    match kind.trim() {
        "i64" => value
            .trim()
            .parse::<i64>()
            .map(Value::I64)
            .map_err(|error| Failure::Usage(format!("i64 inválido `{value}`: {error}"))),
        "bool" => match value.trim() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            other => Err(Failure::Usage(format!("bool inválido `{other}`"))),
        },
        "unit" => Ok(Value::Unit),
        other => Err(Failure::Usage(format!("tipo `{other}` não suportado"))),
    }
}

fn parse_u64(raw: Option<String>, flag: &str) -> Result<u64, Failure> {
    let raw = raw.ok_or_else(|| Failure::Usage(format!("`{flag}` sem valor")))?;
    raw.parse::<u64>()
        .map_err(|error| Failure::Usage(format!("{flag} inválido `{raw}`: {error}")))
}

fn parse_u16(raw: Option<String>, flag: &str) -> Result<u16, Failure> {
    let raw = raw.ok_or_else(|| Failure::Usage(format!("`{flag}` sem valor")))?;
    raw.parse::<u16>()
        .map_err(|error| Failure::Usage(format!("{flag} inválido `{raw}`: {error}")))
}

enum Failure {
    Usage(String),
    Io(String),
    Adapter(String),
    Rvm(RvmError),
}

impl Failure {
    fn exit_code(&self) -> u8 {
        match self {
            Failure::Usage(_) | Failure::Io(_) => EXIT_USAGE,
            // O programa Lift não pôde virar um módulo válido: classe de módulo.
            Failure::Adapter(_) => EXIT_MODULE,
            Failure::Rvm(error) => match error.class() {
                ErrorClass::Module => EXIT_MODULE,
                ErrorClass::Trap => EXIT_TRAP,
                ErrorClass::ResourceLimit => EXIT_LIMIT,
                ErrorClass::Capability => EXIT_CAPABILITY,
            },
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Usage(message) | Failure::Io(message) => write!(formatter, "{message}"),
            Failure::Adapter(message) => write!(formatter, "{message}"),
            Failure::Rvm(error) => write!(formatter, "{error}"),
        }
    }
}
