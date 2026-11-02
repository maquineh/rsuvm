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

use rvm_spec::{
    BinOp, CmpOp, FloatOp, StructId, TypeDef, TypeId, TYPE_BOOL, TYPE_F64, TYPE_I64, TYPE_STR,
};

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
    /// Tabela de tipos do módulo (ADR-0034), indexada pelo próprio [`TypeId`].
    /// Os cinco primeiros são canônicos: `unit`, `bool`, `i64`, `string`, `f64`.
    pub types: Vec<TypeDef>,
    /// Structs declaradas no módulo (ADR-0029), indexadas pelo próprio
    /// [`StructId`]. Campos referenciam a tabela de tipos.
    pub structs: Vec<StructDef>,
    /// Funções do módulo, indexadas pelo próprio `FnId`.
    pub functions: Vec<Function>,
}

/// Struct declarada no módulo (ADR-0029).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructDef {
    /// Identificador denso; deve ser igual ao índice em [`Module::structs`].
    pub id: StructId,
    /// Nome declarado (identificador único no módulo).
    pub name: String,
    /// Campos, na ordem de declaração.
    pub fields: Vec<FieldDef>,
}

impl StructDef {
    /// Índice do campo `name`, se existir.
    #[must_use]
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|field| field.name == name)
    }

    /// Campo pelo nome.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|field| field.name == name)
    }
}

/// Campo de uma struct: nome e tipo (ADR-0029/0034).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDef {
    /// Nome do campo (identificador, único dentro da struct).
    pub name: String,
    /// Tipo do campo, referenciando a tabela do módulo.
    pub ty: TypeId,
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
    pub ret: TypeId,
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
    pub ty: TypeId,
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
    pub ty: TypeId,
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
    /// Define uma constante `f64` (`const_f64`), guardada como bits IEEE-754.
    ConstF64(u64),
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
    /// Aplica [`FloatOp`] a dois `f64` (`add_f64`, `lt_f64`, …), ADR-0026.
    Float {
        /// Operação sobre `f64`.
        op: FloatOp,
        /// Operando esquerdo.
        lhs: ValueId,
        /// Operando direito.
        rhs: ValueId,
    },
    /// Cria um array imutável a partir de elementos já definidos (`array`), ADR-0027.
    ArrayLiteral {
        /// Tipo do array (na tabela do módulo).
        ty: TypeId,
        /// Elementos.
        items: Vec<ValueId>,
    },
    /// Comprimento de um array (`array_len`), produzindo `i64`, ADR-0027.
    ArrayLen {
        /// Operando array.
        array: ValueId,
    },
    /// Lê um elemento de um array (`array_get`), com bounds check, ADR-0027.
    ArrayGet {
        /// Operando array.
        array: ValueId,
        /// Índice (`i64`).
        index: ValueId,
    },
    /// Substitui um elemento de um array (`array_set`), devolvendo um **novo**
    /// array; o array de origem não é alterado (atualização funcional, ADR-0028).
    ArraySet {
        /// Operando array.
        array: ValueId,
        /// Índice (`i64`).
        index: ValueId,
        /// Novo valor do elemento; precisa ter o tipo do elemento.
        value: ValueId,
    },
    /// Constrói uma struct a partir dos valores dos campos, na ordem de declaração
    /// (`struct_new`), ADR-0029.
    StructNew {
        /// Struct construída.
        struct_id: StructId,
        /// Valores dos campos, na ordem de [`StructDef::fields`].
        fields: Vec<ValueId>,
    },
    /// Lê um campo de uma struct (`field_get`), ADR-0029.
    FieldGet {
        /// Operando struct.
        object: ValueId,
        /// Nome do campo; o tipo é resolvido pelo verificador.
        field: String,
    },
    /// Substitui um campo de uma struct (`field_set`), devolvendo uma **nova**
    /// struct; a de origem não é alterada (atualização funcional, ADR-0032).
    FieldSet {
        /// Operando struct.
        object: ValueId,
        /// Nome do campo.
        field: String,
        /// Novo valor do campo; precisa ter o tipo do campo.
        value: ValueId,
    },
    /// Define `none` de um `option`; o `TypeId` é o do próprio `option`, ADR-0030.
    OptionNone(TypeId),
    /// Envolve um valor em `some` (`option_some`), ADR-0030.
    OptionSome {
        /// Tipo do `option` (na tabela do módulo).
        ty: TypeId,
        /// Payload.
        value: ValueId,
    },
    /// Testa se um `option` é `some` (`option_is_some`), produzindo `bool`, ADR-0030.
    OptionIsSome {
        /// Operando `option`.
        value: ValueId,
    },
    /// Extrai o payload de um `option` (`option_unwrap`); `none` produz o trap
    /// `unwrap_none`, ADR-0030.
    OptionUnwrap {
        /// Operando `option`.
        value: ValueId,
    },
    /// Coalesce (`coalesce`): devolve o payload do `option` se `some`, senão o
    /// fallback (ADR-0030). O resultado é o tipo do fallback.
    Coalesce {
        /// Operando `option`.
        option: ValueId,
        /// Fallback (mesmo payload, ou o próprio `option`).
        fallback: ValueId,
    },
    /// Igualdade de dois `option` de mesmo payload (`option_eq`), ADR-0031.
    OptionEq {
        /// Operando esquerdo.
        lhs: ValueId,
        /// Operando direito.
        rhs: ValueId,
    },
    /// Desigualdade de dois `option` de mesmo payload (`option_ne`), ADR-0031.
    OptionNe {
        /// Operando esquerdo.
        lhs: ValueId,
        /// Operando direito.
        rhs: ValueId,
    },
    /// Constrói um `result` com a variante `ok` (`result_ok`), ADR-0033. O
    /// `TypeId` é o do próprio `result`.
    ResultOk {
        /// Tipo do `result` (na tabela do módulo).
        ty: TypeId,
        /// Valor `ok`.
        value: ValueId,
    },
    /// Constrói um `result` com a variante `err` (`result_err`), ADR-0033. O
    /// `TypeId` é o do próprio `result`.
    ResultErr {
        /// Tipo do `result` (na tabela do módulo).
        ty: TypeId,
        /// Valor `err`.
        value: ValueId,
    },
    /// Testa se um `result` é `ok` (`result_is_ok`), ADR-0033.
    ResultIsOk {
        /// Operando `result`.
        value: ValueId,
    },
    /// Extrai o payload `ok` (`result_unwrap`); `err` produz o trap
    /// `unwrap_error`, ADR-0033.
    ResultUnwrap {
        /// Operando `result`.
        value: ValueId,
    },
    /// Extrai o payload `err` (`result_unwrap_err`); `ok` produz o trap
    /// `unwrap_error`, ADR-0033.
    ResultUnwrapErr {
        /// Operando `result`.
        value: ValueId,
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
    /// Tipo do valor definido pela operação, quando conhecido só pela forma da
    /// operação.
    ///
    /// Devolve [`None`] quando o tipo depende dos operandos ou da tabela do
    /// módulo (arrays, structs, opcionais, resultados e chamadas); o verificador
    /// resolve esses casos em `build_values`.
    #[must_use]
    pub fn result_type(&self) -> Option<TypeId> {
        match self {
            Op::ConstI64(_) => Some(TYPE_I64),
            Op::ConstBool(_) => Some(TYPE_BOOL),
            Op::ConstStr(_) => Some(TYPE_STR),
            Op::ConstF64(_) => Some(TYPE_F64),
            Op::Bin { op, .. } => Some(op.result_type()),
            Op::Cmp { op, .. } => Some(op.result_type()),
            Op::StrEq { .. } | Op::StrNe { .. } => Some(TYPE_BOOL),
            Op::StrLen { .. } => Some(TYPE_I64),
            Op::ConcatStr { .. } => Some(TYPE_STR),
            Op::Float { op, .. } => Some(op.result_type()),
            Op::ArrayLiteral { ty, .. } => Some(*ty),
            Op::ArrayLen { .. } => Some(TYPE_I64),
            Op::OptionNone(ty) => Some(*ty),
            Op::OptionSome { ty, .. } => Some(*ty),
            Op::OptionIsSome { .. } => Some(TYPE_BOOL),
            Op::OptionEq { .. } | Op::OptionNe { .. } => Some(TYPE_BOOL),
            Op::ResultOk { ty, .. } | Op::ResultErr { ty, .. } => Some(*ty),
            Op::ResultIsOk { .. } => Some(TYPE_BOOL),
            Op::ArrayGet { .. }
            | Op::ArraySet { .. }
            | Op::StructNew { .. }
            | Op::FieldGet { .. }
            | Op::FieldSet { .. }
            | Op::OptionUnwrap { .. }
            | Op::Coalesce { .. }
            | Op::ResultUnwrap { .. }
            | Op::ResultUnwrapErr { .. }
            | Op::Call { .. } => None,
        }
    }

    /// Identificadores de operandos usados pela operação, em ordem estável.
    ///
    /// Aloca um vetor pequeno (no máximo `max_call_args` posições). Percursos que
    /// não podem alocar devem casar diretamente sobre [`Op`].
    #[must_use]
    pub fn operand_ids(&self) -> Vec<ValueId> {
        match self {
            Op::ConstI64(_) | Op::ConstBool(_) | Op::ConstStr(_) | Op::ConstF64(_) => Vec::new(),
            Op::Bin { lhs, rhs, .. }
            | Op::Cmp { lhs, rhs, .. }
            | Op::StrEq { lhs, rhs }
            | Op::StrNe { lhs, rhs }
            | Op::ConcatStr { lhs, rhs }
            | Op::Float { lhs, rhs, .. } => vec![*lhs, *rhs],
            Op::StrLen { operand } => vec![*operand],
            Op::ArrayLiteral { items, .. } => items.clone(),
            Op::ArrayLen { array } => vec![*array],
            Op::ArrayGet { array, index } => vec![*array, *index],
            Op::ArraySet {
                array,
                index,
                value,
            } => vec![*array, *index, *value],
            Op::StructNew { fields, .. } => fields.clone(),
            Op::FieldGet { object, .. } => vec![*object],
            Op::FieldSet { object, value, .. } => vec![*object, *value],
            Op::OptionNone(_) => Vec::new(),
            Op::OptionSome { value, .. }
            | Op::OptionIsSome { value }
            | Op::OptionUnwrap { value } => {
                vec![*value]
            }
            Op::Coalesce { option, fallback } => vec![*option, *fallback],
            Op::OptionEq { lhs, rhs } | Op::OptionNe { lhs, rhs } => vec![*lhs, *rhs],
            Op::ResultOk { value, .. } | Op::ResultErr { value, .. } => vec![*value],
            Op::ResultIsOk { value }
            | Op::ResultUnwrap { value }
            | Op::ResultUnwrapErr { value } => vec![*value],
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

    /// Struct pelo identificador denso.
    #[must_use]
    pub fn struct_def(&self, id: StructId) -> Option<&StructDef> {
        self.structs.get(id as usize)
    }

    /// Definição do tipo `id`.
    #[must_use]
    pub fn type_def(&self, id: TypeId) -> Option<TypeDef> {
        self.types.get(id as usize).copied()
    }

    /// Primeiro `TypeId` cuja definição é exatamente `def`.
    #[must_use]
    pub fn find_type(&self, def: TypeDef) -> Option<TypeId> {
        self.types
            .iter()
            .position(|candidate| *candidate == def)
            .and_then(|index| TypeId::try_from(index).ok())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvm_spec::{Limits, TYPE_I64};

    #[test]
    fn modulo_de_teste_passa_pela_checagem_estrutural() {
        let module = testing::soma();
        assert!(structure::check(&module, &Limits::default()).is_ok());
        assert_eq!(module.function_by_name("soma").map(|f| f.id), Some(1));
        assert_eq!(module.function(1).map(|f| f.ret), Some(TYPE_I64));
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
