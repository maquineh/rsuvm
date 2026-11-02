//! `rvm-interpreter` — executor de referência da RIR verificada (EXP-0).
//!
//! Execução single-thread, determinística e sem efeitos de host. O executor só
//! aceita [`rvm_verify::VerifiedModule`] e a única operação pública é
//! [`execute`].
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
//! - `add/sub/mul_i64_checked` usam [`rvm_spec::BinOp::eval_i64`]; resultado fora
//!   de `i64` produz o trap `integer_overflow`. Comparações usam
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
//! A execução usa uma pilha de frames na **heap** ([`Frame`]), não a pilha do
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
use rvm_runtime::{frame_cost, ExecConfig, Value};
use rvm_spec::{ErrorCode, Location, RvmError};

use rvm_verify::VerifiedModule;

/// Resultado de uma execução bem-sucedida.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunOutcome {
    /// Valor devolvido pela função de entrada.
    pub value: Value,
    /// Fuel consumido (uma unidade por instrução/terminador despachado).
    pub fuel_used: u64,
    /// Maior profundidade de chamadas observada (frame de entrada = 1).
    pub max_depth: u32,
    /// Pico de memória modelada dos frames, em bytes.
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
        remaining_fuel: config.fuel(),
        initial_fuel: config.fuel(),
        max_call_depth: u32::from(config.max_call_depth().get()),
        observed_depth: 0,
        memory_limit: config.memory_limit().get(),
        memory_live: 0,
        memory_peak: 0,
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
    remaining_fuel: u64,
    initial_fuel: u64,
    max_call_depth: u32,
    observed_depth: u32,
    memory_limit: u64,
    memory_live: u64,
    memory_peak: u64,
}

impl<'a> Interpreter<'a> {
    /// Executa `function` até o retorno, com uma pilha de frames na heap.
    fn run(&mut self, function: &'a Function, args: &[Value]) -> Result<Value, RvmError> {
        let mut stack: Vec<Frame<'a>> = Vec::new();
        stack.push(self.new_frame(function, args, 1, None)?);
        // Buffer reutilizado para avaliar argumentos de desvio sem alocar por
        // iteração de loop.
        let mut branch_args: Vec<Value> = Vec::new();

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
                        self.jump(
                            &mut stack,
                            top,
                            target.target,
                            &target.args,
                            &mut branch_args,
                            &block_site,
                        )?;
                    }
                    Terminator::CondBr {
                        cond,
                        then_target,
                        else_target,
                    } => {
                        let condition = self.read_bool(&stack, top, *cond, &block_site)?;
                        let target = if condition { then_target } else { else_target };
                        self.jump(
                            &mut stack,
                            top,
                            target.target,
                            &target.args,
                            &mut branch_args,
                            &block_site,
                        )?;
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
                    let result = op.eval_i64(left, right).ok_or_else(|| {
                        error_at(
                            ErrorCode::IntegerOverflow,
                            format!("`{op}` transbordou com os operandos {left} e {right}"),
                            &site,
                        )
                    })?;
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
                Op::Call { callee, args } => {
                    let callee_function = self.callee(*callee, &site)?;
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
            let slot = slots.get_mut(index).ok_or_else(|| {
                error_at(
                    ErrorCode::InvalidModule,
                    format!("slot v{value_id} não existe no frame de {slot_count} slot(s)"),
                    &function_site,
                )
            })?;
            *slot = Some(*value);
        }

        Ok(Frame {
            function,
            depth,
            slots,
            block: function.entry,
            instr_index: 0,
            return_slot,
            cost,
        })
    }

    /// Copia argumentos para os parâmetros do bloco alvo e salta.
    fn jump(
        &mut self,
        stack: &mut Vec<Frame<'a>>,
        top: usize,
        target: BlockId,
        args: &[ValueId],
        buffer: &mut Vec<Value>,
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
        buffer.clear();
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
            write_slot(frame, param.value, *value, site)?;
        }
        frame.block = target;
        frame.instr_index = 0;
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

    /// Grava um valor no slot `value` do frame `top`.
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
        write_slot(frame, value, slot_value, site)
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

/// Grava um valor no slot de `value`.
fn write_slot(
    frame: &mut Frame<'_>,
    value: ValueId,
    slot_value: Value,
    site: &Site<'_>,
) -> Result<(), RvmError> {
    let index = slot_index(value, site)?;
    match frame.slots.get_mut(index) {
        Some(slot) => {
            *slot = Some(slot_value);
            Ok(())
        }
        None => Err(error_at(
            ErrorCode::InvalidModule,
            format!(
                "slot v{value} não existe no frame de {len} slot(s)",
                len = frame.slots.len()
            ),
            site,
        )),
    }
}

/// Lê um valor já inicializado do slot de `value`.
fn read_slot(frame: &Frame<'_>, value: ValueId, site: &Site<'_>) -> Result<Value, RvmError> {
    let index = slot_index(value, site)?;
    match frame.slots.get(index).copied().flatten() {
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
