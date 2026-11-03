//! `rvm-interpreter` — executor de referência da RIR verificada (EXP-0).
//!
//! Execução single-thread e determinística no caminho puro. O executor só aceita
//! [`rvm_verify::VerifiedModule`]; [`execute`] usa `NoHost` e nega qualquer efeito,
//! enquanto [`execute_with_host`] permite um backend explícito para
//! `host_read_file` (ADR-0036).
//!
//! # Semântica (contrato RIR-EXP-0, seção 4)
//!
//! - Cada instrução e cada terminador **despachado** custa 1 unidade de fuel. O
//!   débito acontece **antes** do despacho: saldo 0 produz `out_of_fuel` sem
//!   executar a operação. Chamadas não resetam o saldo e
//!   [`RunOutcome::fuel_used`] é `fuel inicial - saldo restante`.
//! - O frame de entrada tem profundidade 1. Entrar em um callee quando
//!   `profundidade + 1 > config.max_call_depth()` produz `call_depth_exceeded`.
//!   [`RunOutcome::max_depth`] registra a maior profundidade observada.
//! - Cada frame é um vetor de [`rvm_ir::Function::value_count`] slots. Slot
//!   ausente, não inicializado ou de tipo divergente vira `invalid_module`
//!   (defesa em profundidade: o verificador deveria impedir esses casos) —
//!   nunca pânico.
//! - A quota de memória modelada cobra o custo do frame **e** o buffer
//!   transitório de argumentos de chamada/desvio enquanto ele existe
//!   (ADR-0016); [`RunOutcome::memory_peak`] registra o pico observado.
//! - `add/sub/mul/div/rem_i64_checked` e os bit a bit `and/or/xor_i64` e
//!   deslocamentos `shl/shr_i64_checked` usam [`rvm_spec::BinOp::eval_i64`];
//!   resultado fora de `i64` produz o trap `integer_overflow`, divisão/resto por
//!   zero produz `division_by_zero` (ADR-0022) e deslocamento fora de `0..=63`
//!   produz `invalid_shift` (ADR-0023). Comparações usam
//!   [`rvm_spec::CmpOp::eval_i64`] e produzem `bool`.
//! - `br`/`cond_br` copiam os argumentos para os slots dos parâmetros do bloco
//!   alvo antes do salto (phi explícito); `return` sem valor produz
//!   [`Value::Unit`].
//! - Erros de execução carregam [`rvm_spec::Location`] de função, bloco ou
//!   instrução; os códigos são os de [`rvm_spec::ErrorCode`] (trap para overflow,
//!   limite de recurso para fuel/profundidade).
//!
//! # Máquina de frames explícita
//!
//! A execução usa uma pilha de frames na **heap** (`Frame`), não a pilha do
//! processo hospedeiro. Uma versão recursiva anterior estourou a pilha de uma
//! thread de teste com 256 frames (ver ADR-0012): profundidade de chamadas é
//! entrada não confiável e não pode decidir o uso de pilha nativa do host.
//! Execuções repetidas não compartilham estado; todo o estado nasce dentro de
//! [`execute`].

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

use rvm_ir::{BlockId, FnId, Function, Module, Op, Terminator, ValueId};
use rvm_runtime::{
    arg_buffer_cost, frame_cost, ExecConfig, Host, NoHost, Value, MAX_VALUE_DEPTH, SLOT_BYTES,
};
use rvm_spec::{
    BinOp, BuiltinHost, ErrorCode, Location, RvmError, TypeDef, TypeId, TYPE_F64, TYPE_STR,
};

use rvm_verify::VerifiedModule;

/// Resultado de uma execução bem-sucedida.
#[derive(Clone, Debug, PartialEq)]
pub struct RunOutcome {
    /// Valor devolvido pela função de entrada.
    pub value: Value,
    /// Fuel consumido (uma unidade por instrução/terminador despachado).
    pub fuel_used: u64,
    /// Maior profundidade de chamadas observada (frame de entrada = 1).
    pub max_depth: u32,
    /// Pico de memória modelada dos frames e buffers de argumentos, em bytes.
    pub memory_peak: u64,
}

/// Executa `entry` com `args` sob as quotas de `config`.
///
/// A chamada do host é validada antes de qualquer débito de fuel: a função de
/// entrada precisa existir e a aridade/tipos de `args` precisam casar com os
/// `params` da função.
///
/// # Errors
///
/// - [`ErrorCode::InvalidModule`] se `entry` não existir no módulo, ou se um
///   invariante interno do frame for violado (slot ausente, não inicializado ou
///   de tipo divergente);
/// - [`ErrorCode::InvalidType`] se a quantidade ou o tipo dos argumentos não
///   casar com a assinatura da função de entrada (erro de uso do host);
/// - [`ErrorCode::OutOfFuel`] quando o saldo se esgota antes de um despacho;
/// - [`ErrorCode::CallDepthExceeded`] quando a chamada excederia
///   [`ExecConfig::max_call_depth`];
/// - [`ErrorCode::IntegerOverflow`] quando uma operação aritmética transborda.
pub fn execute(
    module: &VerifiedModule,
    entry: FnId,
    args: &[Value],
    config: &ExecConfig,
) -> Result<RunOutcome, RvmError> {
    let mut host = NoHost::new();
    execute_with_host(module, entry, args, config, &mut host)
}

/// Executa com um backend de host explícito (ADR-0036).
///
/// O host só é consultado em `host_call`; as demais operações continuam puras. A
/// capability é conferida **antes** de chamar o host; sem concessão, a execução
/// falha com `capability_denied`.
///
/// # Errors
///
/// As mesmas de [`execute`], mais `capability_denied` quando o host nega a
/// chamada e os erros específicos do backend.
pub fn execute_with_host(
    module: &VerifiedModule,
    entry: FnId,
    args: &[Value],
    config: &ExecConfig,
    host: &mut dyn Host,
) -> Result<RunOutcome, RvmError> {
    let module = module.module();
    let function = module.function(entry).ok_or_else(|| {
        error_at(
            ErrorCode::InvalidModule,
            format!("função de entrada {entry} não existe no módulo"),
            &Site::Module,
        )
    })?;
    validate_host_call(function, args)?;

    let mut interpreter = Interpreter {
        module,
        host,
        remaining_fuel: config.fuel(),
        initial_fuel: config.fuel(),
        max_call_depth: u32::from(config.max_call_depth().get()),
        observed_depth: 0,
        memory_limit: config.memory_limit().get(),
        memory_live: 0,
        memory_peak: 0,
        max_string_bytes: config.max_string_bytes(),
    };
    let value = interpreter.run(function, args)?;
    Ok(RunOutcome {
        value,
        fuel_used: config.fuel().saturating_sub(interpreter.remaining_fuel),
        max_depth: interpreter.observed_depth,
        memory_peak: interpreter.memory_peak,
    })
}

/// Valida a chamada do host contra a assinatura da função de entrada.
fn validate_host_call(function: &Function, args: &[Value]) -> Result<(), RvmError> {
    let site = Site::Function(&function.name);
    if args.len() != function.params.len() {
        return Err(error_at(
            ErrorCode::InvalidType,
            format!(
                "chamada do host para `{name}` espera {expected} argumento(s), recebeu {actual}",
                name = function.name,
                expected = function.params.len(),
                actual = args.len()
            ),
            &site,
        ));
    }
    for (param, value) in function.params.iter().zip(args.iter()) {
        if !value.matches_type(param.ty) {
            return Err(error_at(
                ErrorCode::InvalidType,
                format!(
                    "argumento do parâmetro `{name}` deveria ser `{expected}`, recebeu `{actual}`",
                    name = param.name,
                    expected = param.ty,
                    actual = value.ty()
                ),
                &site,
            ));
        }
    }
    Ok(())
}

/// Um frame de execução: função, slots, bloco atual e posição da instrução.
struct Frame<'a> {
    function: &'a Function,
    depth: u32,
    slots: Vec<Option<Value>>,
    /// Bytes modelados retidos por cada slot (ADR-0035).
    slot_bytes: Vec<u64>,
    block: BlockId,
    instr_index: usize,
    /// Slot do frame chamador onde o retorno deve ser gravado; `None` no frame de
    /// entrada.
    return_slot: Option<ValueId>,
    /// Custo modelado do frame, cobrado na criação e liberado no retorno.
    cost: u64,
}

/// Estado de uma execução; criado do zero por [`execute`].
struct Interpreter<'a> {
    module: &'a Module,
    host: &'a mut dyn Host,
    remaining_fuel: u64,
    initial_fuel: u64,
    max_call_depth: u32,
    observed_depth: u32,
    memory_limit: u64,
    memory_live: u64,
    memory_peak: u64,
    max_string_bytes: u32,
}

impl<'a> Interpreter<'a> {
    /// Executa `function` até o retorno, com uma pilha de frames na heap.
    fn run(&mut self, function: &'a Function, args: &[Value]) -> Result<Value, RvmError> {
        let mut stack: Vec<Frame<'a>> = Vec::new();
        stack.push(self.new_frame(function, args, 1, None)?);

        loop {
            let top = match stack.len().checked_sub(1) {
                Some(top) => top,
                None => {
                    return Err(error_at(
                        ErrorCode::InvalidModule,
                        "pilha de frames vazia durante a execução",
                        &Site::Module,
                    ))
                }
            };
            // Metadados copiados para não manter empréstimo ativo.
            let (function, block_id, instr_index, depth) = match stack.get(top) {
                Some(frame) => (frame.function, frame.block, frame.instr_index, frame.depth),
                None => {
                    return Err(error_at(
                        ErrorCode::InvalidModule,
                        "frame ausente na pilha de execução",
                        &Site::Module,
                    ))
                }
            };
            let block = match function.block(block_id) {
                Some(block) => block,
                None => {
                    return Err(error_at(
                        ErrorCode::InvalidModule,
                        format!(
                            "bloco {block_id} não existe na função `{name}`",
                            name = function.name
                        ),
                        &Site::Function(&function.name),
                    ))
                }
            };
            let block_site = Site::Block {
                function: &function.name,
                block: &block.label,
            };

            let Some(instr) = block.instrs.get(instr_index) else {
                // Fim das instruções: despacha o terminador.
                self.debit(&block_site)?;
                match &block.term {
                    Terminator::Return(value_id) => {
                        let frame = match stack.pop() {
                            Some(frame) => frame,
                            None => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    "frame ausente no retorno",
                                    &Site::Module,
                                ))
                            }
                        };
                        self.release_memory(frame.cost);
                        let value = match value_id {
                            Some(value_id) => read_slot(&frame, *value_id, &block_site)?,
                            None => Value::Unit,
                        };
                        let held: u64 = frame
                            .slot_bytes
                            .iter()
                            .copied()
                            .fold(0u64, u64::saturating_add);
                        self.release_memory(held);
                        if !value.matches_type(frame.function.ret) {
                            return Err(error_at(
                                ErrorCode::InvalidModule,
                                format!(
                                    "retorno `{actual}` não casa com o tipo declarado `{expected}` da função `{name}`",
                                    name = frame.function.name,
                                    actual = value.ty(),
                                    expected = frame.function.ret
                                ),
                                &block_site,
                            ));
                        }
                        if stack.is_empty() {
                            return Ok(value);
                        }
                        let caller = match stack.len().checked_sub(1) {
                            Some(caller) => caller,
                            None => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    "frame chamador ausente no retorno",
                                    &Site::Module,
                                ))
                            }
                        };
                        let return_slot = frame.return_slot.ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame chamado sem slot de retorno definido",
                                &Site::Module,
                            )
                        })?;
                        self.write_current(&mut stack, caller, return_slot, value, &block_site)?;
                    }
                    Terminator::Br(target) => {
                        self.jump(&mut stack, top, target.target, &target.args, &block_site)?;
                    }
                    Terminator::CondBr {
                        cond,
                        then_target,
                        else_target,
                    } => {
                        let condition = self.read_bool(&stack, top, *cond, &block_site)?;
                        let target = if condition { then_target } else { else_target };
                        self.jump(&mut stack, top, target.target, &target.args, &block_site)?;
                    }
                }
                continue;
            };

            let site = Site::instruction(&function.name, &block.label, instr_index);
            self.debit(&site)?;
            match &instr.op {
                Op::ConstI64(value) => {
                    self.write_current(&mut stack, top, instr.def, Value::I64(*value), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ConstBool(value) => {
                    self.write_current(&mut stack, top, instr.def, Value::Bool(*value), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::Bin { op, lhs, rhs } => {
                    let left = self.read_i64(&stack, top, *lhs, &site)?;
                    let right = self.read_i64(&stack, top, *rhs, &site)?;
                    let result = match op.eval_i64(left, right) {
                        Some(value) => value,
                        None => {
                            let (code, detail) = match op {
                                BinOp::DivI64Checked | BinOp::RemI64Checked if right == 0 => (
                                    ErrorCode::DivisionByZero,
                                    format!("`{op}` com divisor zero"),
                                ),
                                BinOp::ShlI64Checked | BinOp::ShrI64Checked => (
                                    ErrorCode::InvalidShift,
                                    format!("`{op}` com quantidade {right} fora de 0..=63"),
                                ),
                                _ => (
                                    ErrorCode::IntegerOverflow,
                                    format!("`{op}` transbordou com os operandos {left} e {right}"),
                                ),
                            };
                            return Err(error_at(code, detail, &site));
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, Value::I64(result), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::Cmp { op, lhs, rhs } => {
                    let left = self.read_i64(&stack, top, *lhs, &site)?;
                    let right = self.read_i64(&stack, top, *rhs, &site)?;
                    self.write_current(
                        &mut stack,
                        top,
                        instr.def,
                        Value::Bool(op.eval_i64(left, right)),
                        &site,
                    )?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ConstStr(text) => {
                    self.ensure_memory(u64::try_from(text.len()).unwrap_or(u64::MAX), &site)?;
                    self.write_current(
                        &mut stack,
                        top,
                        instr.def,
                        Value::Str(text.clone()),
                        &site,
                    )?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::StrEq { lhs, rhs } => {
                    let left = self.read_str(&stack, top, *lhs, &site)?;
                    let right = self.read_str(&stack, top, *rhs, &site)?;
                    self.write_current(
                        &mut stack,
                        top,
                        instr.def,
                        Value::Bool(left == right),
                        &site,
                    )?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::StrNe { lhs, rhs } => {
                    let left = self.read_str(&stack, top, *lhs, &site)?;
                    let right = self.read_str(&stack, top, *rhs, &site)?;
                    self.write_current(
                        &mut stack,
                        top,
                        instr.def,
                        Value::Bool(left != right),
                        &site,
                    )?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::StrLen { operand } => {
                    let text = self.read_str(&stack, top, *operand, &site)?;
                    let len = i64::try_from(text.len()).unwrap_or(i64::MAX);
                    self.write_current(&mut stack, top, instr.def, Value::I64(len), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ConcatStr { lhs, rhs } => {
                    let left = self.read_str(&stack, top, *lhs, &site)?;
                    let right = self.read_str(&stack, top, *rhs, &site)?;
                    let len = left.len().saturating_add(right.len());
                    let cap = usize::try_from(self.max_string_bytes).unwrap_or(usize::MAX);
                    if len > cap {
                        return Err(error_at(
                            ErrorCode::StringTooLong,
                            format!("concatenação com {len} bytes excede o limite de {cap} bytes"),
                            &site,
                        ));
                    }
                    // Pré-checa os bytes retidos do resultado; a cobrança exata
                    // acontece ao gravar o slot (ADR-0035).
                    self.ensure_memory(u64::try_from(len).unwrap_or(u64::MAX), &site)?;
                    let mut result = left;
                    result.push_str(&right);
                    self.write_current(&mut stack, top, instr.def, Value::Str(result), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ConstF64(bits) => {
                    self.write_current(
                        &mut stack,
                        top,
                        instr.def,
                        Value::F64(f64::from_bits(*bits)),
                        &site,
                    )?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::Float { op, lhs, rhs } => {
                    let left = self.read_f64(&stack, top, *lhs, &site)?;
                    let right = self.read_f64(&stack, top, *rhs, &site)?;
                    let value = if op.result_type() == TYPE_F64 {
                        Value::F64(op.eval_f64(left, right))
                    } else {
                        Value::Bool(op.eval_cmp(left, right))
                    };
                    self.write_current(&mut stack, top, instr.def, value, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ArrayLiteral { ty, items } => {
                    let mut values: Vec<Value> = Vec::new();
                    {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao ler array",
                                &site,
                            )
                        })?;
                        values.try_reserve_exact(items.len()).map_err(|_| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "não foi possível alocar o array",
                                &site,
                            )
                        })?;
                        for id in items {
                            values.push(read_slot(frame, *id, &site)?);
                        }
                    }
                    // Pré-checa os bytes retidos; a cobrança exata ocorre no slot
                    // (ADR-0035).
                    let bytes = u64::try_from(values.len())
                        .unwrap_or(u64::MAX)
                        .saturating_mul(SLOT_BYTES)
                        .saturating_add(values.iter().fold(0u64, |total, value| {
                            total.saturating_add(value.retained_bytes())
                        }));
                    self.ensure_memory(bytes, &site)?;
                    self.write_current(
                        &mut stack,
                        top,
                        instr.def,
                        Value::Array {
                            ty: *ty,
                            items: values,
                        },
                        &site,
                    )?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ArrayLen { array } => {
                    let length = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao medir array",
                                &site,
                            )
                        })?;
                        let value = read_slot(frame, *array, &site)?;
                        match value.as_array() {
                            Some(items) => i64::try_from(items.len()).unwrap_or(i64::MAX),
                            None => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    format!("v{array} não é um array"),
                                    &site,
                                ))
                            }
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, Value::I64(length), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ArrayGet { array, index } => {
                    let element = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao indexar array",
                                &site,
                            )
                        })?;
                        let array_value = read_slot(frame, *array, &site)?;
                        let items = array_value.as_array().ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                format!("v{array} não é um array"),
                                &site,
                            )
                        })?;
                        let position = read_i64(frame, *index, &site)?;
                        let len = i64::try_from(items.len()).unwrap_or(i64::MAX);
                        if position < 0 || position >= len {
                            return Err(error_at(
                                ErrorCode::IndexOutOfBounds,
                                format!("índice {position} fora de 0..{len}"),
                                &site,
                            ));
                        }
                        let offset = usize::try_from(position).map_err(|_| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "índice não representável neste host",
                                &site,
                            )
                        })?;
                        items.get(offset).cloned().ok_or_else(|| {
                            error_at(
                                ErrorCode::IndexOutOfBounds,
                                format!("índice {position} fora de 0..{len}"),
                                &site,
                            )
                        })?
                    };
                    self.write_current(&mut stack, top, instr.def, element, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ArraySet {
                    array,
                    index,
                    value,
                } => {
                    let (updated, bytes) = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao atualizar array",
                                &site,
                            )
                        })?;
                        let array_value = read_slot(frame, *array, &site)?;
                        let array_ty = array_value.ty();
                        let items = array_value.as_array().ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                format!("v{array} não é um array"),
                                &site,
                            )
                        })?;
                        let position = read_i64(frame, *index, &site)?;
                        let len = i64::try_from(items.len()).unwrap_or(i64::MAX);
                        if position < 0 || position >= len {
                            return Err(error_at(
                                ErrorCode::IndexOutOfBounds,
                                format!("índice {position} fora de 0..{len}"),
                                &site,
                            ));
                        }
                        let offset = usize::try_from(position).map_err(|_| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "índice não representável neste host",
                                &site,
                            )
                        })?;
                        let replacement = read_slot(frame, *value, &site)?;
                        let mut updated: Vec<Value> = Vec::new();
                        updated.try_reserve_exact(items.len()).map_err(|_| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "não foi possível alocar o array",
                                &site,
                            )
                        })?;
                        updated.extend_from_slice(items);
                        if let Some(slot) = updated.get_mut(offset) {
                            *slot = replacement;
                        }
                        let value = Value::Array {
                            ty: array_ty,
                            items: updated,
                        };
                        let bytes = value.retained_bytes();
                        (value, bytes)
                    };
                    // Atualização funcional: pré-checa o array novo; a cobrança
                    // exata ocorre ao gravar o slot (ADR-0035).
                    self.ensure_memory(bytes, &site)?;
                    self.write_current(&mut stack, top, instr.def, updated, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::StructNew { struct_id, fields } => {
                    let mut values: Vec<Value> = Vec::new();
                    {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao construir struct",
                                &site,
                            )
                        })?;
                        values.try_reserve_exact(fields.len()).map_err(|_| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "não foi possível alocar a struct",
                                &site,
                            )
                        })?;
                        for id in fields {
                            values.push(read_slot(frame, *id, &site)?);
                        }
                    }
                    // Pré-checa os bytes retidos; a cobrança exata ocorre no slot
                    // (ADR-0035).
                    let ty = self
                        .module
                        .find_type(TypeDef::Struct(*struct_id))
                        .ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "tipo da struct não declarado no módulo".to_string(),
                                &site,
                            )
                        })?;
                    let value = Value::Struct {
                        ty,
                        struct_id: *struct_id,
                        fields: values,
                    };
                    // Pré-checa os bytes retidos; a cobrança exata ocorre no slot.
                    let bytes = value.retained_bytes();
                    self.ensure_memory(bytes, &site)?;
                    self.write_current(&mut stack, top, instr.def, value, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::FieldGet { object, field } => {
                    let value = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao ler campo",
                                &site,
                            )
                        })?;
                        let object_value = read_slot(frame, *object, &site)?;
                        match &object_value {
                            Value::Struct {
                                struct_id, fields, ..
                            } => {
                                let def = self.module.struct_def(*struct_id).ok_or_else(|| {
                                    error_at(
                                        ErrorCode::InvalidModule,
                                        format!("struct {struct_id} não existe no módulo"),
                                        &site,
                                    )
                                })?;
                                let index = def.field_index(field).ok_or_else(|| {
                                    error_at(
                                        ErrorCode::InvalidType,
                                        format!("struct `{}` não tem campo `{field}`", def.name),
                                        &site,
                                    )
                                })?;
                                fields.get(index).cloned().ok_or_else(|| {
                                    error_at(
                                        ErrorCode::InvalidModule,
                                        "campo ausente na struct",
                                        &site,
                                    )
                                })?
                            }
                            _ => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    format!("v{object} não é uma struct"),
                                    &site,
                                ))
                            }
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, value, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::FieldSet {
                    object,
                    field,
                    value,
                } => {
                    let (updated, bytes) = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao atualizar struct",
                                &site,
                            )
                        })?;
                        let struct_value = read_slot(frame, *object, &site)?;
                        match &struct_value {
                            Value::Struct {
                                ty,
                                struct_id,
                                fields,
                            } => {
                                let def = self.module.struct_def(*struct_id).ok_or_else(|| {
                                    error_at(
                                        ErrorCode::InvalidModule,
                                        format!("struct {struct_id} não existe no módulo"),
                                        &site,
                                    )
                                })?;
                                let index = def.field_index(field).ok_or_else(|| {
                                    error_at(
                                        ErrorCode::InvalidType,
                                        format!("struct `{}` não tem campo `{field}`", def.name),
                                        &site,
                                    )
                                })?;
                                let replacement = read_slot(frame, *value, &site)?;
                                let mut updated: Vec<Value> = Vec::new();
                                updated.try_reserve_exact(fields.len()).map_err(|_| {
                                    error_at(
                                        ErrorCode::InvalidModule,
                                        "não foi possível alocar a struct",
                                        &site,
                                    )
                                })?;
                                updated.extend_from_slice(fields);
                                if let Some(slot) = updated.get_mut(index) {
                                    *slot = replacement;
                                }
                                let value = Value::Struct {
                                    ty: *ty,
                                    struct_id: *struct_id,
                                    fields: updated,
                                };
                                let bytes = value.retained_bytes();
                                (value, bytes)
                            }
                            _ => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    format!("v{object} não é uma struct"),
                                    &site,
                                ))
                            }
                        }
                    };
                    // Atualização funcional: pré-checa a struct nova; a cobrança
                    // exata ocorre ao gravar o slot (ADR-0035).
                    self.ensure_memory(bytes, &site)?;
                    self.write_current(&mut stack, top, instr.def, updated, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::OptionNone(ty) => {
                    self.write_current(
                        &mut stack,
                        top,
                        instr.def,
                        Value::Option {
                            ty: *ty,
                            value: None,
                        },
                        &site,
                    )?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::OptionSome { ty, value } => {
                    let payload = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao envolver option",
                                &site,
                            )
                        })?;
                        read_slot(frame, *value, &site)?
                    };
                    self.ensure_memory(SLOT_BYTES.saturating_add(payload.retained_bytes()), &site)?;
                    self.write_current(
                        &mut stack,
                        top,
                        instr.def,
                        Value::Option {
                            ty: *ty,
                            value: Some(Box::new(payload)),
                        },
                        &site,
                    )?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::OptionIsSome { value } => {
                    let is_some = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao testar option",
                                &site,
                            )
                        })?;
                        match read_slot(frame, *value, &site)? {
                            Value::Option { value, .. } => value.is_some(),
                            _ => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    format!("v{value} não é um option"),
                                    &site,
                                ))
                            }
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, Value::Bool(is_some), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::OptionUnwrap { value } => {
                    let payload = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao extrair option",
                                &site,
                            )
                        })?;
                        match read_slot(frame, *value, &site)? {
                            Value::Option {
                                value: Some(inner), ..
                            } => *inner,
                            Value::Option { value: None, .. } => {
                                return Err(error_at(
                                    ErrorCode::UnwrapNone,
                                    "option_unwrap sobre none".to_string(),
                                    &site,
                                ))
                            }
                            _ => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    format!("v{value} não é um option"),
                                    &site,
                                ))
                            }
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, payload, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::Coalesce { option, fallback } => {
                    let result = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao coalescer option",
                                &site,
                            )
                        })?;
                        let option_value = read_slot(frame, *option, &site)?;
                        let fallback_value = read_slot(frame, *fallback, &site)?;
                        match option_value {
                            Value::Option {
                                ty,
                                value: Some(inner),
                            } => {
                                if matches!(fallback_value, Value::Option { .. }) {
                                    Value::Option {
                                        ty,
                                        value: Some(inner),
                                    }
                                } else {
                                    *inner
                                }
                            }
                            Value::Option { value: None, .. } => fallback_value,
                            _ => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    format!("v{option} não é um option"),
                                    &site,
                                ))
                            }
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, result, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::OptionEq { lhs, rhs } => {
                    let equal = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao comparar option",
                                &site,
                            )
                        })?;
                        let left = read_slot(frame, *lhs, &site)?;
                        let right = read_slot(frame, *rhs, &site)?;
                        left == right
                    };
                    self.write_current(&mut stack, top, instr.def, Value::Bool(equal), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::OptionNe { lhs, rhs } => {
                    let different = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao comparar option",
                                &site,
                            )
                        })?;
                        let left = read_slot(frame, *lhs, &site)?;
                        let right = read_slot(frame, *rhs, &site)?;
                        left != right
                    };
                    self.write_current(&mut stack, top, instr.def, Value::Bool(different), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ResultOk { ty, value } => {
                    let payload = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao construir result",
                                &site,
                            )
                        })?;
                        read_slot(frame, *value, &site)?
                    };
                    self.ensure_memory(SLOT_BYTES.saturating_add(payload.retained_bytes()), &site)?;
                    self.write_current(
                        &mut stack,
                        top,
                        instr.def,
                        Value::Result {
                            ty: *ty,
                            value: Ok(Box::new(payload)),
                        },
                        &site,
                    )?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ResultErr { ty, value } => {
                    let payload = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao construir result",
                                &site,
                            )
                        })?;
                        read_slot(frame, *value, &site)?
                    };
                    self.ensure_memory(SLOT_BYTES.saturating_add(payload.retained_bytes()), &site)?;
                    self.write_current(
                        &mut stack,
                        top,
                        instr.def,
                        Value::Result {
                            ty: *ty,
                            value: Err(Box::new(payload)),
                        },
                        &site,
                    )?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ResultIsOk { value } => {
                    let is_ok = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao testar result",
                                &site,
                            )
                        })?;
                        match read_slot(frame, *value, &site)? {
                            Value::Result { value, .. } => value.is_ok(),
                            _ => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    format!("v{value} não é um result"),
                                    &site,
                                ))
                            }
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, Value::Bool(is_ok), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ResultUnwrap { value } => {
                    let payload = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao extrair result",
                                &site,
                            )
                        })?;
                        match read_slot(frame, *value, &site)? {
                            Value::Result {
                                value: Ok(inner), ..
                            } => *inner,
                            Value::Result { value: Err(_), .. } => {
                                return Err(error_at(
                                    ErrorCode::UnwrapError,
                                    "result_unwrap sobre err".to_string(),
                                    &site,
                                ))
                            }
                            _ => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    format!("v{value} não é um result"),
                                    &site,
                                ))
                            }
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, payload, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::ResultUnwrapErr { value } => {
                    let payload = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao extrair result",
                                &site,
                            )
                        })?;
                        match read_slot(frame, *value, &site)? {
                            Value::Result {
                                value: Err(inner), ..
                            } => *inner,
                            Value::Result { value: Ok(_), .. } => {
                                return Err(error_at(
                                    ErrorCode::UnwrapError,
                                    "result_unwrap_err sobre ok".to_string(),
                                    &site,
                                ))
                            }
                            _ => {
                                return Err(error_at(
                                    ErrorCode::InvalidModule,
                                    format!("v{value} não é um result"),
                                    &site,
                                ))
                            }
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, payload, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::HostCall { function, args } => {
                    // O buffer de argumentos do host também entra na quota
                    // modelada, como o de `call_static` (ADR-0016/ADR-0036).
                    let arg_bytes = arg_buffer_cost(u64::try_from(args.len()).unwrap_or(u64::MAX));
                    self.charge_memory(arg_bytes, &site)?;
                    let host_args = match stack.get(top) {
                        Some(frame) => collect_args(frame, args, &site)?,
                        None => {
                            return Err(error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente na chamada de host",
                                &site,
                            ))
                        }
                    };
                    let effect = function.effect();
                    let scope =
                        match function.scope_arg() {
                            Some(index) => host_args
                                .get(index)
                                .and_then(Value::as_str)
                                .ok_or_else(|| {
                                    error_at(
                                        ErrorCode::InvalidModule,
                                        format!(
                                        "host `{}` exige `string` no argumento de escopo {index}",
                                        function.name()
                                    ),
                                        &site,
                                    )
                                })?,
                            None => "",
                        };
                    if !self.host.capabilities().allows(effect, scope) {
                        return Err(error_at(
                            ErrorCode::CapabilityDenied,
                            format!(
                                "capability `{}` negada para escopo `{scope}` (host_call `{}`)",
                                effect.name(),
                                function.name()
                            ),
                            &site,
                        ));
                    }
                    // Reserva o pior caso antes do IO: se o resultado máximo não
                    // cabe na quota, o host não é chamado (ADR-0038).
                    let reserve = host_effect_reserve(*function, &host_args, &site)?;
                    self.ensure_memory(reserve, &site)?;
                    let ret = host_ret_type(self.module, *function, &site)?;
                    let value = self.host.call(*function, &host_args, ret)?;
                    self.release_memory(arg_bytes);
                    if !value.matches_type(ret) {
                        return Err(error_at(
                            ErrorCode::InvalidType,
                            format!(
                                "host `{}` devolveu `{}`, esperado `{ret}`",
                                function.name(),
                                value.ty()
                            ),
                            &site,
                        ));
                    }
                    self.write_current(&mut stack, top, instr.def, value, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::EnumNew {
                    ty,
                    variant,
                    fields,
                } => {
                    let mut values: Vec<Value> = Vec::new();
                    {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao construir enum",
                                &site,
                            )
                        })?;
                        values.try_reserve_exact(fields.len()).map_err(|_| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "não foi possível alocar o enum",
                                &site,
                            )
                        })?;
                        for id in fields {
                            values.push(read_slot(frame, *id, &site)?);
                        }
                    }
                    let enum_id = match self.module.type_def(*ty) {
                        Some(TypeDef::Enum(id)) => id,
                        _ => {
                            return Err(error_at(
                                ErrorCode::InvalidModule,
                                format!("tipo {ty} de enum_new não é enum"),
                                &site,
                            ))
                        }
                    };
                    let definition = self.module.enum_def(enum_id).ok_or_else(|| {
                        error_at(
                            ErrorCode::InvalidModule,
                            format!("enum {enum_id} não existe no módulo"),
                            &site,
                        )
                    })?;
                    let variant_def = usize::try_from(*variant)
                        .ok()
                        .and_then(|index| definition.variants.get(index))
                        .ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidType,
                                format!(
                                    "variante {variant} não existe no enum `{}`",
                                    definition.name
                                ),
                                &site,
                            )
                        })?;
                    if values.len() != variant_def.fields.len() {
                        return Err(error_at(
                            ErrorCode::InvalidType,
                            format!(
                                "enum_new de `{}.{}` espera {} campo(s), recebeu {}",
                                definition.name,
                                variant_def.name,
                                variant_def.fields.len(),
                                values.len()
                            ),
                            &site,
                        ));
                    }
                    let depth =
                        1_u32.saturating_add(values.iter().map(Value::depth).max().unwrap_or(0));
                    if depth > MAX_VALUE_DEPTH {
                        return Err(error_at(
                            ErrorCode::ValueTooDeep,
                            format!(
                                "enum_new produziria profundidade {depth}, acima do limite de {MAX_VALUE_DEPTH}"
                            ),
                            &site,
                        ));
                    }
                    let value = Value::Enum {
                        ty: *ty,
                        variant: *variant,
                        fields: values,
                    };
                    self.ensure_memory(value.retained_bytes(), &site)?;
                    self.write_current(&mut stack, top, instr.def, value, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::EnumIsVariant { value, variant } => {
                    let matches = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao testar variante",
                                &site,
                            )
                        })?;
                        match read_slot(frame, *value, &site)? {
                            Value::Enum {
                                variant: actual, ..
                            } => actual == *variant,
                            _ => {
                                return Err(error_at(
                                    ErrorCode::InvalidType,
                                    format!("v{value} não é um enum"),
                                    &site,
                                ))
                            }
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, Value::Bool(matches), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::EnumPayload {
                    value,
                    variant,
                    field,
                } => {
                    let payload = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao extrair payload de enum",
                                &site,
                            )
                        })?;
                        match read_slot(frame, *value, &site)? {
                            Value::Enum {
                                variant: actual,
                                fields,
                                ..
                            } => {
                                if actual != *variant {
                                    return Err(error_at(
                                        ErrorCode::InvalidVariant,
                                        format!(
                                            "enum_payload esperava variante {variant}, encontrou {actual}"
                                        ),
                                        &site,
                                    ));
                                }
                                usize::try_from(*field)
                                    .ok()
                                    .and_then(|index| fields.get(index))
                                    .cloned()
                                    .ok_or_else(|| {
                                        error_at(
                                            ErrorCode::InvalidType,
                                            format!(
                                                "campo {field} não existe na variante {variant}"
                                            ),
                                            &site,
                                        )
                                    })?
                            }
                            _ => {
                                return Err(error_at(
                                    ErrorCode::InvalidType,
                                    format!("v{value} não é um enum"),
                                    &site,
                                ))
                            }
                        }
                    };
                    self.write_current(&mut stack, top, instr.def, payload, &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::EnumEq { lhs, rhs } | Op::EnumNe { lhs, rhs } => {
                    let equal = {
                        let frame = stack.get(top).ok_or_else(|| {
                            error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente ao comparar enum",
                                &site,
                            )
                        })?;
                        read_slot(frame, *lhs, &site)? == read_slot(frame, *rhs, &site)?
                    };
                    let result = if matches!(&instr.op, Op::EnumEq { .. }) {
                        equal
                    } else {
                        !equal
                    };
                    self.write_current(&mut stack, top, instr.def, Value::Bool(result), &site)?;
                    self.advance(&mut stack, top, &site)?;
                }
                Op::Call { callee, args } => {
                    let callee_function = self.callee(*callee, &site)?;
                    // O buffer de argumentos é cobrado antes de alocar (falha
                    // fechada) e liberado depois de consumido pelo callee.
                    let arg_bytes = arg_buffer_cost(u64::try_from(args.len()).unwrap_or(u64::MAX));
                    self.charge_memory(arg_bytes, &site)?;
                    let call_args = match stack.get(top) {
                        Some(frame) => collect_args(frame, args, &site)?,
                        None => {
                            return Err(error_at(
                                ErrorCode::InvalidModule,
                                "frame ausente na pilha de execução",
                                &Site::Module,
                            ))
                        }
                    };
                    let child_depth = self.next_depth(depth, &site)?;
                    let child =
                        self.new_frame(callee_function, &call_args, child_depth, Some(instr.def))?;
                    // O frame atual avança antes do push: o retorno do callee grava
                    // no slot `instr.def` deste frame.
                    self.advance(&mut stack, top, &site)?;
                    stack.push(child);
                    self.release_memory(arg_bytes);
                }
            }
        }
    }

    /// Cria um frame com slots alocados e parâmetros gravados.
    fn new_frame(
        &mut self,
        function: &'a Function,
        args: &[Value],
        depth: u32,
        return_slot: Option<ValueId>,
    ) -> Result<Frame<'a>, RvmError> {
        let function_site = Site::Function(&function.name);
        self.observed_depth = self.observed_depth.max(depth);
        let slot_count = usize::try_from(function.value_count()).map_err(|_| {
            error_at(
                ErrorCode::InvalidModule,
                format!(
                    "função `{name}` declara valores demais para este host",
                    name = function.name
                ),
                &function_site,
            )
        })?;
        let cost = frame_cost(u64::from(function.value_count()));
        self.charge_memory(cost, &function_site)?;
        let mut slots: Vec<Option<Value>> = Vec::new();
        slots.try_reserve_exact(slot_count).map_err(|_| {
            error_at(
                ErrorCode::InvalidModule,
                format!(
                    "não foi possível alocar {slot_count} slot(s) para o frame da função `{name}`",
                    name = function.name
                ),
                &function_site,
            )
        })?;
        slots.resize(slot_count, None);

        if args.len() != function.params.len() {
            return Err(error_at(
                ErrorCode::InvalidModule,
                format!(
                    "frame da função `{name}` espera {expected} argumento(s), recebeu {actual}",
                    name = function.name,
                    expected = function.params.len(),
                    actual = args.len()
                ),
                &function_site,
            ));
        }
        let mut frame = Frame {
            function,
            depth,
            slots,
            slot_bytes: vec![0; slot_count],
            block: function.entry,
            instr_index: 0,
            return_slot,
            cost,
        };
        for (index, (param, value)) in function.params.iter().zip(args.iter()).enumerate() {
            if !value.matches_type(param.ty) {
                return Err(error_at(
                    ErrorCode::InvalidModule,
                    format!(
                        "argumento do parâmetro `{name}` tem tipo `{actual}`, esperado `{expected}`",
                        name = param.name,
                        actual = value.ty(),
                        expected = param.ty
                    ),
                    &function_site,
                ));
            }
            let value_id = u32::try_from(index).unwrap_or(u32::MAX);
            self.write_slot_charged(&mut frame, value_id, value.clone(), &function_site)?;
        }

        Ok(frame)
    }

    /// Copia argumentos para os parâmetros do bloco alvo e salta.
    ///
    /// O buffer de argumentos do desvio é memória viva durante o desvio e entra
    /// na quota modelada (ADR-0016); é cobrado antes de alocar e liberado ao fim.
    fn jump(
        &mut self,
        stack: &mut Vec<Frame<'a>>,
        top: usize,
        target: BlockId,
        args: &[ValueId],
        site: &Site<'_>,
    ) -> Result<(), RvmError> {
        let function = match stack.get(top) {
            Some(frame) => frame.function,
            None => {
                return Err(error_at(
                    ErrorCode::InvalidModule,
                    "frame ausente durante desvio",
                    site,
                ))
            }
        };
        let block = function.block(target).ok_or_else(|| {
            error_at(
                ErrorCode::InvalidModule,
                format!(
                    "bloco alvo {target} não existe na função `{name}`",
                    name = function.name
                ),
                site,
            )
        })?;
        if block.params.len() != args.len() {
            return Err(error_at(
                ErrorCode::InvalidModule,
                format!(
                    "bloco `{label}` espera {expected} argumento(s), recebeu {actual}",
                    label = block.label,
                    expected = block.params.len(),
                    actual = args.len()
                ),
                site,
            ));
        }
        let arg_bytes = arg_buffer_cost(u64::try_from(args.len()).unwrap_or(u64::MAX));
        self.charge_memory(arg_bytes, site)?;
        let mut buffer: Vec<Value> = Vec::new();
        buffer.try_reserve_exact(args.len()).map_err(|_| {
            error_at(
                ErrorCode::InvalidModule,
                format!(
                    "não foi possível alocar {count} argumento(s) de desvio",
                    count = args.len()
                ),
                site,
            )
        })?;
        for id in args {
            match stack.get(top) {
                Some(frame) => buffer.push(read_slot(frame, *id, site)?),
                None => {
                    return Err(error_at(
                        ErrorCode::InvalidModule,
                        "frame ausente ao ler argumento de desvio",
                        site,
                    ))
                }
            }
        }
        let frame = match stack.get_mut(top) {
            Some(frame) => frame,
            None => {
                return Err(error_at(
                    ErrorCode::InvalidModule,
                    "frame ausente ao gravar parâmetros de bloco",
                    site,
                ))
            }
        };
        for (param, value) in block.params.iter().zip(buffer.iter()) {
            if !value.matches_type(param.ty) {
                return Err(error_at(
                    ErrorCode::InvalidModule,
                    format!(
                        "argumento do parâmetro de bloco `{name}` tem tipo `{actual}`, esperado `{expected}`",
                        name = param.name,
                        actual = value.ty(),
                        expected = param.ty
                    ),
                    site,
                ));
            }
            self.write_slot_charged(frame, param.value, value.clone(), site)?;
        }
        frame.block = target;
        frame.instr_index = 0;
        self.release_memory(arg_bytes);
        Ok(())
    }

    /// Avança a instrução do frame `top`.
    fn advance(
        &mut self,
        stack: &mut [Frame<'a>],
        top: usize,
        site: &Site<'_>,
    ) -> Result<(), RvmError> {
        let frame = stack
            .get_mut(top)
            .ok_or_else(|| error_at(ErrorCode::InvalidModule, "frame ausente ao avançar", site))?;
        frame.instr_index = frame.instr_index.saturating_add(1);
        Ok(())
    }

    /// Grava um valor no slot `value` do frame `top`, cobrando os bytes retidos e
    /// liberando o valor anterior do mesmo slot (ADR-0035).
    fn write_current(
        &mut self,
        stack: &mut [Frame<'a>],
        top: usize,
        value: ValueId,
        slot_value: Value,
        site: &Site<'_>,
    ) -> Result<(), RvmError> {
        let frame = stack.get_mut(top).ok_or_else(|| {
            error_at(
                ErrorCode::InvalidModule,
                "frame ausente ao gravar valor",
                site,
            )
        })?;
        self.write_slot_charged(frame, value, slot_value, site)
    }

    /// Cobra os bytes retidos e grava no slot, liberando o custo anterior.
    fn write_slot_charged(
        &mut self,
        frame: &mut Frame<'a>,
        value: ValueId,
        slot_value: Value,
        site: &Site<'_>,
    ) -> Result<(), RvmError> {
        let index = slot_index(value, site)?;
        let new_cost = slot_value.retained_bytes();
        // Falha fechada: cobra antes de sobrescrever o slot.
        self.charge_memory(new_cost, site)?;
        let old_cost = frame.slot_bytes.get(index).copied().unwrap_or(0);
        match frame.slots.get_mut(index) {
            Some(slot) => {
                *slot = Some(slot_value);
            }
            None => {
                return Err(error_at(
                    ErrorCode::InvalidModule,
                    format!(
                        "slot v{value} não existe no frame de {len} slot(s)",
                        len = frame.slots.len()
                    ),
                    site,
                ))
            }
        }
        if let Some(cost) = frame.slot_bytes.get_mut(index) {
            *cost = new_cost;
        }
        self.release_memory(old_cost);
        Ok(())
    }

    /// Verifica se `bytes` caberiam na quota, sem cobrar (pré-checagem antes de
    /// alocar, ADR-0035).
    fn ensure_memory(&self, bytes: u64, site: &Site<'_>) -> Result<(), RvmError> {
        let proximo = self.memory_live.saturating_add(bytes);
        if proximo > self.memory_limit {
            return Err(error_at(
                ErrorCode::OutOfMemory,
                format!(
                    "memória modelada excedida: {proximo} bytes > limite de {limite} bytes",
                    limite = self.memory_limit
                ),
                site,
            ));
        }
        Ok(())
    }

    /// Lê um `i64` do frame `top`.
    fn read_i64(
        &self,
        stack: &[Frame<'a>],
        top: usize,
        value: ValueId,
        site: &Site<'_>,
    ) -> Result<i64, RvmError> {
        let frame = stack.get(top).ok_or_else(|| {
            error_at(ErrorCode::InvalidModule, "frame ausente ao ler valor", site)
        })?;
        read_i64(frame, value, site)
    }

    /// Lê um `bool` do frame `top`.
    fn read_bool(
        &self,
        stack: &[Frame<'a>],
        top: usize,
        value: ValueId,
        site: &Site<'_>,
    ) -> Result<bool, RvmError> {
        let frame = stack.get(top).ok_or_else(|| {
            error_at(
                ErrorCode::InvalidModule,
                "frame ausente ao ler condição",
                site,
            )
        })?;
        read_bool(frame, value, site)
    }

    /// Lê um `string` do frame `top`.
    fn read_str(
        &self,
        stack: &[Frame<'a>],
        top: usize,
        value: ValueId,
        site: &Site<'_>,
    ) -> Result<String, RvmError> {
        let frame = stack.get(top).ok_or_else(|| {
            error_at(
                ErrorCode::InvalidModule,
                "frame ausente ao ler string",
                site,
            )
        })?;
        read_str(frame, value, site)
    }

    /// Lê um `f64` do frame `top`.
    fn read_f64(
        &self,
        stack: &[Frame<'a>],
        top: usize,
        value: ValueId,
        site: &Site<'_>,
    ) -> Result<f64, RvmError> {
        let frame = stack
            .get(top)
            .ok_or_else(|| error_at(ErrorCode::InvalidModule, "frame ausente ao ler f64", site))?;
        read_f64(frame, value, site)
    }

    /// Resolve uma função do módulo (o verificador garante a existência).
    fn callee(&self, callee: FnId, site: &Site<'_>) -> Result<&'a Function, RvmError> {
        let module: &'a Module = self.module;
        module.function(callee).ok_or_else(|| {
            error_at(
                ErrorCode::InvalidModule,
                format!("função chamada {callee} não existe no módulo"),
                site,
            )
        })
    }

    /// Calcula a profundidade do callee, aplicando o limite configurado.
    fn next_depth(&self, depth: u32, site: &Site<'_>) -> Result<u32, RvmError> {
        let child = depth.checked_add(1).ok_or_else(|| {
            error_at(
                ErrorCode::CallDepthExceeded,
                format!("profundidade de chamadas transbordou a partir de {depth}"),
                site,
            )
        })?;
        if child > self.max_call_depth {
            return Err(error_at(
                ErrorCode::CallDepthExceeded,
                format!(
                    "profundidade {child} excede o limite de {limit} chamada(s) aninhada(s)",
                    limit = self.max_call_depth
                ),
                site,
            ));
        }
        Ok(child)
    }

    /// Cobra o custo modelado de um frame antes de alocá-lo.
    fn charge_memory(&mut self, bytes: u64, site: &Site<'_>) -> Result<(), RvmError> {
        let proximo = self.memory_live.saturating_add(bytes);
        if proximo > self.memory_limit {
            return Err(error_at(
                ErrorCode::OutOfMemory,
                format!(
                    "memória de frames excedida: {proximo} bytes > limite de {limite} bytes",
                    limite = self.memory_limit
                ),
                site,
            ));
        }
        self.memory_live = proximo;
        self.memory_peak = self.memory_peak.max(proximo);
        Ok(())
    }

    /// Libera o custo modelado de um frame no retorno.
    fn release_memory(&mut self, bytes: u64) {
        self.memory_live = self.memory_live.saturating_sub(bytes);
    }

    /// Debita 1 unidade de fuel antes de um despacho.
    fn debit(&mut self, site: &Site<'_>) -> Result<(), RvmError> {
        match self.remaining_fuel.checked_sub(1) {
            Some(remaining) => {
                self.remaining_fuel = remaining;
                Ok(())
            }
            None => Err(error_at(
                ErrorCode::OutOfFuel,
                format!(
                    "fuel esgotado antes do despacho (saldo inicial {initial})",
                    initial = self.initial_fuel
                ),
                site,
            )),
        }
    }
}

/// Índice de `ValueId` no frame, sem truncamento.
fn slot_index(value: ValueId, site: &Site<'_>) -> Result<usize, RvmError> {
    usize::try_from(value).map_err(|_| {
        error_at(
            ErrorCode::InvalidModule,
            format!("ValueId {value} não é representável nesta plataforma"),
            site,
        )
    })
}

/// Tipo de retorno da função de host embutida (ADR-0036).
fn host_ret_type(
    module: &Module,
    function: BuiltinHost,
    site: &Site<'_>,
) -> Result<TypeId, RvmError> {
    match function {
        BuiltinHost::ReadFile => module
            .find_type(TypeDef::Result(TYPE_STR, TYPE_STR))
            .ok_or_else(|| {
                error_at(
                    ErrorCode::InvalidModule,
                    "tipo `result string string` não declarado no módulo".to_string(),
                    site,
                )
            }),
    }
}

/// Reserva de memória para o pior caso de uma chamada de host (ADR-0038).
///
/// Para `read_file`, o resultado custa `SLOT_BYTES` (wrapper do `result`) mais o
/// conteúdo, limitado pelo `max_bytes` pedido e pelo teto da ABI.
fn host_effect_reserve(
    function: BuiltinHost,
    args: &[Value],
    site: &Site<'_>,
) -> Result<u64, RvmError> {
    match function {
        BuiltinHost::ReadFile => {
            let requested = args.get(1).and_then(Value::as_i64).ok_or_else(|| {
                error_at(
                    ErrorCode::InvalidModule,
                    "host `read_file` exige `max_bytes: i64`".to_string(),
                    site,
                )
            })?;
            let requested = u64::try_from(requested).unwrap_or(0);
            Ok(SLOT_BYTES.saturating_add(requested.min(function.max_effect_bytes())))
        }
    }
}

/// Lê um valor já inicializado do slot de `value`.
fn read_slot(frame: &Frame<'_>, value: ValueId, site: &Site<'_>) -> Result<Value, RvmError> {
    let index = slot_index(value, site)?;
    match frame.slots.get(index).cloned().flatten() {
        Some(value) => Ok(value),
        None => Err(error_at(
            ErrorCode::InvalidModule,
            format!("slot v{value} ausente ou não inicializado"),
            site,
        )),
    }
}

/// Lê um slot exigindo `i64`.
fn read_i64(frame: &Frame<'_>, value: ValueId, site: &Site<'_>) -> Result<i64, RvmError> {
    let slot = read_slot(frame, value, site)?;
    slot.as_i64().ok_or_else(|| {
        error_at(
            ErrorCode::InvalidModule,
            format!(
                "slot v{value} contém `{actual}`, esperado `i64`",
                actual = slot.ty()
            ),
            site,
        )
    })
}

/// Lê um slot exigindo `bool`.
fn read_bool(frame: &Frame<'_>, value: ValueId, site: &Site<'_>) -> Result<bool, RvmError> {
    let slot = read_slot(frame, value, site)?;
    slot.as_bool().ok_or_else(|| {
        error_at(
            ErrorCode::InvalidModule,
            format!(
                "slot v{value} contém `{actual}`, esperado `bool`",
                actual = slot.ty()
            ),
            site,
        )
    })
}

/// Lê um slot exigindo `string`.
fn read_str(frame: &Frame<'_>, value: ValueId, site: &Site<'_>) -> Result<String, RvmError> {
    let slot = read_slot(frame, value, site)?;
    match slot.as_str() {
        Some(text) => Ok(text.to_string()),
        None => Err(error_at(
            ErrorCode::InvalidModule,
            format!("slot v{value} contém `{}`, esperado `string`", slot.ty()),
            site,
        )),
    }
}

/// Lê um slot exigindo `f64`.
fn read_f64(frame: &Frame<'_>, value: ValueId, site: &Site<'_>) -> Result<f64, RvmError> {
    let slot = read_slot(frame, value, site)?;
    slot.as_f64().ok_or_else(|| {
        error_at(
            ErrorCode::InvalidModule,
            format!("slot v{value} contém `{}`, esperado `f64`", slot.ty()),
            site,
        )
    })
}

/// Avalia os argumentos de uma chamada em um vetor novo.
fn collect_args(
    frame: &Frame<'_>,
    ids: &[ValueId],
    site: &Site<'_>,
) -> Result<Vec<Value>, RvmError> {
    let mut values = Vec::new();
    values.try_reserve_exact(ids.len()).map_err(|_| {
        error_at(
            ErrorCode::InvalidModule,
            format!(
                "não foi possível alocar {count} argumento(s) de chamada",
                count = ids.len()
            ),
            site,
        )
    })?;
    for id in ids {
        values.push(read_slot(frame, *id, site)?);
    }
    Ok(values)
}

/// Localização de um ponto de execução, materializada só quando há erro.
enum Site<'a> {
    /// O módulo (por exemplo, `entry` inexistente).
    Module,
    /// Uma função inteira (parâmetros do frame, retorno).
    Function(&'a str),
    /// Um bloco, usado no terminador e em parâmetros de bloco.
    Block {
        /// Nome declarado da função.
        function: &'a str,
        /// Rótulo do bloco.
        block: &'a str,
    },
    /// Uma instrução despachada.
    Instruction {
        /// Nome declarado da função.
        function: &'a str,
        /// Rótulo do bloco.
        block: &'a str,
        /// Índice da instrução no bloco.
        index: u32,
    },
}

impl<'a> Site<'a> {
    /// Localização de uma instrução a partir do índice no bloco.
    fn instruction(function: &'a str, block: &'a str, index: usize) -> Self {
        Site::Instruction {
            function,
            block,
            index: u32::try_from(index).unwrap_or(u32::MAX),
        }
    }

    /// Materializa a [`Location`] do ponto de execução.
    fn location(&self) -> Location {
        match self {
            Site::Module => Location::Module,
            Site::Function(function) => Location::Function {
                function: (*function).to_string(),
            },
            Site::Block { function, block } => Location::Block {
                function: (*function).to_string(),
                block: (*block).to_string(),
            },
            Site::Instruction {
                function,
                block,
                index,
            } => Location::Instruction {
                function: (*function).to_string(),
                block: (*block).to_string(),
                index: *index,
            },
        }
    }
}

/// Constrói um erro com código estável e localização do ponto de execução.
fn error_at(code: ErrorCode, detail: impl Into<String>, site: &Site<'_>) -> RvmError {
    RvmError::new(code, detail).at(site.location())
}
