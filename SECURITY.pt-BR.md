# Política de segurança — RSUVM

[English](./SECURITY.md) | **Português (BR)**

## Estado do projeto

A RSUVM é um **protótipo experimental de pesquisa** (EXP-0). Não é um sandbox
validado, não é autorização para produção e não deve ser usado para executar
código mutuamente desconfiado.

O modelo suportado é **single-tenant**: um processo do host executa um módulo
confiável ou de um único dono. O runtime não implementa isolamento de processo/OS,
não separa tenants e não é uma fronteira de segurança contra código hostil.

## Versões suportadas

| Versão | Suporte |
|---|---|
| `0.1.x` (alpha) | Apenas a última versão publicada; correções de segurança na melhor capacidade |
| Versões anteriores | Sem suporte |

Não há LTS nem promessa de compatibilidade entre versões do formato textual,
bytecode, RIR ou API Rust. Ver `docs/COMPATIBILIDADE.md`.

## Como reportar

**Não abra issue pública para vulnerabilidade.** Use o canal privado:

- GitHub Security Advisories: <https://github.com/maquineh/rsuvm/security/advisories/new>

Inclua, se possível:

- versão/commit e sistema operacional;
- comando e módulo `.rvm`/bytecode que reproduz;
- comportamento esperado e observado;
- impacto (negação de serviço, leitura indevida, bypass de verificação etc.);
- se o achado depende de configuração específica (`--allow-fs-read`, host
  customizado, sistema de arquivos com symlink/hard link).

Não envie segredos, dados pessoais ou conteúdo de arquivos sensíveis. Se o
reproduzir exigir um arquivo sensível, descreva a estrutura em vez do conteúdo.

## Escopo

Dentro do escopo:

- verificador (`rvm-verify`), checagem estrutural (`rvm-ir`) e decoder
  (`rvm-bytecode`);
- intérprete e quotas (`rvm-interpreter`, `rvm-runtime`);
- ABI de host e checagem de capability (`BuiltinHost`, `Capabilities`, `Host`);
- CLI `rsuvm` e seu host de arquivo (`CliHost`);
- parser/impressão textual (`rvm-text`);
- linker (`rvm-link`).

Fora do escopo (e frequentemente configuração do operador):

- isolamento multi-tenant, containers, VMs, seccomp, cgroups/Job Objects;
- escopo amplo, gravável ou com hard links para arquivos sensíveis;
- TOCTOU com escritor concorrente no diretório concedido;
- side channels, DoS de recurso sem isolamento externo, limites de tempo/taxa;
- frontends Lift/Rio em `frontends/` e dependências de fuzz;
- sistema operacional, filesystem, hardware e runtime Rust subjacente.

## Limitações conhecidas

Estas limitações são **conhecidas e declaradas**, não vulnerabilidades novas:

- TOCTOU entre a canonicalização e a abertura no host do CLI;
- hard links dentro do escopo para inodes fora dele não são detectados;
- teto de 1 MiB **por leitura**, sem quota agregada de bytes, tempo ou taxa;
- sem auditoria independente e sem campanha de fuzzing de longa duração;
- API/formato experimentais, sem compatibilidade entre versões;
- `Host` customizado é confiável e deve aplicar sua própria política;
- ausência de isolamento de processo/OS.

## Divulgação

- Processo coordenado, com esforço para corrigir antes de divulgar.
- Prazo-alvo de 90 dias para achados confirmados, ajustável por complexidade e
  risco.
- Crédito ao reporter, se desejado.
- Não há programa de bug bounty.

## Pesquisa de boa-fé

Pesquisa de segurança de boa-fé neste repositório é bem-vinda. Não incentive
acesso a dados de terceiros, não exfiltre conteúdo e não cause dano. Se o teste
exigir um ambiente multi-tenant ou dados reais, pare e descreva o cenário no
relatório privado.
