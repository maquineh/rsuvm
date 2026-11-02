#![no_main]
//! Fuzz target do parser textual (`rvm_text::parse`).
//!
//! Contrato: qualquer entrada (UTF-8 válido ou não) não pode causar pânico; o
//! resultado pode ser `Ok` ou `Err`. Método e limitações: ADR-0020.

use libfuzzer_sys::fuzz_target;
use rvm_spec::Limits;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = rvm_text::parse(text, &Limits::default());
    }
});
