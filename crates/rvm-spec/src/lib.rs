//! `rvm-spec` — núcleo semântico da RSUVM (EXP-0).
//!
//! Este crate é independente de linguagem hospedeira (ADR-0001) e não depende de
//! nenhum outro crate do workspace. Ele define:
//!
//! - os tipos do subset como [`TypeDef`] e os IDs canônicos ([`TypeId`],
//!   [`TYPE_UNIT`], [`TYPE_BOOL`], [`TYPE_I64`], [`TYPE_STR`], [`TYPE_F64`]);
//! - as operações aritméticas verificadas ([`BinOp`]) e as comparações ([`CmpOp`]);
//! - a tabela estável de códigos/categorias de erro ([`ErrorCode`]);
//! - o tipo de diagnóstico [`RvmError`] com localização;
//! - os limites estruturais [`Limits`] usados antes de alocar.
//!
//! A semântica de overflow é fixa e não depende do modo debug/release do Rust:
//! toda operação aritmética usa `i64::checked_*` e devolve [`None`] quando o
//! resultado sai de `[-2^63, 2^63-1]`; quem chama transforma `None` no trap
//! `integer_overflow`.

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

use core::fmt;

/// Versão experimental da representação intermediária implementada por este workspace.
///
/// Módulos com outra versão são rejeitados com [`ErrorCode::UnsupportedVersion`].
pub const IR_VERSION: u16 = 1;

/// Identificador denso de uma struct declarada no módulo (ADR-0029).
pub type StructId = u32;

/// Identificador denso de um tipo na tabela do módulo (ADR-0034).
pub type TypeId = u32;

/// Tipo estrutural da tabela do módulo (ADR-0034).
///
/// Tipos são recursivos **por id**, o que permite `option<option<i64>>`,
/// `array<option<i64>>`, `option<array<i64>>` e `result<Struct, Erro>` sem
/// `Box<Type>` e mantendo `TypeId: Copy`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TypeDef {
    /// Tipo unitário: ausência de valor útil.
    Unit,
    /// Booleano.
    Bool,
    /// Inteiro assinado de 64 bits.
    I64,
    /// Texto UTF-8 imutável.
    Str,
    /// Ponto flutuante IEEE-754 de 64 bits.
    F64,
    /// Array imutável de elementos do tipo referenciado.
    Array(TypeId),
    /// Struct declarada no módulo.
    Struct(StructId),
    /// Valor opcional de payload referenciado.
    Optional(TypeId),
    /// Resultado com payloads `ok`/`err` referenciados.
    Result(TypeId, TypeId),
}

impl TypeDef {
    /// Nome estável da forma do tipo.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            TypeDef::Unit => "unit",
            TypeDef::Bool => "bool",
            TypeDef::I64 => "i64",
            TypeDef::Str => "string",
            TypeDef::F64 => "f64",
            TypeDef::Array(_) => "array",
            TypeDef::Struct(_) => "struct",
            TypeDef::Optional(_) => "option",
            TypeDef::Result(..) => "result",
        }
    }

    /// `true` para tipos escalares (sem referências).
    #[must_use]
    pub const fn is_scalar(self) -> bool {
        matches!(
            self,
            TypeDef::Unit | TypeDef::Bool | TypeDef::I64 | TypeDef::Str | TypeDef::F64
        )
    }
}

impl fmt::Display for TypeDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Tipos embutidos canônicos: todo módulo começa a tabela com estes cinco.
pub const TYPE_UNIT: TypeId = 0;
/// `bool` embutido.
pub const TYPE_BOOL: TypeId = 1;
/// `i64` embutido.
pub const TYPE_I64: TypeId = 2;
/// `string` embutido.
pub const TYPE_STR: TypeId = 3;
/// `f64` embutido.
pub const TYPE_F64: TypeId = 4;

/// Quantidade de tipos embutidos (`types[0..BUILTIN_TYPE_COUNT]`).
pub const BUILTIN_TYPE_COUNT: usize = 5;

/// Tabela canônica dos cinco tipos embutidos.
#[must_use]
pub fn builtin_types() -> Vec<TypeDef> {
    vec![
        TypeDef::Unit,
        TypeDef::Bool,
        TypeDef::I64,
        TypeDef::Str,
        TypeDef::F64,
    ]
}

/// Efeito de host exigível por uma chamada de host (ADR-0036).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Effect {
    /// Leitura de arquivo.
    FilesystemRead,
    /// Escrita de arquivo.
    FilesystemWrite,
    /// Rede.
    Network,
    /// Variáveis de ambiente.
    Environment,
    /// Relógio.
    Clock,
    /// Subprocesso ou sinal.
    Process,
}

impl Effect {
    /// Nome estável do efeito.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Effect::FilesystemRead => "filesystem_read",
            Effect::FilesystemWrite => "filesystem_write",
            Effect::Network => "network",
            Effect::Environment => "environment",
            Effect::Clock => "clock",
            Effect::Process => "process",
        }
    }

    /// Efeito pelo nome estável.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "filesystem_read" => Some(Effect::FilesystemRead),
            "filesystem_write" => Some(Effect::FilesystemWrite),
            "network" => Some(Effect::Network),
            "environment" => Some(Effect::Environment),
            "clock" => Some(Effect::Clock),
            "process" => Some(Effect::Process),
            _ => None,
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Teto absoluto de bytes que uma função de host pode devolver em uma leitura
/// (ADR-0038). Todo host da ABI deve respeitar este limite.
pub const MAX_HOST_READ_BYTES: u64 = 1024 * 1024;

/// Função de host embutida na ABI mínima do EXP-0 (ADR-0036).
///
/// A ABI é um conjunto **fechado** nesta rodada: qualquer extensão exige ADR e
/// revisão de modelo de ameaças.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BuiltinHost {
    /// `read_file(path: string, max_bytes: i64) -> result string string`, com
    /// efeito [`Effect::FilesystemRead`] e escopo = `path`.
    ReadFile,
}

impl BuiltinHost {
    /// Nome estável da função.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            BuiltinHost::ReadFile => "read_file",
        }
    }

    /// Efeito exigido pela função.
    #[must_use]
    pub const fn effect(self) -> Effect {
        match self {
            BuiltinHost::ReadFile => Effect::FilesystemRead,
        }
    }

    /// Código estável no bytecode.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            BuiltinHost::ReadFile => 0,
        }
    }

    /// Função a partir do código do bytecode.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(BuiltinHost::ReadFile),
            _ => None,
        }
    }

    /// Índice do argumento que define o escopo da capability, se houver.
    #[must_use]
    pub const fn scope_arg(self) -> Option<usize> {
        match self {
            BuiltinHost::ReadFile => Some(0),
        }
    }

    /// Teto de bytes que o host pode devolver para esta função (ADR-0038).
    #[must_use]
    pub const fn max_effect_bytes(self) -> u64 {
        match self {
            BuiltinHost::ReadFile => MAX_HOST_READ_BYTES,
        }
    }
}

/// Operação aritmética binária verificada sobre `i64`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    /// Soma com verificação de overflow (`add_i64_checked`).
    AddI64Checked,
    /// Subtração com verificação de overflow (`sub_i64_checked`).
    SubI64Checked,
    /// Multiplicação com verificação de overflow (`mul_i64_checked`).
    MulI64Checked,
    /// Divisão inteira com verificação (`div_i64_checked`). Divisão por zero e
    /// `i64::MIN / -1` devolvem [`None`] (ADR-0022).
    DivI64Checked,
    /// Resto inteiro com verificação (`rem_i64_checked`). Resto por zero e
    /// `i64::MIN % -1` devolvem [`None`] (ADR-0022).
    RemI64Checked,
    /// Deslocamento à esquerda com verificação (`shl_i64_checked`). Quantidade
    /// fora de `0..=63` devolve [`None`] (ADR-0023).
    ShlI64Checked,
    /// Deslocamento aritmético à direita com verificação (`shr_i64_checked`).
    /// Quantidade fora de `0..=63` devolve [`None`] (ADR-0023).
    ShrI64Checked,
    /// E bit a bit (`and_i64`), sem trap (ADR-0023).
    AndI64,
    /// Ou bit a bit (`or_i64`), sem trap (ADR-0023).
    OrI64,
    /// Ou exclusivo bit a bit (`xor_i64`), sem trap (ADR-0023).
    XorI64,
}

impl BinOp {
    /// Nome estável da operação no formato textual.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            BinOp::AddI64Checked => "add_i64_checked",
            BinOp::SubI64Checked => "sub_i64_checked",
            BinOp::MulI64Checked => "mul_i64_checked",
            BinOp::DivI64Checked => "div_i64_checked",
            BinOp::RemI64Checked => "rem_i64_checked",
            BinOp::ShlI64Checked => "shl_i64_checked",
            BinOp::ShrI64Checked => "shr_i64_checked",
            BinOp::AndI64 => "and_i64",
            BinOp::OrI64 => "or_i64",
            BinOp::XorI64 => "xor_i64",
        }
    }

    /// Converte o nome estável da operação em [`BinOp`].
    ///
    /// Devolve [`None`] para operações fora do subset; quem chama deve
    /// transformar isso em [`ErrorCode::UnknownOpcode`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "add_i64_checked" => Some(BinOp::AddI64Checked),
            "sub_i64_checked" => Some(BinOp::SubI64Checked),
            "mul_i64_checked" => Some(BinOp::MulI64Checked),
            "div_i64_checked" => Some(BinOp::DivI64Checked),
            "rem_i64_checked" => Some(BinOp::RemI64Checked),
            "shl_i64_checked" => Some(BinOp::ShlI64Checked),
            "shr_i64_checked" => Some(BinOp::ShrI64Checked),
            "and_i64" => Some(BinOp::AndI64),
            "or_i64" => Some(BinOp::OrI64),
            "xor_i64" => Some(BinOp::XorI64),
            _ => None,
        }
    }

    /// Tipo exigido de cada operando.
    #[must_use]
    pub const fn operand_type(self) -> TypeId {
        TYPE_I64
    }

    /// Tipo do valor definido pela operação.
    #[must_use]
    pub const fn result_type(self) -> TypeId {
        TYPE_I64
    }

    /// Aplica a operação com overflow verificado.
    ///
    /// Devolve [`None`] quando o resultado matemático não cabe em `i64`
    /// (inclusive `i64::MIN * -1` e `i64::MIN - 1`). Para divisão e resto,
    /// devolve [`None`] também em divisão/resto por zero e em `i64::MIN / -1`
    /// (ADR-0022). O contrato não depende de `cfg!(debug_assertions)`.
    #[must_use]
    pub fn eval_i64(self, lhs: i64, rhs: i64) -> Option<i64> {
        match self {
            BinOp::AddI64Checked => lhs.checked_add(rhs),
            BinOp::SubI64Checked => lhs.checked_sub(rhs),
            BinOp::MulI64Checked => lhs.checked_mul(rhs),
            BinOp::DivI64Checked => lhs.checked_div(rhs),
            BinOp::RemI64Checked => lhs.checked_rem(rhs),
            BinOp::ShlI64Checked => u32::try_from(rhs)
                .ok()
                .and_then(|amount| lhs.checked_shl(amount)),
            BinOp::ShrI64Checked => u32::try_from(rhs)
                .ok()
                .and_then(|amount| lhs.checked_shr(amount)),
            BinOp::AndI64 => Some(lhs & rhs),
            BinOp::OrI64 => Some(lhs | rhs),
            BinOp::XorI64 => Some(lhs ^ rhs),
        }
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Comparação sobre `i64` (assinada) que produz `bool`.
///
/// Igualdade (`eq_i64`, `ne_i64`) e ordem (`lt_i64`, `le_i64`, `gt_i64`,
/// `ge_i64`) usam a ordem assinada padrão de `i64`. Comparações de `bool` e
/// `unit` não existem no EXP-0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CmpOp {
    /// Igualdade (`eq_i64`).
    Eq,
    /// Desigualdade (`ne_i64`).
    Ne,
    /// Menor que (`lt_i64`).
    Lt,
    /// Menor ou igual (`le_i64`).
    Le,
    /// Maior que (`gt_i64`).
    Gt,
    /// Maior ou igual (`ge_i64`).
    Ge,
}

impl CmpOp {
    /// Nome estável da comparação no formato textual.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            CmpOp::Eq => "eq_i64",
            CmpOp::Ne => "ne_i64",
            CmpOp::Lt => "lt_i64",
            CmpOp::Le => "le_i64",
            CmpOp::Gt => "gt_i64",
            CmpOp::Ge => "ge_i64",
        }
    }

    /// Converte o nome estável da comparação em [`CmpOp`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "eq_i64" => Some(CmpOp::Eq),
            "ne_i64" => Some(CmpOp::Ne),
            "lt_i64" => Some(CmpOp::Lt),
            "le_i64" => Some(CmpOp::Le),
            "gt_i64" => Some(CmpOp::Gt),
            "ge_i64" => Some(CmpOp::Ge),
            _ => None,
        }
    }

    /// Tipo exigido de cada operando.
    #[must_use]
    pub const fn operand_type(self) -> TypeId {
        TYPE_I64
    }

    /// Tipo do valor definido pela comparação.
    #[must_use]
    pub const fn result_type(self) -> TypeId {
        TYPE_BOOL
    }

    /// Aplica a comparação assinada.
    #[must_use]
    pub fn eval_i64(self, lhs: i64, rhs: i64) -> bool {
        match self {
            CmpOp::Eq => lhs == rhs,
            CmpOp::Ne => lhs != rhs,
            CmpOp::Lt => lhs < rhs,
            CmpOp::Le => lhs <= rhs,
            CmpOp::Gt => lhs > rhs,
            CmpOp::Ge => lhs >= rhs,
        }
    }
}

impl fmt::Display for CmpOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Operação sobre `f64` (ADR-0026): aritmética e comparações IEEE-754.
///
/// Não há trap: divisão por zero produz `inf`/`NaN`, e comparações com `NaN`
/// seguem a IEEE-754 (todas falsas, exceto `!=`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatOp {
    /// Soma (`add_f64`).
    Add,
    /// Subtração (`sub_f64`).
    Sub,
    /// Multiplicação (`mul_f64`).
    Mul,
    /// Divisão (`div_f64`).
    Div,
    /// Igualdade (`eq_f64`).
    Eq,
    /// Desigualdade (`ne_f64`).
    Ne,
    /// Menor que (`lt_f64`).
    Lt,
    /// Menor ou igual (`le_f64`).
    Le,
    /// Maior que (`gt_f64`).
    Gt,
    /// Maior ou igual (`ge_f64`).
    Ge,
}

impl FloatOp {
    /// Nome estável da operação no formato textual.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            FloatOp::Add => "add_f64",
            FloatOp::Sub => "sub_f64",
            FloatOp::Mul => "mul_f64",
            FloatOp::Div => "div_f64",
            FloatOp::Eq => "eq_f64",
            FloatOp::Ne => "ne_f64",
            FloatOp::Lt => "lt_f64",
            FloatOp::Le => "le_f64",
            FloatOp::Gt => "gt_f64",
            FloatOp::Ge => "ge_f64",
        }
    }

    /// Converte o nome estável em [`FloatOp`]; fora da tabela é `unknown_opcode`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "add_f64" => Some(FloatOp::Add),
            "sub_f64" => Some(FloatOp::Sub),
            "mul_f64" => Some(FloatOp::Mul),
            "div_f64" => Some(FloatOp::Div),
            "eq_f64" => Some(FloatOp::Eq),
            "ne_f64" => Some(FloatOp::Ne),
            "lt_f64" => Some(FloatOp::Lt),
            "le_f64" => Some(FloatOp::Le),
            "gt_f64" => Some(FloatOp::Gt),
            "ge_f64" => Some(FloatOp::Ge),
            _ => None,
        }
    }

    /// Tipo do resultado: `f64` para aritmética, `bool` para comparação.
    #[must_use]
    pub const fn result_type(self) -> TypeId {
        match self {
            FloatOp::Add | FloatOp::Sub | FloatOp::Mul | FloatOp::Div => TYPE_F64,
            FloatOp::Eq | FloatOp::Ne | FloatOp::Lt | FloatOp::Le | FloatOp::Gt | FloatOp::Ge => {
                TYPE_BOOL
            }
        }
    }

    /// Aplica a operação aritmética (IEEE-754). Para comparação devolve `NaN`.
    #[must_use]
    pub fn eval_f64(self, lhs: f64, rhs: f64) -> f64 {
        match self {
            FloatOp::Add => lhs + rhs,
            FloatOp::Sub => lhs - rhs,
            FloatOp::Mul => lhs * rhs,
            FloatOp::Div => lhs / rhs,
            _ => f64::NAN,
        }
    }

    /// Aplica a comparação (IEEE-754). Para aritmética devolve `false`.
    #[must_use]
    pub fn eval_cmp(self, lhs: f64, rhs: f64) -> bool {
        match self {
            FloatOp::Eq => lhs == rhs,
            FloatOp::Ne => lhs != rhs,
            FloatOp::Lt => lhs < rhs,
            FloatOp::Le => lhs <= rhs,
            FloatOp::Gt => lhs > rhs,
            FloatOp::Ge => lhs >= rhs,
            _ => false,
        }
    }
}

impl fmt::Display for FloatOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Categoria de diagnóstico, conforme TechSpec §7.
///
/// As categorias distinguem fronteiras do pipeline: [`Decode`](Self::Decode)
/// (texto/bytes malformados), [`Verify`](Self::Verify) (módulo estrutural ou
/// tipadamente inválido), [`Trap`](Self::Trap) (comportamento semântico
/// definido que impede resultado), [`Resource`](Self::Resource) (quota
/// excedida) e [`Capability`](Self::Capability) (efeito de host negado).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Entrada textual/binária malformada, versão desconhecida ou tamanho excedido.
    Decode,
    /// Módulo estrutural ou tipadamente inválido; nunca chega ao executor.
    Verify,
    /// Trap semântico definido (por exemplo `integer_overflow`).
    Trap,
    /// Limite de recurso (fuel ou profundidade) excedido.
    Resource,
    /// Capacidade do host ausente ou negada.
    Capability,
}

impl ErrorCategory {
    /// Nome estável da categoria.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            ErrorCategory::Decode => "decode",
            ErrorCategory::Verify => "verify",
            ErrorCategory::Trap => "trap",
            ErrorCategory::Resource => "resource",
            ErrorCategory::Capability => "capability",
        }
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Classe de falha observável pelo host (RF-05).
///
/// Distingue as três respostas exigidas pelo PRD: módulo inválido, trap
/// semântico e limite de recurso, além da negação de capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// Falha de decode ou verificação: o módulo não é executável.
    Module,
    /// Trap semântico durante execução.
    Trap,
    /// Quota de execução excedida.
    ResourceLimit,
    /// Efeito de host negado.
    Capability,
}

impl ErrorClass {
    /// Nome estável da classe.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            ErrorClass::Module => "module",
            ErrorClass::Trap => "trap",
            ErrorClass::ResourceLimit => "resource_limit",
            ErrorClass::Capability => "capability",
        }
    }
}

impl fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Código de erro estável do protótipo.
///
/// Mensagens humanas podem mudar; estes nomes são o contrato verificável pelo
/// corpus de conformidade (`docs/CONTRATOS/ERROS-EXP-0.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Texto ou bytes fora da gramática/formato experimental.
    SyntaxError,
    /// Imediato numérico ou tamanho fora de faixa representável.
    InvalidNumber,
    /// Nome de instrução ou terminador desconhecido.
    UnknownOpcode,
    /// Nome de tipo desconhecido.
    UnknownType,
    /// Versão de módulo ou de codificação não suportada.
    UnsupportedVersion,
    /// Entrada truncada: faltam bytes para o que foi declarado.
    TruncatedInput,
    /// Entrada maior que [`Limits::max_input_bytes`]; rejeitada antes de ler o corpo.
    InputTooLarge,
    /// Módulo estruturalmente inválido (IDs, referências, limites, nomes).
    InvalidModule,
    /// Tipo ou aridade incompatível.
    InvalidType,
    /// Fluxo de controle inválido (terminador, alvo, dominância, retorno).
    InvalidControlFlow,
    /// Recurso do formato/host fora do suportado pelo EXP-0.
    UnsupportedFeature,
    /// Trap aritmético: resultado fora de `[-2^63, 2^63-1]`.
    IntegerOverflow,
    /// Trap aritmético: divisão ou resto por zero (`division_by_zero`, ADR-0022).
    DivisionByZero,
    /// Trap aritmético: deslocamento com quantidade fora de `0..=63`
    /// (`invalid_shift`, ADR-0023).
    InvalidShift,
    /// Concatenação de `string` excede `Limits::max_string_bytes`
    /// (`string_too_long`, ADR-0025).
    StringTooLong,
    /// Índice de array fora de `0..len` (`index_out_of_bounds`, ADR-0027).
    IndexOutOfBounds,
    /// `option_unwrap`/`option.unwrap` sobre `none` (`unwrap_none`, ADR-0030).
    UnwrapNone,
    /// `result_unwrap`/`result_unwrap_err` sobre a variante errada
    /// (`unwrap_error`, ADR-0033).
    UnwrapError,
    /// Fuel esgotado antes do despacho.
    OutOfFuel,
    /// Profundidade máxima de chamadas excedida.
    CallDepthExceeded,
    /// Quota de memória de frames excedida.
    OutOfMemory,
    /// Capability do host negada (deny-by-default).
    CapabilityDenied,
}

impl ErrorCode {
    /// Nome estável do código, usado no corpus e nas mensagens.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorCode::SyntaxError => "syntax_error",
            ErrorCode::InvalidNumber => "invalid_number",
            ErrorCode::UnknownOpcode => "unknown_opcode",
            ErrorCode::UnknownType => "unknown_type",
            ErrorCode::UnsupportedVersion => "unsupported_version",
            ErrorCode::TruncatedInput => "truncated_input",
            ErrorCode::InputTooLarge => "input_too_large",
            ErrorCode::InvalidModule => "invalid_module",
            ErrorCode::InvalidType => "invalid_type",
            ErrorCode::InvalidControlFlow => "invalid_control_flow",
            ErrorCode::UnsupportedFeature => "unsupported_feature",
            ErrorCode::IntegerOverflow => "integer_overflow",
            ErrorCode::DivisionByZero => "division_by_zero",
            ErrorCode::InvalidShift => "invalid_shift",
            ErrorCode::StringTooLong => "string_too_long",
            ErrorCode::IndexOutOfBounds => "index_out_of_bounds",
            ErrorCode::UnwrapNone => "unwrap_none",
            ErrorCode::UnwrapError => "unwrap_error",
            ErrorCode::OutOfFuel => "out_of_fuel",
            ErrorCode::CallDepthExceeded => "call_depth_exceeded",
            ErrorCode::OutOfMemory => "out_of_memory",
            ErrorCode::CapabilityDenied => "capability_denied",
        }
    }

    /// Converte o nome estável de volta ao código.
    ///
    /// Usado pelo corpus para declarar a expectativa de erro por nome.
    #[must_use]
    pub fn from_stable_name(name: &str) -> Option<Self> {
        match name {
            "syntax_error" => Some(ErrorCode::SyntaxError),
            "invalid_number" => Some(ErrorCode::InvalidNumber),
            "unknown_opcode" => Some(ErrorCode::UnknownOpcode),
            "unknown_type" => Some(ErrorCode::UnknownType),
            "unsupported_version" => Some(ErrorCode::UnsupportedVersion),
            "truncated_input" => Some(ErrorCode::TruncatedInput),
            "input_too_large" => Some(ErrorCode::InputTooLarge),
            "invalid_module" => Some(ErrorCode::InvalidModule),
            "invalid_type" => Some(ErrorCode::InvalidType),
            "invalid_control_flow" => Some(ErrorCode::InvalidControlFlow),
            "unsupported_feature" => Some(ErrorCode::UnsupportedFeature),
            "integer_overflow" => Some(ErrorCode::IntegerOverflow),
            "division_by_zero" => Some(ErrorCode::DivisionByZero),
            "invalid_shift" => Some(ErrorCode::InvalidShift),
            "string_too_long" => Some(ErrorCode::StringTooLong),
            "index_out_of_bounds" => Some(ErrorCode::IndexOutOfBounds),
            "unwrap_none" => Some(ErrorCode::UnwrapNone),
            "unwrap_error" => Some(ErrorCode::UnwrapError),
            "out_of_fuel" => Some(ErrorCode::OutOfFuel),
            "call_depth_exceeded" => Some(ErrorCode::CallDepthExceeded),
            "out_of_memory" => Some(ErrorCode::OutOfMemory),
            "capability_denied" => Some(ErrorCode::CapabilityDenied),
            _ => None,
        }
    }

    /// Categoria do pipeline à qual o código pertence.
    #[must_use]
    pub const fn category(self) -> ErrorCategory {
        match self {
            ErrorCode::SyntaxError
            | ErrorCode::InvalidNumber
            | ErrorCode::UnknownOpcode
            | ErrorCode::UnknownType
            | ErrorCode::UnsupportedVersion
            | ErrorCode::TruncatedInput
            | ErrorCode::InputTooLarge => ErrorCategory::Decode,
            ErrorCode::InvalidModule
            | ErrorCode::InvalidType
            | ErrorCode::InvalidControlFlow
            | ErrorCode::UnsupportedFeature => ErrorCategory::Verify,
            ErrorCode::IntegerOverflow
            | ErrorCode::DivisionByZero
            | ErrorCode::InvalidShift
            | ErrorCode::StringTooLong
            | ErrorCode::IndexOutOfBounds
            | ErrorCode::UnwrapNone
            | ErrorCode::UnwrapError => ErrorCategory::Trap,
            ErrorCode::OutOfFuel | ErrorCode::CallDepthExceeded | ErrorCode::OutOfMemory => {
                ErrorCategory::Resource
            }
            ErrorCode::CapabilityDenied => ErrorCategory::Capability,
        }
    }

    /// Classe de falha observável (RF-05).
    #[must_use]
    pub const fn class(self) -> ErrorClass {
        match self.category() {
            ErrorCategory::Decode | ErrorCategory::Verify => ErrorClass::Module,
            ErrorCategory::Trap => ErrorClass::Trap,
            ErrorCategory::Resource => ErrorClass::ResourceLimit,
            ErrorCategory::Capability => ErrorClass::Capability,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Localização de um diagnóstico dentro de um módulo.
///
/// Os campos são `String`/`u32` de propriedade própria: diagnósticos são frios e
/// não devem carregar referências para dentro de módulos não confiáveis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    /// O módulo inteiro (versão, nome, contagem de funções, entry point).
    Module,
    /// Uma função (nome, limites, assinatura).
    Function {
        /// Nome declarado da função.
        function: String,
    },
    /// Um bloco (parâmetros, terminador, limites).
    Block {
        /// Nome declarado da função.
        function: String,
        /// Rótulo do bloco.
        block: String,
    },
    /// Uma instrução específica, identificada por índice (base zero) no bloco.
    Instruction {
        /// Nome declarado da função.
        function: String,
        /// Rótulo do bloco.
        block: String,
        /// Índice da instrução dentro do bloco.
        index: u32,
    },
    /// Um valor SSA, identificado pelo seu `ValueId`.
    Value {
        /// Nome declarado da função.
        function: String,
        /// Identificador numérico do valor.
        value: u32,
    },
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::Module => f.write_str("módulo"),
            Location::Function { function } => write!(f, "função `{function}`"),
            Location::Block { function, block } => {
                write!(f, "função `{function}`, bloco `{block}`")
            }
            Location::Instruction {
                function,
                block,
                index,
            } => write!(f, "função `{function}`, bloco `{block}`, instrução {index}"),
            Location::Value { function, value } => {
                write!(f, "função `{function}`, valor v{value}")
            }
        }
    }
}

/// Diagnóstico estruturado do protótipo.
///
/// Carrega sempre um [`ErrorCode`] estável; o texto humano (`detail`) e a
/// [`Location`] ajudam a depuração, mas não são contrato de conformidade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RvmError {
    code: ErrorCode,
    detail: String,
    location: Option<Location>,
}

impl RvmError {
    /// Cria um diagnóstico com código estável e detalhe livre.
    pub fn new(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            location: None,
        }
    }

    /// Anexa (ou substitui) a localização do diagnóstico.
    #[must_use]
    pub fn at(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    /// Código estável do erro.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    /// Categoria do pipeline à qual o erro pertence.
    #[must_use]
    pub const fn category(&self) -> ErrorCategory {
        self.code.category()
    }

    /// Classe observável (RF-05).
    #[must_use]
    pub const fn class(&self) -> ErrorClass {
        self.code.class()
    }

    /// Texto humano, instável, para depuração.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Localização opcional dentro do módulo.
    #[must_use]
    pub const fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }
}

impl fmt::Display for RvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.detail)?;
        if let Some(location) = &self.location {
            write!(f, " [{location}]")?;
        }
        Ok(())
    }
}

impl std::error::Error for RvmError {}

/// Limites estruturais aplicados antes de alocar ou executar.
///
/// Todo limite é expresso em unidades do modelo (contagens, não bytes), exceto
/// [`Limits::max_input_bytes`], que limita a entrada textual/binária. O host
/// fornece os limites; o verificador e o decoder comparam **antes** de reservar
/// memória, e falham fechado quando o limite é excedido.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Máximo de funções por módulo.
    pub max_functions: u32,
    /// Máximo de blocos por função.
    pub max_blocks_per_function: u32,
    /// Máximo de parâmetros de função.
    pub max_params: u32,
    /// Máximo de parâmetros por bloco.
    pub max_block_params: u32,
    /// Máximo de instruções por bloco.
    pub max_instrs_per_block: u32,
    /// Máximo de valores SSA (parâmetros + definições + parâmetros de bloco) por função.
    pub max_values_per_function: u32,
    /// Máximo de argumentos em uma chamada ou desvio.
    pub max_call_args: u32,
    /// Tamanho máximo, em bytes, de nomes e rótulos.
    pub max_name_len: u32,
    /// Tamanho máximo, em bytes, de um literal `string` (ADR-0024).
    pub max_string_bytes: u32,
    /// Número máximo de elementos de um literal de array (ADR-0027).
    pub max_array_len: u32,
    /// Número máximo de structs declaradas no módulo (ADR-0029).
    pub max_structs: u32,
    /// Número máximo de campos de uma struct (ADR-0029).
    pub max_fields_per_struct: u32,
    /// Número máximo de tipos na tabela do módulo (ADR-0034).
    pub max_types: u32,
    /// Tamanho máximo, em bytes, da entrada textual ou binária de um módulo.
    pub max_input_bytes: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_functions: 64,
            max_blocks_per_function: 256,
            max_params: 16,
            max_block_params: 8,
            max_instrs_per_block: 1024,
            max_values_per_function: 4096,
            max_call_args: 16,
            max_name_len: 64,
            max_string_bytes: 4096,
            max_array_len: 4096,
            max_structs: 256,
            max_fields_per_struct: 256,
            max_types: 1024,
            max_input_bytes: 1_048_576,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aritmetica_verificada_nao_transborda_em_silencio() {
        assert_eq!(BinOp::AddI64Checked.eval_i64(10, 20), Some(30));
        assert_eq!(BinOp::AddI64Checked.eval_i64(i64::MAX, 1), None);
        assert_eq!(BinOp::SubI64Checked.eval_i64(i64::MIN, 1), None);
        assert_eq!(BinOp::SubI64Checked.eval_i64(-1, -1), Some(0));
        assert_eq!(BinOp::MulI64Checked.eval_i64(2, 3), Some(6));
        assert_eq!(BinOp::MulI64Checked.eval_i64(i64::MAX, 2), None);
        assert_eq!(BinOp::MulI64Checked.eval_i64(i64::MIN, -1), None);
        assert_eq!(BinOp::AddI64Checked.eval_i64(i64::MIN, i64::MAX), Some(-1));
        // Divisão e resto (ADR-0022): truncam para zero; zero e `MIN / -1` falham.
        assert_eq!(BinOp::DivI64Checked.eval_i64(20, 6), Some(3));
        assert_eq!(BinOp::DivI64Checked.eval_i64(-20, 6), Some(-3));
        assert_eq!(BinOp::DivI64Checked.eval_i64(1, 0), None);
        assert_eq!(BinOp::DivI64Checked.eval_i64(i64::MIN, -1), None);
        assert_eq!(BinOp::RemI64Checked.eval_i64(20, 6), Some(2));
        assert_eq!(BinOp::RemI64Checked.eval_i64(-20, 6), Some(-2));
        assert_eq!(BinOp::RemI64Checked.eval_i64(1, 0), None);
        assert_eq!(BinOp::RemI64Checked.eval_i64(i64::MIN, -1), None);
        // Deslocamentos e bit a bit (ADR-0023).
        assert_eq!(BinOp::ShlI64Checked.eval_i64(1, 4), Some(16));
        assert_eq!(BinOp::ShlI64Checked.eval_i64(1, 64), None);
        assert_eq!(BinOp::ShlI64Checked.eval_i64(1, -1), None);
        assert_eq!(BinOp::ShrI64Checked.eval_i64(-16, 2), Some(-4));
        assert_eq!(BinOp::ShrI64Checked.eval_i64(1, 64), None);
        assert_eq!(BinOp::AndI64.eval_i64(0b1100, 0b1010), Some(0b1000));
        assert_eq!(BinOp::OrI64.eval_i64(0b1100, 0b1010), Some(0b1110));
        assert_eq!(BinOp::XorI64.eval_i64(0b1100, 0b1010), Some(0b0110));
    }

    #[test]
    fn comparacoes_sao_assinadas() {
        assert!(CmpOp::Lt.eval_i64(-2, 1));
        assert!(!CmpOp::Lt.eval_i64(1, -2));
        assert!(CmpOp::Ge.eval_i64(i64::MAX, i64::MAX));
        assert!(CmpOp::Ne.eval_i64(i64::MIN, i64::MAX));
        assert!(CmpOp::Eq.eval_i64(0, 0));
    }

    #[test]
    fn nomes_de_tipo_e_operacao_sao_bidirecionais() {
        for ty in [
            TypeDef::Unit,
            TypeDef::Bool,
            TypeDef::I64,
            TypeDef::Str,
            TypeDef::F64,
        ] {
            assert_eq!(ty.name(), format!("{ty}"));
        }
        for op in [
            BinOp::AddI64Checked,
            BinOp::SubI64Checked,
            BinOp::MulI64Checked,
            BinOp::DivI64Checked,
            BinOp::RemI64Checked,
            BinOp::ShlI64Checked,
            BinOp::ShrI64Checked,
            BinOp::AndI64,
            BinOp::OrI64,
            BinOp::XorI64,
        ] {
            assert_eq!(BinOp::from_name(op.name()), Some(op));
        }
        for op in [
            CmpOp::Eq,
            CmpOp::Ne,
            CmpOp::Lt,
            CmpOp::Le,
            CmpOp::Gt,
            CmpOp::Ge,
        ] {
            assert_eq!(CmpOp::from_name(op.name()), Some(op));
            assert_eq!(op.result_type(), TYPE_BOOL);
            assert_eq!(op.operand_type(), TYPE_I64);
        }
        assert_eq!(BinOp::from_name("div_i64"), None);
    }

    #[test]
    fn tabela_de_erros_e_estavel() {
        // Nomes estáveis são contrato com o corpus; reordenar ou renomear exige ADR.
        assert_eq!(ErrorCode::IntegerOverflow.as_str(), "integer_overflow");
        assert_eq!(ErrorCode::OutOfFuel.as_str(), "out_of_fuel");
        assert_eq!(ErrorCode::CallDepthExceeded.as_str(), "call_depth_exceeded");
        assert_eq!(ErrorCode::InvalidType.as_str(), "invalid_type");
        for code in [
            ErrorCode::SyntaxError,
            ErrorCode::TruncatedInput,
            ErrorCode::InvalidModule,
            ErrorCode::InvalidType,
            ErrorCode::InvalidControlFlow,
            ErrorCode::UnsupportedFeature,
            ErrorCode::IntegerOverflow,
            ErrorCode::DivisionByZero,
            ErrorCode::InvalidShift,
            ErrorCode::StringTooLong,
            ErrorCode::IndexOutOfBounds,
            ErrorCode::OutOfFuel,
            ErrorCode::CallDepthExceeded,
            ErrorCode::CapabilityDenied,
        ] {
            assert_eq!(ErrorCode::from_stable_name(code.as_str()), Some(code));
        }
        assert_eq!(
            ErrorCode::DivisionByZero.class(),
            ErrorClass::Trap,
            "divisão por zero é trap semântico"
        );
        assert_eq!(
            ErrorCode::InvalidType.class(),
            ErrorClass::Module,
            "tipo inválido é falha de módulo"
        );
        assert_eq!(
            ErrorCode::IntegerOverflow.class(),
            ErrorClass::Trap,
            "overflow é trap semântico, não erro de verificador"
        );
        assert_eq!(
            ErrorCode::OutOfMemory.class(),
            ErrorClass::ResourceLimit,
            "memória é limite de recurso"
        );
        assert_eq!(
            ErrorCode::OutOfFuel.class(),
            ErrorClass::ResourceLimit,
            "fuel é limite de recurso"
        );
    }

    #[test]
    fn diagnostico_exibe_codigo_e_local() {
        let error =
            RvmError::new(ErrorCode::InvalidType, "condição não é bool").at(Location::Block {
                function: "f".to_string(),
                block: "entry".to_string(),
            });
        assert_eq!(error.code(), ErrorCode::InvalidType);
        assert_eq!(error.category(), ErrorCategory::Verify);
        assert!(error.to_string().contains("invalid_type"));
        assert!(error.to_string().contains("bloco `entry`"));
    }

    #[test]
    fn limites_padrao_sao_ordens_de_grandeza_pequenas() {
        let limits = Limits::default();
        assert!(limits.max_functions > 0);
        assert!(limits.max_blocks_per_function > 0);
        assert!(limits.max_instrs_per_block > 0);
        assert!(limits.max_values_per_function > 0);
        assert!(limits.max_input_bytes > 0);
    }

    #[test]
    fn operacoes_f64_sao_bidirecionais_e_ieee() {
        for op in [
            FloatOp::Add,
            FloatOp::Sub,
            FloatOp::Mul,
            FloatOp::Div,
            FloatOp::Eq,
            FloatOp::Ne,
            FloatOp::Lt,
            FloatOp::Le,
            FloatOp::Gt,
            FloatOp::Ge,
        ] {
            assert_eq!(FloatOp::from_name(op.name()), Some(op));
        }
        assert_eq!(FloatOp::Add.eval_f64(1.5, 2.25), 3.75);
        assert_eq!(FloatOp::Div.eval_f64(1.0, 0.0), f64::INFINITY);
        assert!(FloatOp::Div.eval_f64(0.0, 0.0).is_nan());
        assert!(FloatOp::Lt.eval_cmp(1.0, 2.0));
        assert!(!FloatOp::Lt.eval_cmp(f64::NAN, 2.0));
        assert!(FloatOp::Ne.eval_cmp(f64::NAN, f64::NAN));
        assert_eq!(FloatOp::Add.result_type(), TYPE_F64);
        assert_eq!(FloatOp::Lt.result_type(), TYPE_BOOL);
        assert_eq!(FloatOp::from_name("add_i64_checked"), None);
    }
}
