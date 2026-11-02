//! `rvm-ir` — modelo da RIR experimental EXP-0.
//!
//! A RIR é uma fronteira entre frontends e o runtime (ADR-0002). Este crate
//! define apenas dados e invariantes estruturais; tipos, operações e códigos de
//! erro vêm de [`rvm_spec`], e verificação de tipos/fluxo fica em `rvm-verify`.
//!
//! # Invariantes de codificação (EXP-0)
//!
//! 1. `functions[i].id == i` e `blocks[i].id == i`: IDs são densos, então a busca
//!    é `functions.get(id)`.
//! 2. Parâmetros de função ocupam os `ValueId` `0..params.len()` na ordem de
//!    declaração; os demais valores (parâmetros de bloco e definições de
//!    instrução) completam o conjunto `{0..N-1}` sem repetição.
//! 3. O bloco de entrada é `blocks[function.entry]`, não aceita parâmetros e é
//!    alcançável por definição.
//! 4. Nomes são identificadores ASCII `[A-Za-z_][A-Za-z0-9_]*`, com tamanho
//!    limitado por [`rvm_spec::Limits::max_name_len`]; o prefixo `v` seguido
//!    apenas de dígitos é reservado para a impressão canônica de definições.
//!
//! Todos esses invariantes são checados por [`structure::check`] **antes** de
//! qualquer alocação proporcional ao módulo e antes da verificação de tipos.

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

use rvm_spec::{BinOp, CmpOp, Type};

pub mod structure;
#[cfg(any(test, feature = "test-helpers"))]
pub mod testing;

/// Identificador numérico de função dentro do módulo (denso: `id == índice`).
pub type FnId = u32;

/// Identificador numérico de bloco dentro da função (denso: `id == índice`).
pub type BlockId = u32;

/// Identificador numérico de valor SSA dentro da função (denso e único).
pub type ValueId = u32;

/// Módulo RIR completo.
///
/// Unidade de decodificação, verificação e execução. Não contém imports no
/// EXP-0: chamadas só alcançam funções do próprio módulo e são resolvidas
/// estaticamente pelo verificador.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    /// Nome declarado do módulo (identificador, chave de diagnóstico).
    pub name: String,
    /// Versão do formato RIR. Precisa ser [`rvm_spec::IR_VERSION`].
    pub version: u16,
    /// Função de entrada executada por padrão pelo host.
    pub entry: FnId,
    /// Funções do módulo, indexadas pelo próprio `FnId`.
    pub functions: Vec<Function>,
}

/// Função RIR: assinatura, bloco de entrada e blocos.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    /// Identificador da função; deve ser igual ao índice em [`Module::functions`].
    pub id: FnId,
    /// Nome declarado da função (identificador único no módulo).
    pub name: String,
    /// Parâmetros na ordem de declaração; ocupam os `ValueId` `0..len`.
    pub params: Vec<Param>,
    /// Tipo de retorno declarado.
    pub ret: Type,
    /// Bloco executado primeiro.
    pub entry: BlockId,
    /// Blocos da função, indexados pelo próprio `BlockId`.
    pub blocks: Vec<Block>,
}

/// Parâmetro de função.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    /// Nome do parâmetro (identificador, único entre valores nomeados da função).
    pub name: String,
    /// Tipo exigido na chamada.
    pub ty: Type,
}

/// Bloco básico: parâmetros, instruções e exatamente um terminador.
///
/// O terminador é obrigatório por construção: um bloco sem terminador não é
/// representável como [`Block`] válido. A fronteira textual rejeita bloco sem
/// terminador com `invalid_control_flow`, e [`structure::check`] confirma o
/// restante dos invariantes para módulos construídos por API ou bytecode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// Identificador do bloco; deve ser igual ao índice em [`Function::blocks`].
    pub id: BlockId,
    /// Rótulo declarado (identificador único na função).
    pub label: String,
    /// Parâmetros de bloco: valores definidos na entrada do bloco.
    pub params: Vec<BlockParam>,
    /// Instruções executadas em ordem antes do terminador.
    pub instrs: Vec<Instr>,
    /// Terminador do bloco; encerra o bloco e define o sucessor.
    pub term: Terminator,
}

/// Parâmetro de bloco: valor definido na entrada do bloco (merge explícito).
///
/// Este é o mecanismo experimental de merge do EXP-0: um desvio passa argumentos
/// para o bloco alvo, que os trata como valores definidos em sua entrada
/// (semântica de phi explícito).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockParam {
    /// Identificador do valor definido na entrada do bloco.
    pub value: ValueId,
    /// Nome usado na impressão canônica e em diagnósticos.
    pub name: String,
    /// Tipo do valor.
    pub ty: Type,
}

/// Instrução: define um valor SSA aplicando uma operação.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instr {
    /// Valor definido por esta instrução.
    pub def: ValueId,
    /// Operação aplicada.
    pub op: Op,
}

/// Operação experimental do EXP-0.
///
/// Não existe conversão numérica, ponto flutuante, concatenação de `string` nem
/// operação sobre `bool`/`unit`: qualquer outra operação é rejeitada na fronteira
/// (código `unknown_opcode` ou `unsupported_feature`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// Define uma constante `i64` (`const_i64`).
    ConstI64(i64),
    /// Define uma constante `bool` (`const_bool`).
    ConstBool(bool),
    /// Define uma constante `string` (`const_str`). O literal vive no módulo.
    ConstStr(String),
    /// Aplica [`BinOp`] a dois `i64`; transbordo produz `integer_overflow`.
    Bin {
        /// Operação aritmética verificada.
        op: BinOp,
        /// Operando esquerdo.
        lhs: ValueId,
        /// Operando direito.
        rhs: ValueId,
    },
    /// Aplica [`CmpOp`] a dois `i64`, produzindo `bool`.
    Cmp {
        /// Comparação assinada.
        op: CmpOp,
        /// Operando esquerdo.
        lhs: ValueId,
        /// Operando direito.
        rhs: ValueId,
    },
    /// Compara dois `string` por conteúdo (`str_eq`), produzindo `bool`.
    StrEq {
        /// Operando esquerdo.
        lhs: ValueId,
        /// Operando direito.
        rhs: ValueId,
    },
    /// Desigualdade de `string` (`str_ne`), produzindo `bool`.
    StrNe {
        /// Operando esquerdo.
        lhs: ValueId,
        /// Operando direito.
        rhs: ValueId,
    },
    /// Comprimento em bytes de um `string` (`str_len`), produzindo `i64`.
    StrLen {
        /// Operando `string`.
        operand: ValueId,
    },
    /// Concatena dois `string` (`concat_str`), produzindo `string` (ADR-0025).
    ConcatStr {
        /// Operando esquerdo.
        lhs: ValueId,
        /// Operando direito.
        rhs: ValueId,
    },
    /// Chama estaticamente uma função do mesmo módulo (`call_static`).
    Call {
        /// Função chamada; resolvida estaticamente pelo verificador.
        callee: FnId,
        /// Argumentos, na ordem da assinatura.
        args: Vec<ValueId>,
    },
}

impl Op {
    /// Tipo do valor definido pela operação.
    #[must_use]
    pub fn result_type(&self) -> Type {
        match self {
            Op::ConstI64(_) => Type::I64,
            Op::ConstBool(_) => Type::Bool,
            Op::ConstStr(_) => Type::Str,
            Op::Bin { op, .. } => op.result_type(),
            Op::Cmp { op, .. } => op.result_type(),
            Op::StrEq { .. } | Op::StrNe { .. } => Type::Bool,
            Op::StrLen { .. } => Type::I64,
            Op::ConcatStr { .. } => Type::Str,
            Op::Call { .. } => Type::I64, // substituído pelo tipo de retorno no verificador
        }
    }

    /// Identificadores de operandos usados pela operação, em ordem estável.
    ///
    /// Aloca um vetor pequeno (no máximo `max_call_args` posições). Percursos que
    /// não podem alocar devem casar diretamente sobre [`Op`].
    #[must_use]
    pub fn operand_ids(&self) -> Vec<ValueId> {
        match self {
            Op::ConstI64(_) | Op::ConstBool(_) | Op::ConstStr(_) => Vec::new(),
            Op::Bin { lhs, rhs, .. }
            | Op::Cmp { lhs, rhs, .. }
            | Op::StrEq { lhs, rhs }
            | Op::StrNe { lhs, rhs }
            | Op::ConcatStr { lhs, rhs } => vec![*lhs, *rhs],
            Op::StrLen { operand } => vec![*operand],
            Op::Call { args, .. } => args.clone(),
        }
    }
}

/// Terminador de bloco: exatamente um por bloco.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terminator {
    /// Retorna ao chamador, com valor (`Some`) ou sem valor (`None`, só para `unit`).
    Return(Option<ValueId>),
    /// Desvio incondicional para outro bloco, com argumentos de bloco.
    Br(BranchTarget),
    /// Desvio condicional: `cond` é `bool`; `then_target` se verdadeiro,
    /// `else_target` caso contrário.
    CondBr {
        /// Condição booleana.
        cond: ValueId,
        /// Alvo quando a condição é verdadeira.
        then_target: BranchTarget,
        /// Alvo quando a condição é falsa.
        else_target: BranchTarget,
    },
}

/// Alvo de desvio: bloco destino e argumentos passados aos parâmetros do bloco.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchTarget {
    /// Bloco destino.
    pub target: BlockId,
    /// Argumentos, um por parâmetro do bloco destino.
    pub args: Vec<ValueId>,
}

impl Terminator {
    /// Identificadores de operandos usados pelo terminador (condição, valor de
    /// retorno e argumentos de desvio), em ordem estável.
    #[must_use]
    pub fn operand_ids(&self) -> Vec<ValueId> {
        let mut ids = Vec::new();
        match self {
            Terminator::Return(Some(value)) => ids.push(*value),
            Terminator::Return(None) => {}
            Terminator::Br(target) => ids.extend_from_slice(&target.args),
            Terminator::CondBr {
                cond,
                then_target,
                else_target,
            } => {
                ids.push(*cond);
                ids.extend_from_slice(&then_target.args);
                ids.extend_from_slice(&else_target.args);
            }
        }
        ids
    }
}

impl Module {
    /// Função pelo identificador denso.
    #[must_use]
    pub fn function(&self, id: FnId) -> Option<&Function> {
        self.functions.get(id as usize)
    }

    /// Função pelo nome declarado (primeira ocorrência).
    #[must_use]
    pub fn function_by_name(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|function| function.name == name)
    }
}

impl Function {
    /// Bloco pelo identificador denso.
    #[must_use]
    pub fn block(&self, id: BlockId) -> Option<&Block> {
        self.blocks.get(id as usize)
    }

    /// Quantidade total de valores SSA da função (parâmetros + parâmetros de
    /// bloco + definições de instrução), de forma saturante.
    #[must_use]
    pub fn value_count(&self) -> u32 {
        let mut total: u32 = self.params.len().try_into().unwrap_or(u32::MAX);
        for block in &self.blocks {
            total = total.saturating_add(block.params.len().try_into().unwrap_or(u32::MAX));
            total = total.saturating_add(block.instrs.len().try_into().unwrap_or(u32::MAX));
        }
        total
    }

    /// `true` quando a função é uma **declaração externa** (sem blocos).
    ///
    /// Uma declaração externa existe para a ligação inter-módulo
    /// (`rvm-link`, ADR-0018): ela nomeia uma função a ser resolvida por outro
    /// módulo. O verificador (`rvm-verify`) rejeita módulos que ainda contenham
    /// externas, então o executor nunca vê uma declaração externa.
    #[must_use]
    pub fn is_external(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Tipo de um valor conhecido.
    ///
    /// Pressupõe que [`structure::check`] já aceitou o módulo (IDs densos). Para
    /// módulos não verificados devolve [`None`] em vez de arriscar indexação.
    ///
    /// Devolve [`None`] também para valores definidos por `call_static`: o tipo
    /// depende da assinatura da função chamada, que só o verificador (`rvm-verify`)
    /// resolve. Não use este atalho para inferência de tipos.
    #[must_use]
    pub fn value_type(&self, id: ValueId) -> Option<Type> {
        if let Some(param) = self.params.get(id as usize) {
            return Some(param.ty);
        }
        for block in &self.blocks {
            for param in &block.params {
                if param.value == id {
                    return Some(param.ty);
                }
            }
            for instr in &block.instrs {
                if instr.def == id {
                    return match &instr.op {
                        Op::Call { .. } => None,
                        op => Some(op.result_type()),
                    };
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvm_spec::Limits;

    #[test]
    fn modulo_de_teste_passa_pela_checagem_estrutural() {
        let module = testing::soma();
        assert!(structure::check(&module, &Limits::default()).is_ok());
        assert_eq!(module.function_by_name("soma").map(|f| f.id), Some(1));
        assert_eq!(module.function(1).map(|f| f.ret), Some(Type::I64));
    }

    #[test]
    fn ids_novos_sao_densos_e_unicos() {
        let module = testing::soma();
        let soma = module.function(1).unwrap();
        let mut seen = vec![false; soma.value_count() as usize];
        for slot in seen.iter_mut().take(soma.params.len()) {
            *slot = true;
        }
        for (index, block) in soma.blocks.iter().enumerate() {
            assert_eq!(block.id as usize, index);
            for param in &block.params {
                assert!(!seen[param.value as usize]);
                seen[param.value as usize] = true;
            }
            for instr in &block.instrs {
                assert!(!seen[instr.def as usize]);
                seen[instr.def as usize] = true;
            }
        }
        assert!(
            seen[..2].iter().all(|v| *v),
            "parâmetros ocupam 0..params.len()"
        );
        assert!(seen.iter().all(|v| *v), "todos os valores são definidos");
    }
}
