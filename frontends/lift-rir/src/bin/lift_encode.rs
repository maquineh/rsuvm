//! `lift-encode` — compila um programa Lift para bytecode RVM (`.rsvmbc`).
//!
//! Pipeline: lexer/parser/analisador do Lift → HIR → RIR (`lift-rir`) →
//! verificação RVM → `rvm_bytecode::encode`. Não executa o programa.
//!
//! O subconjunto aceito é o documentado em `frontends/lift-rir/README.md`.
//! Recursos que o RIR não representa (enums, unions, closures, generics,
//! `match`, `for-of`, `try/catch`, FFI) são rejeitados com `Unsupported`.

#![forbid(unsafe_code)]

use std::fmt;
use std::process::ExitCode;

use lift_rir::lower_source;
use rvm_spec::{ErrorClass, Limits, RvmError};

const EXIT_USAGE: u8 = 1;
const EXIT_MODULE: u8 = 2;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("lift-encode: {failure}");
            ExitCode::from(failure.exit_code())
        }
    }
}

fn usage() -> String {
    format!(
        "lift-encode {version} — compila Lift para bytecode RVM (ADR-0021)\n\
         \n\
         USO:\n    \
         lift-encode <arquivo.lift> <entry> <saida.rsvmbc>\n\
         \n\
         O arquivo .lift deve estar sem BOM. O módulo é verificado antes de ser\n\
         codificado. Este binário não executa o programa e não concede efeitos.\n",
        version = env!("CARGO_PKG_VERSION")
    )
}

fn run() -> Result<(), Failure> {
    let mut positional: Vec<String> = Vec::new();
    for token in std::env::args().skip(1) {
        match token.as_str() {
            "-h" | "--help" => {
                println!("{}", usage());
                return Ok(());
            }
            other if other.starts_with("--") => {
                return Err(Failure::Usage(format!("opção desconhecida `{other}`")));
            }
            other => positional.push(other.to_string()),
        }
    }
    let [file, entry, output] = positional.as_slice() else {
        return Err(Failure::Usage(format!(
            "esperava 3 posicionais: <arquivo.lift> <entry> <saida.rsvmbc>\n{}",
            usage()
        )));
    };

    let source =
        std::fs::read_to_string(file).map_err(|error| Failure::Io(format!("{file}: {error}")))?;
    let module =
        lower_source(&source, entry).map_err(|error| Failure::Adapter(error.to_string()))?;
    let verified =
        rvm_verify::VerifiedModule::verify(module, &Limits::default()).map_err(Failure::Rvm)?;
    let bytes = rvm_bytecode::encode(verified.module());
    std::fs::write(output, &bytes).map_err(|error| Failure::Io(format!("{output}: {error}")))?;
    println!("{} bytes em {output}", bytes.len());
    Ok(())
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
            Failure::Adapter(_) => EXIT_MODULE,
            Failure::Rvm(error) => match error.class() {
                ErrorClass::Module => EXIT_MODULE,
                ErrorClass::Trap | ErrorClass::ResourceLimit | ErrorClass::Capability => {
                    // A verificação não executa; estes não devem ocorrer.
                    EXIT_MODULE
                }
            },
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Usage(message) | Failure::Io(message) | Failure::Adapter(message) => {
                write!(formatter, "{message}")
            }
            Failure::Rvm(error) => write!(formatter, "{error}"),
        }
    }
}
