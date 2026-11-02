# ADR-0037 — Escopo de arquivo: canonicalização no host e handles adiados

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** item 2 do próximo passo do relatório do item 3; ADR-0036
  (ABI de host com capabilities); pedido do mantenedor.

## Contexto

O ADR-0036 entregou a primeira ABI de host (`read_file`/`filesystem_read`) com
escopo lexical: `Capabilities::allows` compara componentes de caminho e nega
`..`. Isso impede o escape por prefixo textual e por `..`, mas **não** impede um
symlink dentro do escopo de apontar para um arquivo fora dele. Também restava a
pergunta de um handle de arquivo (em vez de caminho textual), que fecharia a
janela de TOCTOU entre a checagem e a abertura.

Este ADR avalia as duas opções e decide o que entra agora.

## Decisão

1. **`rvm-runtime` continua puro.** `Capabilities::allows` permanece uma
   comparação lexical por componentes, sem IO e sem canonicalização. Negar `..`
   continua obrigatório.
2. **Canonicalização no host do CLI (agora).** `CliHost` passa a exigir escopo
   **absoluto**, sem `..`, existente e que seja **diretório**; a raiz é
   canonicalizada na criação (`std::fs::canonicalize`). Em cada chamada:
   - revalida o escopo lexical (defesa em profundidade);
   - canonicaliza o caminho pedido;
   - exige que o caminho canônico esteja sob a raiz canônica;
   - abre o caminho canônico, não o textual;
   - aceita apenas arquivo regular (rejeita diretório, dispositivo e FIFO);
   - mantém o teto de 1 MiB e a leitura limitada a `limit + 1` bytes.

   Um symlink dentro do escopo que resolva para fora é negado com
   `capability_denied`; falha de canonicalização devolve `Err` genérico no
   `result`, sem revelar se o alvo existe fora do escopo.
3. **Hosts customizados não são obrigados a canonicalizar.** A ABI só exige a
   capability e o tipo do retorno; um host que implemente `Host` pode aplicar
   política mais forte. A canonicalização é uma decisão do adaptador de arquivo
   do CLI, não do núcleo.
4. **Handle de arquivo fica adiado.** `std` não expõe `openat`/`O_NOFOLLOW`
   portáveis; obter isso exigiria `unsafe`, FFI ou dependência externa, todos
   proibidos pelo workspace sem ADR específico. Um handle também exigiria um
   tipo opaco no `Value`, fechamento determinístico, limite de handles e
   semântica de erro própria. Fica para uma decisão futura.
5. **Conformidade:** o caso `C-062` prova que `host_read_file` **verifica** e que
   `execute` sem host falha com `capability_denied` (classe `capability`). O
   harness ganha a expectativa `expect: capability <código>`.
6. **Multi-tenant continua fora de escopo.** Canonicalização reduz o escape por
   symlink, mas não prova isolamento de processo/OS, não elimina TOCTOU e não
   cobre hard links. Uso multi-tenant exige isolamento externo revisado.

## Alternativas

1. **Manter só a checagem lexical:** rejeitada; um symlink dentro do escopo
   lê fora dele.
2. **Canonicalizar em `rvm-runtime`:** rejeitada; o runtime é puro e não pode
   tocar o sistema de arquivos. A canonicalização depende do host.
3. **Handle/`openat` agora:** adiada; ver item 4. Sem `std` portável, a opção
   exigiria `unsafe`/FFI/dependência e um novo tipo na ABI.
4. **Resolver symlinks manualmente:** rejeitada; reimplementaria `realpath` com
   corridas e casos de borda piores que `std::fs::canonicalize`.

## Consequências

- **Positivas:** escape por symlink é negado para caminhos existentes; o escopo
  fica absoluto e explícito; erros de escopo aparecem antes de ler o módulo;
  apenas arquivos regulares são lidos; a checagem do intérprete continua
  independente da do host.
- **Negativas:** o diretório concedido precisa existir no início; a
  canonicalização adiciona syscalls; a comparação de caminhos canônicos no
  Windows pode ser sensível a caixa; hard links não são distinguíveis por
  caminho; a janela TOCTOU entre canonicalizar e abrir permanece.
- **Residual:** sem `openat`/`O_NOFOLLOW`, sem handle, sem auditoria de acessos,
  sem limite de taxa/tempo, sem isolamento de SO. O tipo do arquivo é checado
  antes da abertura, mas a corrida entre checar e abrir permanece (TOCTOU). **Não
  é sandbox.**

## Validação

- `crates/rvm-cli/tests/cli.rs`: escopo dentro, fora, `..`, vazio, relativo,
  inexistente, symlink para fora (`cfg(unix)`) e round-trip `encode`/`run-bc`.
- `crates/rvm-conformance/corpus/valid/C-062-host-sem-capability.rvm` e
  `expect: capability capability_denied` no harness.
- Gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets --
  -D warnings` e `cargo fmt --all -- --check` verdes.
