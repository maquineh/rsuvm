# Contrato experimental da RIR — EXP-0

**Estado:** proposta para protótipo; não é formato estável.  
**Objetivo:** expressar subset de funções puras de forma verificável sem dependência da linguagem de origem.

## 1. Modelo

- **Módulo:** identificador, versão experimental, imports explícitos e funções.
- **Função:** identificador, parâmetros tipados, tipo de retorno, bloco de entrada e blocos.
- **Bloco:** identificador, instruções e exatamente um terminador.
- **Valor:** ID definido uma vez; parâmetros são valores disponíveis na entrada.
- **Instrução:** operação tipada com argumentos já definidos.
- **Terminador:** return, branch ou conditional_branch.

O modelo se inspira em SSA para simplificar verificação; SSA como formato definitivo está aberto. Merge pode ser modelado por argumentos de bloco nesta experiência.

## 2. Tipos iniciais

unit, bool e i64. Sem coerção implícita. string, f64, arrays, registros, referências, closures e tipos estruturais são futuros até definição própria.

## 3. Operações experimentais

- const_i64 e const_bool
- add_i64_checked, sub_i64_checked, mul_i64_checked
- comparações assinadas e igualdade definidas no contrato da operação
- call_static
- terminadores return, br e cond_br

Overflow em soma/subtração/multiplicação resulta em trap integer_overflow. Não depender de comportamento debug/release do Rust. Division, shifts, casts e ponto flutuante ficam fora do corpus inicial.

## 4. Exemplo ilustrativo, não serialização normativa

~~~text
module exemplo
fn soma(a: i64, b: i64) -> i64 {
  entry:
    v0 = add_i64_checked a, b
    return v0
}
~~~

É notação legível. Não define gramática, JSON, binário ou API Rust.

## 5. Invariantes de verificação

- IDs de módulo/função/bloco/valor não duplicados no escopo aplicável.
- Tipo e aridade de cada operação/chamada correspondem à assinatura.
- Alvos de branches existem; condição de cond_br é bool.
- Blocos terminam em exatamente um terminador.
- Valores são definidos uma vez e disponíveis antes do uso, respeitando dominância/escopo.
- Retorno coincide com assinatura.
- Imports e entry point são explícitos; sem resolução dinâmica.
- Tamanho e contagens são limitados durante decode/verificação.
- Operação/tipo/versão desconhecidos falham de modo fechado.

## 6. Semântica inicial

Execução determinística no subset puro. Cada instrução/terminador despachado debita uma unidade de fuel conforme regra preliminar; saldo insuficiente gera out_of_fuel antes do despacho. Chamada aumenta depth e retorna ao chamador. Erros do verificador nunca são traps semânticos.

## 7. Categorias iniciais

invalid_module, invalid_type, invalid_control_flow, integer_overflow, out_of_fuel, call_depth_exceeded, unsupported_feature. Casos de conformidade vinculam categoria à condição.

## 8. Fora do EXP-0

Formato wire, compatibilidade semântica duradoura, heap geral, GC, exceptions recuperáveis, corrotinas, async, threads, atores e imports com efeitos.
