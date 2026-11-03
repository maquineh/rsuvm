//! Construtores de módulos RIR usados em testes de outros crates.
//!
//! Compilado em testes deste crate e em dependentes que ativam a feature
//! `test-helpers`. Não faz parte da API de produção: nada aqui valida limites ou
//! tipos, apenas monta estruturas densas e legíveis.

use rvm_spec::{
    builtin_types, BinOp, BuiltinHost, CmpOp, TypeDef, TypeId, BUILTIN_TYPE_COUNT, IR_VERSION,
    TYPE_BOOL, TYPE_I64, TYPE_STR, TYPE_UNIT,
};

use crate::{
    Block, BlockParam, BranchTarget, EnumDef, EnumFieldDef, EnumVariantDef, Function, Instr,
    Module, Op, Param, Terminator, ValueId,
};

fn bloco(id: u32, label: &str, instrs: Vec<Instr>, term: Terminator) -> Block {
    Block {
        id,
        label: label.to_string(),
        params: Vec::new(),
        instrs,
        term,
    }
}

fn bloco_com_parametros(
    id: u32,
    label: &str,
    params: Vec<BlockParam>,
    instrs: Vec<Instr>,
    term: Terminator,
) -> Block {
    Block {
        id,
        label: label.to_string(),
        params,
        instrs,
        term,
    }
}

fn instr(def: ValueId, op: Op) -> Instr {
    Instr { def, op }
}

fn funcao(id: u32, name: &str, params: Vec<Param>, ret: TypeId, blocks: Vec<Block>) -> Function {
    Function {
        id,
        name: name.to_string(),
        params,
        ret,
        entry: 0,
        blocks,
    }
}

fn modulo(name: &str, entry: u32, functions: Vec<Function>) -> Module {
    Module {
        name: name.to_string(),
        version: IR_VERSION,
        entry,
        types: builtin_types(),
        structs: Vec::new(),
        enums: Vec::new(),
        functions,
    }
}

fn param(name: &str, ty: TypeId) -> Param {
    Param {
        name: name.to_string(),
        ty,
    }
}

/// Módulo `exemplo` com `main` chamando `soma(a, b)`.
///
/// ```text
/// fn main() -> i64 { v0 = const_i64 10; v1 = const_i64 20; v2 = call_static soma(v0, v1); return v2 }
/// fn soma(a: i64, b: i64) -> i64 { v2 = add_i64_checked a, b; return v2 }
/// ```
#[must_use]
pub fn soma() -> Module {
    let main = funcao(
        0,
        "main",
        Vec::new(),
        TYPE_I64,
        vec![bloco(
            0,
            "entry",
            vec![
                instr(0, Op::ConstI64(10)),
                instr(1, Op::ConstI64(20)),
                instr(
                    2,
                    Op::Call {
                        callee: 1,
                        args: vec![0, 1],
                    },
                ),
            ],
            Terminator::Return(Some(2)),
        )],
    );
    let soma = funcao(
        1,
        "soma",
        vec![param("a", TYPE_I64), param("b", TYPE_I64)],
        TYPE_I64,
        vec![bloco(
            0,
            "entry",
            vec![instr(
                2,
                Op::Bin {
                    op: BinOp::AddI64Checked,
                    lhs: 0,
                    rhs: 1,
                },
            )],
            Terminator::Return(Some(2)),
        )],
    );
    modulo("exemplo", 0, vec![main, soma])
}

/// Módulo com uma única função constante `fn main() -> i64`.
#[must_use]
pub fn constante(valor: i64) -> Module {
    let main = funcao(
        0,
        "main",
        Vec::new(),
        TYPE_I64,
        vec![bloco(
            0,
            "entry",
            vec![instr(0, Op::ConstI64(valor))],
            Terminator::Return(Some(0)),
        )],
    );
    modulo("constante", 0, vec![main])
}

/// Função que retorna `unit` sem valor.
#[must_use]
pub fn unidade() -> Module {
    let main = funcao(
        0,
        "main",
        Vec::new(),
        TYPE_UNIT,
        vec![bloco(0, "entry", Vec::new(), Terminator::Return(None))],
    );
    modulo("unidade", 0, vec![main])
}

/// Desvio condicional com dois blocos de retorno distintos.
#[must_use]
pub fn desvio_verdadeiro() -> Module {
    let main = funcao(
        0,
        "main",
        Vec::new(),
        TYPE_I64,
        vec![
            bloco(
                0,
                "entry",
                vec![instr(0, Op::ConstBool(true))],
                Terminator::CondBr {
                    cond: 0,
                    then_target: BranchTarget {
                        target: 1,
                        args: Vec::new(),
                    },
                    else_target: BranchTarget {
                        target: 2,
                        args: Vec::new(),
                    },
                },
            ),
            bloco(
                1,
                "entao",
                vec![instr(1, Op::ConstI64(1))],
                Terminator::Return(Some(1)),
            ),
            bloco(
                2,
                "senao",
                vec![instr(2, Op::ConstI64(2))],
                Terminator::Return(Some(2)),
            ),
        ],
    );
    modulo("desvio", 0, vec![main])
}

/// Merge explícito por parâmetros de bloco.
///
/// ```text
/// fn escolhe(c: bool) -> i64 {
///   entry: v1 = 1; v2 = 2; cond_br c, then(v1), else(v2)
///   then(b0: i64): br merge(b0)
///   else(b1: i64): br merge(b1)
///   merge(x: i64): return x
/// }
/// ```
#[must_use]
pub fn com_merge() -> Module {
    let escolhe = funcao(
        0,
        "escolhe",
        vec![param("c", TYPE_BOOL)],
        TYPE_I64,
        vec![
            bloco(
                0,
                "entry",
                vec![instr(1, Op::ConstI64(1)), instr(2, Op::ConstI64(2))],
                Terminator::CondBr {
                    cond: 0,
                    then_target: BranchTarget {
                        target: 1,
                        args: vec![1],
                    },
                    else_target: BranchTarget {
                        target: 2,
                        args: vec![2],
                    },
                },
            ),
            bloco_com_parametros(
                1,
                "entao",
                vec![BlockParam {
                    value: 3,
                    name: "b0".to_string(),
                    ty: TYPE_I64,
                }],
                Vec::new(),
                Terminator::Br(BranchTarget {
                    target: 3,
                    args: vec![3],
                }),
            ),
            bloco_com_parametros(
                2,
                "senao",
                vec![BlockParam {
                    value: 4,
                    name: "b1".to_string(),
                    ty: TYPE_I64,
                }],
                Vec::new(),
                Terminator::Br(BranchTarget {
                    target: 3,
                    args: vec![4],
                }),
            ),
            bloco_com_parametros(
                3,
                "merge",
                vec![BlockParam {
                    value: 5,
                    name: "escolhido".to_string(),
                    ty: TYPE_I64,
                }],
                Vec::new(),
                Terminator::Return(Some(5)),
            ),
        ],
    );
    modulo("merge", 0, vec![escolhe])
}

/// Loop infinito sem valores: útil para esgotar fuel deterministicamente.
#[must_use]
pub fn loop_infinito() -> Module {
    let main = funcao(
        0,
        "main",
        Vec::new(),
        TYPE_I64,
        vec![
            bloco(
                0,
                "entry",
                Vec::new(),
                Terminator::Br(BranchTarget {
                    target: 1,
                    args: Vec::new(),
                }),
            ),
            bloco(
                1,
                "loop",
                Vec::new(),
                Terminator::Br(BranchTarget {
                    target: 1,
                    args: Vec::new(),
                }),
            ),
        ],
    );
    modulo("loop", 0, vec![main])
}

/// Chamada recursiva infinita: útil para exceder profundidade.
#[must_use]
pub fn recursao() -> Module {
    let main = funcao(
        0,
        "main",
        Vec::new(),
        TYPE_I64,
        vec![bloco(
            0,
            "entry",
            vec![instr(
                0,
                Op::Call {
                    callee: 0,
                    args: Vec::new(),
                },
            )],
            Terminator::Return(Some(0)),
        )],
    );
    modulo("recursao", 0, vec![main])
}

/// Soma `i64::MAX + 1`: trap `integer_overflow` na execução.
///
/// O verificador aceita; só a execução falha.
#[must_use]
pub fn overflow() -> Module {
    let main = funcao(
        0,
        "main",
        Vec::new(),
        TYPE_I64,
        vec![bloco(
            0,
            "entry",
            vec![
                instr(0, Op::ConstI64(i64::MAX)),
                instr(1, Op::ConstI64(1)),
                instr(
                    2,
                    Op::Bin {
                        op: BinOp::AddI64Checked,
                        lhs: 0,
                        rhs: 1,
                    },
                ),
            ],
            Terminator::Return(Some(2)),
        )],
    );
    modulo("overflow", 0, vec![main])
}

/// Comparação com valor esperado: `eq_i64(10, 10)`.
#[must_use]
pub fn comparacao() -> Module {
    let main = funcao(
        0,
        "main",
        Vec::new(),
        TYPE_BOOL,
        vec![bloco(
            0,
            "entry",
            vec![
                instr(0, Op::ConstI64(10)),
                instr(1, Op::ConstI64(10)),
                instr(
                    2,
                    Op::Cmp {
                        op: CmpOp::Eq,
                        lhs: 0,
                        rhs: 1,
                    },
                ),
            ],
            Terminator::Return(Some(2)),
        )],
    );
    modulo("comparacao", 0, vec![main])
}

/// Chamada de host `read_file(path, max)` com retorno `result string string`
/// (ADR-0036).
#[must_use]
pub fn host_read_file() -> Module {
    let mut types = builtin_types();
    types.push(TypeDef::Result(TYPE_STR, TYPE_STR));
    let ret: TypeId = u32::try_from(BUILTIN_TYPE_COUNT).unwrap_or(u32::MAX);
    let le = funcao(
        0,
        "le",
        vec![param("path", TYPE_STR), param("max", TYPE_I64)],
        ret,
        vec![bloco(
            0,
            "entry",
            vec![instr(
                2,
                Op::HostCall {
                    function: BuiltinHost::ReadFile,
                    args: vec![0, 1],
                },
            )],
            Terminator::Return(Some(2)),
        )],
    );
    let mut module = modulo("host", 0, vec![le]);
    module.types = types;
    module
}

/// Enum nominal com variante posicional e variante vazia (ADR-0039).
///
/// ```text
/// enum E { A(i64), B }
/// fn main() -> i64 { v0 = 7; v1 = enum_new E.A v0; v2 = enum_is_variant v1, 0; v3 = enum_payload v1, 0, 0; return v3 }
/// ```
#[must_use]
pub fn enum_simples() -> Module {
    let mut types = builtin_types();
    types.push(TypeDef::Enum(0));
    let enum_ty = u32::try_from(BUILTIN_TYPE_COUNT).unwrap_or(u32::MAX);
    let main = funcao(
        0,
        "main",
        Vec::new(),
        TYPE_I64,
        vec![bloco(
            0,
            "entry",
            vec![
                instr(0, Op::ConstI64(7)),
                instr(
                    1,
                    Op::EnumNew {
                        ty: enum_ty,
                        variant: 0,
                        fields: vec![0],
                    },
                ),
                instr(
                    2,
                    Op::EnumIsVariant {
                        value: 1,
                        variant: 0,
                    },
                ),
                instr(
                    3,
                    Op::EnumPayload {
                        value: 1,
                        variant: 0,
                        field: 0,
                    },
                ),
            ],
            Terminator::Return(Some(3)),
        )],
    );
    let mut module = modulo("enum", 0, vec![main]);
    module.types = types;
    module.enums = vec![EnumDef {
        id: 0,
        name: "E".to_string(),
        variants: vec![
            EnumVariantDef {
                name: "A".to_string(),
                fields: vec![EnumFieldDef {
                    name: None,
                    ty: TYPE_I64,
                }],
            },
            EnumVariantDef {
                name: "B".to_string(),
                fields: Vec::new(),
            },
        ],
    }];
    module
}
