# Bytecode experimental — EXP-0

**Estado:** pesquisa aberta. Nenhuma codificação binária ou ABI de arquivo está congelada.

## Objetivo

Medir benefício/custo de baixar RIR validada para instruções lineares e verificar novamente no carregamento. O executor pode interpretar RIR diretamente; bytecode não é requisito para demonstrar semântica.

## Alternativas

- bytecode de registradores com IDs SSA;
- bytecode de pilha;
- representação linear próxima da RIR com branches explícitos.

Comparar simplicidade do verificador, tamanho, decode, desempenho, depuração/source maps e risco em índices/offsets. Não escolher por preferência estética.

## Invariantes para qualquer formato

1. Decoder recebe limites explícitos de bytes e contagens.
2. Tamanhos e offsets são validados antes de alocar ou saltar.
3. Opcode desconhecido, versão incompatível e seção duplicada são rejeitados.
4. Módulo é integralmente decodificado/verificado antes da execução.
5. Nenhum código nativo, ponteiro ou endereço de processo fica no módulo.
6. Metadado de tipo do emissor não é confiado sem conferência.
7. Entrada inválida não causa panic.

## Cabeçalho candidato

Se formato binário for prototipado, pode-se experimentar magic bytes, versão, tamanho declarado e seções com comprimento. Isso não aprova nomes, ordem, endianidade, hashes, assinatura nem compatibilidade. ADR-0004 governa a decisão.

## Testes planejados

Gerar RIR→bytecode, decodificar→verificar→executar e comparar com intérprete RIR. Mutar bytes/índices/lengths; testar truncamento, overflow, referências fora de faixa, op desconhecido e quotas. Registrar corpus e seed do fuzzing.
