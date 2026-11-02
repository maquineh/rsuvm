#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! Micro-benchmark pré-registrado de custo de `call_static` intra-módulo.
//!
//! Método e hipóteses: `docs/RELATORIOS/BENCHMARKS-PREREGISTRADOS.md` (H06/H03).
//! Este alvo **não** classifica H06/H03 (que exigem chamada inter-módulo, fora do
//! EXP-0): mede o custo de uma chamada **intra-módulo** como substituto
//! exploratório. Não é teste, não roda em CI e não faz asserção de limiar; os
//! números dependem da máquina.
//!
//! Executar: `cargo bench -p rvm-interpreter --bench chamada`.

use std::time::Instant;

use rvm_interpreter::execute;
use rvm_ir::{Block, Function, Instr, Module, Op, Param, Terminator};
use rvm_runtime::{CallDepthLimit, ExecConfig};
use rvm_spec::{BinOp, Limits, Type, IR_VERSION};
use rvm_verify::VerifiedModule;

/// Módulo com `k` chamadas encadeadas a `inc` mais um literal inicial.
///
/// ```text
/// fn main() -> i64 { v0 = 0; v1 = inc(v0); ...; return v_k }
/// fn inc(a: i64) -> i64 { v1 = 1; v2 = a + v1; return v2 }
/// ```
fn cadeia(k: u32) -> Module {
    let mut instrs = vec![Instr {
        def: 0,
        op: Op::ConstI64(0),
    }];
    let mut last = 0_u32;
    for index in 1..=k {
        instrs.push(Instr {
            def: index,
            op: Op::Call {
                callee: 1,
                args: vec![last],
            },
        });
        last = index;
    }
    let main = Function {
        id: 0,
        name: "main".to_string(),
        params: Vec::new(),
        ret: Type::I64,
        entry: 0,
        blocks: vec![Block {
            id: 0,
            label: "entry".to_string(),
            params: Vec::new(),
            instrs,
            term: Terminator::Return(Some(last)),
        }],
    };
    let inc = Function {
        id: 1,
        name: "inc".to_string(),
        params: vec![Param {
            name: "a".to_string(),
            ty: Type::I64,
        }],
        ret: Type::I64,
        entry: 0,
        blocks: vec![Block {
            id: 0,
            label: "entry".to_string(),
            params: Vec::new(),
            instrs: vec![
                Instr {
                    def: 1,
                    op: Op::ConstI64(1),
                },
                Instr {
                    def: 2,
                    op: Op::Bin {
                        op: BinOp::AddI64Checked,
                        lhs: 0,
                        rhs: 1,
                    },
                },
            ],
            term: Terminator::Return(Some(2)),
        }],
    };
    Module {
        name: format!("cadeia{k}"),
        version: IR_VERSION,
        entry: 0,
        functions: vec![main, inc],
    }
}

/// Mede `reps` execuções e devolve `(ns por execução, fuel por execução)`.
fn medir(module: &VerifiedModule, reps: u32) -> (f64, u64) {
    let entry = module.entry();
    let config = ExecConfig::new(1_000_000, CallDepthLimit::new(64).expect("limite válido"));
    // Aquecimento (JIT de SO/cache), descartado.
    let _ = execute(module, entry, &[], &config).expect("execução de aquecimento");

    let start = Instant::now();
    let mut fuel = 0_u64;
    for _ in 0..reps {
        let outcome = execute(module, entry, &[], &config).expect("execução medida");
        fuel = outcome.fuel_used;
    }
    let elapsed = start.elapsed();
    (elapsed.as_nanos() as f64 / f64::from(reps), fuel)
}

fn main() {
    let limits = Limits::default();
    let reps: u32 = 5_000;
    println!("chamada intra-modulo (substituto exploratorio; nao classifica H06/H03)");
    println!("reps por ponto: {reps}");
    println!(
        "{:<8} {:>12} {:>10} {:>12}",
        "k", "ns/exec", "fuel", "ns/fuel"
    );

    let mut anterior: Option<(u32, f64)> = None;
    for k in [1_u32, 4, 16, 64] {
        let module = cadeia(k);
        let verified = VerifiedModule::verify(module, &limits).expect("módulo de bench verifica");
        let (ns, fuel) = medir(&verified, reps);
        let ns_por_fuel = ns / f64::from(u32::try_from(fuel).unwrap_or(u32::MAX));
        println!("{k:<8} {ns:>12.1} {fuel:>10} {ns_por_fuel:>12.3}");
        if let Some((k0, ns0)) = anterior {
            let delta_k = f64::from(k - k0);
            let custo_marginal = (ns - ns0) / delta_k;
            println!(
                "  marginal de {} a {}: {custo_marginal:.1} ns por chamada",
                k0, k
            );
        }
        anterior = Some((k, ns));
    }
}
