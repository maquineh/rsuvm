//! Adaptador protótipo **Lift → RIR** (ADR-0021).
//!
//! Traduz um subconjunto de programas Lift para a RIR do EXP-0, usando a **HIR
//! pública** do próprio Lift (`lift_sema::analyze`). Fica **fora** do núcleo
//! (ADR-0001): o núcleo RSUVM não importa Lift nem ganha dependências externas.
//!
//! # Subconjunto suportado
//!
//! - funções com parâmetros/retorno em `int` (`i64`), `boolean` (`bool`) ou `void`
//!   (`unit`);
//! - corpo com `let`/`const` e `return` (fluxo reto) e um `if/else` **terminal**
//!   cujos dois ramos terminam em `return`;
//! - expressões: literais `int`/`boolean`, variáveis locais, `+ - *`,
//!   comparações (`== != < <= > >=`), negação unária (`-x`) e chamadas a outras
//!   funções do próprio programa.
//!
//! Fora do subconjunto (`while`, atribuição, `else if`, divisão, `f64`, strings,
//! arrays, objetos, closures, `console.log`, `import`, `not`, `and`, `or`):
//! devolve [`AdapterError::Unsupported`].
//!
//! # Semântica
//!
//! O lowering é **estrutural**: cada `let` vira uma definição de valor SSA; um
//! `if/else` terminal vira `cond_br` com dois blocos que recebem o ambiente atual
//! como parâmetros de bloco (sem phi de junção, pois os ramos retornam). `+ - *`
//! usam `*_i64_checked` (overflow vira trap), como no EXP-0.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeMap;
use std::fmt;

use lift_ast::{BinaryOperator, UnaryOperator};
use lift_sema::{
    HirBlock, HirDeclaration, HirElseBranch, HirExpr, HirExprKind, HirFunction, HirProgram,
    HirStatement, ResolvedType,
};
use rvm_ir::{
    Block, BlockId, BlockParam, BranchTarget, FnId, Function, Instr, Module, Op, Param, Terminator,
    ValueId,
};
use rvm_spec::{BinOp, CmpOp, Type, IR_VERSION};

/// Erro do adaptador.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterError {
    /// O pipeline do Lift (lexer/parser/analisador) falhou.
    LiftPipeline(String),
    /// O programa é válido no Lift, mas usa algo fora do subconjunto do adaptador.
    Unsupported(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LiftPipeline(detail) => write!(formatter, "pipeline do Lift falhou: {detail}"),
            Self::Unsupported(detail) => write!(formatter, "fora do subconjunto: {detail}"),
        }
    }
}

impl std::error::Error for AdapterError {}

/// Traduz código-fonte Lift em um módulo RIR, usando `entry` como função de entrada.
///
/// # Errors
///
/// Devolve [`AdapterError::LiftPipeline`] se o Lift rejeitar a fonte e
/// [`AdapterError::Unsupported`] se o programa usar algo fora do subconjunto.
pub fn lower_source(source: &str, entry: &str) -> Result<Module, AdapterError> {
    let tokens = lift_lexer::tokenize(source)
        .map_err(|error| AdapterError::LiftPipeline(format!("{error:?}")))?;
    let ast = lift_parser::parse(&tokens)
        .map_err(|error| AdapterError::LiftPipeline(format!("{error:?}")))?;
    let hir = lift_sema::analyze(ast)
        .map_err(|error| AdapterError::LiftPipeline(format!("{error:?}")))?;
    lower(&hir, entry)
}

/// Traduz uma HIR do Lift já analisada em um módulo RIR.
///
/// # Errors
///
/// Devolve [`AdapterError::Unsupported`] para declarações, tipos ou expressões
/// fora do subconjunto, ou quando `entry` não existe.
pub fn lower(program: &HirProgram, entry: &str) -> Result<Module, AdapterError> {
    let mut names: BTreeMap<String, FnId> = BTreeMap::new();
    let mut functions: Vec<&HirFunction> = Vec::new();
    for declaration in &program.declarations {
        match declaration {
            HirDeclaration::Function(function) => {
                let id = u32::try_from(functions.len())
                    .map_err(|_| AdapterError::Unsupported("funções demais".to_string()))?;
                if names.insert(function.name.clone(), id).is_some() {
                    return Err(AdapterError::Unsupported(format!(
                        "função duplicada `{}`",
                        function.name
                    )));
                }
                functions.push(function);
            }
            HirDeclaration::Struct(_) | HirDeclaration::Import(_) => {
                return Err(AdapterError::Unsupported(
                    "declaração struct/import de topo".to_string(),
                ));
            }
            HirDeclaration::Statement(_) => {
                return Err(AdapterError::Unsupported(
                    "statement de topo (programa deve declarar apenas funções)".to_string(),
                ));
            }
        }
    }

    let entry_id = names.get(entry).copied().ok_or_else(|| {
        AdapterError::Unsupported(format!("função de entrada `{entry}` não declarada"))
    })?;

    let mut lowered: Vec<Function> = Vec::new();
    for (index, function) in functions.iter().enumerate() {
        let id = u32::try_from(index)
            .map_err(|_| AdapterError::Unsupported("funções demais".to_string()))?;
        lowered.push(lower_function(function, id, &names)?);
    }

    Ok(Module {
        name: "lift_program".to_string(),
        version: IR_VERSION,
        entry: entry_id,
        functions: lowered,
    })
}

/// Mapeia um tipo Lift resolvido para um tipo do subset EXP-0.
fn map_type(ty: &ResolvedType) -> Result<Type, AdapterError> {
    match ty {
        ResolvedType::Int => Ok(Type::I64),
        ResolvedType::Boolean => Ok(Type::Bool),
        ResolvedType::Void => Ok(Type::Unit),
        other => Err(AdapterError::Unsupported(format!("tipo `{other}`"))),
    }
}

/// Estado de lowering de uma função.
struct FuncLower {
    blocks: Vec<Option<Block>>,
    next_value: ValueId,
    next_label: u32,
}

/// Ambiente de nomes locais: nome → (ValueId, tipo RIR).
type Env = BTreeMap<String, (ValueId, Type)>;

impl FuncLower {
    fn new(first_value: ValueId) -> Self {
        Self {
            blocks: Vec::new(),
            next_value: first_value,
            next_label: 0,
        }
    }

    fn new_value(&mut self) -> Result<ValueId, AdapterError> {
        let value = self.next_value;
        self.next_value = self
            .next_value
            .checked_add(1)
            .ok_or_else(|| AdapterError::Unsupported("valores SSA demais".to_string()))?;
        Ok(value)
    }

    fn alloc_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(None);
        u32::try_from(id).unwrap_or(u32::MAX)
    }

    fn next_label(&mut self, base: &str) -> String {
        let label = format!("{base}{}", self.next_label);
        self.next_label = self.next_label.saturating_add(1);
        label
    }

    fn finish_block(
        &mut self,
        id: BlockId,
        label: String,
        params: Vec<BlockParam>,
        instrs: Vec<Instr>,
        term: Terminator,
    ) {
        if let Some(slot) = self
            .blocks
            .get_mut(usize::try_from(id).unwrap_or(usize::MAX))
        {
            *slot = Some(Block {
                id,
                label,
                params,
                instrs,
                term,
            });
        }
    }

    /// Loweriza uma lista de statements em um bloco, até um `return` ou um `if`
    /// terminal.
    fn lower_block(
        &mut self,
        names: &BTreeMap<String, FnId>,
        hir: &HirBlock,
        env: &mut Env,
        block_id: BlockId,
        label: &str,
        params: Vec<BlockParam>,
    ) -> Result<(), AdapterError> {
        let mut instrs: Vec<Instr> = Vec::new();
        let count = hir.statements.len();
        for (index, statement) in hir.statements.iter().enumerate() {
            match statement {
                HirStatement::Variable(declaration) => {
                    let value =
                        self.lower_expr(names, &declaration.initializer, env, &mut instrs)?;
                    env.insert(
                        declaration.name.clone(),
                        (value, map_type(&declaration.ty)?),
                    );
                }
                HirStatement::Return(ret) => {
                    let value = match &ret.value {
                        Some(expression) => {
                            Some(self.lower_expr(names, expression, env, &mut instrs)?)
                        }
                        None => None,
                    };
                    self.finish_block(
                        block_id,
                        label.to_string(),
                        params,
                        instrs,
                        Terminator::Return(value),
                    );
                    return Ok(());
                }
                HirStatement::If(node) => {
                    if index + 1 != count {
                        return Err(AdapterError::Unsupported(
                            "`if` não terminal (código após o if)".to_string(),
                        ));
                    }
                    return self
                        .lower_terminal_if(names, node, env, block_id, label, params, instrs);
                }
                HirStatement::While(_) => {
                    return Err(AdapterError::Unsupported("`while`".to_string()))
                }
                HirStatement::Assignment(_) => {
                    return Err(AdapterError::Unsupported("atribuição".to_string()))
                }
                HirStatement::Expression(_) => {
                    return Err(AdapterError::Unsupported(
                        "statement de expressão".to_string(),
                    ))
                }
            }
        }
        Err(AdapterError::Unsupported(
            "bloco sem `return` terminal".to_string(),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_terminal_if(
        &mut self,
        names: &BTreeMap<String, FnId>,
        node: &lift_sema::HirIf,
        env: &mut Env,
        block_id: BlockId,
        label: &str,
        params: Vec<BlockParam>,
        mut instrs: Vec<Instr>,
    ) -> Result<(), AdapterError> {
        let condition = self.lower_expr(names, &node.condition, env, &mut instrs)?;
        let incoming: Vec<(String, ValueId, Type)> = env
            .iter()
            .map(|(name, (value, ty))| (name.clone(), *value, *ty))
            .collect();
        let args: Vec<ValueId> = incoming.iter().map(|(_, value, _)| *value).collect();

        let then_id = self.alloc_block();
        let else_id = self.alloc_block();
        let then_label = self.next_label("then");
        let else_label = self.next_label("else");

        self.finish_block(
            block_id,
            label.to_string(),
            params,
            instrs,
            Terminator::CondBr {
                cond: condition,
                then_target: BranchTarget {
                    target: then_id,
                    args: args.clone(),
                },
                else_target: BranchTarget {
                    target: else_id,
                    args,
                },
            },
        );

        let else_block = match &node.else_branch {
            Some(HirElseBranch::Block(block)) => block,
            Some(HirElseBranch::ElseIf(_)) => {
                return Err(AdapterError::Unsupported("`else if`".to_string()))
            }
            None => {
                return Err(AdapterError::Unsupported(
                    "`if` sem `else` (ramos precisam retornar)".to_string(),
                ))
            }
        };

        self.lower_branch(names, &node.then_branch, then_id, &then_label, &incoming)?;
        self.lower_branch(names, else_block, else_id, &else_label, &incoming)
    }

    /// Loweriza um ramo recebendo o ambiente como parâmetros de bloco.
    fn lower_branch(
        &mut self,
        names: &BTreeMap<String, FnId>,
        hir: &HirBlock,
        block_id: BlockId,
        label: &str,
        incoming: &[(String, ValueId, Type)],
    ) -> Result<(), AdapterError> {
        let mut params: Vec<BlockParam> = Vec::new();
        let mut env: Env = BTreeMap::new();
        for (index, (name, _, ty)) in incoming.iter().enumerate() {
            let value = self.new_value()?;
            let param_name = format!("bb{block_id}_{index}");
            params.push(BlockParam {
                value,
                name: param_name,
                ty: *ty,
            });
            env.insert(name.clone(), (value, *ty));
        }
        self.lower_block(names, hir, &mut env, block_id, label, params)
    }

    /// Loweriza uma expressão, anexando as instruções ao bloco corrente.
    fn lower_expr(
        &mut self,
        names: &BTreeMap<String, FnId>,
        expression: &HirExpr,
        env: &Env,
        instrs: &mut Vec<Instr>,
    ) -> Result<ValueId, AdapterError> {
        match &expression.kind {
            HirExprKind::IntegerLiteral(value) => {
                let id = self.new_value()?;
                instrs.push(Instr {
                    def: id,
                    op: Op::ConstI64(*value),
                });
                Ok(id)
            }
            HirExprKind::BooleanLiteral(value) => {
                let id = self.new_value()?;
                instrs.push(Instr {
                    def: id,
                    op: Op::ConstBool(*value),
                });
                Ok(id)
            }
            HirExprKind::Local { name, .. } => env
                .get(name)
                .map(|(value, _)| *value)
                .ok_or_else(|| AdapterError::Unsupported(format!("local `{name}` não mapeado"))),
            HirExprKind::Grouping(inner) => self.lower_expr(names, inner, env, instrs),
            HirExprKind::Move(inner)
            | HirExprKind::Clone {
                expression: inner, ..
            } => self.lower_expr(names, inner, env, instrs),
            HirExprKind::Unary { operator, operand } => match operator {
                UnaryOperator::Negate => {
                    let zero = self.new_value()?;
                    instrs.push(Instr {
                        def: zero,
                        op: Op::ConstI64(0),
                    });
                    let value = self.lower_expr(names, operand, env, instrs)?;
                    let id = self.new_value()?;
                    instrs.push(Instr {
                        def: id,
                        op: Op::Bin {
                            op: BinOp::SubI64Checked,
                            lhs: zero,
                            rhs: value,
                        },
                    });
                    Ok(id)
                }
                UnaryOperator::Not => Err(AdapterError::Unsupported("operador `not`".to_string())),
            },
            HirExprKind::Binary {
                left,
                operator,
                right,
            } => {
                let lhs = self.lower_expr(names, left, env, instrs)?;
                let rhs = self.lower_expr(names, right, env, instrs)?;
                let id = self.new_value()?;
                let op = match operator {
                    BinaryOperator::Add => Op::Bin {
                        op: BinOp::AddI64Checked,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Subtract => Op::Bin {
                        op: BinOp::SubI64Checked,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Multiply => Op::Bin {
                        op: BinOp::MulI64Checked,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Equal => Op::Cmp {
                        op: CmpOp::Eq,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::NotEqual => Op::Cmp {
                        op: CmpOp::Ne,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Less => Op::Cmp {
                        op: CmpOp::Lt,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::LessEqual => Op::Cmp {
                        op: CmpOp::Le,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Greater => Op::Cmp {
                        op: CmpOp::Gt,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::GreaterEqual => Op::Cmp {
                        op: CmpOp::Ge,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Divide
                    | BinaryOperator::Remainder
                    | BinaryOperator::Coalesce => {
                        return Err(AdapterError::Unsupported(
                            "divisão/resto/coalesce".to_string(),
                        ))
                    }
                    BinaryOperator::And | BinaryOperator::Or => {
                        return Err(AdapterError::Unsupported("`and`/`or`".to_string()))
                    }
                };
                instrs.push(Instr { def: id, op });
                Ok(id)
            }
            HirExprKind::Call {
                function,
                arguments,
            } => {
                let callee = names.get(function).copied().ok_or_else(|| {
                    AdapterError::Unsupported(format!("chamada a função externa `{function}`"))
                })?;
                let mut args: Vec<ValueId> = Vec::new();
                for argument in arguments {
                    args.push(self.lower_expr(names, &argument.expression, env, instrs)?);
                }
                let id = self.new_value()?;
                instrs.push(Instr {
                    def: id,
                    op: Op::Call { callee, args },
                });
                Ok(id)
            }
            other => Err(AdapterError::Unsupported(format!("expressão {other:?}"))),
        }
    }
}

/// Loweriza uma função Lift para uma função RIR.
fn lower_function(
    function: &HirFunction,
    id: FnId,
    names: &BTreeMap<String, FnId>,
) -> Result<Function, AdapterError> {
    let ret = map_type(&function.return_type)?;
    let mut params: Vec<Param> = Vec::new();
    let mut env: Env = BTreeMap::new();
    for (index, parameter) in function.parameters.iter().enumerate() {
        let ty = map_type(&parameter.ty)?;
        let value = u32::try_from(index)
            .map_err(|_| AdapterError::Unsupported("parâmetros demais".to_string()))?;
        if env.insert(parameter.name.clone(), (value, ty)).is_some() {
            return Err(AdapterError::Unsupported(format!(
                "parâmetro duplicado `{}`",
                parameter.name
            )));
        }
        params.push(Param {
            name: parameter.name.clone(),
            ty,
        });
    }

    let first_value = u32::try_from(function.parameters.len())
        .map_err(|_| AdapterError::Unsupported("parâmetros demais".to_string()))?;
    let mut lower = FuncLower::new(first_value);
    let entry = lower.alloc_block();
    lower.lower_block(names, &function.body, &mut env, entry, "entry", Vec::new())?;

    let mut blocks: Vec<Block> = Vec::with_capacity(lower.blocks.len());
    for slot in lower.blocks {
        match slot {
            Some(block) => blocks.push(block),
            None => {
                return Err(AdapterError::Unsupported(
                    "bloco não preenchido (bug do adaptador)".to_string(),
                ))
            }
        }
    }

    Ok(Function {
        id,
        name: function.name.clone(),
        params,
        ret,
        entry,
        blocks,
    })
}
