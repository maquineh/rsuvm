# ADR-0038 — Reserva de quota antes do IO de host

- **Status:** Aceito experimentalmente
- **Data:** 2026-11-02
- **Requisitos:** lacuna registrada no ADR-0036/0037 ("a string é lida antes de
  ser cobrada"); pedido do mantenedor para implementar as lacunas.

## Contexto

No ADR-0036, `host_read_file` podia ler até `max_bytes` (limitado a 1 MiB pelo
CLI) e só depois o intérprete cobrava os bytes retidos ao gravar o slot. Com
`memory` pequeno e `max_bytes` grande, o host realizava o IO e a execução só
trapava `out_of_memory` **depois** da leitura. A quota limitava a retenção, não o
IO.

Também não havia um teto explícito na ABI: um host poderia devolver mais que o
`max_bytes` pedido sem violar um contrato escrito.

## Decisão

1. **Teto da ABI:** `rvm_spec::MAX_HOST_READ_BYTES = 1 MiB`. Toda função de host
   deve respeitar `BuiltinHost::max_effect_bytes()`; para `read_file`, o conteúdo
   devolvido não pode exceder `min(max_bytes, 1 MiB)`.
2. **Reserva antes do IO:** depois de conferir a capability e antes de chamar o
   host, o intérprete reserva o pior caso do resultado:
   `SLOT_BYTES + min(max_bytes, 1 MiB)`. Se a reserva não couber na quota viva, a
   execução falha com `out_of_memory` e o host **não é chamado**.
3. **Reserva é checagem, não cobrança.** A cobrança definitiva continua sendo os
   bytes retidos do valor gravado (`Value::retained_bytes`, ADR-0035). A reserva
   apenas garante que o pior caso cabe; não é somada duas vezes.
4. **Ordem determinística:** capability → reserva de quota → chamada do host. Uma
   capability negada vence `out_of_memory`; a reserva vence o IO.
5. **CLI:** usa o mesmo teto e o mesmo `max_bytes`; o teste de memória baixa
   confirma a saída 4 sem abrir o arquivo.
6. **Host é confiável para o teto.** Se um host violar `max_bytes`, o
   `write_current` ainda pode trapacar depois do IO; o contrato da ABI declara a
   violação como bug do host, não como comportamento do convidado.

## Alternativas

1. **Sem reserva:** rejeitada; mantém IO antes da falha de quota.
2. **Cobrar a reserva e liberar depois:** funcionaria, mas duplicaria a
   contabilidade e complicaria a liberação em erro; a checagem é suficiente.
3. **API de pré-voo no `Host` (`effect_cost`):** adiada; exigiria mais um método
   no trait e uma chamada antes de cada efeito, sem ganho sobre o teto fixo.
4. **Streaming/handle:** adiado com o ADR-0037; `std` não oferece `openat`
   portável e o handle exigiria um tipo novo na ABI.

## Consequências

- **Positivas:** o host não é chamado quando o pior caso não cabe; IO fica
  subordinado à quota; o teto de 1 MiB é único e explícito; a ordem dos erros é
  determinística.
- **Negativas:** a reserva é conservadora — pedir `max_bytes` grande com quota
  pequena falha mesmo que o arquivo real seja minúsculo; o convidado precisa
  pedir um limite compatível. A reserva cobre o wrapper `result` e o conteúdo,
  mas não eventuais custos internos do backend.
- **Residual:** não há handle/`openat` (ADR-0037), TOCTOU, auditoria de IO ou
  limite de tempo/taxa. **Não é sandbox.**

## Validação

- `crates/rvm-interpreter/tests/host.rs`: reserva de 1 MiB em quota de 500 bytes
  falha com `out_of_memory` e `host.calls == 0`; `max_bytes = i64::MAX` é
  limitado ao teto e executa.
- `crates/rvm-cli/tests/cli.rs`: `--memory 2000` com `max_bytes = 1 MiB` sai com
  4 (`out_of_memory`); teste de junção no Windows e symlink no Unix confirmam o
  escape negado.
- `C-062` continua provando a negação por capability antes da reserva.
- Gates e CI Linux (debug/release) verdes.
