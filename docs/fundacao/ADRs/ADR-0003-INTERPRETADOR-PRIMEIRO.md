# ADR-0003 — Intérprete antes de JIT/AOT

- **Status:** Proposto
- **Data:** 2026-11-01
- **Contexto:** Compilação nativa pode obscurecer erros semânticos e ampliar superfície de ataque; é necessário executor simples para validar contrato.

## Decisão proposta

Começar com intérprete de RIR/bytecode, single-thread, determinístico e sob quotas. Adiar JIT/AOT até correção/segurança e necessidade de desempenho serem demonstradas.

## Alternativas

- Emitir Rust e delegar ao compilador host.
- JIT desde o início.
- Apenas validar formato sem executar.

## Consequências

- Intérprete ajuda como oráculo semântico, mas pode ser lento.
- Emissão de Rust não fornece sandbox ao programa gerado.
- Sem benchmark, não justificar investimento em otimização.

## Validação

Semântica e quotas observáveis e cobertas por corpus. JIT/AOT exige ADR separado, threat model, medição e equivalência diferencial.
