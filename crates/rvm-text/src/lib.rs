//! `rvm-text` — formato textual experimental da RIR (EXP-0).
//!
//! O formato existe para que o corpus de conformidade seja **dado**, não código
//! Rust, e para dar ao CLI uma entrada legível. Ele não é formato wire estável:
//! ADR-0004 permanece aberto e `docs/CONTRATOS/TEXTO-EXP-0.md` é a especificação
//! vigente.
//!
//! # Gramática (resumo)
//!
//! ```text
//! module <ident> version <int>
//! entry <ident>
//! fn <ident>(<ident>: <type>{, ...}) -> <type> {
//!   <label>: [ (<ident>: <type>{, ...}) ]
//!     [<ident> = <op>]*
//!     <terminator>
//! }
//! ```
//!
//! `;` inicia comentário até o fim da linha. `type` ∈ {`unit`, `bool`, `i64`}.
//! Terminadores: `return [<v>]`, `br <target>`, `cond_br <v>, <target>, <target>`,
//! onde `<target>` é `<label>` ou `<label>(<v>{, ...})`.
//!
//! O parser faz resolução de nomes (`nome` → `ValueId`), checa a forma do bloco
//! (exatamente um terminador, por último) e então roda
//! [`rvm_ir::structure::check`]. Tipos, CFG e dominância ficam para `rvm-verify`.
//!
//! # Duas passadas
//!
//! Declarações são coletadas antes dos usos, então uma referência a um valor
//! definido mais adiante no arquivo é resolvida; se o uso é legalmente
//! alcançável é decisão do verificador (dominância), não do parser. IDs de valor
//! são atribuídos na ordem do arquivo: parâmetros da função, depois, por bloco,
//! parâmetros de bloco e definições de instrução.
//!
//! # Ambiguidade conhecida de `return`
//!
//! Como o formato não tem terminador de linha, `return <nome>` é distinguido de
//! `return` seguido de um novo bloco (`<nome>:`) olhando o token seguinte ao
//! nome: se for `:` ou `(`, o nome abre um bloco e o retorno não tem valor.

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

use std::collections::BTreeMap;

use rvm_ir::{
    Block, BlockId, BlockParam, BranchTarget, FnId, Function, Instr, Module, Op, Param, Terminator,
    ValueId,
};
use rvm_spec::{BinOp, CmpOp, ErrorCode, Limits, Location, RvmError, Type, IR_VERSION};

/// Converte o texto experimental em um módulo RIR estruturalmente válido.
///
/// # Errors
///
/// Falhas de sintaxe/nomes usam códigos `Decode` (`syntax_error`,
/// `unknown_opcode`, `unknown_type`, `invalid_number`, `input_too_large`) ou
/// `invalid_module`/`invalid_control_flow`/`unsupported_feature` quando o texto é
/// legível mas descreve um módulo que a RIR EXP-0 não admite.
pub fn parse(input: &str, limits: &Limits) -> Result<Module, RvmError> {
    if input.len() > limit(limits.max_input_bytes) {
        return Err(RvmError::new(
            ErrorCode::InputTooLarge,
            format!(
                "entrada tem {} bytes; limite de {} bytes",
                input.len(),
                limits.max_input_bytes
            ),
        )
        .at(Location::Module));
    }
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let raw = parser.parse_module()?;
    lower(raw, limits)
}

/// Imprime o módulo em forma canônica, reparseável por [`parse`].
///
/// A impressão não preserva nomes de variáveis de instrução (definições são
/// impressas como `v<id>`), mas preserva IDs, estrutura, tipos, nomes de função,
/// rótulos de bloco e nomes de parâmetros. Determinística e sem alocação
/// proporcional a contagens não verificadas.
#[must_use]
pub fn print(module: &Module) -> String {
    let mut out = String::new();
    out.push_str("module ");
    out.push_str(&module.name);
    out.push_str(" version ");
    out.push_str(&module.version.to_string());
    out.push('\n');
    out.push_str("entry ");
    match module.function(module.entry) {
        Some(function) => out.push_str(&function.name),
        None => {
            out.push_str("fn");
            out.push_str(&module.entry.to_string());
        }
    }
    out.push('\n');

    for function in &module.functions {
        if function.is_external() {
            out.push_str("import ");
            out.push_str(&function.name);
            out.push('\n');
        }
    }

    for function in &module.functions {
        if function.is_external() {
            continue;
        }
        out.push('\n');
        out.push_str("fn ");
        out.push_str(&function.name);
        out.push('(');
        for (index, param) in function.params.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            out.push_str(&param.name);
            out.push_str(": ");
            out.push_str(param.ty.name());
        }
        out.push_str(") -> ");
        out.push_str(function.ret.name());
        out.push_str(" {\n");

        for block in &function.blocks {
            out.push_str("  ");
            out.push_str(&block.label);
            if !block.params.is_empty() {
                out.push('(');
                for (index, param) in block.params.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&param.name);
                    out.push_str(": ");
                    out.push_str(param.ty.name());
                }
                out.push(')');
            }
            out.push_str(":\n");

            for instr in &block.instrs {
                out.push_str("    v");
                out.push_str(&instr.def.to_string());
                out.push_str(" = ");
                print_op(module, function, &instr.op, &mut out);
                out.push('\n');
            }

            out.push_str("    ");
            match &block.term {
                Terminator::Return(None) => out.push_str("return"),
                Terminator::Return(Some(value)) => {
                    out.push_str("return ");
                    print_value(function, *value, &mut out);
                }
                Terminator::Br(target) => {
                    out.push_str("br ");
                    print_target(function, target, &mut out);
                }
                Terminator::CondBr {
                    cond,
                    then_target,
                    else_target,
                } => {
                    out.push_str("cond_br ");
                    print_value(function, *cond, &mut out);
                    out.push_str(", ");
                    print_target(function, then_target, &mut out);
                    out.push_str(", ");
                    print_target(function, else_target, &mut out);
                }
            }
            out.push('\n');
        }
        out.push_str("}\n");
    }
    out
}

fn print_op(module: &Module, function: &Function, op: &Op, out: &mut String) {
    match op {
        Op::ConstI64(value) => {
            out.push_str("const_i64 ");
            out.push_str(&value.to_string());
        }
        Op::ConstBool(value) => {
            out.push_str("const_bool ");
            out.push_str(if *value { "true" } else { "false" });
        }
        Op::Bin { op, lhs, rhs } => {
            out.push_str(op.name());
            out.push(' ');
            print_value(function, *lhs, out);
            out.push_str(", ");
            print_value(function, *rhs, out);
        }
        Op::Cmp { op, lhs, rhs } => {
            out.push_str(op.name());
            out.push(' ');
            print_value(function, *lhs, out);
            out.push_str(", ");
            print_value(function, *rhs, out);
        }
        Op::Call { callee, args } => {
            out.push_str("call_static ");
            match module.function(*callee) {
                Some(target) => out.push_str(&target.name),
                None => {
                    out.push_str("fn");
                    out.push_str(&callee.to_string());
                }
            }
            out.push('(');
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                print_value(function, *arg, out);
            }
            out.push(')');
        }
    }
}

fn print_target(function: &Function, target: &BranchTarget, out: &mut String) {
    match function.block(target.target) {
        Some(block) => out.push_str(&block.label),
        None => {
            out.push_str("bloco");
            out.push_str(&target.target.to_string());
        }
    }
    if !target.args.is_empty() {
        out.push('(');
        for (index, arg) in target.args.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            print_value(function, *arg, out);
        }
        out.push(')');
    }
}

/// Nome canônico de um valor: nome do parâmetro (de função ou bloco) quando
/// existe; caso contrário, `v<id>`.
fn print_value(function: &Function, value: ValueId, out: &mut String) {
    if let Some(param) = function.params.get(value as usize) {
        out.push_str(&param.name);
        return;
    }
    for block in &function.blocks {
        for param in &block.params {
            if param.value == value {
                out.push_str(&param.name);
                return;
            }
        }
    }
    out.push('v');
    out.push_str(&value.to_string());
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Ident(String),
    Int(i64),
    LBrace,
    RBrace,
    LParen,
    RParen,
    Colon,
    Comma,
    Arrow,
    Assign,
    Eof,
}

static EOF_TOKEN: Token = Token::Eof;

fn tokenize(input: &str) -> Result<Vec<Token>, RvmError> {
    let bytes = input.as_bytes();
    let mut tokens: Vec<Token> = Vec::new();
    let mut index = 0_usize;
    while let Some(byte) = bytes.get(index).copied() {
        match byte {
            b' ' | b'\t' | b'\r' | b'\n' => index += 1,
            b';' => {
                while let Some(current) = bytes.get(index).copied() {
                    if current == b'\n' {
                        break;
                    }
                    index += 1;
                }
            }
            b'{' => {
                tokens.push(Token::LBrace);
                index += 1;
            }
            b'}' => {
                tokens.push(Token::RBrace);
                index += 1;
            }
            b'(' => {
                tokens.push(Token::LParen);
                index += 1;
            }
            b')' => {
                tokens.push(Token::RParen);
                index += 1;
            }
            b':' => {
                tokens.push(Token::Colon);
                index += 1;
            }
            b',' => {
                tokens.push(Token::Comma);
                index += 1;
            }
            b'=' => {
                tokens.push(Token::Assign);
                index += 1;
            }
            b'-' => match bytes.get(index + 1).copied() {
                Some(b'>') => {
                    tokens.push(Token::Arrow);
                    index += 2;
                }
                Some(b'0'..=b'9') => {
                    let (token, next) = lex_number(input, bytes, index)?;
                    tokens.push(token);
                    index = next;
                }
                _ => {
                    return Err(syntax_error(format!(
                        "`-` inesperado no byte {index} (esperado `->` ou número)"
                    )))
                }
            },
            b'0'..=b'9' => {
                let (token, next) = lex_number(input, bytes, index)?;
                tokens.push(token);
                index = next;
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let (token, next) = lex_ident(input, bytes, index)?;
                tokens.push(token);
                index = next;
            }
            _ => {
                return Err(syntax_error(format!(
                    "caractere inesperado no byte {index}"
                )))
            }
        }
    }
    tokens.push(Token::Eof);
    Ok(tokens)
}

fn lex_number(input: &str, bytes: &[u8], start: usize) -> Result<(Token, usize), RvmError> {
    let mut end = start;
    if bytes.get(end).copied() == Some(b'-') {
        end += 1;
    }
    while let Some(b'0'..=b'9') = bytes.get(end).copied() {
        end += 1;
    }
    let slice = input
        .get(start..end)
        .ok_or_else(|| syntax_error(format!("número inválido no byte {start}")))?;
    let value = slice.parse::<i64>().map_err(|_| {
        RvmError::new(
            ErrorCode::InvalidNumber,
            format!("literal `{slice}` não cabe em i64"),
        )
        .at(Location::Module)
    })?;
    Ok((Token::Int(value), end))
}

fn lex_ident(input: &str, bytes: &[u8], start: usize) -> Result<(Token, usize), RvmError> {
    let mut end = start;
    while let Some(byte) = bytes.get(end).copied() {
        if byte.is_ascii_alphanumeric() || byte == b'_' {
            end += 1;
        } else {
            break;
        }
    }
    let slice = input
        .get(start..end)
        .ok_or_else(|| syntax_error(format!("identificador inválido no byte {start}")))?;
    Ok((Token::Ident(slice.to_string()), end))
}

fn syntax_error(detail: impl Into<String>) -> RvmError {
    RvmError::new(ErrorCode::SyntaxError, detail).at(Location::Module)
}

fn limit(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

// ---------------------------------------------------------------------------
// Estruturas intermediárias (nomes antes da resolução)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct RawModule {
    name: String,
    version: u16,
    entry: String,
    functions: Vec<RawFunction>,
    /// Nomes de funções externas (`import <nome>`), resolvidas pelo linker.
    imports: Vec<String>,
}

#[derive(Debug)]
struct RawFunction {
    name: String,
    params: Vec<(String, Type)>,
    ret: Type,
    blocks: Vec<RawBlock>,
}

#[derive(Debug)]
struct RawBlock {
    label: String,
    params: Vec<(String, Type)>,
    instrs: Vec<RawInstr>,
    term: RawTerm,
}

#[derive(Debug)]
struct RawInstr {
    def: String,
    op: RawOp,
}

#[derive(Debug)]
enum RawOp {
    ConstI64(i64),
    ConstBool(bool),
    Bin(BinOp, String, String),
    Cmp(CmpOp, String, String),
    Call(String, Vec<String>),
}

#[derive(Debug)]
enum RawTerm {
    Return(Option<String>),
    Br(RawTarget),
    CondBr {
        cond: String,
        then_target: RawTarget,
        else_target: RawTarget,
    },
}

#[derive(Debug)]
struct RawTarget {
    label: String,
    args: Vec<String>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    fn peek(&self) -> &Token {
        match self.tokens.get(self.index) {
            Some(token) => token,
            None => &EOF_TOKEN,
        }
    }

    fn peek_at(&self, offset: usize) -> &Token {
        let target = self.index.saturating_add(offset);
        match self.tokens.get(target) {
            Some(token) => token,
            None => &EOF_TOKEN,
        }
    }

    fn bump(&mut self) -> Token {
        let token = self.peek().clone();
        self.index = self.index.saturating_add(1);
        token
    }

    fn syntax(&self, detail: impl Into<String>) -> RvmError {
        syntax_error(format!("{} (token {})", detail.into(), self.index))
    }

    fn expect_token(&mut self, expected: Token, what: &str) -> Result<(), RvmError> {
        if self.peek() == &expected {
            self.bump();
            Ok(())
        } else {
            Err(self.syntax(format!("esperado {what}")))
        }
    }

    fn expect_ident(&mut self, what: &str) -> Result<String, RvmError> {
        match self.peek() {
            Token::Ident(name) => {
                let name = name.clone();
                self.bump();
                Ok(name)
            }
            _ => Err(self.syntax(format!("esperado {what}"))),
        }
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<(), RvmError> {
        if self.at_keyword(keyword) {
            self.bump();
            Ok(())
        } else {
            Err(self.syntax(format!("esperado `{keyword}`")))
        }
    }

    fn at_keyword(&self, keyword: &str) -> bool {
        matches!(self.peek(), Token::Ident(name) if name == keyword)
    }

    fn expect_int(&mut self, what: &str) -> Result<i64, RvmError> {
        match self.peek() {
            Token::Int(value) => {
                let value = *value;
                self.bump();
                Ok(value)
            }
            _ => Err(self.syntax(format!("esperado {what}"))),
        }
    }

    fn parse_type(&mut self) -> Result<Type, RvmError> {
        let name = self.expect_ident("tipo (`unit`, `bool` ou `i64`)")?;
        Type::from_name(&name).ok_or_else(|| {
            RvmError::new(
                ErrorCode::UnknownType,
                format!("tipo desconhecido `{name}`"),
            )
            .at(Location::Module)
        })
    }

    fn parse_module(&mut self) -> Result<RawModule, RvmError> {
        self.expect_keyword("module")?;
        let name = self.expect_ident("nome do módulo")?;
        self.expect_keyword("version")?;
        let version_raw = self.expect_int("versão do módulo")?;
        let version = u16::try_from(version_raw).map_err(|_| {
            RvmError::new(
                ErrorCode::InvalidNumber,
                format!("versão {version_raw} não cabe em u16"),
            )
            .at(Location::Module)
        })?;
        if version != IR_VERSION {
            return Err(RvmError::new(
                ErrorCode::UnsupportedVersion,
                format!("versão {version} não suportada; esperada {IR_VERSION}"),
            )
            .at(Location::Module));
        }

        let mut entry: Option<String> = None;
        let mut functions: Vec<RawFunction> = Vec::new();
        let mut imports: Vec<String> = Vec::new();
        loop {
            if self.peek() == &Token::Eof {
                break;
            }
            if self.at_keyword("import") {
                self.bump();
                let name = self.expect_ident("nome da função importada")?;
                imports.push(name);
                continue;
            }
            if self.at_keyword("entry") {
                self.bump();
                let name = self.expect_ident("nome da função de entrada")?;
                if entry.is_some() {
                    return Err(RvmError::new(
                        ErrorCode::InvalidModule,
                        "declaração `entry` duplicada",
                    )
                    .at(Location::Module));
                }
                entry = Some(name);
                continue;
            }
            if self.at_keyword("fn") {
                functions.push(self.parse_function()?);
                continue;
            }
            return Err(self.syntax("esperado `entry`, `fn` ou fim do arquivo"));
        }

        let entry = entry.ok_or_else(|| {
            RvmError::new(ErrorCode::InvalidModule, "declaração `entry` ausente")
                .at(Location::Module)
        })?;
        Ok(RawModule {
            name,
            version,
            entry,
            functions,
            imports,
        })
    }

    fn parse_function(&mut self) -> Result<RawFunction, RvmError> {
        self.expect_keyword("fn")?;
        let name = self.expect_ident("nome de função")?;
        self.expect_token(Token::LParen, "`(` na assinatura")?;
        let params = self.parse_parameters()?;
        self.expect_token(Token::RParen, "`)` na assinatura")?;
        self.expect_token(Token::Arrow, "`->` antes do tipo de retorno")?;
        let ret = self.parse_type()?;
        self.expect_token(Token::LBrace, "`{` no corpo da função")?;
        let mut blocks: Vec<RawBlock> = Vec::new();
        while self.peek() != &Token::RBrace {
            if self.peek() == &Token::Eof {
                return Err(self.syntax("bloco de função sem `}`"));
            }
            blocks.push(self.parse_block()?);
        }
        self.expect_token(Token::RBrace, "`}` no fim da função")?;
        Ok(RawFunction {
            name,
            params,
            ret,
            blocks,
        })
    }

    fn parse_parameters(&mut self) -> Result<Vec<(String, Type)>, RvmError> {
        let mut params: Vec<(String, Type)> = Vec::new();
        if self.peek() == &Token::RParen {
            return Ok(params);
        }
        loop {
            let name = self.expect_ident("nome de parâmetro")?;
            self.expect_token(Token::Colon, "`:` entre nome e tipo")?;
            let ty = self.parse_type()?;
            params.push((name, ty));
            if self.peek() == &Token::Comma {
                self.bump();
            } else {
                return Ok(params);
            }
        }
    }

    fn parse_block(&mut self) -> Result<RawBlock, RvmError> {
        let label = self.expect_ident("rótulo de bloco")?;
        let mut params: Vec<(String, Type)> = Vec::new();
        if self.peek() == &Token::LParen {
            self.bump();
            params = self.parse_parameters()?;
            self.expect_token(Token::RParen, "`)` nos parâmetros do bloco")?;
        }
        self.expect_token(Token::Colon, "`:` após o rótulo do bloco")?;

        let mut instrs: Vec<RawInstr> = Vec::new();
        let term = loop {
            if self.at_keyword("return") {
                self.bump();
                // `return v` só é valor quando o identificador seguinte não abre
                // um novo bloco (`ident:` ou `ident(...)`).
                let value = if matches!(self.peek(), Token::Ident(_))
                    && !matches!(self.peek_at(1), Token::Colon | Token::LParen)
                {
                    Some(self.expect_ident("valor de retorno")?)
                } else {
                    None
                };
                break RawTerm::Return(value);
            }
            if self.at_keyword("br") {
                self.bump();
                break RawTerm::Br(self.parse_target()?);
            }
            if self.at_keyword("cond_br") {
                self.bump();
                let cond = self.expect_ident("condição de `cond_br`")?;
                self.expect_token(Token::Comma, "`,` após a condição")?;
                let then_target = self.parse_target()?;
                self.expect_token(Token::Comma, "`,` entre os alvos")?;
                let else_target = self.parse_target()?;
                break RawTerm::CondBr {
                    cond,
                    then_target,
                    else_target,
                };
            }
            if self.peek() == &Token::RBrace {
                return Err(RvmError::new(
                    ErrorCode::InvalidControlFlow,
                    format!("bloco `{label}` sem terminador"),
                )
                .at(Location::Module));
            }
            if self.peek() == &Token::Eof {
                return Err(RvmError::new(
                    ErrorCode::InvalidControlFlow,
                    format!("bloco `{label}` sem terminador (fim do arquivo)"),
                )
                .at(Location::Module));
            }
            if matches!(self.peek(), Token::Ident(_)) {
                if matches!(self.peek_at(1), Token::Assign) {
                    instrs.push(self.parse_instr()?);
                    continue;
                }
                let next = self.peek().clone();
                let label_text = match next {
                    Token::Ident(name) => name,
                    _ => String::from("<token>"),
                };
                return Err(RvmError::new(
                    ErrorCode::InvalidControlFlow,
                    format!("bloco `{label}` sem terminador antes de `{label_text}`"),
                )
                .at(Location::Module));
            }
            return Err(self.syntax("esperado instrução ou terminador"));
        };

        // O terminador precisa ser a última instrução do bloco.
        match self.peek() {
            Token::RBrace | Token::Eof => {}
            Token::Ident(_) if matches!(self.peek_at(1), Token::Colon | Token::LParen) => {}
            Token::Ident(_) => {
                return Err(RvmError::new(
                    ErrorCode::InvalidControlFlow,
                    format!("bloco `{label}` tem instrução depois do terminador"),
                )
                .at(Location::Module))
            }
            _ => return Err(self.syntax("esperado novo bloco ou `}` após o terminador")),
        }

        Ok(RawBlock {
            label,
            params,
            instrs,
            term,
        })
    }

    fn parse_instr(&mut self) -> Result<RawInstr, RvmError> {
        let def = self.expect_ident("nome do valor definido")?;
        self.expect_token(Token::Assign, "`=` após o valor definido")?;
        let op_name = self.expect_ident("operação")?;
        let op = match op_name.as_str() {
            "const_i64" => RawOp::ConstI64(self.expect_int("literal i64")?),
            "const_bool" => {
                if self.at_keyword("true") {
                    self.bump();
                    RawOp::ConstBool(true)
                } else if self.at_keyword("false") {
                    self.bump();
                    RawOp::ConstBool(false)
                } else {
                    return Err(self.syntax("esperado `true` ou `false`"));
                }
            }
            "call_static" => {
                let callee = self.expect_ident("nome da função chamada")?;
                self.expect_token(Token::LParen, "`(` na chamada")?;
                let mut args: Vec<String> = Vec::new();
                if self.peek() != &Token::RParen {
                    loop {
                        args.push(self.expect_ident("argumento de chamada")?);
                        if self.peek() == &Token::Comma {
                            self.bump();
                        } else {
                            break;
                        }
                    }
                }
                self.expect_token(Token::RParen, "`)` no fim da chamada")?;
                RawOp::Call(callee, args)
            }
            other => {
                if let Some(bin) = BinOp::from_name(other) {
                    let lhs = self.expect_ident("operando esquerdo")?;
                    self.expect_token(Token::Comma, "`,` entre operandos")?;
                    let rhs = self.expect_ident("operando direito")?;
                    RawOp::Bin(bin, lhs, rhs)
                } else if let Some(cmp) = CmpOp::from_name(other) {
                    let lhs = self.expect_ident("operando esquerdo")?;
                    self.expect_token(Token::Comma, "`,` entre operandos")?;
                    let rhs = self.expect_ident("operando direito")?;
                    RawOp::Cmp(cmp, lhs, rhs)
                } else {
                    return Err(RvmError::new(
                        ErrorCode::UnknownOpcode,
                        format!("operação desconhecida `{other}`"),
                    )
                    .at(Location::Module));
                }
            }
        };
        Ok(RawInstr { def, op })
    }

    fn parse_target(&mut self) -> Result<RawTarget, RvmError> {
        let label = self.expect_ident("rótulo alvo")?;
        let mut args: Vec<String> = Vec::new();
        if self.peek() == &Token::LParen {
            self.bump();
            if self.peek() != &Token::RParen {
                loop {
                    args.push(self.expect_ident("argumento de desvio")?);
                    if self.peek() == &Token::Comma {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
            self.expect_token(Token::RParen, "`)` nos argumentos do desvio")?;
        }
        Ok(RawTarget { label, args })
    }
}

// ---------------------------------------------------------------------------
// Lowering para a RIR (resolução de nomes e IDs)
// ---------------------------------------------------------------------------

fn lower(raw: RawModule, limits: &Limits) -> Result<Module, RvmError> {
    let RawModule {
        name,
        version,
        entry,
        functions: raw_functions,
        imports,
    } = raw;

    let mut function_ids: BTreeMap<String, FnId> = BTreeMap::new();
    for (index, function) in raw_functions.iter().enumerate() {
        let id = u32::try_from(index).map_err(|_| {
            RvmError::new(
                ErrorCode::InvalidModule,
                "quantidade de funções não representável".to_string(),
            )
            .at(Location::Module)
        })?;
        if function_ids.insert(function.name.clone(), id).is_some() {
            return Err(RvmError::new(
                ErrorCode::InvalidModule,
                format!("função duplicada `{}`", function.name),
            )
            .at(Location::Module));
        }
    }
    // Declarações externas ocupam IDs densos após as funções declaradas; o
    // linker (`rvm-link`, ADR-0018) as substitui por funções concretas.
    let declared = u32::try_from(raw_functions.len()).map_err(|_| {
        RvmError::new(
            ErrorCode::InvalidModule,
            "quantidade de funções não representável".to_string(),
        )
        .at(Location::Module)
    })?;
    for (offset, import) in imports.iter().enumerate() {
        let offset = u32::try_from(offset).map_err(|_| {
            RvmError::new(
                ErrorCode::InvalidModule,
                "quantidade de importações não representável".to_string(),
            )
            .at(Location::Module)
        })?;
        let id = declared.saturating_add(offset);
        if function_ids.insert(import.clone(), id).is_some() {
            return Err(RvmError::new(
                ErrorCode::InvalidModule,
                format!("função duplicada `{import}`"),
            )
            .at(Location::Module));
        }
    }

    let entry = function_ids.get(entry.as_str()).copied().ok_or_else(|| {
        RvmError::new(
            ErrorCode::InvalidModule,
            format!("função de entrada `{entry}` não declarada"),
        )
        .at(Location::Module)
    })?;

    let mut functions: Vec<Function> = Vec::new();
    for (index, function) in raw_functions.into_iter().enumerate() {
        let id = u32::try_from(index).map_err(|_| {
            RvmError::new(
                ErrorCode::InvalidModule,
                "quantidade de funções não representável".to_string(),
            )
            .at(Location::Module)
        })?;
        functions.push(lower_function(id, function, &function_ids)?);
    }
    for (offset, import) in imports.into_iter().enumerate() {
        let offset = u32::try_from(offset).map_err(|_| {
            RvmError::new(
                ErrorCode::InvalidModule,
                "quantidade de importações não representável".to_string(),
            )
            .at(Location::Module)
        })?;
        functions.push(external_function(declared.saturating_add(offset), import));
    }

    let module = Module {
        name,
        version,
        entry,
        functions,
    };
    rvm_ir::structure::check(&module, limits)?;
    Ok(module)
}

/// Cria uma declaração externa: função sem corpo, resolvida pelo linker.
fn external_function(id: FnId, name: String) -> Function {
    Function {
        id,
        name,
        params: Vec::new(),
        ret: Type::Unit,
        entry: 0,
        blocks: Vec::new(),
    }
}

fn lower_function(
    id: FnId,
    raw: RawFunction,
    function_ids: &BTreeMap<String, FnId>,
) -> Result<Function, RvmError> {
    let function_name = raw.name.clone();
    let location = Location::Function {
        function: function_name.clone(),
    };

    // IDs densos na ordem do arquivo: parâmetros, depois por bloco, parâmetros
    // de bloco e definições de instrução.
    let mut names: BTreeMap<String, ValueId> = BTreeMap::new();
    let mut next_id: ValueId = 0;
    for (name, _) in &raw.params {
        insert_name(&mut names, name, next_id, &location)?;
        next_id = next_id.saturating_add(1);
    }
    for block in &raw.blocks {
        for (name, _) in &block.params {
            insert_name(&mut names, name, next_id, &location)?;
            next_id = next_id.saturating_add(1);
        }
        for instr in &block.instrs {
            insert_name(&mut names, &instr.def, next_id, &location)?;
            next_id = next_id.saturating_add(1);
        }
    }

    let mut labels: BTreeMap<String, BlockId> = BTreeMap::new();
    for (index, block) in raw.blocks.iter().enumerate() {
        let block_id = u32::try_from(index).map_err(|_| {
            RvmError::new(
                ErrorCode::InvalidModule,
                format!("função `{function_name}` tem blocos demais"),
            )
            .at(Location::Module)
        })?;
        if labels.insert(block.label.clone(), block_id).is_some() {
            return Err(RvmError::new(
                ErrorCode::InvalidModule,
                format!("rótulo de bloco duplicado `{}`", block.label),
            )
            .at(location.clone()));
        }
    }

    let params: Vec<Param> = raw
        .params
        .iter()
        .map(|(name, ty)| Param {
            name: name.clone(),
            ty: *ty,
        })
        .collect();

    let mut blocks: Vec<Block> = Vec::new();
    for raw_block in raw.blocks {
        let block_label = raw_block.label.clone();
        let block_location = Location::Block {
            function: function_name.clone(),
            block: block_label.clone(),
        };
        let mut block_params: Vec<BlockParam> = Vec::new();
        for (name, ty) in &raw_block.params {
            let value = *names.get(name.as_str()).ok_or_else(|| {
                RvmError::new(
                    ErrorCode::InvalidModule,
                    format!("parâmetro de bloco `{name}` sem ID atribuído"),
                )
                .at(block_location.clone())
            })?;
            block_params.push(BlockParam {
                value,
                name: name.clone(),
                ty: *ty,
            });
        }

        let mut instrs: Vec<Instr> = Vec::new();
        for raw_instr in &raw_block.instrs {
            let def = *names.get(raw_instr.def.as_str()).ok_or_else(|| {
                RvmError::new(
                    ErrorCode::InvalidModule,
                    format!("valor `{}` sem ID atribuído", raw_instr.def),
                )
                .at(block_location.clone())
            })?;
            let op = lower_op(&raw_instr.op, &names, function_ids, &block_location)?;
            instrs.push(Instr { def, op });
        }

        let term = lower_term(&raw_block.term, &names, &labels, &block_location)?;
        let block_id = *labels.get(raw_block.label.as_str()).ok_or_else(|| {
            RvmError::new(
                ErrorCode::InvalidModule,
                format!("bloco `{}` sem ID atribuído", raw_block.label),
            )
            .at(block_location.clone())
        })?;
        blocks.push(Block {
            id: block_id,
            label: block_label,
            params: block_params,
            instrs,
            term,
        });
    }

    Ok(Function {
        id,
        name: function_name,
        params,
        ret: raw.ret,
        entry: 0,
        blocks,
    })
}

fn insert_name(
    names: &mut BTreeMap<String, ValueId>,
    name: &str,
    id: ValueId,
    location: &Location,
) -> Result<(), RvmError> {
    if names.insert(name.to_string(), id).is_some() {
        return Err(RvmError::new(
            ErrorCode::InvalidModule,
            format!("valor duplicado `{name}`"),
        )
        .at(location.clone()));
    }
    Ok(())
}

fn lower_op(
    raw: &RawOp,
    names: &BTreeMap<String, ValueId>,
    function_ids: &BTreeMap<String, FnId>,
    location: &Location,
) -> Result<Op, RvmError> {
    let value = |name: &str| -> Result<ValueId, RvmError> {
        names.get(name).copied().ok_or_else(|| {
            RvmError::new(
                ErrorCode::InvalidModule,
                format!("valor `{name}` não declarado"),
            )
            .at(location.clone())
        })
    };
    Ok(match raw {
        RawOp::ConstI64(value) => Op::ConstI64(*value),
        RawOp::ConstBool(value) => Op::ConstBool(*value),
        RawOp::Bin(op, lhs, rhs) => Op::Bin {
            op: *op,
            lhs: value(lhs)?,
            rhs: value(rhs)?,
        },
        RawOp::Cmp(op, lhs, rhs) => Op::Cmp {
            op: *op,
            lhs: value(lhs)?,
            rhs: value(rhs)?,
        },
        RawOp::Call(callee, args) => {
            let target = function_ids.get(callee.as_str()).copied().ok_or_else(|| {
                RvmError::new(
                    ErrorCode::InvalidModule,
                    format!("função chamada `{callee}` não declarada"),
                )
                .at(location.clone())
            })?;
            let mut resolved: Vec<ValueId> = Vec::new();
            for arg in args {
                resolved.push(value(arg)?);
            }
            Op::Call {
                callee: target,
                args: resolved,
            }
        }
    })
}

fn lower_term(
    raw: &RawTerm,
    names: &BTreeMap<String, ValueId>,
    labels: &BTreeMap<String, BlockId>,
    location: &Location,
) -> Result<Terminator, RvmError> {
    let value = |name: &str| -> Result<ValueId, RvmError> {
        names.get(name).copied().ok_or_else(|| {
            RvmError::new(
                ErrorCode::InvalidModule,
                format!("valor `{name}` não declarado"),
            )
            .at(location.clone())
        })
    };
    let target = |raw_target: &RawTarget| -> Result<BranchTarget, RvmError> {
        let block = labels
            .get(raw_target.label.as_str())
            .copied()
            .ok_or_else(|| {
                RvmError::new(
                    ErrorCode::InvalidControlFlow,
                    format!("bloco alvo `{}` não declarado", raw_target.label),
                )
                .at(location.clone())
            })?;
        let mut args: Vec<ValueId> = Vec::new();
        for arg in &raw_target.args {
            args.push(value(arg)?);
        }
        Ok(BranchTarget {
            target: block,
            args,
        })
    };
    Ok(match raw {
        RawTerm::Return(None) => Terminator::Return(None),
        RawTerm::Return(Some(name)) => Terminator::Return(Some(value(name)?)),
        RawTerm::Br(raw_target) => Terminator::Br(target(raw_target)?),
        RawTerm::CondBr {
            cond,
            then_target,
            else_target,
        } => Terminator::CondBr {
            cond: value(cond)?,
            then_target: target(then_target)?,
            else_target: target(else_target)?,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvm_ir::testing;

    const SOMA: &str = r#"; exemplo
module exemplo version 1
entry soma

fn soma(a: i64, b: i64) -> i64 {
  entry:
    v2 = add_i64_checked a, b
    return v2
}
"#;

    fn parse_ok(text: &str) -> Module {
        parse(text, &Limits::default()).expect("texto deveria ser aceito")
    }

    fn error_code(text: &str) -> ErrorCode {
        parse(text, &Limits::default())
            .expect_err("texto deveria ser rejeitado")
            .code()
    }

    #[test]
    fn parseia_o_exemplo_minimo() {
        let module = parse_ok(SOMA);
        assert_eq!(module.name, "exemplo");
        assert_eq!(module.entry, 0);
        let function = module.function(0).unwrap();
        assert_eq!(function.name, "soma");
        assert_eq!(function.params.len(), 2);
        assert_eq!(function.ret, Type::I64);
        assert_eq!(function.blocks.len(), 1);
        let block = function.block(0).unwrap();
        assert_eq!(block.label, "entry");
        assert_eq!(block.instrs.len(), 1);
        assert_eq!(block.instrs[0].def, 2);
        assert_eq!(block.term, Terminator::Return(Some(2)));
    }

    #[test]
    fn round_trip_preserva_estrutura_e_ids() {
        let module = parse_ok(SOMA);
        let printed = print(&module);
        let reparsed = parse_ok(&printed);
        assert_eq!(
            module, reparsed,
            "impressão canônica precisa ser reparseável:\n{printed}"
        );
    }

    #[test]
    fn round_trip_do_merge() {
        let texto = r#"module m version 1
entry escolhe

fn escolhe(c: bool) -> i64 {
  entry:
    v1 = const_i64 1
    v2 = const_i64 2
    cond_br c, entao(v1), senao(v2)
  entao(b0: i64):
    br merge(b0)
  senao(b1: i64):
    br merge(b1)
  merge(escolhido: i64):
    return escolhido
}
"#;
        let module = parse_ok(texto);
        let printed = print(&module);
        let reparsed = parse_ok(&printed);
        assert_eq!(module, reparsed);
        assert!(printed.contains("merge(escolhido: i64):"), "{printed}");
    }

    #[test]
    fn imprime_modulos_construidos_por_api() {
        for module in [
            testing::soma(),
            testing::constante(-7),
            testing::desvio_verdadeiro(),
            testing::com_merge(),
            testing::unidade(),
            testing::loop_infinito(),
            testing::comparacao(),
        ] {
            let printed = print(&module);
            let reparsed = parse_ok(&printed);
            assert_eq!(
                reparsed.functions.len(),
                module.functions.len(),
                "impressão de `{}` perdeu funções:\n{printed}",
                module.name
            );
        }
    }

    #[test]
    fn erros_de_decode_tem_codigos_estaveis() {
        assert_eq!(error_code(""), ErrorCode::SyntaxError);
        assert_eq!(error_code("; só comentário\n"), ErrorCode::SyntaxError);
        assert_eq!(
            error_code("module m version 1\nentry f\n"),
            ErrorCode::InvalidModule
        );
        assert_eq!(
            error_code("module m version 2\nentry f\nfn f() -> unit {\n  entry:\n    return\n}\n"),
            ErrorCode::UnsupportedVersion
        );
        assert_eq!(
            error_code("module m version 99999\nentry f\n"),
            ErrorCode::InvalidNumber
        );
        assert_eq!(
            error_code(
                "module m version 1\nentry f\nfn f(a: f64) -> unit {\n  entry:\n    return\n}\n"
            ),
            ErrorCode::UnknownType
        );
        assert_eq!(
            error_code(
                "module m version 1\nentry f\nfn f() -> i64 {\n  entry:\n    v0 = div_i64 1, 2\n    return v0\n}\n"
            ),
            ErrorCode::UnknownOpcode
        );
        assert_eq!(
            error_code("module m version 1\nentry f\nimport x\n"),
            ErrorCode::InvalidModule,
            "entrada `f` não declarada; só a externa `x` existe"
        );
    }

    #[test]
    fn import_declara_funcao_externa() {
        let texto = "module m version 1\nentry main\nimport soma\n\nfn main() -> i64 {\n  entry:\n    v0 = const_i64 10\n    v1 = const_i64 20\n    v2 = call_static soma(v0, v1)\n    return v2\n}\n";
        let module = parse_ok(texto);
        let externa = module
            .function_by_name("soma")
            .expect("declaração externa existe");
        assert!(externa.is_external(), "`soma` deveria ser externa");
        assert_eq!(externa.id, 1, "externa ocupa ID após as declaradas");
        let printed = print(&module);
        assert!(printed.contains("import soma"), "{printed}");
        let reparsed = parse_ok(&printed);
        assert_eq!(module, reparsed, "import sobrevive ao round-trip");
    }

    #[test]
    fn bloco_sem_terminador_ou_terminador_fora_de_posicao() {
        assert_eq!(
            error_code(
                "module m version 1\nentry f\nfn f() -> i64 {\n  entry:\n    v0 = const_i64 1\n}\n"
            ),
            ErrorCode::InvalidControlFlow
        );
        assert_eq!(
            error_code(
                "module m version 1\nentry f\nfn f() -> i64 {\n  entry:\n    v0 = const_i64 1\n    return v0\n    return v0\n}\n"
            ),
            ErrorCode::InvalidControlFlow
        );
    }

    #[test]
    fn alvo_inexistente_e_dominio_de_nomes() {
        assert_eq!(
            error_code(
                "module m version 1\nentry f\nfn f() -> i64 {\n  entry:\n    v0 = const_i64 1\n    br naoexiste\n}\n"
            ),
            ErrorCode::InvalidControlFlow
        );
        assert_eq!(
            error_code(
                "module m version 1\nentry f\nfn f() -> i64 {\n  entry:\n    v0 = const_i64 1\n    v0 = const_i64 2\n    return v0\n}\n"
            ),
            ErrorCode::InvalidModule
        );
        assert_eq!(
            error_code(
                "module m version 1\nentry f\nfn f() -> i64 {\n  entry:\n    v0 = add_i64_checked x, x\n    return v0\n}\n"
            ),
            ErrorCode::InvalidModule
        );
    }

    #[test]
    fn limite_de_entrada_e_respeitado() {
        let limits = Limits {
            max_input_bytes: 8,
            ..Limits::default()
        };
        let error = parse(SOMA, &limits).expect_err("entrada acima do limite");
        assert_eq!(error.code(), ErrorCode::InputTooLarge);
    }

    #[test]
    fn entradas_hostis_nao_entram_em_panico() {
        let casos = [
            "module",
            "module m",
            "module m version",
            "module m version 1 entry",
            "module m version 1\nentry f\nfn",
            "module m version 1\nentry f\nfn f(",
            "module m version 1\nentry f\nfn f() -> i64 {",
            "module m version 1\nentry f\nfn f() -> i64 {\n  entry:",
            "}}}}",
            "module m version 1\nentry f\nfn f() -> i64 {\n  entry: return 1,2\n}",
            "module m version 1\nentry f\nfn f() -> i64 {\n  entry: v0 = const_i64 999999999999999999999999\n  return v0\n}",
            "module m version 1\r\nentry f\r\nfn f() -> i64 {\r\n  entry:\r\n    v0 = const_i64 1\r\n    return v0\r\n}\r\n",
            "module m version 1\nentry f\nfn f() -> i64 {\n  entry: ; comentário\n    v0 = const_i64 1 ; fim\n    return v0\n}\n",
            "module m version 1\nentry f\nfn f() -> i64 {\n  entry:\n    v0 = const_i64 1\n    v1 = const_i64 2\n    cond_br v0, a, b\n}\n",
        ];
        for caso in casos {
            let _ = parse(caso, &Limits::default());
        }
    }

    #[test]
    fn aceita_funcao_unit_e_chamada_sem_argumentos() {
        let texto = r#"module vazio version 1
entry principal

fn principal() -> unit {
  entry:
    v0 = const_i64 1
    v1 = const_i64 2
    v2 = lt_i64 v0, v1
    v3 = call_static nada()
    return
}

fn nada() -> unit {
  entry:
    return
}
"#;
        let module = parse_ok(texto);
        assert_eq!(module.functions.len(), 2);
        assert_eq!(module.function(0).unwrap().ret, Type::Unit);
        assert_eq!(module.function(1).unwrap().blocks.len(), 1);
        let printed = print(&module);
        assert!(printed.contains("call_static nada()"), "{printed}");
    }
}
