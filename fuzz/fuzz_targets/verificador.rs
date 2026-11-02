#![no_main]
//! Fuzz target do verificador (`structure::check` + `verify`).
//!
//! Parte de texto UTF-8, faz o parse e, se o módulo parseia, roda a verificação.
//! Contrato: não pode causar pânico. Método e limitações: ADR-0020.

use libfuzzer_sys::fuzz_target;
use rvm_spec::Limits;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let limits = Limits::default();
        if let Ok(module) = rvm_text::parse(text, &limits) {
            let _ = rvm_verify::VerifiedModule::verify(module, &limits);
        }
    }
});
