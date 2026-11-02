//! Testes hostis do decoder binário de `rvm-bytecode`.
//!
//! Este alvo de teste libera `unwrap`/`panic`/indexação apenas para montar bytes
//! e relatar falhas; a biblioteca segue as lints do workspace (`src/lib.rs`).
//!
//! O teste de mutação é **determinístico e limitado**: 10 módulos válidos com 200
//! mutações de 1–3 bytes cada (seed `0x2545F4914F6CDD1D`), exigindo apenas
//! ausência de pânico. Ele não prova ausência de bugs nem cobre todas as entradas.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rvm_bytecode::{decode, encode, FORMAT_VERSION, MAGIC};
use rvm_ir::testing;
use rvm_spec::{ErrorCode, Limits};

/// Exige que `decode` rejeite a entrada e devolve o código estável do erro.
fn error_code(bytes: &[u8], limits: &Limits) -> ErrorCode {
    match decode(bytes, limits) {
        Ok(module) => panic!("decodificação inesperada do módulo `{}`", module.name),
        Err(error) => error.code(),
    }
}

/// Bytes de um módulo mínimo válido e deslocamentos dos campos mutáveis.
///
/// Layout de referência:
/// `module m version 1; entry main; fn main() -> i64 { entry: v0 = const_i64 42; return v0 }`
struct Fixture {
    bytes: Vec<u8>,
    format_version: usize,
    flags: usize,
    module_version: usize,
    entry: usize,
    function_count: usize,
    type_tag: usize,
    ret_type: usize,
    instr_count: usize,
    instr_opcode: usize,
}

impl Fixture {
    /// Monta os bytes na ordem do contrato e registra os deslocamentos.
    fn new() -> Self {
        let mut bytes = MAGIC.to_vec();
        let format_version = bytes.len();
        push_u16(&mut bytes, FORMAT_VERSION);
        let flags = bytes.len();
        push_u16(&mut bytes, 0);
        push_string(&mut bytes, "m");
        let module_version = bytes.len();
        push_u16(&mut bytes, 1);
        let entry = bytes.len();
        push_u32(&mut bytes, 0);
        push_u16(&mut bytes, 5); // type_count
        let type_tag = bytes.len();
        bytes.extend_from_slice(&[0, 1, 2, 3, 4]); // unit, bool, i64, string, f64
        push_u16(&mut bytes, 0); // struct_count
        let function_count = bytes.len();
        push_u32(&mut bytes, 1);

        // Função `main`: id, nome, tipo de retorno, entrada, parâmetros e blocos.
        push_u32(&mut bytes, 0);
        push_string(&mut bytes, "main");
        let ret_type = bytes.len();
        push_u32(&mut bytes, 2); // i64
        push_u32(&mut bytes, 0); // entry_block
        push_u16(&mut bytes, 0); // param_count
        push_u32(&mut bytes, 1); // block_count

        // Bloco `entry`: id, rótulo, parâmetros, instruções e terminador.
        push_u32(&mut bytes, 0);
        push_string(&mut bytes, "entry");
        push_u16(&mut bytes, 0); // block_param_count
        let instr_count = bytes.len();
        push_u32(&mut bytes, 1);
        push_u32(&mut bytes, 0); // def v0
        let instr_opcode = bytes.len();
        bytes.push(0); // const_i64
        push_i64(&mut bytes, 42);
        bytes.push(1); // return com valor
        push_u32(&mut bytes, 0);

        Self {
            bytes,
            format_version,
            flags,
            module_version,
            entry,
            function_count,
            type_tag,
            ret_type,
            instr_count,
            instr_opcode,
        }
    }
}

/// Escreve um `u16` little-endian.
fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Escreve um `u32` little-endian.
fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Escreve um `i64` little-endian.
fn push_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Escreve uma string como `u16` de comprimento + UTF-8.
fn push_string(bytes: &mut Vec<u8>, text: &str) {
    let len = u16::try_from(text.len()).expect("nome de teste curto");
    push_u16(bytes, len);
    bytes.extend_from_slice(text.as_bytes());
}

/// Xorshift64 determinístico: sem `std::rand`, sem relógio e sem estado global.
fn next_u64(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

#[test]
fn fixture_minima_decodifica() {
    let fixture = Fixture::new();
    let module = decode(&fixture.bytes, &Limits::default()).expect("fixture mínima válida");
    assert_eq!(module.name, "m");
    assert_eq!(module.functions.len(), 1);
}

#[test]
fn entrada_exatamente_no_limite_e_aceita() {
    let fixture = Fixture::new();
    let max = u32::try_from(fixture.bytes.len()).expect("fixture cabe em u32");
    let limits = Limits {
        max_input_bytes: max,
        ..Limits::default()
    };
    assert!(decode(&fixture.bytes, &limits).is_ok());
}

#[test]
fn arquivo_vazio_e_cabecalho_truncado_sao_truncated_input() {
    let limits = Limits::default();
    assert_eq!(error_code(&[], &limits), ErrorCode::TruncatedInput);

    let bytes = Fixture::new().bytes;
    for end in 0..10 {
        if let Some(prefix) = bytes.get(..end) {
            assert_eq!(
                error_code(prefix, &limits),
                ErrorCode::TruncatedInput,
                "prefixo de {end} bytes deveria ser truncated_input"
            );
        }
    }
}

#[test]
fn magic_invalido_e_syntax_error() {
    let limits = Limits::default();

    let mut fixture = Fixture::new();
    fixture.bytes[0] = b'X';
    assert_eq!(error_code(&fixture.bytes, &limits), ErrorCode::SyntaxError);

    let mut fixture = Fixture::new();
    fixture.bytes[MAGIC.len() - 1] = b'2';
    assert_eq!(error_code(&fixture.bytes, &limits), ErrorCode::SyntaxError);
}

#[test]
fn versao_e_flags_invalidas_sao_unsupported_version() {
    let limits = Limits::default();

    let mut fixture = Fixture::new();
    fixture.bytes[fixture.format_version] = 2;
    assert_eq!(
        error_code(&fixture.bytes, &limits),
        ErrorCode::UnsupportedVersion
    );

    let mut fixture = Fixture::new();
    fixture.bytes[fixture.flags] = 1;
    assert_eq!(
        error_code(&fixture.bytes, &limits),
        ErrorCode::UnsupportedVersion
    );

    // A versão do módulo (RIR) é reconferida por `structure::check`.
    let mut fixture = Fixture::new();
    fixture.bytes[fixture.module_version] = 2;
    assert_eq!(
        error_code(&fixture.bytes, &limits),
        ErrorCode::UnsupportedVersion
    );
}

#[test]
fn contagens_gigantes_sao_truncated_input_sem_alocar() {
    let limits = Limits::default();

    let mut fixture = Fixture::new();
    fixture.bytes[fixture.function_count..fixture.function_count + 4]
        .copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        error_code(&fixture.bytes, &limits),
        ErrorCode::TruncatedInput
    );

    let mut fixture = Fixture::new();
    fixture.bytes[fixture.instr_count..fixture.instr_count + 4]
        .copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        error_code(&fixture.bytes, &limits),
        ErrorCode::TruncatedInput
    );
}

#[test]
fn opcode_desconhecido_e_unknown_opcode() {
    let limits = Limits::default();

    let mut fixture = Fixture::new();
    fixture.bytes[fixture.instr_opcode] = 200;
    assert_eq!(
        error_code(&fixture.bytes, &limits),
        ErrorCode::UnknownOpcode
    );

    // O último terminador ocupa 5 bytes (tipo + u32); tipo 200 também é opcode.
    let mut fixture = Fixture::new();
    let terminator = fixture.bytes.len() - 5;
    fixture.bytes[terminator] = 200;
    assert_eq!(
        error_code(&fixture.bytes, &limits),
        ErrorCode::UnknownOpcode
    );
}

#[test]
fn tag_de_tipo_desconhecida_e_unknown_type() {
    let mut fixture = Fixture::new();
    fixture.bytes[fixture.type_tag] = 200;
    assert_eq!(
        error_code(&fixture.bytes, &Limits::default()),
        ErrorCode::UnknownType
    );
}

#[test]
fn const_bool_fora_de_zero_um_e_invalid_number() {
    let mut fixture = Fixture::new();
    fixture.bytes[fixture.instr_opcode] = 1; // const_bool
    fixture.bytes[fixture.instr_opcode + 1] = 2; // imediato inválido
    assert_eq!(
        error_code(&fixture.bytes, &Limits::default()),
        ErrorCode::InvalidNumber
    );
}

#[test]
fn bytes_excedentes_sao_invalid_module() {
    let mut bytes = encode(&testing::constante(7));
    bytes.push(0);
    assert_eq!(
        error_code(&bytes, &Limits::default()),
        ErrorCode::InvalidModule
    );
}

#[test]
fn nome_com_byte_invalido_e_invalid_module() {
    let mut fixture = Fixture::new();
    // `ret_type` vem logo após o nome `main`; corromper o último byte quebra o UTF-8.
    let last_name_byte = fixture.ret_type - 1;
    fixture.bytes[last_name_byte] = 0xFF;
    assert_eq!(
        error_code(&fixture.bytes, &Limits::default()),
        ErrorCode::InvalidModule
    );
}

#[test]
fn modulo_estruturalmente_invalido_e_invalid_module() {
    let mut fixture = Fixture::new();
    // `entry_fn` passa a apontar para uma função inexistente.
    fixture.bytes[fixture.entry..fixture.entry + 4].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(
        error_code(&fixture.bytes, &Limits::default()),
        ErrorCode::InvalidModule
    );
}

#[test]
fn entrada_maior_que_o_limite_e_input_too_large_antes_do_corpo() {
    let limits = Limits {
        max_input_bytes: 4,
        ..Limits::default()
    };
    // A checagem de tamanho vence a de magic: nada do corpo é lido.
    assert_eq!(
        error_code(&Fixture::new().bytes, &limits),
        ErrorCode::InputTooLarge
    );
    assert_eq!(
        error_code(b"magic invalido", &limits),
        ErrorCode::InputTooLarge
    );
}

/// Mutação determinística com seed fixa: 10 módulos válidos × 200 mutações.
///
/// Orçamento: 2 000 decodificações de bytes mutados em 1–3 posições. O teste
/// exige apenas ausência de pânico; não afirma cobertura exaustiva de entradas.
#[test]
fn mutacao_xorshift_seed_2545f4914f6cdd1d_10_modulos_200_mutacoes() {
    let limits = Limits::default();
    let mut bases: Vec<Vec<u8>> = vec![
        encode(&testing::soma()),
        encode(&testing::constante(42)),
        encode(&testing::desvio_verdadeiro()),
        encode(&testing::com_merge()),
        encode(&testing::unidade()),
        encode(&testing::loop_infinito()),
        encode(&testing::recursao()),
        encode(&testing::overflow()),
        encode(&testing::comparacao()),
    ];
    bases.push(Fixture::new().bytes);
    assert_eq!(bases.len(), 10, "o orçamento pressupõe 10 módulos válidos");

    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut decodificacoes = 0usize;
    for base in &bases {
        for _ in 0..200u32 {
            let mut bytes = base.clone();
            let mutacoes = 1 + next_u64(&mut state) % 3;
            for _ in 0..mutacoes {
                if bytes.is_empty() {
                    break;
                }
                let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
                let pos = usize::try_from(next_u64(&mut state) % len).unwrap_or(0);
                // Delta em 1..=255: a posição escolhida sempre muda de valor.
                let delta = 1 + u8::try_from(next_u64(&mut state) % 255).unwrap_or(0);
                if let Some(byte) = bytes.get_mut(pos) {
                    *byte ^= delta;
                }
            }
            let _ = decode(&bytes, &limits);
            decodificacoes += 1;
        }
    }
    assert_eq!(decodificacoes, 2_000);
}
