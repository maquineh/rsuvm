#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! Campanha de fuzzing determinística (item 5 do §12).
//!
//! Sem dependências externas não há fuzzer guiado por cobertura (libFuzzer/AFL):
//! esta campanha é um **harness em processo, determinístico e orçado**. Ela cobre
//! três superfícies — parser textual, decoder binário e verificador (com execução
//! limitada do que verifica) — e verifica apenas "não entrou em pânico e
//! terminou". Não prova ausência de bugs nem cobre o espaço de entrada
//! (`docs/fundacao/TESTES/CONFORMIDADE.md` §6).
//!
//! Orçamento declarado: [`SEEDS`] sementes fixas × base × [`MUTACOES`] mutações por
//! base. Registro da rodada em `docs/RELATORIOS/FUZZING-EXP-0.md`.

use std::fs;
use std::path::{Path, PathBuf};

use rvm_ir::{testing, Block, BranchTarget, Function, Instr, Module, Op, Param, Terminator};
use rvm_runtime::{CallDepthLimit, ExecConfig};
use rvm_spec::{builtin_types, BinOp, CmpOp, Limits, TYPE_BOOL, TYPE_I64, TYPE_UNIT};

/// Sementes fixas do xorshift; a campanha é reprodutível e sem relógio.
const SEEDS: [u64; 6] = [
    0x2545_F491_4F6C_DD1D,
    0x9E37_79B9_7F4A_7C15,
    0xD1B5_4A32_D192_ED03,
    0x0123_4567_89AB_CDEF,
    0xA076_1D64_78BD_642F,
    0xE703_7ED1_A0B4_28DB,
];

/// Mutações por base e por semente.
const MUTACOES: u64 = 200;

fn next(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

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

/// Módulos de base: casos válidos do corpus + módulos sintéticos do `testing`.
fn modulos_base() -> Vec<Module> {
    let limits = Limits::default();
    let mut modulos: Vec<Module> = Vec::new();
    for path in case_files("valid") {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("falha ao ler {}: {error}", path.display()));
        match rvm_text::parse(&source, &limits) {
            Ok(module) => modulos.push(module),
            Err(error) => panic!("{}: parse do corpus falhou: {error}", path.display()),
        }
    }
    modulos.push(testing::soma());
    modulos.push(testing::constante(42));
    modulos.push(testing::desvio_verdadeiro());
    modulos.push(testing::com_merge());
    modulos.push(testing::unidade());
    modulos.push(testing::loop_infinito());
    modulos.push(testing::recursao());
    modulos.push(testing::overflow());
    modulos.push(testing::comparacao());
    modulos
}

fn pick<'a, T>(state: &mut u64, items: &'a mut [T]) -> Option<&'a mut T> {
    if items.is_empty() {
        return None;
    }
    let index = (next(state) as usize) % items.len();
    items.get_mut(index)
}

fn tipo(state: &mut u64) -> rvm_spec::TypeId {
    match next(state) % 3 {
        0 => TYPE_UNIT,
        1 => TYPE_BOOL,
        _ => TYPE_I64,
    }
}

fn alvo(state: &mut u64) -> BranchTarget {
    BranchTarget {
        target: (next(state) % 8) as u32,
        args: vec![(next(state) % 64) as u32],
    }
}

fn terminador(state: &mut u64) -> Terminator {
    match next(state) % 3 {
        0 => {
            if next(state) % 2 == 0 {
                Terminator::Return(None)
            } else {
                Terminator::Return(Some((next(state) % 64) as u32))
            }
        }
        1 => Terminator::Br(alvo(state)),
        _ => Terminator::CondBr {
            cond: (next(state) % 64) as u32,
            then_target: alvo(state),
            else_target: alvo(state),
        },
    }
}

fn operacao(state: &mut u64) -> Op {
    match next(state) % 5 {
        0 => Op::ConstI64(next(state) as i64),
        1 => Op::ConstBool(next(state) % 2 == 0),
        2 => Op::Bin {
            op: match next(state) % 3 {
                0 => BinOp::AddI64Checked,
                1 => BinOp::SubI64Checked,
                _ => BinOp::MulI64Checked,
            },
            lhs: (next(state) % 64) as u32,
            rhs: (next(state) % 64) as u32,
        },
        3 => Op::Cmp {
            op: match next(state) % 6 {
                0 => CmpOp::Eq,
                1 => CmpOp::Ne,
                2 => CmpOp::Lt,
                3 => CmpOp::Le,
                4 => CmpOp::Gt,
                _ => CmpOp::Ge,
            },
            lhs: (next(state) % 64) as u32,
            rhs: (next(state) % 64) as u32,
        },
        _ => Op::Call {
            callee: (next(state) % 8) as u32,
            args: vec![(next(state) % 64) as u32],
        },
    }
}

/// Aplica uma mutação estrutural a um módulo (IDs, tipos, terminadores, ops).
fn mutar(module: &mut Module, state: &mut u64) {
    match next(state) % 9 {
        0 => module.version = (next(state) % 4) as u16,
        1 => module.entry = (next(state) % 8) as u32,
        2 => {
            if let Some(function) = pick(state, &mut module.functions) {
                function.ret = tipo(state);
            }
        }
        3 => {
            if let Some(function) = pick(state, &mut module.functions) {
                if let Some(param) = pick(state, &mut function.params) {
                    param.ty = tipo(state);
                }
            }
        }
        4 => {
            if let Some(function) = pick(state, &mut module.functions) {
                if let Some(block) = pick(state, &mut function.blocks) {
                    block.id = (next(state) % 8) as u32;
                }
            }
        }
        5 => {
            if let Some(function) = pick(state, &mut module.functions) {
                if let Some(block) = pick(state, &mut function.blocks) {
                    block.term = terminador(state);
                }
            }
        }
        6 => {
            if let Some(function) = pick(state, &mut module.functions) {
                if let Some(block) = pick(state, &mut function.blocks) {
                    if let Some(instr) = pick(state, &mut block.instrs) {
                        instr.def = (next(state) % 64) as u32;
                    }
                }
            }
        }
        7 => {
            if let Some(function) = pick(state, &mut module.functions) {
                if let Some(block) = pick(state, &mut function.blocks) {
                    if let Some(instr) = pick(state, &mut block.instrs) {
                        instr.op = operacao(state);
                    }
                }
            }
        }
        _ => {
            if let Some(function) = pick(state, &mut module.functions) {
                if let Some(block) = pick(state, &mut function.blocks) {
                    if next(state) % 2 == 0 {
                        let _ = block.instrs.pop();
                    } else {
                        block.instrs.push(Instr {
                            def: (next(state) % 64) as u32,
                            op: operacao(state),
                        });
                    }
                }
            }
        }
    }
}

#[test]
fn campanha_texto_nao_causa_panico() {
    let limits = Limits::default();
    let bases: Vec<Vec<u8>> = case_files("valid")
        .into_iter()
        .map(|path| fs::read(&path).unwrap_or_else(|error| panic!("leitura: {error}")))
        .collect();
    assert!(!bases.is_empty(), "sem casos válidos para mutar");

    let mut total = 0_u64;
    for seed in SEEDS {
        let mut state = seed;
        for base in &bases {
            for _ in 0..MUTACOES {
                let mut bytes = base.clone();
                let changes = 1 + next(&mut state) % 4;
                for _ in 0..changes {
                    if bytes.is_empty() {
                        break;
                    }
                    let index = (next(&mut state) as usize) % bytes.len();
                    if let Some(byte) = bytes.get_mut(index) {
                        *byte = 0x20 + (next(&mut state) % 95) as u8;
                    }
                }
                if let Ok(text) = String::from_utf8(bytes) {
                    let _ = rvm_text::parse(&text, &limits);
                }
                total += 1;
            }
        }
    }
    assert_eq!(
        total,
        SEEDS.len() as u64 * bases.len() as u64 * MUTACOES,
        "orçamento textual não cumprido"
    );
}

#[test]
fn campanha_bytecode_nao_causa_panico() {
    let limits = Limits::default();
    let bases: Vec<Vec<u8>> = modulos_base().iter().map(rvm_bytecode::encode).collect();
    assert!(!bases.is_empty());

    let mut total = 0_u64;
    for seed in SEEDS {
        let mut state = seed;
        for base in &bases {
            for _ in 0..MUTACOES {
                let mut bytes = base.clone();
                let changes = 1 + next(&mut state) % 3;
                for _ in 0..changes {
                    if bytes.is_empty() {
                        break;
                    }
                    let index = (next(&mut state) as usize) % bytes.len();
                    if let Some(byte) = bytes.get_mut(index) {
                        *byte ^= 1 + (next(&mut state) % 255) as u8;
                    }
                }
                let _ = rvm_bytecode::decode(&bytes, &limits);
                total += 1;
            }
        }
    }
    assert_eq!(
        total,
        SEEDS.len() as u64 * bases.len() as u64 * MUTACOES,
        "orçamento de bytecode não cumprido"
    );
}

#[test]
fn campanha_verificador_nao_causa_panico() {
    let limits = Limits::default();
    let bases = modulos_base();
    let config = ExecConfig::new(500, CallDepthLimit::new(32).expect("limite válido"));

    let mut total = 0_u64;
    let mut aceitos = 0_u64;
    for seed in SEEDS {
        let mut state = seed;
        for base in &bases {
            for _ in 0..MUTACOES {
                let mut module = base.clone();
                mutar(&mut module, &mut state);
                if rvm_ir::structure::check(&module, &limits).is_ok() {
                    if let Ok(verified) = rvm_verify::VerifiedModule::verify(module, &limits) {
                        aceitos += 1;
                        // Módulo verificado: executa com orçamento pequeno; o
                        // contrato é "não entra em pânico", não "sucesso".
                        let _ = rvm_interpreter::execute(&verified, verified.entry(), &[], &config);
                    }
                }
                total += 1;
            }
        }
    }
    assert_eq!(
        total,
        SEEDS.len() as u64 * bases.len() as u64 * MUTACOES,
        "orçamento do verificador não cumprido"
    );
    // Sanidade: a mutação precisa produzir módulos que ainda verificam às vezes,
    // senão a campanha só testaria rejeição.
    assert!(
        aceitos > 0,
        "nenhuma mutação foi aceita pelo verificador; campanha degenerada"
    );
    eprintln!(
        "campanha_verificador: total={total} aceitos_pelo_verificador={aceitos} \
         seeds={} bases={} mutacoes={MUTACOES}",
        SEEDS.len(),
        bases.len()
    );
}

/// Módulo provedor com uma função concreta `alvo(a, b) = a + b`.
fn modulo_provedor() -> Module {
    Module {
        name: "prov".to_string(),
        version: rvm_spec::IR_VERSION,
        entry: 0,
        types: builtin_types(),
        structs: Vec::new(),
        functions: vec![Function {
            id: 0,
            name: "alvo".to_string(),
            params: vec![
                Param {
                    name: "a".to_string(),
                    ty: TYPE_I64,
                },
                Param {
                    name: "b".to_string(),
                    ty: TYPE_I64,
                },
            ],
            ret: TYPE_I64,
            entry: 0,
            blocks: vec![Block {
                id: 0,
                label: "entry".to_string(),
                params: Vec::new(),
                instrs: vec![Instr {
                    def: 2,
                    op: Op::Bin {
                        op: BinOp::AddI64Checked,
                        lhs: 0,
                        rhs: 1,
                    },
                }],
                term: Terminator::Return(Some(2)),
            }],
        }],
    }
}

/// Módulo com `main` chamando a externa `alvo` (id 1, sem corpo).
fn modulo_com_import() -> Module {
    Module {
        name: "app".to_string(),
        version: rvm_spec::IR_VERSION,
        entry: 0,
        types: builtin_types(),
        structs: Vec::new(),
        functions: vec![
            Function {
                id: 0,
                name: "main".to_string(),
                params: Vec::new(),
                ret: TYPE_I64,
                entry: 0,
                blocks: vec![Block {
                    id: 0,
                    label: "entry".to_string(),
                    params: Vec::new(),
                    instrs: vec![
                        Instr {
                            def: 0,
                            op: Op::ConstI64(1),
                        },
                        Instr {
                            def: 1,
                            op: Op::ConstI64(2),
                        },
                        Instr {
                            def: 2,
                            op: Op::Call {
                                callee: 1,
                                args: vec![0, 1],
                            },
                        },
                    ],
                    term: Terminator::Return(Some(2)),
                }],
            },
            Function {
                id: 1,
                name: "alvo".to_string(),
                params: Vec::new(),
                ret: TYPE_UNIT,
                entry: 0,
                blocks: Vec::new(),
            },
        ],
    }
}

#[test]
fn campanha_linker_nao_causa_panico() {
    let limits = Limits::default();
    let mut bases = modulos_base();
    bases.push(modulo_com_import());
    let config = ExecConfig::new(300, CallDepthLimit::new(16).expect("limite válido"));

    let mut total = 0_u64;
    let mut ligados = 0_u64;
    for seed in SEEDS {
        let mut state = seed;
        for base in &bases {
            for _ in 0..MUTACOES {
                let mut app = base.clone();
                mutar(&mut app, &mut state);
                let mut prov = modulo_provedor();
                mutar(&mut prov, &mut state);
                if app.name == prov.name {
                    app.name = format!("{}_a", app.name);
                }
                if let Ok(linked) = rvm_link::link(&[app, prov], 0, 0, &limits) {
                    if let Ok(verified) = rvm_verify::VerifiedModule::verify(linked, &limits) {
                        ligados += 1;
                        let _ = rvm_interpreter::execute(&verified, verified.entry(), &[], &config);
                    }
                }
                total += 1;
            }
        }
    }
    assert_eq!(
        total,
        SEEDS.len() as u64 * bases.len() as u64 * MUTACOES,
        "orçamento do linker não cumprido"
    );
    assert!(
        ligados > 0,
        "nenhuma ligação foi aceita; campanha do linker degenerada"
    );
    eprintln!(
        "campanha_linker: total={total} ligados={ligados} seeds={} bases={} mutacoes={MUTACOES}",
        SEEDS.len(),
        bases.len()
    );
}
