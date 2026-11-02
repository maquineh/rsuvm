//! `rvm-verify` — verificação de módulos RIR antes de qualquer execução.
//!
//! A verificação é a fronteira entre módulos não confiáveis e o executor: a
//! única forma de obter um [`VerifiedModule`] é [`VerifiedModule::verify`], que
//! roda primeiro a checagem estrutural (`rvm_ir::structure`) e depois valida
//! fluxo de controle, dominância, definição antes do uso, tipos, chamadas e
//! terminadores.
//!
//! # Etapas (EXP-0)
//!
//! 1. **Estrutural.** [`rvm_ir::structure::check`] valida versão, nomes, IDs
//!    densos, limites e referências antes de qualquer alocação proporcional ao
//!    módulo.
//! 2. **CFG.** Sucessores vêm de `br`/`cond_br`; todo bloco precisa ser
//!    alcançável a partir do bloco de entrada (senão `invalid_control_flow`).
//! 3. **Dominância.** Conjuntos de dominadores são calculados por iteração de
//!    ponto fixo com bitsets (`Vec<u64>`), `dom[entry] = {entry}` e
//!    `dom[b] = {b} ∪ ∩ dom[pred]`.
//! 4. **Definição antes do uso.** Cada operando — incluindo condição de desvio,
//!    valor de retorno e argumentos de desvio — precisa de uma definição que
//!    domine o ponto de uso; no mesmo bloco, a definição precisa vir antes
//!    (`invalid_control_flow`). Parâmetro de função domina a função inteira;
//!    parâmetro de bloco é definido na entrada do bloco.
//! 5. **Tipos.** Os tipos dos valores são derivados dos parâmetros, das
//!    operações e da assinatura da função chamada (`call_static` usa
//!    [`rvm_ir::Function::ret`] do alvo). Operandos, chamadas e terminadores
//!    precisam casar com essas assinaturas (`invalid_type`).
//!
//! A ordem de diagnóstico é determinística: funções por índice, blocos por
//! índice, instruções por índice e terminador por último; o primeiro erro vence.
//!
//! # Custos
//!
//! A verificação é sequencial e não usa `HashMap`. Para uma função com `B`
//! blocos e `V` valores, a dominância custa até `O(B² · ceil(B/64))` operações de
//! palavra de 64 bits e a memória é `O(B · ceil(B/64) + V)`; `B` e `V` já foram
//! limitados por [`rvm_spec::Limits`] na etapa estrutural.
//!
//! Falhas do verificador são erros de módulo (`ErrorClass::Module`), nunca traps
//! de execução.

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

use rvm_ir::{
    Block, BlockId, BranchTarget, FnId, Function, Instr, Module, Op, Terminator, ValueId,
};
use rvm_spec::{ErrorCode, Limits, Location, RvmError, Type};

/// Módulo que passou pela verificação completa.
///
/// Não implementa `From<Module>` nem expõe construtor público: a única produção
/// é [`VerifiedModule::verify`]. O executor e o CLI dependem apenas deste tipo.
#[derive(Clone, Debug)]
pub struct VerifiedModule {
    module: Module,
}

impl VerifiedModule {
    /// Verifica o módulo inteiro e o consome.
    ///
    /// Roda [`rvm_ir::structure::check`] e, em seguida, para cada função na
    /// ordem de índice: alcançabilidade do CFG, dominadores, definição antes do
    /// uso, tipos de operação, assinaturas de chamada e terminadores. Blocos
    /// inalcançáveis e violações de dominância são `invalid_control_flow`;
    /// divergências de tipo, aridade, retorno e condição são `invalid_type`.
    ///
    /// # Errors
    ///
    /// Devolve o primeiro diagnóstico estrutural, de fluxo de controle,
    /// de dominância ou de tipo encontrado, na ordem documentada no crate.
    pub fn verify(module: Module, limits: &Limits) -> Result<Self, RvmError> {
        rvm_ir::structure::check(&module, limits)?;
        // O executor não pode receber declarações externas: elas precisam ser
        // resolvidas pelo linker (`rvm-link`, ADR-0018) antes da verificação.
        for function in &module.functions {
            if function.is_external() {
                return Err(RvmError::new(
                    ErrorCode::InvalidModule,
                    format!(
                        "função `{}` é uma declaração externa; linke o módulo (rvm-link) antes de verificar",
                        function.name
                    ),
                )
                .at(Location::Function {
                    function: function.name.clone(),
                }));
            }
        }
        for function in &module.functions {
            verify_function(&module, function)?;
        }
        Ok(Self { module })
    }

    /// Módulo verificado.
    #[must_use]
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// Função de entrada declarada pelo módulo.
    #[must_use]
    pub fn entry(&self) -> FnId {
        self.module.entry
    }

    /// Função de entrada, se o identificador existir (garantido pela verificação).
    #[must_use]
    pub fn entry_function(&self) -> Option<&Function> {
        self.module.function(self.module.entry)
    }

    /// Função pelo identificador denso.
    #[must_use]
    pub fn function(&self, id: FnId) -> Option<&Function> {
        self.module.function(id)
    }
}

/// Origem de um valor SSA dentro da função.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Def {
    /// Parâmetro de função: definido na entrada da função e domina todos os blocos.
    FunctionParam,
    /// Parâmetro de bloco: definido na entrada do bloco `block`.
    BlockParam {
        /// Índice do bloco dono do parâmetro.
        block: usize,
    },
    /// Definição de instrução, identificada por bloco e índice no bloco.
    Instruction {
        /// Índice do bloco da definição.
        block: usize,
        /// Índice da instrução que define o valor.
        index: usize,
    },
}

/// Grafo de fluxo de controle de uma função, com índices densos de bloco.
struct Cfg {
    /// Sucessores de cada bloco, na ordem em que os terminadores os declaram.
    successors: Vec<Vec<usize>>,
    /// Predecessores de cada bloco, em ordem crescente de índice de origem.
    predecessors: Vec<Vec<usize>>,
    /// Índice do bloco de entrada da função.
    entry: usize,
}

/// Mapas de definição e de tipo de todos os valores de uma função.
struct ValueMaps {
    /// Definição de cada `ValueId` (`None` só em módulo estruturalmente inválido).
    definitions: Vec<Option<Def>>,
    /// Tipo de cada `ValueId`, incluindo o retorno de chamadas.
    types: Vec<Option<Type>>,
}

/// Fatos imutáveis de uma função usados na checagem de uso e de tipos.
struct FunctionFacts<'a> {
    /// Módulo dono da função (para resolver `call_static`).
    module: &'a Module,
    /// Função em verificação.
    function: &'a Function,
    /// Definições e tipos dos valores da função.
    values: ValueMaps,
    /// Conjuntos de dominadores por bloco, em bitset de 64 bits por palavra.
    dominators: Vec<Vec<u64>>,
}

impl<'a> FunctionFacts<'a> {
    /// Verifica as instruções e o terminador de um bloco, nesta ordem.
    fn check_block(&self, block_index: usize, block: &Block) -> Result<(), RvmError> {
        let block_location = Location::Block {
            function: self.function.name.clone(),
            block: block.label.clone(),
        };
        for (instr_index, instr) in block.instrs.iter().enumerate() {
            let location = Location::Instruction {
                function: self.function.name.clone(),
                block: block.label.clone(),
                index: u32::try_from(instr_index).map_err(|_| {
                    invalid(
                        ErrorCode::InvalidModule,
                        "índice de instrução não representável",
                        block_location.clone(),
                    )
                })?,
            };
            self.check_instruction(block_index, instr_index, instr, &location)?;
        }
        self.check_terminator(block_index, block, &block_location)
    }

    /// Verifica operandos, dominância e tipo definido por uma instrução.
    fn check_instruction(
        &self,
        block_index: usize,
        instr_index: usize,
        instr: &Instr,
        location: &Location,
    ) -> Result<(), RvmError> {
        match &instr.op {
            Op::ConstI64(_) | Op::ConstBool(_) | Op::ConstStr(_) => Ok(()),
            Op::Bin { op, lhs, rhs } => {
                self.check_use(block_index, instr_index, *lhs, location)?;
                self.check_use(block_index, instr_index, *rhs, location)?;
                let operand_type = op.operand_type();
                self.check_operand_type(*lhs, operand_type, location)?;
                self.check_operand_type(*rhs, operand_type, location)
            }
            Op::Cmp { op, lhs, rhs } => {
                self.check_use(block_index, instr_index, *lhs, location)?;
                self.check_use(block_index, instr_index, *rhs, location)?;
                let operand_type = op.operand_type();
                self.check_operand_type(*lhs, operand_type, location)?;
                self.check_operand_type(*rhs, operand_type, location)
            }
            Op::StrEq { lhs, rhs } | Op::StrNe { lhs, rhs } => {
                self.check_use(block_index, instr_index, *lhs, location)?;
                self.check_use(block_index, instr_index, *rhs, location)?;
                self.check_operand_type(*lhs, Type::Str, location)?;
                self.check_operand_type(*rhs, Type::Str, location)
            }
            Op::StrLen { operand } => {
                self.check_use(block_index, instr_index, *operand, location)?;
                self.check_operand_type(*operand, Type::Str, location)
            }
            Op::ConcatStr { lhs, rhs } => {
                self.check_use(block_index, instr_index, *lhs, location)?;
                self.check_use(block_index, instr_index, *rhs, location)?;
                self.check_operand_type(*lhs, Type::Str, location)?;
                self.check_operand_type(*rhs, Type::Str, location)
            }
            Op::Call { callee, args } => {
                let target = self.module.function(*callee).ok_or_else(|| {
                    invalid(
                        ErrorCode::InvalidModule,
                        format!("função chamada {callee} não existe no módulo"),
                        location.clone(),
                    )
                })?;
                if args.len() != target.params.len() {
                    return Err(invalid(
                        ErrorCode::InvalidType,
                        format!(
                            "chamada a `{}` espera {} argumento(s), recebeu {}",
                            target.name,
                            target.params.len(),
                            args.len()
                        ),
                        location.clone(),
                    ));
                }
                for arg in args {
                    self.check_use(block_index, instr_index, *arg, location)?;
                }
                for (arg, param) in args.iter().zip(target.params.iter()) {
                    self.check_operand_type(*arg, param.ty, location)?;
                }
                Ok(())
            }
        }
    }

    /// Verifica o terminador do bloco, usando `instrs.len()` como ponto de uso.
    fn check_terminator(
        &self,
        block_index: usize,
        block: &Block,
        location: &Location,
    ) -> Result<(), RvmError> {
        let position = block.instrs.len();
        match &block.term {
            Terminator::Return(None) => {
                if self.function.ret == Type::Unit {
                    Ok(())
                } else {
                    Err(invalid(
                        ErrorCode::InvalidType,
                        format!(
                            "função `{}` retorna {} e exige valor no `return`",
                            self.function.name, self.function.ret
                        ),
                        location.clone(),
                    ))
                }
            }
            Terminator::Return(Some(value)) => {
                if self.function.ret == Type::Unit {
                    return Err(invalid(
                        ErrorCode::InvalidType,
                        format!(
                            "função `{}` retorna unit e não pode devolver valor",
                            self.function.name
                        ),
                        location.clone(),
                    ));
                }
                self.check_use(block_index, position, *value, location)?;
                self.check_operand_type(*value, self.function.ret, location)
            }
            Terminator::Br(target) => self.check_target(block_index, position, target, location),
            Terminator::CondBr {
                cond,
                then_target,
                else_target,
            } => {
                self.check_use(block_index, position, *cond, location)?;
                self.check_operand_type(*cond, Type::Bool, location)?;
                self.check_target(block_index, position, then_target, location)?;
                self.check_target(block_index, position, else_target, location)
            }
        }
    }

    /// Verifica aridade, dominância e tipos dos argumentos de um desvio.
    fn check_target(
        &self,
        block_index: usize,
        position: usize,
        target: &BranchTarget,
        location: &Location,
    ) -> Result<(), RvmError> {
        let target_index = resolve_block_index(self.function, target.target, location)?;
        let target_block = self.function.blocks.get(target_index).ok_or_else(|| {
            invalid(
                ErrorCode::InvalidModule,
                format!("bloco alvo {} não existe na função", target.target),
                location.clone(),
            )
        })?;
        if target.args.len() != target_block.params.len() {
            return Err(invalid(
                ErrorCode::InvalidType,
                format!(
                    "bloco `{}` espera {} argumento(s), recebeu {}",
                    target_block.label,
                    target_block.params.len(),
                    target.args.len()
                ),
                location.clone(),
            ));
        }
        for arg in &target.args {
            self.check_use(block_index, position, *arg, location)?;
        }
        for (arg, param) in target.args.iter().zip(target_block.params.iter()) {
            self.check_operand_type(*arg, param.ty, location)?;
        }
        Ok(())
    }

    /// Exige que `value` tenha definição disponível no ponto de uso.
    fn check_use(
        &self,
        block_index: usize,
        position: usize,
        value: ValueId,
        location: &Location,
    ) -> Result<(), RvmError> {
        let index = value_index(value, location)?;
        let definition = match self.values.definitions.get(index).copied().flatten() {
            Some(definition) => definition,
            None => {
                return Err(invalid(
                    ErrorCode::InvalidModule,
                    format!("valor v{value} não definido na função"),
                    location.clone(),
                ))
            }
        };
        let available = match definition {
            Def::FunctionParam => true,
            Def::BlockParam { block } => dom_contains(&self.dominators, block_index, block),
            Def::Instruction {
                block,
                index: definition_index,
            } => {
                if block == block_index {
                    definition_index < position
                } else {
                    dom_contains(&self.dominators, block_index, block)
                }
            }
        };
        if available {
            Ok(())
        } else {
            Err(invalid(
                ErrorCode::InvalidControlFlow,
                format!("valor v{value} usado no bloco {block_index} sem definição dominante"),
                location.clone(),
            ))
        }
    }

    /// Exige que o tipo registrado de `value` seja exatamente `expected`.
    fn check_operand_type(
        &self,
        value: ValueId,
        expected: Type,
        location: &Location,
    ) -> Result<(), RvmError> {
        let actual = self.type_of_value(value, location)?;
        if actual == expected {
            Ok(())
        } else {
            Err(invalid(
                ErrorCode::InvalidType,
                format!("valor v{value} tem tipo {actual}, esperado {expected}"),
                location.clone(),
            ))
        }
    }

    /// Tipo registrado de `value` no mapa da função.
    fn type_of_value(&self, value: ValueId, location: &Location) -> Result<Type, RvmError> {
        let index = value_index(value, location)?;
        match self.values.types.get(index).copied().flatten() {
            Some(ty) => Ok(ty),
            None => Err(invalid(
                ErrorCode::InvalidModule,
                format!("tipo do valor v{value} desconhecido"),
                location.clone(),
            )),
        }
    }
}

/// Verifica uma função inteira, do CFG ao terminador do último bloco.
fn verify_function(module: &Module, function: &Function) -> Result<(), RvmError> {
    let location = Location::Function {
        function: function.name.clone(),
    };
    let cfg = build_cfg(function)?;
    check_reachability(function, &cfg)?;
    let dominators = compute_dominators(&cfg, &location)?;
    let values = build_values(module, function, &location)?;
    let facts = FunctionFacts {
        module,
        function,
        values,
        dominators,
    };
    for (block_index, block) in function.blocks.iter().enumerate() {
        facts.check_block(block_index, block)?;
    }
    Ok(())
}

/// Constrói sucessores e predecessores a partir dos terminadores.
///
/// A ordem de varredura é a de índice de bloco, então as listas de
/// predecessores são determinísticas.
fn build_cfg(function: &Function) -> Result<Cfg, RvmError> {
    let block_count = function.blocks.len();
    let mut successors: Vec<Vec<usize>> = Vec::new();
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); block_count];
    for (block_index, block) in function.blocks.iter().enumerate() {
        let location = Location::Block {
            function: function.name.clone(),
            block: block.label.clone(),
        };
        let mut outgoing = Vec::new();
        match &block.term {
            Terminator::Return(_) => {}
            Terminator::Br(target) => {
                outgoing.push(resolve_block_index(function, target.target, &location)?);
            }
            Terminator::CondBr {
                then_target,
                else_target,
                ..
            } => {
                outgoing.push(resolve_block_index(
                    function,
                    then_target.target,
                    &location,
                )?);
                let else_index = resolve_block_index(function, else_target.target, &location)?;
                outgoing.push(else_index);
            }
        }
        for &target in &outgoing {
            if let Some(list) = predecessors.get_mut(target) {
                list.push(block_index);
            }
        }
        successors.push(outgoing);
    }
    let entry = resolve_block_index(
        function,
        function.entry,
        &Location::Function {
            function: function.name.clone(),
        },
    )?;
    Ok(Cfg {
        successors,
        predecessors,
        entry,
    })
}

/// Rejeita o primeiro bloco inalcançável a partir da entrada, por índice.
fn check_reachability(function: &Function, cfg: &Cfg) -> Result<(), RvmError> {
    let mut reached = vec![false; function.blocks.len()];
    if let Some(slot) = reached.get_mut(cfg.entry) {
        *slot = true;
    }
    let mut pending = vec![cfg.entry];
    while let Some(block) = pending.pop() {
        if let Some(successors) = cfg.successors.get(block) {
            for &successor in successors {
                match reached.get_mut(successor) {
                    Some(slot) if !*slot => {
                        *slot = true;
                        pending.push(successor);
                    }
                    _ => {}
                }
            }
        }
    }
    for (index, block) in function.blocks.iter().enumerate() {
        if !reached.get(index).copied().unwrap_or(false) {
            return Err(invalid(
                ErrorCode::InvalidControlFlow,
                format!(
                    "bloco `{}` não é alcançável a partir do bloco de entrada",
                    block.label
                ),
                Location::Block {
                    function: function.name.clone(),
                    block: block.label.clone(),
                },
            ));
        }
    }
    Ok(())
}

/// Calcula os conjuntos de dominadores por iteração de ponto fixo.
///
/// `dom[entry] = {entry}` e `dom[b] = {b} ∪ ∩ dom[pred]`. Como a
/// alcançabilidade já foi exigida, todo bloco não-entrada tem ao menos um
/// predecessor. Cada conjunto é um bitset de `Vec<u64>` com palavras fixas.
fn compute_dominators(cfg: &Cfg, location: &Location) -> Result<Vec<Vec<u64>>, RvmError> {
    let block_count = cfg.predecessors.len();
    let words = match block_count.checked_add(63) {
        Some(total) => total / 64,
        None => {
            return Err(invalid(
                ErrorCode::InvalidModule,
                "contagem de blocos transbordou",
                location.clone(),
            ))
        }
    };
    let mut dominators: Vec<Vec<u64>> = Vec::new();
    for block in 0..block_count {
        if block == cfg.entry {
            let mut set = vec![0u64; words];
            set_bit(&mut set, cfg.entry);
            dominators.push(set);
        } else {
            dominators.push(vec![u64::MAX; words]);
        }
    }
    if words == 0 {
        return Ok(dominators);
    }

    let mut scratch = vec![0u64; words];
    loop {
        let mut changed = false;
        for block in 0..block_count {
            if block == cfg.entry {
                continue;
            }
            let predecessors = match cfg.predecessors.get(block) {
                Some(list) => list,
                None => continue,
            };
            let first = match predecessors.first() {
                Some(predecessor) => *predecessor,
                None => continue,
            };
            let first_dominators = match dominators.get(first) {
                Some(set) => set,
                None => continue,
            };
            for (slot, value) in scratch.iter_mut().zip(first_dominators.iter()) {
                *slot = *value;
            }
            for predecessor in predecessors.iter().skip(1) {
                if let Some(set) = dominators.get(*predecessor) {
                    for (slot, value) in scratch.iter_mut().zip(set.iter()) {
                        *slot &= *value;
                    }
                }
            }
            set_bit(&mut scratch, block);
            let changed_here = match dominators.get(block) {
                Some(current) => current
                    .iter()
                    .zip(scratch.iter())
                    .any(|(left, right)| left != right),
                None => false,
            };
            if changed_here {
                if let Some(current) = dominators.get_mut(block) {
                    for (slot, value) in current.iter_mut().zip(scratch.iter()) {
                        *slot = *value;
                    }
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    Ok(dominators)
}

/// Liga o bit `bit` do bitset, ignorando posições fora da faixa.
fn set_bit(words: &mut [u64], bit: usize) {
    let word_index = bit / 64;
    let offset = bit % 64;
    if let Some(word) = words.get_mut(word_index) {
        *word |= 1u64 << offset;
    }
}

/// Testa se o conjunto de dominadores do bloco `block` contém `candidate`.
fn dom_contains(dominators: &[Vec<u64>], block: usize, candidate: usize) -> bool {
    let set = match dominators.get(block) {
        Some(set) => set,
        None => return false,
    };
    let word_index = candidate / 64;
    let offset = candidate % 64;
    set.get(word_index)
        .is_some_and(|word| (*word & (1u64 << offset)) != 0)
}

/// Monta os mapas de definição e de tipo de todos os valores da função.
///
/// O comprimento é a contagem verificada de parâmetros, parâmetros de bloco e
/// definições de instrução; `structure::check` já garantiu que o conjunto de
/// `ValueId` é exatamente `{0..N-1}`.
fn build_values(
    module: &Module,
    function: &Function,
    location: &Location,
) -> Result<ValueMaps, RvmError> {
    let mut value_count = function.params.len();
    for block in &function.blocks {
        value_count = value_count
            .checked_add(block.params.len())
            .and_then(|total| total.checked_add(block.instrs.len()))
            .ok_or_else(|| {
                invalid(
                    ErrorCode::InvalidModule,
                    "contagem de valores transbordou",
                    location.clone(),
                )
            })?;
    }
    let mut definitions = vec![None; value_count];
    let mut types = vec![None; value_count];
    for (index, param) in function.params.iter().enumerate() {
        record_definition(&mut definitions, index, Def::FunctionParam, location)?;
        record_type(&mut types, index, param.ty, location)?;
    }
    for (block_index, block) in function.blocks.iter().enumerate() {
        for param in &block.params {
            let index = value_index(param.value, location)?;
            record_definition(
                &mut definitions,
                index,
                Def::BlockParam { block: block_index },
                location,
            )?;
            record_type(&mut types, index, param.ty, location)?;
        }
        for (instr_index, instr) in block.instrs.iter().enumerate() {
            let index = value_index(instr.def, location)?;
            record_definition(
                &mut definitions,
                index,
                Def::Instruction {
                    block: block_index,
                    index: instr_index,
                },
                location,
            )?;
            let ty = operation_type(module, &instr.op, location)?;
            record_type(&mut types, index, ty, location)?;
        }
    }
    Ok(ValueMaps { definitions, types })
}

/// Tipo do valor definido por uma operação.
///
/// Para `call_static` o tipo vem de [`Function::ret`] da função chamada; não se
/// usa o atalho [`Function::value_type`], que devolve [`None`] para chamadas.
fn operation_type(module: &Module, op: &Op, location: &Location) -> Result<Type, RvmError> {
    match op {
        Op::ConstI64(_) => Ok(Type::I64),
        Op::ConstBool(_) => Ok(Type::Bool),
        Op::ConstStr(_) => Ok(Type::Str),
        Op::Bin { op, .. } => Ok(op.result_type()),
        Op::Cmp { op, .. } => Ok(op.result_type()),
        Op::StrEq { .. } | Op::StrNe { .. } => Ok(Type::Bool),
        Op::StrLen { .. } => Ok(Type::I64),
        Op::ConcatStr { .. } => Ok(Type::Str),
        Op::Call { callee, .. } => module
            .function(*callee)
            .map(|target| target.ret)
            .ok_or_else(|| {
                invalid(
                    ErrorCode::InvalidModule,
                    format!("função chamada {callee} não existe no módulo"),
                    location.clone(),
                )
            }),
    }
}

/// Registra a definição de um valor, recusando posição repetida ou fora da faixa.
fn record_definition(
    definitions: &mut [Option<Def>],
    index: usize,
    definition: Def,
    location: &Location,
) -> Result<(), RvmError> {
    match definitions.get_mut(index) {
        Some(slot) if slot.is_none() => {
            *slot = Some(definition);
            Ok(())
        }
        Some(_) => Err(invalid(
            ErrorCode::InvalidModule,
            format!("valor na posição {index} definido mais de uma vez"),
            location.clone(),
        )),
        None => Err(invalid(
            ErrorCode::InvalidModule,
            format!("posição de valor {index} fora da faixa"),
            location.clone(),
        )),
    }
}

/// Registra o tipo de um valor, recusando posição repetida ou fora da faixa.
fn record_type(
    types: &mut [Option<Type>],
    index: usize,
    ty: Type,
    location: &Location,
) -> Result<(), RvmError> {
    match types.get_mut(index) {
        Some(slot) if slot.is_none() => {
            *slot = Some(ty);
            Ok(())
        }
        Some(_) => Err(invalid(
            ErrorCode::InvalidModule,
            format!("tipo do valor na posição {index} definido mais de uma vez"),
            location.clone(),
        )),
        None => Err(invalid(
            ErrorCode::InvalidModule,
            format!("posição de valor {index} fora da faixa"),
            location.clone(),
        )),
    }
}

/// Converte um `ValueId` em índice `usize`, falhando fechado.
fn value_index(value: ValueId, location: &Location) -> Result<usize, RvmError> {
    usize::try_from(value).map_err(|_| {
        invalid(
            ErrorCode::InvalidModule,
            format!("ValueId {value} não representável nesta plataforma"),
            location.clone(),
        )
    })
}

/// Converte um `BlockId` em índice e exige que o bloco exista na função.
fn resolve_block_index(
    function: &Function,
    id: BlockId,
    location: &Location,
) -> Result<usize, RvmError> {
    let index = usize::try_from(id).map_err(|_| {
        invalid(
            ErrorCode::InvalidControlFlow,
            format!("bloco {id} não representável nesta plataforma"),
            location.clone(),
        )
    })?;
    if index < function.blocks.len() {
        Ok(index)
    } else {
        Err(invalid(
            ErrorCode::InvalidControlFlow,
            format!("bloco {id} não existe na função `{}`", function.name),
            location.clone(),
        ))
    }
}

/// Constrói um diagnóstico com código estável e localização.
fn invalid(code: ErrorCode, detail: impl Into<String>, location: Location) -> RvmError {
    RvmError::new(code, detail).at(location)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvm_ir::testing;
    use rvm_ir::Param;
    use rvm_spec::{BinOp, CmpOp, ErrorClass, IR_VERSION};

    fn funcao_mut(module: &mut Module, index: usize) -> &mut Function {
        module.functions.get_mut(index).expect("função de teste")
    }

    fn bloco_mut(function: &mut Function, index: usize) -> &mut Block {
        function.blocks.get_mut(index).expect("bloco de teste")
    }

    fn instr_mut(block: &mut Block, index: usize) -> &mut Instr {
        block.instrs.get_mut(index).expect("instrução de teste")
    }

    fn erro(module: Module) -> RvmError {
        VerifiedModule::verify(module, &Limits::default())
            .expect_err("módulo deveria ser rejeitado")
    }

    fn assert_erro(module: Module, code: ErrorCode) -> RvmError {
        let error = erro(module);
        assert_eq!(error.code(), code);
        assert_eq!(error.class(), ErrorClass::Module);
        assert!(
            error.location().is_some(),
            "diagnóstico sem localização: {error}"
        );
        error
    }

    #[test]
    fn modulos_de_referencia_verificam() {
        for module in [
            testing::soma(),
            testing::constante(42),
            testing::com_merge(),
            testing::loop_infinito(),
            testing::desvio_verdadeiro(),
            testing::unidade(),
            testing::overflow(),
            testing::comparacao(),
        ] {
            let name = module.name.clone();
            assert!(
                VerifiedModule::verify(module, &Limits::default()).is_ok(),
                "módulo de referência `{name}` deveria verificar"
            );
        }
    }

    #[test]
    fn recursao_verifica() {
        assert!(VerifiedModule::verify(testing::recursao(), &Limits::default()).is_ok());
    }

    #[test]
    fn entrada_com_parametros_verifica() {
        let module = Module {
            name: "entrada".to_string(),
            version: IR_VERSION,
            entry: 0,
            functions: vec![Function {
                id: 0,
                name: "main".to_string(),
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
                    instrs: Vec::new(),
                    term: Terminator::Return(Some(0)),
                }],
            }],
        };
        assert!(VerifiedModule::verify(module, &Limits::default()).is_ok());
    }

    #[test]
    fn rejeita_argumento_de_chamada_com_tipo_errado() {
        let mut module = testing::soma();
        *instr_mut(bloco_mut(funcao_mut(&mut module, 0), 0), 1) = Instr {
            def: 1,
            op: Op::ConstBool(true),
        };
        let error = assert_erro(module, ErrorCode::InvalidType);
        assert!(matches!(
            error.location(),
            Some(Location::Instruction { .. })
        ));
    }

    #[test]
    fn rejeita_aridade_de_chamada() {
        let mut module = testing::soma();
        match &mut instr_mut(bloco_mut(funcao_mut(&mut module, 0), 0), 2).op {
            Op::Call { args, .. } => *args = vec![0],
            _ => panic!("instrução de chamada esperada"),
        }
        let error = assert_erro(module, ErrorCode::InvalidType);
        assert!(matches!(
            error.location(),
            Some(Location::Instruction { .. })
        ));
    }

    #[test]
    fn propaga_operando_inexistente_como_invalid_module() {
        let mut module = testing::soma();
        match &mut instr_mut(bloco_mut(funcao_mut(&mut module, 1), 0), 0).op {
            Op::Bin { lhs, .. } => *lhs = 99,
            _ => panic!("instrução binária esperada"),
        }
        let error = assert_erro(module, ErrorCode::InvalidModule);
        assert!(matches!(
            error.location(),
            Some(Location::Instruction { .. })
        ));
    }

    #[test]
    fn rejeita_retorno_sem_valor_em_funcao_i64() {
        let mut module = testing::constante(1);
        bloco_mut(funcao_mut(&mut module, 0), 0).term = Terminator::Return(None);
        let error = assert_erro(module, ErrorCode::InvalidType);
        assert!(matches!(error.location(), Some(Location::Block { .. })));
    }

    #[test]
    fn rejeita_retorno_com_valor_em_funcao_unit() {
        let mut module = testing::unidade();
        let block = bloco_mut(funcao_mut(&mut module, 0), 0);
        block.instrs.push(Instr {
            def: 0,
            op: Op::ConstI64(1),
        });
        block.term = Terminator::Return(Some(0));
        assert_erro(module, ErrorCode::InvalidType);
    }

    #[test]
    fn rejeita_tipo_de_retorno_divergente() {
        let mut module = testing::constante(1);
        funcao_mut(&mut module, 0).ret = Type::Bool;
        assert_erro(module, ErrorCode::InvalidType);
    }

    #[test]
    fn rejeita_condicao_de_desvio_i64() {
        let mut module = testing::desvio_verdadeiro();
        *instr_mut(bloco_mut(funcao_mut(&mut module, 0), 0), 0) = Instr {
            def: 0,
            op: Op::ConstI64(1),
        };
        let error = assert_erro(module, ErrorCode::InvalidType);
        assert!(matches!(error.location(), Some(Location::Block { .. })));
    }

    #[test]
    fn rejeita_aridade_de_argumento_de_bloco() {
        let mut module = testing::com_merge();
        match &mut bloco_mut(funcao_mut(&mut module, 0), 0).term {
            Terminator::CondBr { then_target, .. } => then_target.args = vec![1, 2],
            _ => panic!("cond_br esperado"),
        }
        let error = assert_erro(module, ErrorCode::InvalidType);
        assert!(matches!(error.location(), Some(Location::Block { .. })));
    }

    #[test]
    fn rejeita_tipo_de_argumento_de_bloco() {
        let mut module = testing::com_merge();
        match &mut bloco_mut(funcao_mut(&mut module, 0), 0).term {
            Terminator::CondBr { then_target, .. } => then_target.args = vec![0],
            _ => panic!("cond_br esperado"),
        }
        assert_erro(module, ErrorCode::InvalidType);
    }

    #[test]
    fn rejeita_uso_antes_da_definicao_no_mesmo_bloco() {
        let mut module = testing::soma();
        *instr_mut(bloco_mut(funcao_mut(&mut module, 0), 0), 0) = Instr {
            def: 0,
            op: Op::Bin {
                op: BinOp::AddI64Checked,
                lhs: 1,
                rhs: 1,
            },
        };
        let error = assert_erro(module, ErrorCode::InvalidControlFlow);
        assert!(matches!(
            error.location(),
            Some(Location::Instruction { .. })
        ));
    }

    #[test]
    fn rejeita_valor_de_bloco_nao_dominante() {
        let mut module = testing::desvio_verdadeiro();
        bloco_mut(funcao_mut(&mut module, 0), 2).term = Terminator::Return(Some(1));
        let error = assert_erro(module, ErrorCode::InvalidControlFlow);
        assert!(matches!(
            error.location(),
            Some(Location::Block { block, .. }) if block == "senao"
        ));
    }

    #[test]
    fn rejeita_bloco_inalcancavel() {
        let mut module = testing::desvio_verdadeiro();
        match &mut bloco_mut(funcao_mut(&mut module, 0), 0).term {
            Terminator::CondBr { else_target, .. } => else_target.target = 1,
            _ => panic!("cond_br esperado"),
        }
        let error = assert_erro(module, ErrorCode::InvalidControlFlow);
        assert!(matches!(
            error.location(),
            Some(Location::Block { block, .. }) if block == "senao"
        ));
    }

    #[test]
    fn rejeita_aritmetica_com_operando_nao_i64() {
        let mut module = testing::com_merge();
        *instr_mut(bloco_mut(funcao_mut(&mut module, 0), 0), 0) = Instr {
            def: 1,
            op: Op::Bin {
                op: BinOp::SubI64Checked,
                lhs: 0,
                rhs: 0,
            },
        };
        assert_erro(module, ErrorCode::InvalidType);
    }

    #[test]
    fn rejeita_comparacao_com_operando_nao_i64() {
        let mut module = testing::com_merge();
        *instr_mut(bloco_mut(funcao_mut(&mut module, 0), 0), 0) = Instr {
            def: 1,
            op: Op::Cmp {
                op: CmpOp::Eq,
                lhs: 0,
                rhs: 0,
            },
        };
        assert_erro(module, ErrorCode::InvalidType);
    }
}
