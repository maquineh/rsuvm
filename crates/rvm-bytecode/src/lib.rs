//! `rvm-bytecode` — codificação binária experimental (EXP-0).
//!
//! **Não normativo.** O ADR-0004 continua aberto: nenhum formato wire é estável e
//! este crate existe para medir o custo e o risco de decodificar entrada hostil.
//! A codificação é um espelho linear da RIR; o decoder valida limites e contagens
//! **antes** de alocar, e nunca devolve [`rvm_ir::Module`] sem passar por
//! [`rvm_ir::structure::check`].
//!
//! Layout documentado em `docs/CONTRATOS/BYTECODE-EXP-0-IMPLEMENTACAO.md`.
//!
//! # Ordem de validação do decoder
//!
//! 1. [`Limits::max_input_bytes`] antes de qualquer leitura de corpo;
//! 2. magic, versão de codificação e flags;
//! 3. cada leitura com checagem de bytes restantes (`truncated_input`);
//! 4. cada contagem contra os bytes restantes e contra o limite estrutural
//!    correspondente, antes de `Vec::with_capacity`;
//! 5. bytes excedentes (`invalid_module`) e [`rvm_ir::structure::check`].
//!
//! Nenhum caminho indexa diretamente a entrada, usa `unsafe` ou pode entrar em
//! pânico.

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

use rvm_ir::structure;
use rvm_ir::{
    Block, BlockParam, BranchTarget, Function, Instr, Module, Op, Param, Terminator, ValueId,
};
use rvm_spec::{BinOp, CmpOp, ErrorCode, Limits, RvmError, Type};

/// Magic bytes no início de todo arquivo codificado: `RSVMB1`.
pub const MAGIC: [u8; 6] = *b"RSVMB1";

/// Versão da codificação atual. Qualquer outro valor é rejeitado.
pub const FORMAT_VERSION: u16 = 1;

// Tags de tipo do layout. `Type` não é `#[repr(u8)]`; o mapeamento é explícito.
const TAG_UNIT: u8 = 0;
const TAG_BOOL: u8 = 1;
const TAG_I64: u8 = 2;

// Opcodes de instrução (tabela de `BYTECODE-EXP-0-IMPLEMENTACAO.md` §1).
const OP_CONST_I64: u8 = 0;
const OP_CONST_BOOL: u8 = 1;
const OP_ADD_I64_CHECKED: u8 = 2;
const OP_SUB_I64_CHECKED: u8 = 3;
const OP_MUL_I64_CHECKED: u8 = 4;
const OP_EQ_I64: u8 = 5;
const OP_NE_I64: u8 = 6;
const OP_LT_I64: u8 = 7;
const OP_LE_I64: u8 = 8;
const OP_GT_I64: u8 = 9;
const OP_GE_I64: u8 = 10;
const OP_CALL_STATIC: u8 = 11;

// Tipos de terminador (tabela de `BYTECODE-EXP-0-IMPLEMENTACAO.md` §1).
const TERM_RETURN: u8 = 0;
const TERM_RETURN_VALUE: u8 = 1;
const TERM_BR: u8 = 2;
const TERM_COND_BR: u8 = 3;

/// Codifica um módulo RIR na forma binária experimental.
///
/// A codificação é determinística: o mesmo módulo produz sempre os mesmos bytes,
/// na ordem do layout, sem padding, checksum ou campo derivado. Strings são
/// `u16` de comprimento + UTF-8 e todo inteiro é little-endian.
///
/// O resultado não é validado aqui; use [`decode`] para o caminho hostil.
/// Comprimentos de string e contagens acima da largura do campo (`u16`/`u32`) são
/// gravados com o campo saturando no máximo: módulos assim já violam os limites
/// do EXP-0 e o resultado não é garantidamente decodificável.
#[must_use]
pub fn encode(module: &Module) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    push_u16(&mut out, FORMAT_VERSION);
    push_u16(&mut out, 0);
    push_string(&mut out, &module.name);
    push_u16(&mut out, module.version);
    push_u32(&mut out, module.entry);
    push_u32(&mut out, len_u32(module.functions.len()));
    for function in &module.functions {
        encode_function(&mut out, function);
    }
    out
}

/// Decodifica bytes hostis em um módulo RIR estruturalmente válido.
///
/// # Errors
///
/// `input_too_large` quando a entrada excede [`Limits::max_input_bytes`]
/// (antes de ler o corpo), `truncated_input` para bytes faltando, `syntax_error`
/// para magic inválido, `unsupported_version` para versão de codificação ou de
/// módulo diferente de [`FORMAT_VERSION`], `unknown_opcode`/`unknown_type` para
/// tags desconhecidas, `invalid_number` para `const_bool` fora de `0|1` e
/// `invalid_module` para limites estruturais, UTF-8 inválido e bytes excedentes.
/// O primeiro erro encontrado vence, na ordem das invariantes do contrato.
pub fn decode(bytes: &[u8], limits: &Limits) -> Result<Module, RvmError> {
    // Invariante 1: tamanho antes de qualquer leitura de corpo.
    let max_input = to_usize(limits.max_input_bytes);
    if bytes.len() > max_input {
        return Err(decode_error(
            ErrorCode::InputTooLarge,
            format!(
                "entrada de {} bytes excede o limite de {max_input} bytes",
                bytes.len()
            ),
        ));
    }

    let mut reader = Reader::new(bytes);

    // Invariante 2: magic inválido é `syntax_error`.
    let magic = reader.take(MAGIC.len(), "magic")?;
    if magic != MAGIC.as_slice() {
        return Err(decode_error(
            ErrorCode::SyntaxError,
            "magic diferente de RSVMB1",
        ));
    }

    // Invariante 3: versão de codificação e flags reservadas.
    let format_version = reader.read_u16("format_version")?;
    if format_version != FORMAT_VERSION {
        return Err(decode_error(
            ErrorCode::UnsupportedVersion,
            format!("versão de codificação {format_version}; esperada {FORMAT_VERSION}"),
        ));
    }
    let flags = reader.read_u16("flags")?;
    if flags != 0 {
        return Err(decode_error(
            ErrorCode::UnsupportedVersion,
            format!("flags reservadas deveriam ser 0, encontradas {flags}"),
        ));
    }

    let name = reader.read_name(limits, "nome do módulo")?;
    let module_version = reader.read_u16("module_version")?;
    let entry = reader.read_u32("entry_fn")?;

    // Contagem antes da alocação (invariante 5): `read_count_u32` compara com os
    // bytes restantes e com `max_functions` antes do `with_capacity` abaixo.
    let function_count = reader.read_count_u32(limits.max_functions, "contagem de funções")?;
    let mut functions = Vec::with_capacity(function_count);
    for _ in 0..function_count {
        functions.push(read_function(&mut reader, limits)?);
    }

    // Invariante 7: bytes excedentes são rejeitados.
    let leftover = reader.remaining();
    if leftover != 0 {
        return Err(decode_error(
            ErrorCode::InvalidModule,
            format!("{leftover} bytes excedentes após o módulo"),
        ));
    }

    let module = Module {
        name,
        version: module_version,
        entry,
        functions,
    };
    // Invariante 8: nenhum `Module` é devolvido sem a checagem estrutural. Ela
    // também reconfere a versão do módulo (`unsupported_version` quando != 1),
    // IDs densos, nomes, alvos de desvio e limites.
    structure::check(&module, limits)?;
    Ok(module)
}

/// Escreve os campos de uma função na ordem do layout.
fn encode_function(out: &mut Vec<u8>, function: &Function) {
    push_u32(out, function.id);
    push_string(out, &function.name);
    push_u8(out, type_tag(function.ret));
    push_u32(out, function.entry);
    push_u16(out, len_u16(function.params.len()));
    for param in &function.params {
        push_string(out, &param.name);
        push_u8(out, type_tag(param.ty));
    }
    push_u32(out, len_u32(function.blocks.len()));
    for block in &function.blocks {
        encode_block(out, block);
    }
}

/// Escreve os campos de um bloco na ordem do layout.
fn encode_block(out: &mut Vec<u8>, block: &Block) {
    push_u32(out, block.id);
    push_string(out, &block.label);
    push_u16(out, len_u16(block.params.len()));
    for param in &block.params {
        push_u32(out, param.value);
        push_string(out, &param.name);
        push_u8(out, type_tag(param.ty));
    }
    push_u32(out, len_u32(block.instrs.len()));
    for instr in &block.instrs {
        encode_instr(out, instr);
    }
    encode_terminator(out, &block.term);
}

/// Escreve `def` e o payload do opcode da instrução.
fn encode_instr(out: &mut Vec<u8>, instr: &Instr) {
    push_u32(out, instr.def);
    match &instr.op {
        Op::ConstI64(value) => {
            push_u8(out, OP_CONST_I64);
            push_i64(out, *value);
        }
        Op::ConstBool(value) => {
            push_u8(out, OP_CONST_BOOL);
            push_u8(out, u8::from(*value));
        }
        Op::Bin { op, lhs, rhs } => {
            push_u8(out, bin_opcode(*op));
            push_u32(out, *lhs);
            push_u32(out, *rhs);
        }
        Op::Cmp { op, lhs, rhs } => {
            push_u8(out, cmp_opcode(*op));
            push_u32(out, *lhs);
            push_u32(out, *rhs);
        }
        Op::Call { callee, args } => {
            push_u8(out, OP_CALL_STATIC);
            push_u32(out, *callee);
            push_u16(out, len_u16(args.len()));
            for arg in args {
                push_u32(out, *arg);
            }
        }
    }
}

/// Escreve o terminador com o tipo e o payload correspondente.
fn encode_terminator(out: &mut Vec<u8>, term: &Terminator) {
    match term {
        Terminator::Return(None) => push_u8(out, TERM_RETURN),
        Terminator::Return(Some(value)) => {
            push_u8(out, TERM_RETURN_VALUE);
            push_u32(out, *value);
        }
        Terminator::Br(target) => {
            push_u8(out, TERM_BR);
            encode_target(out, target);
        }
        Terminator::CondBr {
            cond,
            then_target,
            else_target,
        } => {
            push_u8(out, TERM_COND_BR);
            push_u32(out, *cond);
            encode_target(out, then_target);
            encode_target(out, else_target);
        }
    }
}

/// Escreve alvo de desvio: `u32` de bloco, `u16` de contagem e os argumentos.
fn encode_target(out: &mut Vec<u8>, target: &BranchTarget) {
    push_u32(out, target.target);
    push_u16(out, len_u16(target.args.len()));
    for arg in &target.args {
        push_u32(out, *arg);
    }
}

/// Lê uma função inteira, já com contagens validadas antes de alocar.
fn read_function(reader: &mut Reader<'_>, limits: &Limits) -> Result<Function, RvmError> {
    let id = reader.read_u32("id de função")?;
    let name = reader.read_name(limits, "nome de função")?;
    let ret = read_type(reader, "tipo de retorno")?;
    let entry = reader.read_u32("bloco de entrada")?;

    // Contagem antes da alocação: bytes restantes e `max_params`.
    let param_count = reader.read_count_u16(limits.max_params, "contagem de parâmetros")?;
    let mut params = Vec::with_capacity(param_count);
    for _ in 0..param_count {
        let param_name = reader.read_name(limits, "nome de parâmetro")?;
        let ty = read_type(reader, "tipo de parâmetro")?;
        params.push(Param {
            name: param_name,
            ty,
        });
    }

    // Contagem antes da alocação: bytes restantes e `max_blocks_per_function`.
    let block_count =
        reader.read_count_u32(limits.max_blocks_per_function, "contagem de blocos")?;
    let mut blocks = Vec::with_capacity(block_count);
    for _ in 0..block_count {
        blocks.push(read_block(reader, limits)?);
    }

    Ok(Function {
        id,
        name,
        params,
        ret,
        entry,
        blocks,
    })
}

/// Lê um bloco inteiro, já com contagens validadas antes de alocar.
fn read_block(reader: &mut Reader<'_>, limits: &Limits) -> Result<Block, RvmError> {
    let id = reader.read_u32("id de bloco")?;
    let label = reader.read_name(limits, "rótulo de bloco")?;

    // Contagem antes da alocação: bytes restantes e `max_block_params`.
    let param_count =
        reader.read_count_u16(limits.max_block_params, "contagem de parâmetros de bloco")?;
    let mut params = Vec::with_capacity(param_count);
    for _ in 0..param_count {
        let value = reader.read_u32("valor do parâmetro de bloco")?;
        let name = reader.read_name(limits, "nome de parâmetro de bloco")?;
        let ty = read_type(reader, "tipo de parâmetro de bloco")?;
        params.push(BlockParam { value, name, ty });
    }

    // Contagem antes da alocação: bytes restantes e `max_instrs_per_block`.
    let instr_count =
        reader.read_count_u32(limits.max_instrs_per_block, "contagem de instruções")?;
    let mut instrs = Vec::with_capacity(instr_count);
    for _ in 0..instr_count {
        instrs.push(read_instr(reader, limits)?);
    }

    let term = read_terminator(reader, limits)?;
    Ok(Block {
        id,
        label,
        params,
        instrs,
        term,
    })
}

/// Lê `def`, o opcode e o payload; opcode fora da tabela é `unknown_opcode`.
fn read_instr(reader: &mut Reader<'_>, limits: &Limits) -> Result<Instr, RvmError> {
    let def = reader.read_u32("def da instrução")?;
    let opcode = reader.read_u8("opcode da instrução")?;
    let op = match opcode {
        OP_CONST_I64 => Op::ConstI64(reader.read_i64("imediato const_i64")?),
        OP_CONST_BOOL => {
            let value = reader.read_u8("imediato const_bool")?;
            match value {
                0 => Op::ConstBool(false),
                1 => Op::ConstBool(true),
                other => {
                    return Err(decode_error(
                        ErrorCode::InvalidNumber,
                        format!("const_bool {other} fora de 0|1"),
                    ));
                }
            }
        }
        OP_ADD_I64_CHECKED => read_bin_operands(reader, BinOp::AddI64Checked)?,
        OP_SUB_I64_CHECKED => read_bin_operands(reader, BinOp::SubI64Checked)?,
        OP_MUL_I64_CHECKED => read_bin_operands(reader, BinOp::MulI64Checked)?,
        OP_EQ_I64 => read_cmp_operands(reader, CmpOp::Eq)?,
        OP_NE_I64 => read_cmp_operands(reader, CmpOp::Ne)?,
        OP_LT_I64 => read_cmp_operands(reader, CmpOp::Lt)?,
        OP_LE_I64 => read_cmp_operands(reader, CmpOp::Le)?,
        OP_GT_I64 => read_cmp_operands(reader, CmpOp::Gt)?,
        OP_GE_I64 => read_cmp_operands(reader, CmpOp::Ge)?,
        OP_CALL_STATIC => {
            let callee = reader.read_u32("função chamada")?;
            let args = reader.read_args(limits, "contagem de argumentos da chamada")?;
            Op::Call { callee, args }
        }
        other => {
            return Err(decode_error(
                ErrorCode::UnknownOpcode,
                format!("opcode de instrução {other} desconhecido"),
            ));
        }
    };
    Ok(Instr { def, op })
}

/// Lê os dois operandos `u32` de uma operação aritmética verificada.
fn read_bin_operands(reader: &mut Reader<'_>, op: BinOp) -> Result<Op, RvmError> {
    let lhs = reader.read_u32("operando esquerdo")?;
    let rhs = reader.read_u32("operando direito")?;
    Ok(Op::Bin { op, lhs, rhs })
}

/// Lê os dois operandos `u32` de uma comparação.
fn read_cmp_operands(reader: &mut Reader<'_>, op: CmpOp) -> Result<Op, RvmError> {
    let lhs = reader.read_u32("operando esquerdo")?;
    let rhs = reader.read_u32("operando direito")?;
    Ok(Op::Cmp { op, lhs, rhs })
}

/// Lê o terminador; tipo fora da tabela é `unknown_opcode`.
fn read_terminator(reader: &mut Reader<'_>, limits: &Limits) -> Result<Terminator, RvmError> {
    let kind = reader.read_u8("tipo de terminador")?;
    match kind {
        TERM_RETURN => Ok(Terminator::Return(None)),
        TERM_RETURN_VALUE => {
            let value = reader.read_u32("valor de retorno")?;
            Ok(Terminator::Return(Some(value)))
        }
        TERM_BR => {
            let target = reader.read_u32("alvo do desvio")?;
            let args = reader.read_args(limits, "contagem de argumentos do desvio")?;
            Ok(Terminator::Br(BranchTarget { target, args }))
        }
        TERM_COND_BR => {
            let cond = reader.read_u32("condição do desvio")?;
            let then_target = reader.read_u32("alvo verdadeiro")?;
            let then_args = reader.read_args(limits, "argumentos do alvo verdadeiro")?;
            let else_target = reader.read_u32("alvo falso")?;
            let else_args = reader.read_args(limits, "argumentos do alvo falso")?;
            Ok(Terminator::CondBr {
                cond,
                then_target: BranchTarget {
                    target: then_target,
                    args: then_args,
                },
                else_target: BranchTarget {
                    target: else_target,
                    args: else_args,
                },
            })
        }
        other => Err(decode_error(
            ErrorCode::UnknownOpcode,
            format!("tipo de terminador {other} desconhecido"),
        )),
    }
}

/// Lê a tag de tipo; fora de `0..=2` é `unknown_type`.
fn read_type(reader: &mut Reader<'_>, what: &'static str) -> Result<Type, RvmError> {
    match reader.read_u8(what)? {
        TAG_UNIT => Ok(Type::Unit),
        TAG_BOOL => Ok(Type::Bool),
        TAG_I64 => Ok(Type::I64),
        tag => Err(decode_error(
            ErrorCode::UnknownType,
            format!("{what}: tag de tipo {tag} desconhecida"),
        )),
    }
}

/// Mapeia o [`Type`] para a tag `u8` do layout.
const fn type_tag(ty: Type) -> u8 {
    match ty {
        Type::Unit => TAG_UNIT,
        Type::Bool => TAG_BOOL,
        Type::I64 => TAG_I64,
    }
}

/// Mapeia [`BinOp`] para o opcode do layout.
const fn bin_opcode(op: BinOp) -> u8 {
    match op {
        BinOp::AddI64Checked => OP_ADD_I64_CHECKED,
        BinOp::SubI64Checked => OP_SUB_I64_CHECKED,
        BinOp::MulI64Checked => OP_MUL_I64_CHECKED,
    }
}

/// Mapeia [`CmpOp`] para o opcode do layout.
const fn cmp_opcode(op: CmpOp) -> u8 {
    match op {
        CmpOp::Eq => OP_EQ_I64,
        CmpOp::Ne => OP_NE_I64,
        CmpOp::Lt => OP_LT_I64,
        CmpOp::Le => OP_LE_I64,
        CmpOp::Gt => OP_GT_I64,
        CmpOp::Ge => OP_GE_I64,
    }
}

/// Escreve um byte.
fn push_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

/// Escreve um `u16` little-endian.
fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Escreve um `u32` little-endian.
fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Escreve um `i64` little-endian.
fn push_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Escreve uma string como `u16` de comprimento + bytes UTF-8.
fn push_string(out: &mut Vec<u8>, text: &str) {
    push_u16(out, len_u16(text.len()));
    out.extend_from_slice(text.as_bytes());
}

/// Leitor estritamente monotônico sobre a entrada hostil.
///
/// A posição só avança depois que `get` comprova que os bytes existem; nenhum
/// método indexa `bytes` diretamente.
struct Reader<'a> {
    /// Entrada completa, imutável.
    bytes: &'a [u8],
    /// Próximo byte ainda não consumido (`<= bytes.len()`).
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Cria um leitor na posição zero.
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Bytes ainda não consumidos.
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    /// Consome `len` bytes ou falha com `truncated_input` sem avançar a posição.
    fn take(&mut self, len: usize, what: &'static str) -> Result<&'a [u8], RvmError> {
        let start = self.pos;
        let end = start.checked_add(len).ok_or_else(|| {
            decode_error(
                ErrorCode::TruncatedInput,
                format!("{what}: deslocamento excedeu usize"),
            )
        })?;
        let remaining = self.remaining();
        let bytes = self.bytes;
        let slice = bytes.get(start..end).ok_or_else(|| {
            decode_error(
                ErrorCode::TruncatedInput,
                format!("{what}: faltam bytes; restam {remaining}"),
            )
        })?;
        self.pos = end;
        Ok(slice)
    }

    /// Lê um byte.
    fn read_u8(&mut self, what: &'static str) -> Result<u8, RvmError> {
        let slice = self.take(1, what)?;
        slice
            .first()
            .copied()
            .ok_or_else(|| bytes_insufficient(what))
    }

    /// Lê um `u16` little-endian.
    fn read_u16(&mut self, what: &'static str) -> Result<u16, RvmError> {
        let slice = self.take(2, what)?;
        let bytes: [u8; 2] = slice.try_into().map_err(|_| bytes_insufficient(what))?;
        Ok(u16::from_le_bytes(bytes))
    }

    /// Lê um `u32` little-endian.
    fn read_u32(&mut self, what: &'static str) -> Result<u32, RvmError> {
        let slice = self.take(4, what)?;
        let bytes: [u8; 4] = slice.try_into().map_err(|_| bytes_insufficient(what))?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Lê um `i64` little-endian.
    fn read_i64(&mut self, what: &'static str) -> Result<i64, RvmError> {
        let slice = self.take(8, what)?;
        let bytes: [u8; 8] = slice.try_into().map_err(|_| bytes_insufficient(what))?;
        Ok(i64::from_le_bytes(bytes))
    }

    /// Lê uma string com comprimento `u16`, checando bytes restantes e o limite
    /// de nomes **antes** de alocar.
    fn read_name(&mut self, limits: &Limits, what: &'static str) -> Result<String, RvmError> {
        let raw_len = self.read_u16(what)?;
        let len = usize::from(raw_len);
        // Antes de alocar: a string precisa caber nos bytes restantes...
        if len > self.remaining() {
            return Err(decode_error(
                ErrorCode::TruncatedInput,
                format!(
                    "{what}: {len} bytes declarados, restam {} bytes",
                    self.remaining()
                ),
            ));
        }
        // ...e no limite estrutural de nomes/rótulos.
        let max = to_usize(limits.max_name_len);
        if len > max {
            return Err(decode_error(
                ErrorCode::InvalidModule,
                format!("{what}: {len} bytes excedem o limite de {max}"),
            ));
        }
        let slice = self.take(len, what)?;
        let text = std::str::from_utf8(slice).map_err(|_| {
            decode_error(
                ErrorCode::InvalidModule,
                format!("{what}: conteúdo não é UTF-8 válido"),
            )
        })?;
        Ok(text.to_string())
    }

    /// Lê uma contagem `u32` e a valida antes de qualquer alocação.
    fn read_count_u32(&mut self, max: u32, what: &'static str) -> Result<usize, RvmError> {
        let count = self.read_u32(what)?;
        self.check_count(count, max, what)
    }

    /// Lê uma contagem `u16` e a valida antes de qualquer alocação.
    fn read_count_u16(&mut self, max: u32, what: &'static str) -> Result<usize, RvmError> {
        let count = u32::from(self.read_u16(what)?);
        self.check_count(count, max, what)
    }

    /// Invariante 5: compara a contagem com os bytes restantes (cada item consome
    /// ao menos um byte) e com o limite estrutural, nesta ordem. Nada é alocado
    /// aqui.
    fn check_count(&self, count: u32, max: u32, what: &'static str) -> Result<usize, RvmError> {
        let count_usize = to_usize(count);
        if count_usize > self.remaining() {
            return Err(decode_error(
                ErrorCode::TruncatedInput,
                format!(
                    "{what}: {count} itens declarados, restam {} bytes",
                    self.remaining()
                ),
            ));
        }
        let max_usize = to_usize(max);
        if count_usize > max_usize {
            return Err(decode_error(
                ErrorCode::InvalidModule,
                format!("{what}: {count} excede o limite estrutural de {max}"),
            ));
        }
        Ok(count_usize)
    }

    /// Lê `argc` e os argumentos, com contagem validada antes da alocação.
    fn read_args(&mut self, limits: &Limits, what: &'static str) -> Result<Vec<ValueId>, RvmError> {
        let argc = self.read_count_u16(limits.max_call_args, what)?;
        let mut args = Vec::with_capacity(argc);
        for _ in 0..argc {
            args.push(self.read_u32("argumento")?);
        }
        Ok(args)
    }
}

/// Erro de decode com código estável e detalhe humano (instável).
fn decode_error(code: ErrorCode, detail: impl Into<String>) -> RvmError {
    RvmError::new(code, detail)
}

/// Erro de truncamento para um campo já delimitado por `take`.
fn bytes_insufficient(what: &'static str) -> RvmError {
    decode_error(
        ErrorCode::TruncatedInput,
        format!("{what}: bytes insuficientes"),
    )
}

/// Converte `u32` em `usize` sem truncar (satura se a plataforma não couber).
fn to_usize(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// Converte um comprimento em `u16`, saturando no máximo do campo.
fn len_u16(len: usize) -> u16 {
    u16::try_from(len).unwrap_or(u16::MAX)
}

/// Converte um comprimento em `u32`, saturando no máximo do campo.
fn len_u32(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvm_ir::testing;

    /// Corpus válido compartilhado (`rvm_ir::testing`).
    fn fixtures() -> [Module; 9] {
        [
            testing::soma(),
            testing::constante(42),
            testing::desvio_verdadeiro(),
            testing::com_merge(),
            testing::unidade(),
            testing::loop_infinito(),
            testing::recursao(),
            testing::overflow(),
            testing::comparacao(),
        ]
    }

    #[test]
    fn round_trip_reproduz_os_fixtures() {
        let limits = Limits::default();
        for module in fixtures() {
            let encoded = encode(&module);
            let decoded = decode(&encoded, &limits)
                .unwrap_or_else(|error| panic!("decode falhou para `{}`: {error}", module.name));
            assert_eq!(
                decoded, module,
                "round-trip divergiu para `{}`",
                module.name
            );
        }
    }

    #[test]
    fn encode_e_deterministico_em_duas_chamadas() {
        for module in fixtures() {
            assert_eq!(
                encode(&module),
                encode(&module),
                "encode divergiu entre chamadas para `{}`",
                module.name
            );
        }
    }

    #[test]
    fn codificacao_de_unidade_segue_o_layout_byte_a_byte() {
        // `module unidade version 1; fn main() -> unit { entry: return }`
        let encoded = encode(&testing::unidade());
        let mut expected = Vec::new();
        expected.extend_from_slice(b"RSVMB1");
        expected.extend_from_slice(&1u16.to_le_bytes()); // format_version
        expected.extend_from_slice(&0u16.to_le_bytes()); // flags
        expected.extend_from_slice(&7u16.to_le_bytes()); // comprimento de "unidade"
        expected.extend_from_slice(b"unidade");
        expected.extend_from_slice(&1u16.to_le_bytes()); // module_version
        expected.extend_from_slice(&0u32.to_le_bytes()); // entry_fn
        expected.extend_from_slice(&1u32.to_le_bytes()); // function_count
        expected.extend_from_slice(&0u32.to_le_bytes()); // id da função
        expected.extend_from_slice(&4u16.to_le_bytes()); // comprimento de "main"
        expected.extend_from_slice(b"main");
        expected.push(0); // ret_type: unit
        expected.extend_from_slice(&0u32.to_le_bytes()); // entry_block
        expected.extend_from_slice(&0u16.to_le_bytes()); // param_count
        expected.extend_from_slice(&1u32.to_le_bytes()); // block_count
        expected.extend_from_slice(&0u32.to_le_bytes()); // id do bloco
        expected.extend_from_slice(&5u16.to_le_bytes()); // comprimento de "entry"
        expected.extend_from_slice(b"entry");
        expected.extend_from_slice(&0u16.to_le_bytes()); // block_param_count
        expected.extend_from_slice(&0u32.to_le_bytes()); // instr_count
        expected.push(0); // terminador: return sem valor
        assert_eq!(encoded, expected);
    }

    #[test]
    fn prefixos_truncados_nunca_produzem_modulo() {
        let limits = Limits::default();
        for module in fixtures() {
            let encoded = encode(&module);
            for end in 0..encoded.len() {
                if let Some(prefix) = encoded.get(..end) {
                    assert!(
                        decode(prefix, &limits).is_err(),
                        "prefixo de {end} bytes de `{}` decodificou parcialmente",
                        module.name
                    );
                }
            }
        }
    }
}
