//! CLI de desenvolvimento da RSUVM (`rsuvm`).
//!
//! Não concede capabilities: todos os subcomandos executam apenas código puro do
//! convidado. Códigos de saída (`docs/CONTRATOS/ERROS-EXP-0.md` §5):
//!
//! - `0` sucesso;
//! - `1` erro de uso ou de IO da ferramenta;
//! - `2` erro de módulo (decode ou verificação);
//! - `3` trap semântico;
//! - `4` limite de recurso;
//! - `5` capability negada (não alcançável no MVP).

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rvm_ir::Module;
use rvm_runtime::{CallDepthLimit, ExecConfig, MemoryLimit, Value};
use rvm_spec::{ErrorClass, Limits, RvmError};

const EXIT_USAGE: u8 = 1;
const EXIT_MODULE: u8 = 2;
const EXIT_TRAP: u8 = 3;
const EXIT_LIMIT: u8 = 4;
const EXIT_CAPABILITY: u8 = 5;

const DEFAULT_MAX_DEPTH: u16 = 64;

fn main() -> ExitCode {
    match dispatch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("{failure}");
            ExitCode::from(failure.exit_code())
        }
    }
}

fn dispatch() -> Result<(), Failure> {
    let mut args = std::env::args().skip(1);
    let command = match args.next() {
        Some(command) => command,
        None => return Err(Failure::Usage(usage())),
    };
    let rest: Vec<String> = args.collect();
    match command.as_str() {
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("rsuvm {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "run" => command_run(&rest, Source::Text),
        "run-bc" => command_run(&rest, Source::Bytecode),
        "check" => command_check(&rest),
        "fmt" => command_fmt(&rest),
        "encode" => command_encode(&rest),
        other => Err(Failure::Usage(format!(
            "subcomando desconhecido `{other}`\n{}",
            usage()
        ))),
    }
}

/// Origem do módulo: texto experimental ou bytecode experimental.
#[derive(Clone, Copy)]
enum Source {
    Text,
    Bytecode,
}

struct Options {
    path: PathBuf,
    entry: Option<String>,
    args: Vec<Value>,
    fuel: Option<u64>,
    max_depth: Option<u16>,
    memory: Option<u64>,
}

impl Options {
    fn config(&self) -> Result<ExecConfig, Failure> {
        let depth = match self.max_depth {
            Some(value) => CallDepthLimit::new(value).ok_or_else(|| {
                Failure::Usage(format!(
                    "`--max-depth {value}` fora da faixa 1..={}",
                    CallDepthLimit::HARD_MAX
                ))
            })?,
            None => CallDepthLimit::new(DEFAULT_MAX_DEPTH)
                .ok_or_else(|| Failure::Usage("profundidade padrão inválida".to_string()))?,
        };
        let fuel = self.fuel.unwrap_or_else(|| ExecConfig::default().fuel());
        let mut config = ExecConfig::new(fuel, depth);
        if let Some(bytes) = self.memory {
            config = config.with_memory(MemoryLimit::new(bytes));
        }
        Ok(config)
    }
}

fn take_value(rest: &[String], index: &mut usize, what: &str) -> Result<String, Failure> {
    *index += 1;
    rest.get(*index)
        .cloned()
        .ok_or_else(|| Failure::Usage(format!("falta valor para `{what}`")))
}

fn parse_options(rest: &[String], allow_value_options: bool) -> Result<Options, Failure> {
    let mut path: Option<PathBuf> = None;
    let mut entry: Option<String> = None;
    let mut values: Vec<Value> = Vec::new();
    let mut fuel: Option<u64> = None;
    let mut max_depth: Option<u16> = None;
    let mut memory: Option<u64> = None;

    let mut index = 0;
    while index < rest.len() {
        let token = rest
            .get(index)
            .ok_or_else(|| Failure::Usage("argumento ausente".to_string()))?
            .clone();
        match token.as_str() {
            "--entry" if allow_value_options => {
                entry = Some(take_value(rest, &mut index, "--entry")?);
            }
            "--arg" if allow_value_options => {
                let raw = take_value(rest, &mut index, "--arg")?;
                values.push(parse_argument(&raw)?);
            }
            "--memory" => {
                let raw = take_value(rest, &mut index, "--memory")?;
                memory = Some(raw.parse().map_err(|error| {
                    Failure::Usage(format!("`--memory {raw}` inválido: {error}"))
                })?);
            }

            "--fuel" => {
                let raw = take_value(rest, &mut index, "--fuel")?;
                fuel = Some(raw.parse().map_err(|error| {
                    Failure::Usage(format!("`--fuel {raw}` inválido: {error}"))
                })?);
            }
            "--max-depth" => {
                let raw = take_value(rest, &mut index, "--max-depth")?;
                max_depth = Some(raw.parse().map_err(|error| {
                    Failure::Usage(format!("`--max-depth {raw}` inválido: {error}"))
                })?);
            }
            other if other.starts_with("--") => {
                return Err(Failure::Usage(format!(
                    "opção desconhecida `{other}`\n{}",
                    usage()
                )));
            }
            other => {
                if path.is_some() {
                    return Err(Failure::Usage(format!(
                        "argumento posicional extra `{other}`\n{}",
                        usage()
                    )));
                }
                path = Some(PathBuf::from(other));
            }
        }
        index += 1;
    }

    Ok(Options {
        path: path.ok_or_else(|| Failure::Usage(usage()))?,
        entry,
        args: values,
        fuel,
        max_depth,
        memory,
    })
}

fn parse_argument(raw: &str) -> Result<Value, Failure> {
    if raw == "unit" {
        return Ok(Value::Unit);
    }
    let (kind, value) = raw.split_once(':').ok_or_else(|| {
        Failure::Usage(format!(
            "argumento `{raw}` fora do formato `<tipo>:<valor>` (`i64:10`, `bool:true`, `unit`)"
        ))
    })?;
    match kind.trim() {
        "i64" => Ok(Value::I64(value.trim().parse().map_err(|error| {
            Failure::Usage(format!("i64 inválido em `{raw}`: {error}"))
        })?)),
        "bool" => match value.trim() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            other => Err(Failure::Usage(format!("bool inválido `{other}`"))),
        },
        other => Err(Failure::Usage(format!(
            "tipo de argumento não suportado `{other}` (use i64, bool ou unit)"
        ))),
    }
}

fn load_module(path: &Path, source: Source, limits: &Limits) -> Result<Module, Failure> {
    match source {
        Source::Text => {
            let text = std::fs::read_to_string(path).map_err(|error| {
                Failure::Io(format!("não foi possível ler {}: {error}", path.display()))
            })?;
            Ok(rvm_text::parse(&text, limits)?)
        }
        Source::Bytecode => {
            let bytes = std::fs::read(path).map_err(|error| {
                Failure::Io(format!("não foi possível ler {}: {error}", path.display()))
            })?;
            Ok(rvm_bytecode::decode(&bytes, limits)?)
        }
    }
}

fn resolve_entry(module: &Module, requested: Option<&str>) -> Result<u32, Failure> {
    match requested {
        None => Ok(module.entry),
        Some(name) => module
            .function_by_name(name)
            .map(|function| function.id)
            .ok_or_else(|| {
                Failure::Usage(format!(
                    "função de entrada `{name}` não existe em `{}`",
                    module.name
                ))
            }),
    }
}

fn command_run(rest: &[String], source: Source) -> Result<(), Failure> {
    let options = parse_options(rest, true)?;
    let config = options.config()?;
    let limits = Limits::default();
    let module = load_module(&options.path, source, &limits)?;
    let entry = resolve_entry(&module, options.entry.as_deref())?;
    let verified = rvm_verify::VerifiedModule::verify(module, &limits)?;
    let outcome = rvm_interpreter::execute(&verified, entry, &options.args, &config)?;
    println!("{}", outcome.value);
    Ok(())
}

fn command_check(rest: &[String]) -> Result<(), Failure> {
    let options = parse_options(rest, false)?;
    let limits = Limits::default();
    let module = load_module(&options.path, Source::Text, &limits)?;
    let name = module.name.clone();
    let verified = rvm_verify::VerifiedModule::verify(module, &limits)?;
    let entry = verified
        .entry_function()
        .map(|function| function.name.as_str())
        .unwrap_or("<desconhecida>");
    println!("ok: módulo `{name}`, entrada `{entry}`");
    Ok(())
}

fn command_fmt(rest: &[String]) -> Result<(), Failure> {
    let options = parse_options(rest, false)?;
    let limits = Limits::default();
    let module = load_module(&options.path, Source::Text, &limits)?;
    print!("{}", rvm_text::print(&module));
    Ok(())
}

fn command_encode(rest: &[String]) -> Result<(), Failure> {
    let mut positional: Vec<PathBuf> = Vec::new();
    for token in rest {
        if token.starts_with("--") {
            return Err(Failure::Usage(format!(
                "`encode` não aceita opções (`{token}`)\n{}",
                usage()
            )));
        }
        positional.push(PathBuf::from(token));
    }
    if positional.len() != 2 {
        return Err(Failure::Usage(format!(
            "uso: rsuvm encode <entrada.rvm> <saida.bin>\n{}",
            usage()
        )));
    }
    let input = positional
        .first()
        .ok_or_else(|| Failure::Usage(usage()))?
        .clone();
    let output = positional
        .get(1)
        .ok_or_else(|| Failure::Usage(usage()))?
        .clone();
    let limits = Limits::default();
    let module = load_module(&input, Source::Text, &limits)?;
    // `parse` já garantiu a validade estrutural; o decoder revalida na leitura.
    let bytes = rvm_bytecode::encode(&module);
    std::fs::write(&output, &bytes).map_err(|error| {
        Failure::Io(format!(
            "não foi possível escrever {}: {error}",
            output.display()
        ))
    })?;
    println!("{} bytes em {}", bytes.len(), output.display());
    Ok(())
}

fn usage() -> String {
    format!(
        "rsuvm {version} — Rust Universal Virtual Machine (protótipo EXP-0)\n\
         \n\
         Uso:\n  \
         rsuvm run <arquivo.rvm> [--entry <fn>] [--arg <tipo:valor>...] [--fuel <n>] [--max-depth <n>] [--memory <bytes>]\n  \
         rsuvm run-bc <arquivo.bin> [--entry <fn>] [--arg <tipo:valor>...] [--fuel <n>] [--max-depth <n>] [--memory <bytes>]\n  \
         rsuvm check <arquivo.rvm>\n  \
         rsuvm fmt <arquivo.rvm>\n  \
         rsuvm encode <arquivo.rvm> <saida.bin>\n  \
         rsuvm version\n\
         \n\
         Tipos de argumento: i64:<inteiro>, bool:true|false, unit.\n\
         Este binário não concede filesystem, rede, ambiente, relógio ou subprocessos.\n",
        version = env!("CARGO_PKG_VERSION")
    )
}

enum Failure {
    Usage(String),
    Io(String),
    Rvm(RvmError),
}

impl Failure {
    fn exit_code(&self) -> u8 {
        match self {
            Failure::Usage(_) | Failure::Io(_) => EXIT_USAGE,
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Usage(message) => write!(f, "rsuvm: {message}"),
            Failure::Io(message) => write!(f, "rsuvm: {message}"),
            Failure::Rvm(error) => write!(f, "rsuvm: {error}"),
        }
    }
}

impl From<RvmError> for Failure {
    fn from(error: RvmError) -> Self {
        Failure::Rvm(error)
    }
}
