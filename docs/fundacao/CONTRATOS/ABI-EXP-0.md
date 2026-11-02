# Contrato de chamadas entre módulos — ABI EXP-0

**Estado:** candidato para chamadas estáticas por valor dentro do mesmo processo. Não oferece FFI nativa.

## Fronteira mínima

- Símbolos resolvidos estaticamente antes da execução.
- Assinatura contém quantidade e tipos ordenados de argumentos e retorno.
- Tipos iniciais: unit, bool e i64.
- Valores passam pelo domínio da VM, sem ponteiro Rust exposto ao módulo.
- Trap encerra execução atual; exceção recuperável ainda não existe.
- Depth e fuel atravessam chamadas e não podem ser resetados pelo callee.
- Nenhum import de host no MVP puro.

## Tipos compostos e efeitos futuros

Strings, arrays, objetos, handles, callbacks, I/O e capabilities exigem contrato adicional. Definir identidade, ownership (borrow/move/copy), lifetime, erro, quota, representação e comportamento entre versões/linguagens. Nunca mapear ponteiros nativos diretamente.

## Propriedades

- Caller e callee concordam sobre assinatura antes da chamada.
- Tipo/arity errados são rejeitados antes da execução.
- Valores não acessam estado de outro runtime sem representação aprovada.
- Quotas globais persistem entre módulos.
- Comportamento independe do nome da linguagem de origem.
