## Resumo

Descreva o que muda e por quê.

## Requisito / ADR

- Requisito ou issue:
- ADR (se aplicável):

## Checklist

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings`
- [ ] `cargo test --workspace --locked`
- [ ] `cargo test --release --workspace --locked`
- [ ] `cargo +1.75.0 check --workspace --all-targets --locked` (quando tocar o núcleo)
- [ ] testes novos/atualizados cobrem a mudança
- [ ] contratos/ADRs/READMEs atualizados quando necessário
- [ ] sem `unsafe`, FFI ou dependência externa nova sem aprovação
- [ ] sem segredo, dado pessoal ou artefato de build no diff
- [ ] limitações e riscos residuais descritos

## Verificação executada

```text
comandos e resultados
```

## Riscos / lacunas

O que não foi testado e o que permanece em aberto.
