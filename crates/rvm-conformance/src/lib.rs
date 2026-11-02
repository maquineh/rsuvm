//! `rvm-conformance` — corpus e testes de conformidade do EXP-0.
//!
//! O corpus é **dado**, não código: arquivos `.rvm` em `corpus/valid/` e
//! `corpus/invalid/`, com expectativas declaradas em comentários (`; expect:`).
//! Os testes de integração em `tests/` percorrem o diretório, executam o
//! pipeline `texto → estrutura → verificação → execução` e comparam o resultado
//! com a expectativa declarada.
//!
//! Nenhuma expectativa é derivada da implementação: os casos seguem
//! `docs/fundacao/TESTES/CONFORMIDADE.md` (C-001..C-006, N-001..N-012) e
//! `docs/CONTRATOS/`.

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
