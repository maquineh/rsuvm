//! Adaptador protótipo **Lift → RIR** (ADR-0021).
//!
//! Traduz um subconjunto de programas Lift para a RIR do EXP-0, usando a **HIR
//! pública** do próprio Lift (`lift_sema::analyze`). Fica **fora** do núcleo
//! (ADR-0001): o núcleo RSUVM não importa Lift nem ganha dependências externas.
//!
//! # Subconjunto suportado
//!
//! - funções com parâmetros/retorno em `int` (`i64`), `boolean` (`bool`),
//!   `string` (`string`), `number` (`f64`), arrays desses tipos (`array`), structs
//!   (`interface`) com campos escalares, opcionais (`T?`) ou `void` (`unit`);
//! - `let`/`const`, **atribuição** a binding mutável, a **elemento de array**
//!   (`xs[i] = v`) e a **campo** (`obj.campo = v`), `return`, `if`/`else if`/
//!   `else` (com merge) e `while`;
//! - expressões: literais `int`/`boolean`/`string`/`number`, **literais de array**
//!   `[...]`, **indexação** `xs[i]`, **literais de struct** `{ campo: valor }`,
//!   **acesso a campo** `obj.campo`, `null`, `option.some`/`is_some`/`unwrap`, `??`,
//!   `== null`/`!= null`, variáveis locais, `+ - * / %`, comparações
//!   (`== != < <= > >=`; `==`/`!=` de `string` por conteúdo), negação `-x`, lógica
//!   `!`/`&&`/`||` (**curto-circuito**) e chamadas a outras funções do programa.
//!
//! Fora do subconjunto (closures, `console.log`, `import`, statements de topo):
//! devolve [`AdapterError::Unsupported`].
//! Divisão/resto por zero e `i64::MIN / -1` produzem os traps
//! `division_by_zero`/`integer_overflow` (ADR-0022). `xs[i] = v` e `obj.campo = v`
//! só são baixados quando o alvo é um array/struct local **não-aliasado**
//! (ADR-0028/0032); structs aceitam apenas campos escalares (ADR-0029); `??`
//! preserva o fallback preguiçoso via fluxo de controle (ADR-0030).
//!
//! # Semântica (construção CFG/SSA)
//!
//! Cada binding Lift (`BindingId`) é rastreado para o `ValueId` corrente. `let`
//! vira uma definição SSA; `if`/`else` e `while` viram blocos com **parâmetros de
//! bloco** que carregam o ambiente (phi explícito). `&&`, `||` e `!` viram fluxo
//! com merge de um `bool` (curto-circuito). `+ - *` usam `*_i64_checked`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use lift_ast::{BinaryOperator, UnaryOperator};
use lift_sema::{
    BindingId, HirDeclaration, HirElseBranch, HirExpr, HirExprKind, HirFunction, HirIf, HirProgram,
    HirStatement, HirWhile, ResolvedType, OPTION_IS_SOME_BUILTIN, OPTION_UNWRAP_BUILTIN,
};
use rvm_ir::{
    Block, BlockId, BlockParam, BranchTarget, FieldDef, FnId, Function, Instr, Module, Op, Param,
    StructDef, Terminator, ValueId,
};
use rvm_spec::{ArrayElem, BinOp, CmpOp, FloatOp, OptionalElem, StructId, Type, IR_VERSION};

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
    // Nome de struct -> (id, ordem dos campos). Preenchido em duas passadas para
    // permitir referências entre structs (embora campos só aceitem escalares).
    let mut structs: BTreeMap<String, (StructId, Vec<String>)> = BTreeMap::new();
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
            HirDeclaration::Struct(def) => {
                let id = u32::try_from(structs.len())
                    .map_err(|_| AdapterError::Unsupported("structs demais".to_string()))?;
                let order = def.fields.iter().map(|field| field.name.clone()).collect();
                if structs.insert(def.name.clone(), (id, order)).is_some() {
                    return Err(AdapterError::Unsupported(format!(
                        "struct duplicada `{}`",
                        def.name
                    )));
                }
            }
            HirDeclaration::Import(_) => {
                return Err(AdapterError::Unsupported(
                    "declaração import de topo".to_string(),
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

    // Tabela de structs do módulo, com campos escalares resolvidos.
    let mut struct_defs: Vec<StructDef> = Vec::new();
    for declaration in &program.declarations {
        if let HirDeclaration::Struct(def) = declaration {
            let (id, _) = structs
                .get(&def.name)
                .cloned()
                .ok_or_else(|| AdapterError::Unsupported("struct sem id (bug)".to_string()))?;
            let mut fields: Vec<FieldDef> = Vec::new();
            for field in &def.fields {
                let ty = map_type(&field.ty, &structs)?;
                if !is_scalar_type(ty) {
                    return Err(AdapterError::Unsupported(format!(
                        "campo `{}` de `{}` não é escalar",
                        field.name, def.name
                    )));
                }
                fields.push(FieldDef {
                    name: field.name.clone(),
                    ty,
                });
            }
            struct_defs.push(StructDef {
                id,
                name: def.name.clone(),
                fields,
            });
        }
    }

    let mut lowered: Vec<Function> = Vec::new();
    for (index, function) in functions.iter().enumerate() {
        let id = u32::try_from(index)
            .map_err(|_| AdapterError::Unsupported("funções demais".to_string()))?;
        lowered.push(lower_function(function, id, &names, &structs)?);
    }

    Ok(Module {
        name: "lift_program".to_string(),
        version: IR_VERSION,
        entry: entry_id,
        structs: struct_defs,
        functions: lowered,
    })
}

/// `true` para tipos escalares aceitos em campos de struct (ADR-0029).
fn is_scalar_type(ty: Type) -> bool {
    matches!(ty, Type::Bool | Type::I64 | Type::Str | Type::F64)
}

/// Mapeia um tipo Lift resolvido para um tipo do subset EXP-0.
fn map_type(
    ty: &ResolvedType,
    structs: &BTreeMap<String, (StructId, Vec<String>)>,
) -> Result<Type, AdapterError> {
    match ty {
        ResolvedType::Int => Ok(Type::I64),
        ResolvedType::Boolean => Ok(Type::Bool),
        ResolvedType::Void => Ok(Type::Unit),
        ResolvedType::String => Ok(Type::Str),
        ResolvedType::Number => Ok(Type::F64),
        ResolvedType::Struct(name) => {
            let (id, _) = structs.get(name).cloned().ok_or_else(|| {
                AdapterError::Unsupported(format!("struct `{name}` não declarada"))
            })?;
            Ok(Type::Struct(id))
        }
        ResolvedType::Optional(inner) => {
            let inner = map_type(inner, structs)?;
            let elem = OptionalElem::from_type(inner)
                .ok_or_else(|| AdapterError::Unsupported(format!("option de `{inner}`")))?;
            Ok(Type::Optional(elem))
        }
        ResolvedType::Array(inner) => {
            let element = match inner.as_ref() {
                ResolvedType::Int => ArrayElem::I64,
                ResolvedType::Boolean => ArrayElem::Bool,
                ResolvedType::String => ArrayElem::Str,
                ResolvedType::Number => ArrayElem::F64,
                other => return Err(AdapterError::Unsupported(format!("array de `{other}`"))),
            };
            Ok(Type::Array(element))
        }
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

/// Construtor de função (CFG/SSA) com bloco corrente.
struct FuncLower {
    blocks: Vec<Option<Block>>,
    next_value: ValueId,
    next_label: u32,
    current: Option<Cursor>,
    /// Locais comprovadamente não-aliasados (ADR-0028/0032): arrays e structs
    /// declarados por literal que podem receber atribuição funcional a elemento/
    /// campo.
    mutable_locals: BTreeSet<BindingId>,
    /// Structs do programa: nome → (id, ordem dos campos) (ADR-0029).
    structs: BTreeMap<String, (StructId, Vec<String>)>,
}

impl FuncLower {
    fn new(first_value: ValueId) -> Self {
        Self {
            blocks: Vec::new(),
            next_value: first_value,
            next_label: 0,
            current: None,
            mutable_locals: BTreeSet::new(),
            structs: BTreeMap::new(),
        }
    }

    /// Mapeia um tipo Lift resolvido para o tipo RIR.
    fn map_type(&self, ty: &ResolvedType) -> Result<Type, AdapterError> {
        map_type(ty, &self.structs)
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

    fn emit(&mut self, op: Op) -> Result<ValueId, AdapterError> {
        let id = self.new_value()?;
        let cursor = self
            .current
            .as_mut()
            .ok_or_else(|| AdapterError::Unsupported("emit sem bloco corrente".to_string()))?;
        cursor.instrs.push(Instr { def: id, op });
        Ok(id)
    }

    fn terminate(&mut self, term: Terminator) -> Result<(), AdapterError> {
        let cursor = self
            .current
            .take()
            .ok_or_else(|| AdapterError::Unsupported("terminate sem bloco corrente".to_string()))?;
        self.finish(cursor, term);
        Ok(())
    }

    fn start(&mut self, block: BlockId, label: String, params: Vec<BlockParam>) {
        self.current = Some(Cursor {
            block,
            label,
            params,
            instrs: Vec::new(),
        });
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

    /// Loweriza uma lista de statements no bloco corrente.
    ///
    /// Devolve `Some(env)` se o fluxo continua vivo e `None` se terminou em
    /// `return` (ou num `if` terminal).
    fn lower_block(
        &mut self,
        names: &BTreeMap<String, FnId>,
        statements: &[HirStatement],
        mut env: Env,
    ) -> Result<Option<Env>, AdapterError> {
        for statement in statements {
            if self.current.is_none() {
                break;
            }
            match statement {
                HirStatement::Variable(declaration) => {
                    let ty = self.map_type(&declaration.ty)?;
                    let value = self.lower_expr(names, &declaration.initializer, &env)?;
                    env.insert(declaration.id, (value, ty));
                }
                HirStatement::Assignment(assignment) => {
                    match &assignment.target.kind {
                        HirExprKind::Local { id, .. } => {
                            let binding = *id;
                            let ty = env.get(&binding).map(|(_, ty)| *ty).ok_or_else(|| {
                                AdapterError::Unsupported(
                                    "atribuição a binding não mapeado".to_string(),
                                )
                            })?;
                            let value = self.lower_expr(names, &assignment.value, &env)?;
                            env.insert(binding, (value, ty));
                        }
                        HirExprKind::Index { object, index } => {
                            // `xs[i] = v`: atualização funcional (ADR-0028). Só é
                            // baixada quando `xs` é um array local não-aliasado;
                            // caso contrário, `Unsupported` (nunca módulo errado).
                            let binding = place_root_binding(object).ok_or_else(|| {
                                AdapterError::Unsupported(
                                    "atribuição indexada exige array local".to_string(),
                                )
                            })?;
                            if !self.mutable_locals.contains(&binding) {
                                return Err(AdapterError::Unsupported(
                                    "atribuição a elemento de array possivelmente aliasado"
                                        .to_string(),
                                ));
                            }
                            let (array_value, ty) =
                                env.get(&binding).copied().ok_or_else(|| {
                                    AdapterError::Unsupported(
                                        "atribuição indexada a binding não mapeado".to_string(),
                                    )
                                })?;
                            if !matches!(ty, Type::Array(_)) {
                                return Err(AdapterError::Unsupported(
                                    "atribuição indexada a valor que não é array".to_string(),
                                ));
                            }
                            let position = self.lower_expr(names, index, &env)?;
                            let element = self.lower_expr(names, &assignment.value, &env)?;
                            let updated = self.emit(Op::ArraySet {
                                array: array_value,
                                index: position,
                                value: element,
                            })?;
                            env.insert(binding, (updated, ty));
                        }
                        HirExprKind::FieldAccess { object, field } => {
                            // `obj.campo = v`: atualização funcional (ADR-0032). Só é
                            // baixada quando `obj` é uma struct local não-aliasada.
                            let binding = place_root_binding(object).ok_or_else(|| {
                                AdapterError::Unsupported(
                                    "atribuição a campo exige struct local".to_string(),
                                )
                            })?;
                            if !self.mutable_locals.contains(&binding) {
                                return Err(AdapterError::Unsupported(
                                    "atribuição a campo de struct possivelmente aliasada"
                                        .to_string(),
                                ));
                            }
                            let (struct_value, ty) =
                                env.get(&binding).copied().ok_or_else(|| {
                                    AdapterError::Unsupported(
                                        "atribuição a campo de binding não mapeado".to_string(),
                                    )
                                })?;
                            if !matches!(ty, Type::Struct(_)) {
                                return Err(AdapterError::Unsupported(
                                    "atribuição a campo de valor que não é struct".to_string(),
                                ));
                            }
                            let element = self.lower_expr(names, &assignment.value, &env)?;
                            let updated = self.emit(Op::FieldSet {
                                object: struct_value,
                                field: field.clone(),
                                value: element,
                            })?;
                            env.insert(binding, (updated, ty));
                        }
                        _ => {
                            return Err(AdapterError::Unsupported(
                                "alvo de atribuição não é variável, elemento ou campo".to_string(),
                            ))
                        }
                    }
                }
                HirStatement::Expression(statement) => {
                    let _ = self.lower_expr(names, &statement.expression, &env)?;
                }
                HirStatement::Return(ret) => {
                    let value = match &ret.value {
                        Some(expression) => Some(self.lower_expr(names, expression, &env)?),
                        None => None,
                    };
                    self.terminate(Terminator::Return(value))?;
                    return Ok(None);
                }
                HirStatement::If(node) => match self.lower_if(names, node, env)? {
                    Some(next_env) => env = next_env,
                    None => return Ok(None),
                },
                HirStatement::While(node) => {
                    env = self.lower_while(names, node, env)?;
                }
            }
        }
        Ok(if self.current.is_some() {
            Some(env)
        } else {
            None
        })
    }

    /// Loweriza `if`/`else if`/`else` com merge. `None` = fluxo terminou.
    fn lower_if(
        &mut self,
        names: &BTreeMap<String, FnId>,
        node: &HirIf,
        env: Env,
    ) -> Result<Option<Env>, AdapterError> {
        let condition = self.lower_expr(names, &node.condition, &env)?;
        let incoming = env;

        let then_id = self.alloc_block();
        let else_id = self.alloc_block();
        let (then_params, then_env) = self.params_for(&incoming, then_id)?;
        let (else_params, else_env) = self.params_for(&incoming, else_id)?;
        let args = Self::args_of(&incoming, &incoming)?;
        self.terminate(Terminator::CondBr {
            cond: condition,
            then_target: BranchTarget {
                target: then_id,
                args: args.clone(),
            },
            else_target: BranchTarget {
                target: else_id,
                args,
            },
        })?;

        let then_label = self.next_label("then");
        self.start(then_id, then_label, then_params);
        let then_out = self.lower_block(names, &node.then_branch.statements, then_env)?;
        let then_saved = if then_out.is_some() {
            self.current.take()
        } else {
            None
        };

        let else_label = self.next_label("else");
        self.start(else_id, else_label, else_params);
        let else_out = match &node.else_branch {
            Some(HirElseBranch::Block(block)) => {
                self.lower_block(names, &block.statements, else_env)?
            }
            Some(HirElseBranch::ElseIf(nested)) => self.lower_if(names, nested, else_env)?,
            None => Some(else_env),
        };
        let else_saved = if else_out.is_some() {
            self.current.take()
        } else {
            None
        };

        if then_saved.is_none() && else_saved.is_none() {
            return Ok(None);
        }

        let merge_id = self.alloc_block();
        let (merge_params, merge_env) = self.params_for(&incoming, merge_id)?;
        let merge_label = self.next_label("merge");
        if let Some(cursor) = then_saved {
            let env_then = then_out.expect("cursor vivo implica ambiente");
            let args = Self::args_of(&env_then, &incoming)?;
            self.finish(
                cursor,
                Terminator::Br(BranchTarget {
                    target: merge_id,
                    args,
                }),
            );
        }
        if let Some(cursor) = else_saved {
            let env_else = else_out.expect("cursor vivo implica ambiente");
            let args = Self::args_of(&env_else, &incoming)?;
            self.finish(
                cursor,
                Terminator::Br(BranchTarget {
                    target: merge_id,
                    args,
                }),
            );
        }
        self.start(merge_id, merge_label, merge_params);
        Ok(Some(merge_env))
    }

    /// Loweriza `while`, com variáveis carregadas no cabeçalho (phi).
    fn lower_while(
        &mut self,
        names: &BTreeMap<String, FnId>,
        node: &HirWhile,
        env: Env,
    ) -> Result<Env, AdapterError> {
        let header_id = self.alloc_block();
        let (header_params, header_env) = self.params_for(&env, header_id)?;
        let header_label = self.next_label("while");
        let entry_args = Self::args_of(&env, &env)?;
        self.terminate(Terminator::Br(BranchTarget {
            target: header_id,
            args: entry_args,
        }))?;

        self.start(header_id, header_label, header_params);
        let condition = self.lower_expr(names, &node.condition, &header_env)?;

        let body_id = self.alloc_block();
        let exit_id = self.alloc_block();
        let (body_params, body_env) = self.params_for(&header_env, body_id)?;
        let (exit_params, exit_env) = self.params_for(&header_env, exit_id)?;
        let body_label = self.next_label("do");
        let exit_label = self.next_label("done");
        let header_args = Self::args_of(&header_env, &header_env)?;
        self.terminate(Terminator::CondBr {
            cond: condition,
            then_target: BranchTarget {
                target: body_id,
                args: header_args.clone(),
            },
            else_target: BranchTarget {
                target: exit_id,
                args: header_args,
            },
        })?;

        self.start(body_id, body_label, body_params);
        let body_out = self.lower_block(names, &node.body.statements, body_env)?;
        if let Some(body_env_out) = body_out {
            let args = Self::args_of(&body_env_out, &header_env)?;
            self.terminate(Terminator::Br(BranchTarget {
                target: header_id,
                args,
            }))?;
        }

        self.start(exit_id, exit_label, exit_params);
        Ok(exit_env)
    }

    /// `a && b` com curto-circuito: se `a` é falso, o resultado é `false`.
    fn lower_and(
        &mut self,
        names: &BTreeMap<String, FnId>,
        lhs: ValueId,
        rhs: &HirExpr,
        env: &Env,
    ) -> Result<ValueId, AdapterError> {
        self.lower_logical(names, lhs, rhs, env, false)
    }

    /// `a || b` com curto-circuito: se `a` é verdadeiro, o resultado é `true`.
    fn lower_or(
        &mut self,
        names: &BTreeMap<String, FnId>,
        lhs: ValueId,
        rhs: &HirExpr,
        env: &Env,
    ) -> Result<ValueId, AdapterError> {
        self.lower_logical(names, lhs, rhs, env, true)
    }

    fn lower_logical(
        &mut self,
        names: &BTreeMap<String, FnId>,
        lhs: ValueId,
        rhs: &HirExpr,
        env: &Env,
        short_when_true: bool,
    ) -> Result<ValueId, AdapterError> {
        let rhs_id = self.alloc_block();
        let short_id = self.alloc_block();
        let merge_id = self.alloc_block();
        let (then_target, else_target) = if short_when_true {
            (short_id, rhs_id)
        } else {
            (rhs_id, short_id)
        };
        self.terminate(Terminator::CondBr {
            cond: lhs,
            then_target: BranchTarget {
                target: then_target,
                args: Vec::new(),
            },
            else_target: BranchTarget {
                target: else_target,
                args: Vec::new(),
            },
        })?;

        let rhs_label = self.next_label("rhs");
        self.start(rhs_id, rhs_label, Vec::new());
        let rhs_value = self.lower_expr(names, rhs, env)?;
        self.terminate(Terminator::Br(BranchTarget {
            target: merge_id,
            args: vec![rhs_value],
        }))?;

        let short_label = self.next_label("short");
        self.start(short_id, short_label, Vec::new());
        let short_value = self.emit(Op::ConstBool(short_when_true))?;
        self.terminate(Terminator::Br(BranchTarget {
            target: merge_id,
            args: vec![short_value],
        }))?;

        let merge_label = self.next_label("logic");
        let result = self.new_value()?;
        self.start(
            merge_id,
            merge_label,
            vec![BlockParam {
                value: result,
                name: format!("logic{merge_id}"),
                ty: Type::Bool,
            }],
        );
        Ok(result)
    }

    /// `!b` como fluxo: se `b` é verdadeiro, `false`; senão, `true`.
    fn lower_not(
        &mut self,
        names: &BTreeMap<String, FnId>,
        operand: &HirExpr,
        env: &Env,
    ) -> Result<ValueId, AdapterError> {
        let value = self.lower_expr(names, operand, env)?;
        let then_id = self.alloc_block();
        let else_id = self.alloc_block();
        let merge_id = self.alloc_block();
        self.terminate(Terminator::CondBr {
            cond: value,
            then_target: BranchTarget {
                target: then_id,
                args: Vec::new(),
            },
            else_target: BranchTarget {
                target: else_id,
                args: Vec::new(),
            },
        })?;

        let then_label = self.next_label("not_then");
        self.start(then_id, then_label, Vec::new());
        let then_value = self.emit(Op::ConstBool(false))?;
        self.terminate(Terminator::Br(BranchTarget {
            target: merge_id,
            args: vec![then_value],
        }))?;

        let else_label = self.next_label("not_else");
        self.start(else_id, else_label, Vec::new());
        let else_value = self.emit(Op::ConstBool(true))?;
        self.terminate(Terminator::Br(BranchTarget {
            target: merge_id,
            args: vec![else_value],
        }))?;

        let merge_label = self.next_label("not");
        let result = self.new_value()?;
        self.start(
            merge_id,
            merge_label,
            vec![BlockParam {
                value: result,
                name: format!("not{merge_id}"),
                ty: Type::Bool,
            }],
        );
        Ok(result)
    }

    /// Loweriza `option ?? fallback` com **fallback preguiçoso**: o fallback só é
    /// avaliado quando o `option` é `none` (ADR-0030).
    fn lower_coalesce(
        &mut self,
        names: &BTreeMap<String, FnId>,
        option_expr: &HirExpr,
        fallback_expr: &HirExpr,
        env: &Env,
    ) -> Result<ValueId, AdapterError> {
        let ResolvedType::Optional(payload) = &option_expr.ty else {
            return Err(AdapterError::Unsupported(
                "`??` exige `T?` à esquerda".to_string(),
            ));
        };
        let payload_ty = self.map_type(payload)?;
        let result_ty = self.map_type(&fallback_expr.ty)?;
        let option = self.lower_expr(names, option_expr, env)?;
        let is_some = self.emit(Op::OptionIsSome { value: option })?;

        let some_id = self.alloc_block();
        let none_id = self.alloc_block();
        let merge_id = self.alloc_block();
        self.terminate(Terminator::CondBr {
            cond: is_some,
            then_target: BranchTarget {
                target: some_id,
                args: Vec::new(),
            },
            else_target: BranchTarget {
                target: none_id,
                args: Vec::new(),
            },
        })?;

        let some_label = self.next_label("some");
        self.start(some_id, some_label, Vec::new());
        // Modo `Value`: extrai o payload. Modo `Optional`: repassa o próprio option.
        let some_value = if result_ty == payload_ty {
            self.emit(Op::OptionUnwrap { value: option })?
        } else {
            option
        };
        self.terminate(Terminator::Br(BranchTarget {
            target: merge_id,
            args: vec![some_value],
        }))?;

        let none_label = self.next_label("none");
        self.start(none_id, none_label, Vec::new());
        let fallback_value = self.lower_expr(names, fallback_expr, env)?;
        self.terminate(Terminator::Br(BranchTarget {
            target: merge_id,
            args: vec![fallback_value],
        }))?;

        let merge_label = self.next_label("coalesce");
        let result = self.new_value()?;
        self.start(
            merge_id,
            merge_label,
            vec![BlockParam {
                value: result,
                name: format!("coalesce{merge_id}"),
                ty: result_ty,
            }],
        );
        Ok(result)
    }

    /// Loweriza uma expressão no bloco corrente (pode criar blocos).
    fn lower_expr(
        &mut self,
        names: &BTreeMap<String, FnId>,
        expression: &HirExpr,
        env: &Env,
    ) -> Result<ValueId, AdapterError> {
        match &expression.kind {
            HirExprKind::IntegerLiteral(value) => self.emit(Op::ConstI64(*value)),
            HirExprKind::BooleanLiteral(value) => self.emit(Op::ConstBool(*value)),
            HirExprKind::StringLiteral(text) => self.emit(Op::ConstStr(text.clone())),
            HirExprKind::NumberLiteral(value) => self.emit(Op::ConstF64(value.to_bits())),
            HirExprKind::ArrayLiteral(items) => {
                let mut values: Vec<ValueId> = Vec::new();
                for item in items {
                    values.push(self.lower_expr(names, item, env)?);
                }
                self.emit(Op::ArrayLiteral(values))
            }
            HirExprKind::Index { object, index } => {
                let array = self.lower_expr(names, object, env)?;
                let position = self.lower_expr(names, index, env)?;
                self.emit(Op::ArrayGet {
                    array,
                    index: position,
                })
            }
            HirExprKind::StructLiteral {
                struct_name,
                fields,
            } => {
                let (struct_id, order) =
                    self.structs.get(struct_name).cloned().ok_or_else(|| {
                        AdapterError::Unsupported(format!("struct `{struct_name}` não declarada"))
                    })?;
                let mut values: BTreeMap<String, ValueId> = BTreeMap::new();
                for field in fields {
                    let value = self.lower_expr(names, &field.value, env)?;
                    values.insert(field.name.clone(), value);
                }
                let mut ordered: Vec<ValueId> = Vec::new();
                for name in &order {
                    let value = values.get(name).copied().ok_or_else(|| {
                        AdapterError::Unsupported(format!(
                            "campo `{name}` ausente no literal de `{struct_name}`"
                        ))
                    })?;
                    ordered.push(value);
                }
                self.emit(Op::StructNew {
                    struct_id,
                    fields: ordered,
                })
            }
            HirExprKind::FieldAccess { object, field } => {
                let object_value = self.lower_expr(names, object, env)?;
                self.emit(Op::FieldGet {
                    object: object_value,
                    field: field.clone(),
                })
            }
            HirExprKind::OptionNone => {
                let Type::Optional(elem) = self.map_type(&expression.ty)? else {
                    return Err(AdapterError::Unsupported(
                        "`null` sem tipo option".to_string(),
                    ));
                };
                self.emit(Op::OptionNone(elem))
            }
            HirExprKind::OptionSome(payload) => {
                let value = self.lower_expr(names, payload, env)?;
                self.emit(Op::OptionSome { value })
            }
            HirExprKind::Coalesce {
                option, fallback, ..
            } => self.lower_coalesce(names, option, fallback, env),
            HirExprKind::Local { id, name } => env
                .get(id)
                .map(|(value, _)| *value)
                .ok_or_else(|| AdapterError::Unsupported(format!("local `{name}` não mapeado"))),
            HirExprKind::Grouping(inner) => self.lower_expr(names, inner, env),
            HirExprKind::Move(inner)
            | HirExprKind::Clone {
                expression: inner, ..
            } => self.lower_expr(names, inner, env),
            HirExprKind::Unary { operator, operand } => match operator {
                UnaryOperator::Negate => {
                    if matches!(operand.ty, ResolvedType::Number) {
                        let zero = self.emit(Op::ConstF64(0.0_f64.to_bits()))?;
                        let value = self.lower_expr(names, operand, env)?;
                        self.emit(Op::Float {
                            op: FloatOp::Sub,
                            lhs: zero,
                            rhs: value,
                        })
                    } else {
                        let zero = self.emit(Op::ConstI64(0))?;
                        let value = self.lower_expr(names, operand, env)?;
                        self.emit(Op::Bin {
                            op: BinOp::SubI64Checked,
                            lhs: zero,
                            rhs: value,
                        })
                    }
                }
                UnaryOperator::Not => self.lower_not(names, operand, env),
            },
            HirExprKind::Binary {
                left,
                operator,
                right,
            } => {
                if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                    let lhs = self.lower_expr(names, left, env)?;
                    return if matches!(operator, BinaryOperator::And) {
                        self.lower_and(names, lhs, right, env)
                    } else {
                        self.lower_or(names, lhs, right, env)
                    };
                }
                if matches!(left.ty, ResolvedType::Optional(_))
                    || matches!(right.ty, ResolvedType::Optional(_))
                {
                    let lhs = self.lower_expr(names, left, env)?;
                    let rhs = self.lower_expr(names, right, env)?;
                    return match operator {
                        BinaryOperator::Equal => self.emit(Op::OptionEq { lhs, rhs }),
                        BinaryOperator::NotEqual => self.emit(Op::OptionNe { lhs, rhs }),
                        _ => Err(AdapterError::Unsupported(
                            "operação de option não suportada".to_string(),
                        )),
                    };
                }
                let lhs = self.lower_expr(names, left, env)?;
                let rhs = self.lower_expr(names, right, env)?;
                let string_operands = matches!(left.ty, ResolvedType::String);
                let number_operands = matches!(left.ty, ResolvedType::Number);
                let op = match operator {
                    BinaryOperator::Add if string_operands => Op::ConcatStr { lhs, rhs },
                    BinaryOperator::Add if number_operands => Op::Float {
                        op: FloatOp::Add,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Add => Op::Bin {
                        op: BinOp::AddI64Checked,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Subtract if number_operands => Op::Float {
                        op: FloatOp::Sub,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Subtract => Op::Bin {
                        op: BinOp::SubI64Checked,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Multiply if number_operands => Op::Float {
                        op: FloatOp::Mul,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Multiply => Op::Bin {
                        op: BinOp::MulI64Checked,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Divide if number_operands => Op::Float {
                        op: FloatOp::Div,
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
                    BinaryOperator::Equal if string_operands => Op::StrEq { lhs, rhs },
                    BinaryOperator::NotEqual if string_operands => Op::StrNe { lhs, rhs },
                    BinaryOperator::Equal if number_operands => Op::Float {
                        op: FloatOp::Eq,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::NotEqual if number_operands => Op::Float {
                        op: FloatOp::Ne,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Less if number_operands => Op::Float {
                        op: FloatOp::Lt,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::LessEqual if number_operands => Op::Float {
                        op: FloatOp::Le,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::Greater if number_operands => Op::Float {
                        op: FloatOp::Gt,
                        lhs,
                        rhs,
                    },
                    BinaryOperator::GreaterEqual if number_operands => Op::Float {
                        op: FloatOp::Ge,
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
                    BinaryOperator::And | BinaryOperator::Or => unreachable!("tratado acima"),
                };
                self.emit(op)
            }
            HirExprKind::Call {
                function,
                arguments,
            } => {
                if function == OPTION_IS_SOME_BUILTIN || function == OPTION_UNWRAP_BUILTIN {
                    let argument = arguments.first().ok_or_else(|| {
                        AdapterError::Unsupported(format!("`{function}` sem argumento"))
                    })?;
                    if arguments.len() != 1 {
                        return Err(AdapterError::Unsupported(format!(
                            "`{function}` espera exatamente um argumento"
                        )));
                    }
                    let value = self.lower_expr(names, &argument.expression, env)?;
                    let op = if function == OPTION_IS_SOME_BUILTIN {
                        Op::OptionIsSome { value }
                    } else {
                        Op::OptionUnwrap { value }
                    };
                    return self.emit(op);
                }
                let callee = names.get(function).copied().ok_or_else(|| {
                    AdapterError::Unsupported(format!("chamada a função externa `{function}`"))
                })?;
                let mut args: Vec<ValueId> = Vec::new();
                for argument in arguments {
                    args.push(self.lower_expr(names, &argument.expression, env)?);
                }
                self.emit(Op::Call { callee, args })
            }
            other => Err(AdapterError::Unsupported(format!("expressão {other:?}"))),
        }
    }
}

/// Raiz de binding de um "lugar" (objeto de indexação ou acesso a campo),
/// desembrulhando agrupamentos, `Move`, `Clone`, `FieldAccess` e `Index`.
fn place_root_binding(expression: &HirExpr) -> Option<BindingId> {
    match &expression.kind {
        HirExprKind::Local { id, .. } => Some(*id),
        HirExprKind::Grouping(inner) | HirExprKind::Move(inner) => place_root_binding(inner),
        HirExprKind::Clone { expression, .. } => place_root_binding(expression),
        HirExprKind::FieldAccess { object, .. } | HirExprKind::Index { object, .. } => {
            place_root_binding(object)
        }
        _ => None,
    }
}

/// `true` se a expressão é um literal de array ou struct (possivelmente agrupado).
fn initializer_is_literal(expression: &HirExpr) -> bool {
    match &expression.kind {
        HirExprKind::ArrayLiteral(_) | HirExprKind::StructLiteral { .. } => true,
        HirExprKind::Grouping(inner) => initializer_is_literal(inner),
        _ => false,
    }
}

/// Coleta locais de array/struct declarados por um literal na função.
fn collect_literal_locals(statements: &[HirStatement], out: &mut BTreeSet<BindingId>) {
    for statement in statements {
        match statement {
            HirStatement::Variable(declaration) => {
                if matches!(
                    declaration.ty,
                    ResolvedType::Array(_) | ResolvedType::Struct(_)
                ) && initializer_is_literal(&declaration.initializer)
                {
                    out.insert(declaration.id);
                }
            }
            HirStatement::If(node) => collect_literal_locals_if(node, out),
            HirStatement::While(node) => {
                collect_literal_locals(&node.body.statements, out);
            }
            HirStatement::Assignment(_) | HirStatement::Expression(_) | HirStatement::Return(_) => {
            }
        }
    }
}

fn collect_literal_locals_if(node: &HirIf, out: &mut BTreeSet<BindingId>) {
    collect_literal_locals(&node.then_branch.statements, out);
    match &node.else_branch {
        Some(HirElseBranch::Block(block)) => {
            collect_literal_locals(&block.statements, out);
        }
        Some(HirElseBranch::ElseIf(nested)) => collect_literal_locals_if(nested, out),
        None => {}
    }
}

/// Marca os bindings usados como **valor** (qualquer uso que não seja um "lugar":
/// objeto de indexação ou acesso a campo). Um local que apareça aqui escapa e não
/// pode ser mutado com segurança sob semântica funcional.
fn scan_block(statements: &[HirStatement], escaping: &mut BTreeSet<BindingId>) {
    for statement in statements {
        scan_statement(statement, escaping);
    }
}

fn scan_statement(statement: &HirStatement, escaping: &mut BTreeSet<BindingId>) {
    match statement {
        HirStatement::Variable(declaration) => scan_expr(&declaration.initializer, escaping),
        HirStatement::Assignment(assignment) => {
            scan_expr(&assignment.target, escaping);
            scan_expr(&assignment.value, escaping);
        }
        HirStatement::Expression(expression) => scan_expr(&expression.expression, escaping),
        HirStatement::Return(ret) => {
            if let Some(value) = &ret.value {
                scan_expr(value, escaping);
            }
        }
        HirStatement::If(node) => scan_if(node, escaping),
        HirStatement::While(node) => {
            scan_expr(&node.condition, escaping);
            scan_block(&node.body.statements, escaping);
        }
    }
}

fn scan_if(node: &HirIf, escaping: &mut BTreeSet<BindingId>) {
    scan_expr(&node.condition, escaping);
    scan_block(&node.then_branch.statements, escaping);
    match &node.else_branch {
        Some(HirElseBranch::Block(block)) => scan_block(&block.statements, escaping),
        Some(HirElseBranch::ElseIf(nested)) => scan_if(nested, escaping),
        None => {}
    }
}

fn scan_expr(expression: &HirExpr, escaping: &mut BTreeSet<BindingId>) {
    match &expression.kind {
        // O objeto de uma indexação é um "lugar": não é uso de valor.
        HirExprKind::Index { object, index } => {
            if place_root_binding(object).is_none() {
                scan_expr(object, escaping);
            }
            scan_expr(index, escaping);
        }
        // O objeto de um acesso a campo também é um "lugar".
        HirExprKind::FieldAccess { object, .. } => {
            if place_root_binding(object).is_none() {
                scan_expr(object, escaping);
            }
        }
        HirExprKind::Local { id, .. } => {
            escaping.insert(*id);
        }
        HirExprKind::Grouping(inner) | HirExprKind::Move(inner) => scan_expr(inner, escaping),
        HirExprKind::Clone { expression, .. } => scan_expr(expression, escaping),
        HirExprKind::Unary { operand, .. } => scan_expr(operand, escaping),
        HirExprKind::Binary { left, right, .. } => {
            scan_expr(left, escaping);
            scan_expr(right, escaping);
        }
        HirExprKind::Call { arguments, .. } => {
            for argument in arguments {
                scan_expr(&argument.expression, escaping);
            }
        }
        HirExprKind::ArrayLiteral(items) => {
            for item in items {
                scan_expr(item, escaping);
            }
        }
        HirExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                scan_expr(&field.value, escaping);
            }
        }
        HirExprKind::OptionSome(payload) => scan_expr(payload, escaping),
        HirExprKind::Coalesce {
            option, fallback, ..
        } => {
            scan_expr(option, escaping);
            scan_expr(fallback, escaping);
        }
        HirExprKind::Closure { captures, .. } => {
            for (_, value) in captures {
                scan_expr(value, escaping);
            }
        }
        HirExprKind::ClosureCall { callee, arguments } => {
            scan_expr(callee, escaping);
            for argument in arguments {
                scan_expr(argument, escaping);
            }
        }
        HirExprKind::IntegerLiteral(_)
        | HirExprKind::NumberLiteral(_)
        | HirExprKind::StringLiteral(_)
        | HirExprKind::BooleanLiteral(_)
        | HirExprKind::OptionNone => {}
    }
}

/// Locais de array/struct **não-aliasados** (ADR-0028/0032): declarados por
/// literal e nunca usados como valor. Podem ser atualizados por
/// `array_set`/`field_set` + religação.
fn collect_mutable_locals(function: &HirFunction) -> BTreeSet<BindingId> {
    let mut candidates: BTreeSet<BindingId> = BTreeSet::new();
    collect_literal_locals(&function.body.statements, &mut candidates);
    let mut escaping: BTreeSet<BindingId> = BTreeSet::new();
    scan_block(&function.body.statements, &mut escaping);
    candidates.retain(|id| !escaping.contains(id));
    candidates
}

/// Loweriza uma função Lift para uma função RIR.
fn lower_function(
    function: &HirFunction,
    id: FnId,
    names: &BTreeMap<String, FnId>,
    structs: &BTreeMap<String, (StructId, Vec<String>)>,
) -> Result<Function, AdapterError> {
    let ret = map_type(&function.return_type, structs)?;
    let mut params: Vec<Param> = Vec::new();
    let mut env: Env = BTreeMap::new();
    for (index, parameter) in function.parameters.iter().enumerate() {
        let ty = map_type(&parameter.ty, structs)?;
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
    lower.mutable_locals = collect_mutable_locals(function);
    lower.structs = structs.clone();
    let entry = lower.alloc_block();
    lower.start(entry, "entry".to_string(), Vec::new());
    let out = lower.lower_block(names, &function.body.statements, env)?;
    if out.is_some() {
        if ret != Type::Unit {
            return Err(AdapterError::Unsupported(
                "função não retorna em todos os caminhos".to_string(),
            ));
        }
        lower.terminate(Terminator::Return(None))?;
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
