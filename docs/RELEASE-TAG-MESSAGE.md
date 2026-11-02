# Tag `v0.1.0-alpha.1` — rascunho

> **Não criar nem pushar esta tag sem confirmação explícita do mantenedor.**

## Comando previsto

```bash
git tag -a v0.1.0-alpha.1 -m "RSUVM 0.1.0-alpha.1 — alpha experimental"
git push origin v0.1.0-alpha.1
```

## Mensagem anotada proposta

```text
RSUVM 0.1.0-alpha.1 — alpha experimental

VM/RIR em Rust para pesquisa: verificador, intérprete determinístico, quotas,
formato textual/bytecode experimentais e ABI mínima de host.

Estado:
- 176 testes verdes em Linux e Windows (debug/release), MSRV 1.75 e docs.
- CI: run 41541923317 (matriz) e 41542405756 (final).
- Release dry-run: run 41543773653; publicação desativada.
- Fuzzing curto: run 41543777109, 24.751.525 execuções, 0 crashes observados.
- H06 confirmada no runner dedicado (run 41520845695).

Limitações:
- single-tenant; NÃO é sandbox e não suporta multi-tenant;
- TOCTOU/hard links residuais no acesso a arquivos;
- sem quota agregada de IO/tempo/taxa;
- sem auditoria independente e sem campanha de fuzzing longa;
- formatos/API experimentais, sem compatibilidade entre versões.

Leia SECURITY.pt-BR.md e docs/COMPATIBILIDADE.md.
```

## Pré-condições

- [ ] `HEAD` é o commit validado (`49c8cbe` ou posterior com CI verde).
- [ ] `Cargo.toml` em `0.1.0-alpha.1`.
- [ ] `CHANGELOG.md` com a data.
- [ ] `docs/RELEASE-NOTES.md` atualizado.
- [ ] `docs/RELEASE-CHECKLIST.md` revisado.
- [ ] CI da `main` verde.
- [ ] Dry-run do release verde (run `41543773653`).
- [ ] Fuzzing curto registrado (run `41543777109`).
- [ ] Confirmação explícita do mantenedor.
