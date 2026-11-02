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
//! - `let`/`const`, **atribuição** a binding mutável, `return`, `if/else`
//!   (inclusive com merge) e `while`;
//! - expressões: literais `int`/`boolean`, variáveis locais, `+ - * / %`,
//!   comparações (`== != < <= > >=`), negação unária (`-x`) e chamadas a outras
//!   funções do próprio programa.
//!
//! Fora do subconjunto (`else if`, `coalesce`, `f64`, strings, arrays, objetos,
//! closures, `console.log`, `import`, `not`, `and`, `or`, `struct`, statements de
//! topo): devolve [`AdapterError::Unsupported`]. Divisão/resto por zero e
//! `i64::MIN / -1` produzem os traps `division_by_zero`/`integer_overflow`
//! (ADR-0022), como no núcleo.
//!
//! # Semântica (construção CFG/SSA)
//!
//! Cada binding Lift (`BindingId`) é rastreado para o `ValueId` corrente. `let`
//! vira uma definição SSA; `if/else` e `while` viram blocos com **parâmetros de
//! bloco** que carregam o ambiente (phi explícito): no merge do `if` e no cabeçalho
//! do `while`, o ambiente é passado adiante. `+ - *` usam `*_i64_checked` (overflow
//! vira trap), como no EXP-0.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeMap;
use std::fmt;

use lift_ast::{BinaryOperator, UnaryOperator};
use lift_sema::{
    BindingId, HirDeclaration, HirElseBranch, HirExpr, HirExprKind, HirFunction, HirIf, HirProgram,
    HirStatement, HirWhile, ResolvedType,
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

/// Ambiente de bindings: `BindingId` → (ValueId corrente, tipo RIR).
type Env = BTreeMap<BindingId, (ValueId, Type)>;

/// Bloco em construção (ainda sem terminador).
struct Cursor {
    block: BlockId,
    label: String,
    params: Vec<BlockParam>,
    instrs: Vec<Instr>,
}

/// Construtor de função (CFG/SSA).
struct FuncLower {
    blocks: Vec<Option<Block>>,
    next_value: ValueId,
    next_label: u32,
}

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

    fn finish(&mut self, cursor: Cursor, term: Terminator) {
        if let Some(slot) = self
            .blocks
            .get_mut(usize::try_from(cursor.block).unwrap_or(usize::MAX))
        {
            *slot = Some(Block {
                id: cursor.block,
                label: cursor.label,
                params: cursor.params,
                instrs: cursor.instrs,
                term,
            });
        }
    }

    /// Cria parâmetros de bloco para todo o ambiente e devolve o ambiente do bloco.
    fn params_for(
        &mut self,
        incoming: &Env,
        block_id: BlockId,
    ) -> Result<(Vec<BlockParam>, Env), AdapterError> {
        let mut params: Vec<BlockParam> = Vec::new();
        let mut env: Env = BTreeMap::new();
        for (index, (binding, (_, ty))) in incoming.iter().enumerate() {
            let value = self.new_value()?;
            params.push(BlockParam {
                value,
                name: format!("p{block_id}_{index}"),
                ty: *ty,
            });
            env.insert(*binding, (value, *ty));
        }
        Ok((params, env))
    }

    /// Argumentos de desvio na ordem do ambiente `incoming`, lidos de `values`.
    fn args_of(values: &Env, incoming: &Env) -> Result<Vec<ValueId>, AdapterError> {
        let mut args: Vec<ValueId> = Vec::new();
        for binding in incoming.keys() {
            let value = values
                .get(binding)
                .map(|(value, _)| *value)
                .ok_or_else(|| {
                    AdapterError::Unsupported("binding ausente no desvio (bug)".to_string())
                })?;
            args.push(value);
        }
        Ok(args)
    }

    /// Loweriza uma lista de statements; devolve o cursor vivo (ou `None` se o
    /// fluxo terminou em `return`) e o ambiente de saída.
    fn lower_block(
        &mut self,
        names: &BTreeMap<String, FnId>,
        statements: &[HirStatement],
        mut cursor: Cursor,
        mut env: Env,
    ) -> Result<(Option<Cursor>, Env), AdapterError> {
        for statement in statements {
            match statement {
                HirStatement::Variable(declaration) => {
                    let ty = map_type(&declaration.ty)?;
                    let value =
                        self.lower_expr(names, &declaration.initializer, &env, &mut cursor.instrs)?;
                    env.insert(declaration.id, (value, ty));
                }
                HirStatement::Assignment(assignment) => {
                    let binding = match &assignment.target.kind {
                        HirExprKind::Local { id, .. } => *id,
                        _ => {
                            return Err(AdapterError::Unsupported(
                                "alvo de atribuição não é variável".to_string(),
                            ))
                        }
                    };
                    let ty = env.get(&binding).map(|(_, ty)| *ty).ok_or_else(|| {
                        AdapterError::Unsupported("atribuição a binding não mapeado".to_string())
                    })?;
                    let value =
                        self.lower_expr(names, &assignment.value, &env, &mut cursor.instrs)?;
                    env.insert(binding, (value, ty));
                }
                HirStatement::Expression(statement) => {
                    // Sem efeitos no subset: avalia (pode trapar) e descarta.
                    let _ =
                        self.lower_expr(names, &statement.expression, &env, &mut cursor.instrs)?;
                }
                HirStatement::Return(ret) => {
                    let value = match &ret.value {
                        Some(expression) => {
                            Some(self.lower_expr(names, expression, &env, &mut cursor.instrs)?)
                        }
                        None => None,
                    };
                    self.finish(cursor, Terminator::Return(value));
                    return Ok((None, env));
                }
                HirStatement::If(node) => match self.lower_if(names, node, cursor, env)? {
                    Some((next, next_env)) => {
                        cursor = next;
                        env = next_env;
                    }
                    None => return Ok((None, BTreeMap::new())),
                },
                HirStatement::While(node) => {
                    let (next, next_env) = self.lower_while(names, node, cursor, env)?;
                    cursor = next;
                    env = next_env;
                }
            }
        }
        Ok((Some(cursor), env))
    }

    /// Loweriza um `if/else`, com merge quando os ramos não terminam em `return`.
    fn lower_if(
        &mut self,
        names: &BTreeMap<String, FnId>,
        node: &HirIf,
        mut cursor: Cursor,
        env: Env,
    ) -> Result<Option<(Cursor, Env)>, AdapterError> {
        let condition = self.lower_expr(names, &node.condition, &env, &mut cursor.instrs)?;
        let incoming = env;

        let then_id = self.alloc_block();
        let else_id = self.alloc_block();
        let (then_params, then_env) = self.params_for(&incoming, then_id)?;
        let (else_params, else_env) = self.params_for(&incoming, else_id)?;
        let then_label = self.next_label("then");
        let else_label = self.next_label("else");
        let args = Self::args_of(&incoming, &incoming)?;

        self.finish(
            cursor,
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

        let then_cursor = Cursor {
            block: then_id,
            label: then_label,
            params: then_params,
            instrs: Vec::new(),
        };
        let (then_out, then_env) =
            self.lower_block(names, &node.then_branch.statements, then_cursor, then_env)?;

        let else_cursor = Cursor {
            block: else_id,
            label: else_label,
            params: else_params,
            instrs: Vec::new(),
        };
        let (else_out, else_env) = match &node.else_branch {
            Some(HirElseBranch::Block(block)) => {
                self.lower_block(names, &block.statements, else_cursor, else_env)?
            }
            Some(HirElseBranch::ElseIf(_)) => {
                return Err(AdapterError::Unsupported("`else if`".to_string()))
            }
            // `if` sem `else`: o ramo vazio cai para o merge.
            None => (Some(else_cursor), else_env),
        };

        if then_out.is_none() && else_out.is_none() {
            // Ambos os ramos terminam em `return`: o `if` é terminal.
            return Ok(None);
        }

        let merge_id = self.alloc_block();
        let (merge_params, merge_env) = self.params_for(&incoming, merge_id)?;
        let merge_label = self.next_label("merge");

        if let Some(then_out) = then_out {
            let args = Self::args_of(&then_env, &incoming)?;
            self.finish(
                then_out,
                Terminator::Br(BranchTarget {
                    target: merge_id,
                    args,
                }),
            );
        }
        if let Some(else_out) = else_out {
            let args = Self::args_of(&else_env, &incoming)?;
            self.finish(
                else_out,
                Terminator::Br(BranchTarget {
                    target: merge_id,
                    args,
                }),
            );
        }

        Ok(Some((
            Cursor {
                block: merge_id,
                label: merge_label,
                params: merge_params,
                instrs: Vec::new(),
            },
            merge_env,
        )))
    }

    /// Loweriza um `while`, com variáveis carregadas no cabeçalho (phi).
    fn lower_while(
        &mut self,
        names: &BTreeMap<String, FnId>,
        node: &HirWhile,
        cursor: Cursor,
        env: Env,
    ) -> Result<(Cursor, Env), AdapterError> {
        let header_id = self.alloc_block();
        let (header_params, header_env) = self.params_for(&env, header_id)?;
        let header_label = self.next_label("while");
        let entry_args = Self::args_of(&env, &env)?;
        self.finish(
            cursor,
            Terminator::Br(BranchTarget {
                target: header_id,
                args: entry_args,
            }),
        );

        let mut header_cursor = Cursor {
            block: header_id,
            label: header_label,
            params: header_params,
            instrs: Vec::new(),
        };
        let condition = self.lower_expr(
            names,
            &node.condition,
            &header_env,
            &mut header_cursor.instrs,
        )?;

        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();
        let (body_params, body_env) = self.params_for(&header_env, body_id)?;
        let (exit_params, exit_env) = self.params_for(&header_env, exit_id)?;
        let body_label = self.next_label("do");
        let exit_label = self.next_label("done");
        let header_args = Self::args_of(&header_env, &header_env)?;

        self.finish(
            header_cursor,
            Terminator::CondBr {
                cond: condition,
                then_target: BranchTarget {
                    target: body_id,
                    args: header_args.clone(),
                },
                else_target: BranchTarget {
                    target: exit_id,
                    args: header_args,
                },
            },
        );

        let body_cursor = Cursor {
            block: body_id,
            label: body_label,
            params: body_params,
            instrs: Vec::new(),
        };
        let (body_out, body_env_out) =
            self.lower_block(names, &node.body.statements, body_cursor, body_env)?;
        if let Some(body_out) = body_out {
            let args = Self::args_of(&body_env_out, &header_env)?;
            self.finish(
                body_out,
                Terminator::Br(BranchTarget {
                    target: header_id,
                    args,
                }),
            );
        }

        Ok((
            Cursor {
                block: exit_id,
                label: exit_label,
                params: exit_params,
                instrs: Vec::new(),
            },
            exit_env,
        ))
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
            HirExprKind::Local { id, name } => env
                .get(id)
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
                    BinaryOperator::Divide => Op::Bin {
                        op: BinOp::DivI64Checked,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Remainder => Op::Bin {
                        op: BinOp::RemI64Checked,
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
                    BinaryOperator::Coalesce => {
                        return Err(AdapterError::Unsupported("coalesce".to_string()))
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
        if env.insert(parameter.id, (value, ty)).is_some() {
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
    let entry_cursor = Cursor {
        block: entry,
        label: "entry".to_string(),
        params: Vec::new(),
        instrs: Vec::new(),
    };
    let (cursor, _env) = lower.lower_block(names, &function.body.statements, entry_cursor, env)?;
    if let Some(cursor) = cursor {
        if ret != Type::Unit {
            return Err(AdapterError::Unsupported(
                "função não retorna em todos os caminhos".to_string(),
            ));
        }
        lower.finish(cursor, Terminator::Return(None));
    }

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
