# ADR-0036 — ABI mínima de host com capabilities, escopo e quota por efeito

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** item 3 do pedido do mantenedor ("definir a ABI de host com
  capabilities, escopo e testes de negação"); ADR-0005 (deny-by-default); H05 do
  `RELATORIO-IMPLEMENTACAO.md`.

## Contexto

Até esta decisão, `Effect` e `Capabilities` existiam apenas como **negação
estrutural**: não havia instrução de host, então nenhuma capability podia ser
exercitada e o ramo de concessão era inalcançável. Para a VM ser viável falta uma
ABI mínima com um efeito real, escopo verificável, quota por efeito e testes de
negativação. O risco é abrir I/O sem contrato fechado; a resposta é uma ABI
pequena, explícita e sem caminho de escape.

## Decisão

1. **ABI fechada nesta rodada:** uma única função embutida,
   `BuiltinHost::ReadFile`:
   - assinatura `read_file(path: string, max_bytes: i64) -> result string string`;
   - efeito `filesystem_read`;
   - escopo no argumento 0 (`path`);
   - teto rígido de 1 MiB e reserva de quota antes do IO (ADR-0038);
   - código estável `0` no bytecode.
   Nenhuma outra função de host é aceita; extensão exige novo ADR e revisão do
   modelo de ameaças.
2. **RIR:** `Op::HostCall { function, args }`. A operação define um valor do tipo
   de retorno da ABI, que precisa existir na tabela do módulo (`result string
   string`).
3. **Texto:** `host_read_file <path>, <max>`.
4. **Bytecode:** opcode `54`, payload `u8 function`, `u16 argc`,
   `u32 args[argc]`; código de função desconhecido ⇒ `unknown_opcode`.
5. **Verificação:** a aridade e os tipos dos argumentos são conferidos contra a
   assinatura da ABI; divergência ⇒ `invalid_type`. O tipo de retorno é resolvido
   na tabela do módulo; ausência ⇒ `invalid_type`/`invalid_module`.
6. **Runtime:** trait `Host` (`capabilities()`, `call(function, args, ret)`),
   `NoHost` (nega toda chamada) e `execute_with_host`. `execute` continua sem host
   de efeitos e usa `NoHost`, de modo que qualquer `host_call` termina em
   `capability_denied`.
7. **Capabilities com escopo:** `Capabilities::none()`, `grant(effect, scope)` e
   `with_grant(effect, scope)`; `allows(effect, scope)` compara o escopo por
   **componentes de caminho** (`Path::starts_with`), não por prefixo textual.
   Caminhos com componente `..` são negados (sem canonicalização, `..` escaparia
   do escopo). Escopo vazio concede qualquer caminho sem `..`. A checagem é feita
   pelo intérprete **antes** de chamar o host, e o host revalida o escopo (defesa
   em profundidade).
8. **CLI:** `--allow-fs-read <dir>` concede `filesystem_read` restrito ao
   diretório; escopo vazio é erro de uso. Sem a flag, o host do CLI não concede
   nada. A leitura é limitada a 1 MiB e a quota de memória do intérprete cobra a
   string resultante (ADR-0035).
9. **Erros do efeito:** falha de IO, arquivo acima do limite ou conteúdo não-UTF-8
   viram `Err(string)` no `result` do convidado, não trap. Negação de capability é
   erro de execução (`capability_denied`, classe `capability`).
10. **Default deny preservado:** `HostApi::standalone()` não concede nada, o
    adaptador Lift não emite `HostCall` e nenhum efeito é implícito.

## Alternativas

1. **Callback/FFI genérica:** rejeitada; exigiria `unsafe`/ponteiros, proibidos
   pelo workspace.
2. **Handle de arquivo aberto:** adiada; exigiria tipo opaco, fechamento
   determinístico, limite de handles e semântica de erro própria.
3. **Escopo por `starts_with` textual:** rejeitada; `/tmp/a` casaria `/tmp/abc`.
   A comparação usa componentes de caminho.
4. **Sem `max_bytes`:** rejeitada; abriria leitura ilimitada e negaria a quota por
   efeito.
5. **Conceder tudo por padrão no CLI:** rejeitada; mantém deny-by-default e exige
   opt-in explícito.

## Consequências

- **Positivas:** primeiro efeito de host exercitável; escopo e quota testados;
  `execute` sem host permanece puro; ABI pequena, fechada e determinística;
  negação antes do backend.
- **Negativas:** ainda não há escrita, rede, ambiente, relógio ou processo; não há
  handle nem streaming; a leitura é bloqueante; a comparação de escopo não
  canonicaliza symlinks, então um link dentro do escopo pode apontar para fora
  (`..` é negado, mas symlinks não); o host confia no caminho textual recebido.
- **Residual:** symlinks/TOCTOU (mitigados no ADR-0037 pela canonicalização no
  host, não eliminados), autorização por usuário, isolamento de SO, limites de
  tempo, auditoria de arquivos lidos. A comparação de escopo é sensível a caixa
  em componentes normais (no Windows, `C:\Temp` não cobre `c:\temp`). A leitura
  era feita **antes** da cobrança; o ADR-0038 passou a reservar o pior caso antes
  do IO. **Não é sandbox.**

## Validação

- `crates/rvm-runtime` (5 testes): concessão cobre escopo/subcaminhos, não casa
  prefixo textual de componente, nega `..`, escopo vazio concede qualquer caminho
  e `HostApi::request` aceita o escopo concedido.
- `crates/rvm-interpreter/tests/host.rs` (5 testes): negação antes do backend,
  concessão com escopo, escopo fora da concessão, `execute` sem host e aridade
  inválida na verificação.
- `crates/rvm-cli/tests/cli.rs` (6 testes): sem `--allow-fs-read` ⇒ saída 5;
  dentro do escopo ⇒ lê; fora do escopo ⇒ saída 5; `..` para fora do escopo ⇒
  saída 5; escopo vazio ⇒ erro de uso; round-trip `encode`/`run-bc` com
  `host_read_file`.
- `crates/rvm-text`: round-trip `parse → print → parse` do `host_read_file`.
- `crates/rvm-bytecode`: o fixture `testing::host_read_file` entra no round-trip
  `encode → decode` existente.
- Gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets --
  -D warnings` e `cargo fmt --all -- --check` verdes.
