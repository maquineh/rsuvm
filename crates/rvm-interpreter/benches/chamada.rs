#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! Micro-benchmark pré-registrado: custo de `call_static` intra e inter-módulo.
//!
//! Método e hipóteses: `docs/RELATORIOS/BENCHMARKS-PREREGISTRADOS.md` (H06/H03).
//! A variante inter-módulo usa `rvm-link` (ADR-0018) e é ligada **antes** da
//! medição: por construção, a ligação estática elimina a fronteira de módulo em
//! tempo de execução, então o custo medido é o de uma chamada direta por índice
//! (sem serialização). Não é teste, não roda em CI e não faz asserção de limiar;
//! os números dependem da máquina.
//!
//! Executar: `cargo bench -p rvm-interpreter --bench chamada`.

use std::time::Instant;

use rvm_interpreter::execute;
use rvm_ir::{Block, Function, Instr, Module, Op, Param, Terminator};
use rvm_runtime::{CallDepthLimit, ExecConfig};
use rvm_spec::{BinOp, Limits, Type, IR_VERSION};
use rvm_verify::VerifiedModule;

/// Função `inc(a) = a + 1` como bloco único.
fn funcao_inc(id: u32) -> Function {
    Function {
        id,
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
    }
}

/// Corpo de `main`: `v0 = 0; k× call_static <callee>(...); return v_k`.
fn corpo_main(k: u32, callee: u32) -> Vec<Instr> {
    let mut instrs = vec![Instr {
        def: 0,
        op: Op::ConstI64(0),
    }];
    let mut last = 0_u32;
    for index in 1..=k {
        instrs.push(Instr {
            def: index,
            op: Op::Call {
                callee,
                args: vec![last],
            },
        });
        last = index;
    }
    instrs
}

/// Módulo único com `k` chamadas intra-módulo a `inc`.
fn cadeia(k: u32) -> Module {
    let instrs = corpo_main(k, 1);
    let last = u32::try_from(instrs.len()).unwrap_or(0) - 1;
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
    Module {
        name: format!("cadeia{k}"),
        version: IR_VERSION,
        entry: 0,
        functions: vec![main, funcao_inc(1)],
    }
}

/// Módulo `app` com `k` chamadas a uma externa `inc` (id 1, sem corpo).
fn app_externo(k: u32) -> Module {
    let instrs = corpo_main(k, 1);
    let last = u32::try_from(instrs.len()).unwrap_or(0) - 1;
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
    let externa = Function {
        id: 1,
        name: "inc".to_string(),
        params: Vec::new(),
        ret: Type::Unit,
        entry: 0,
        blocks: Vec::new(),
    };
    Module {
        name: "app".to_string(),
        version: IR_VERSION,
        entry: 0,
        functions: vec![main, externa],
    }
}

/// Módulo provedor `prov` com a `inc` concreta.
fn prov() -> Module {
    Module {
        name: "prov".to_string(),
        version: IR_VERSION,
        entry: 0,
        functions: vec![funcao_inc(0)],
    }
}

/// Mede `reps` execuções e devolve `(ns por execução, fuel por execução)`.
fn medir(module: &VerifiedModule, reps: u32) -> (f64, u64) {
    let entry = module.entry();
    let config = ExecConfig::new(1_000_000, CallDepthLimit::new(64).expect("limite válido"));
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

/// Mediana de uma amostra (ordena uma cópia).
fn mediana(mut valores: Vec<f64>) -> f64 {
    valores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let n = valores.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        valores[n / 2]
    } else {
        (valores[n / 2 - 1] + valores[n / 2]) / 2.0
    }
}

/// Dispersão relativa `(max − min) / mediana`.
fn dispersao(valores: &[f64], mediana: f64) -> f64 {
    if mediana <= 0.0 {
        return f64::INFINITY;
    }
    let min = valores.iter().copied().fold(f64::INFINITY, f64::min);
    let max = valores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (max - min) / mediana
}

fn amostrar(module: &VerifiedModule, reps: u32, n: usize) -> Vec<f64> {
    (0..n).map(|_| medir(module, reps).0).collect()
}

fn main() {
    let limits = Limits::default();
    let reps: u32 = 20_000;
    let n: usize = 11;

    // Aquecimento longo para estabilizar frequência de CPU antes de medir.
    let aquecimento = VerifiedModule::verify(cadeia(64), &limits).expect("aquecimento");
    let _ = medir(&aquecimento, 100_000);

    println!("benchmark pre-registrado de chamada (docs/RELATORIOS/BENCHMARKS-PREREGISTRADOS.md)");
    println!("reps por amostra: {reps}; amostras por ponto: {n}");
    println!(
        "{:<6} {:>8} {:>12} {:>12} {:>10} {:>10}",
        "k", "fuel", "intra ns", "inter ns", "H06 rel", "dispersao"
    );

    let mut intra_anterior: Option<(u32, f64)> = None;
    let mut inter_anterior: Option<(u32, f64)> = None;
    let mut overhead_final = f64::NAN;
    let mut dispersao_maxima = 0.0_f64;
    // Dispersão do **ponto classificador** (o maior `k`, usado no overhead de
    // H06). O ponto `k = 1` é baseline e não entra no overhead, então a sua
    // dispersão não decide a classificação (ver addendum no pré-registro).
    let mut dispersao_classificadora = f64::NAN;
    for k in [1_u32, 4, 16, 64, 256] {
        let intra = VerifiedModule::verify(cadeia(k), &limits).expect("intra verifica");

        let linked = rvm_link::link(&[app_externo(k), prov()], 0, 0, &limits).expect("ligação");
        let inter = VerifiedModule::verify(linked, &limits).expect("inter verifica");

        let amostras_intra = amostrar(&intra, reps, n);
        let amostras_inter = amostrar(&inter, reps, n);
        let intra_ns = mediana(amostras_intra.clone());
        let inter_ns = mediana(amostras_inter.clone());
        let disp = dispersao(&amostras_intra, intra_ns).max(dispersao(&amostras_inter, inter_ns));
        dispersao_maxima = dispersao_maxima.max(disp);
        dispersao_classificadora = disp;

        let fuel_intra = medir(&intra, 1).1;
        let fuel_inter = medir(&inter, 1).1;
        assert_eq!(
            fuel_intra, fuel_inter,
            "fuel deve ser idêntico entre variantes (controle)"
        );

        let (marginal_intra, marginal_inter) = match (intra_anterior, inter_anterior) {
            (Some((k0, ns0)), Some((k1, ns1))) => (
                (intra_ns - ns0) / f64::from(k - k0),
                (inter_ns - ns1) / f64::from(k - k1),
            ),
            _ => (f64::NAN, f64::NAN),
        };
        let rel = if marginal_intra.is_finite() && marginal_intra > 0.0 {
            (marginal_inter - marginal_intra) / marginal_intra
        } else {
            f64::NAN
        };
        if rel.is_finite() {
            overhead_final = rel;
        }
        println!(
            "{k:<6} {fuel_intra:>8} {intra_ns:>12.1} {inter_ns:>12.1} {rel:>10.4} {disp:>10.4}"
        );
        intra_anterior = Some((k, intra_ns));
        inter_anterior = Some((k, inter_ns));
    }

    let ligacoes = 200_u32;
    let start = Instant::now();
    for _ in 0..ligacoes {
        let _ = rvm_link::link(&[app_externo(64), prov()], 0, 0, &limits).expect("ligação");
    }
    let link_ns = start.elapsed().as_nanos() as f64 / f64::from(ligacoes);

    println!();
    println!("custo de ligacao (k=64): {link_ns:.1} ns por ligacao (uma vez, fora da execucao)");
    println!("dispersao maxima observada (todos os k): {dispersao_maxima:.4}");
    println!("dispersao no ponto classificador (maior k): {dispersao_classificadora:.4}");
    if dispersao_classificadora > 0.10 {
        println!("H06 INCONCLUSIVA: dispersao > 10% no ponto classificador");
    } else if overhead_final <= 0.25 {
        println!("H06 CONFIRMADA: overhead relativo {overhead_final:.4} <= 0,25");
    } else {
        println!("H06 REFUTADA: overhead relativo {overhead_final:.4} > 0,25");
    }
    println!(
        "H03: chamada inter-modulo vira chamada direta por indice apos rvm-link; \
         sem serializacao de argumentos (ver ADR-0018)."
    );
}
