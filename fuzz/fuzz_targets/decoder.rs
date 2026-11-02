#![no_main]
//! Fuzz target do decoder binário (`rvm_bytecode::decode`).
//!
//! Contrato: qualquer sequência de bytes não pode causar pânico; o resultado pode
//! ser `Ok` ou `Err`. Método e limitações: ADR-0020.

use libfuzzer_sys::fuzz_target;
use rvm_spec::Limits;

fuzz_target!(|data: &[u8]| {
    let _ = rvm_bytecode::decode(data, &Limits::default());
});
