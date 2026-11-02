# Capabilities — EXP-0 (deny-by-default, com ABI mínima de host)

**Estado:** proposta implementada de forma restritiva. ADR-0005 é a decisão de
origem e ADR-0036 define a primeira ABI de host exercitável. Este contrato
descreve o que o código atual garante e o que ele não garante.

## 1. Regra

Nenhuma capability é implícita. Arquivos, rede, ambiente, relógio e subprocessos
continuam negados por padrão. A única função de host do EXP-0 é `read_file`,
gated por `filesystem_read`, e só executa quando o **host** concede a capability
para um escopo. `execute` (sem host) nega toda chamada com `capability_denied`.

## 2. ABI de host (ADR-0036)

A ABI é **fechada** nesta rodada:

| Função | Assinatura | Efeito | Escopo | Bytecode |
|---|---|---|---|---:|
| `read_file` | `(path: string, max_bytes: i64) -> result string string` | `filesystem_read` | argumento 0 (`path`) | `0` |

- `path` é o caminho textual; `max_bytes` é o limite de bytes que o convidado
  aceita ler (o host pode impor teto menor).
- Sucesso devolve `ok(conteúdo)`; falha de IO, arquivo acima do limite ou conteúdo
  não-UTF-8 devolve `err(mensagem)`. Nenhum desses casos é trap.
- O tipo de retorno (`result string string`) precisa existir na tabela de tipos do
  módulo.
- A aridade e os tipos dos argumentos são verificados; divergência ⇒
  `invalid_type`.

## 3. API

- `rvm_spec::Effect` enumera `filesystem_read`, `filesystem_write`, `network`,
  `environment`, `clock` e `process`. `rvm_spec::BuiltinHost` contém apenas
  `ReadFile` e declara `effect()`, `code()` e `scope_arg()`.
- `rvm_runtime::Capabilities` tem campo privado e construtores explícitos:
  `none()`, `grant(effect, scope)` e `with_grant(effect, scope)`.
  `allows(effect, scope)` compara o escopo por **componentes de caminho**
  (`Path::starts_with`); caminhos com componente `..` são negados; escopo vazio
  concede qualquer caminho sem `..`.
- `rvm_runtime::Host` é o backend de efeitos: `capabilities()` e
  `call(function, args, ret)`. `NoHost` nega toda chamada.
- `rvm_interpreter::execute_with_host(module, entry, args, config, host)` consulta
  o host; `execute` usa `NoHost`. O intérprete confere a capability **antes** de
  chamar o backend e valida o tipo do valor devolvido.
- `rvm_runtime::HostApi::request(effect, scope)` devolve `Ok(())` somente se
  `Capabilities::allows` for verdadeiro e `Err(capability_denied)` caso contrário.
- `rvm-cli` aceita `--allow-fs-read <dir>` em `run`/`run-bc`; sem a flag, o host do
  CLI não concede nada. A leitura é limitada a 1 MiB.

## 4. Obrigações de teste

1. Um teste por efeito (`Effect`), verificando `capability_denied` e
   `ErrorClass::Capability` quando não há concessão (em
   `crates/rvm-runtime/src/lib.rs`).
2. `Capabilities::none().granted()` vazio e `is_granted`/`allows` falsos para todos
   os efeitos.
3. Testes de negação **antes do backend**: o host não pode ser chamado sem
   concessão (`crates/rvm-interpreter/tests/host.rs`).
4. Testes de escopo: caminho dentro da concessão executa; fora, é negado.
5. Testes de CLI: sem `--allow-fs-read` ⇒ saída 5; dentro do diretório ⇒ lê; fora
   ⇒ saída 5; round-trip `encode`/`run-bc` com `host_read_file`.
6. Nenhum teste pode depender de rede, ambiente, relógio ou subprocesso; os testes
   de arquivo usam diretório temporário próprio.

## 5. O que esta regra **não** prova

- Isolamento de processo/OS: quotas e verificação são in-process.
- Ausência de DoS: fuel e profundidade limitam trabalho; a leitura é limitada por
  `max_bytes`/1 MiB, mas não há limite de tempo nem de taxa. A string é lida antes
  de ser cobrada na quota de memória; a quota limita a retenção, não o IO.
- Segurança do adaptador de arquivo: a comparação de escopo não canonicaliza
  symlinks, não resolve `..` para um diretório real (caminhos com `..` são
  negados) e está sujeita a TOCTOU entre a checagem e a leitura. Um link dentro do
  escopo pode apontar para fora. A comparação é sensível a caixa em componentes
  normais (no Windows, `C:\Temp` não cobre `c:\temp`).
- Confidencialidade do conteúdo lido: o convidado recebe os bytes; não há
  filtragem, redação nem auditoria.
- Que um bug no verificador não permita efeitos: a ABI é pequena, mas qualquer
  extensão reabre a análise.

## 6. Extensão futura (fora do EXP-0)

Adicionar escrita, rede, ambiente, relógio, processo ou handle de arquivo exigirá:
ADR com justificativa de ameaça, tipo de handle sem ponteiro nativo, escopo e
quotas por efeito, erro específico, testes de negação e de abuso, além de decisão
explícita do mantenedor. Nada disso existe hoje e nada deve ser inferido a partir
deste documento.
