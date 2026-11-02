# Programa de pesquisa e hipóteses falsificáveis

**Documento:** RVM-RP-001  
**Horizonte de planejamento:** 24 semanas, estimativa e não compromisso  
**Estado:** proposta; experimentos ainda não executados.

## 1. Pergunta central

É possível representar e executar um subset comum de pelo menos duas linguagens com tipos e regras de memória diferentes em runtime compartilhado, mantendo semântica explícita, módulos verificáveis, capacidades controladas e custo de fronteira mensurável?

Sucesso exige evidência de correção, limites de segurança no modelo declarado e viabilidade para workloads escolhidos. Um protótipo funcional isolado não responde à pergunta.

## 2. Princípios de método

- Pré-registre workloads, baseline, métrica, limiar e método antes de observar resultados usados para decidir.
- Use especificação independente da implementação como oráculo; sem ela, diferencial é inconclusivo.
- Separe compilação, inicialização, execução, chamada interna, chamada entre módulos e efeitos do host.
- Publique resultados negativos, distribuições/intervalos e limites do experimento.
- Não use microbenchmarks para afirmar desempenho de aplicação inteira.
- Registre commit, ferramenta, compilador, flags, dependências, sistema, CPU, memória, comando e dados brutos.
- Zero falhas observadas em fuzzing não significa ausência de vulnerabilidade.

## 3. Hipóteses

Limiar preliminar deve ser congelado antes da rodada relevante. Não é resultado nem compromisso.

| ID | Hipótese falsificável | Experimento e medição | Critério preliminar |
|---|---|---|---|
| H01 | Um núcleo RIR expressa subset formal comum de Rio e Lift sem regra especial do runtime por linguagem | Corpus de funções, bool, i64, chamadas e fluxo; medir cobertura e casos sem mapeamento | Corroborada se ambos gerarem os mesmos programas do corpus com semântica definida; refutada se RIR precisar codificar linguagem ou houver divergência insolúvel |
| H02 | Verificador rejeita classes de módulo inválido antes de qualquer execução | Corpus válido/inválido e instrumentação da entrada do executor | 100% do corpus classificado; um inválido executado refuta o gate |
| H03 | Chamada inter-módulo com primitivos evita serialização geral sem perder tipo/resultado | Comparar chamada local, inter-módulo e codec binário | Resultado equivalente; publicar overhead, alocações e latências |
| H04 | Transferência exclusiva impede alias mutável observável entre contextos | Protótipo futuro com identidade, aliases e mutação | Qualquer observação de dois mutadores simultâneos sem contrato refuta a hipótese |
| H05 | Capabilities explícitas negam efeitos não autorizados cobertos pelo corpus | Tentativas de filesystem, rede, env, subprocesso e relógio no host deny-by-default | Nenhum efeito sem capability nos cenários; documentar limites de processo/OS |
| H06 | Chamada entre módulos custa no máximo 25% a mais que chamada local equivalente no mesmo executor | Bench pareado, inicialmente 30 amostras/configuração, aquecimento e distribuição | Mediana ≤25% em configurações pré-registradas; acima disso refutada no escopo. Não comparar diretamente com nativo |
| H07 | Fuel e profundidade são aplicados determinísticamente em loop e recursão | Casos repetidos e fronteiras exatas | Mesmo limite/trap para mesma entrada; execução além da regra refuta mecanismo |
| H08 | Integração Lift ocorre sem alterar semântica ou quebrar backend padrão documentado | Auditoria de commit/API/testes/licença e protótipo de subset | Só corroborada com integração reproduzível e regressão; ponto não reutilizável refuta rota escolhida, não a RVM |

## 4. Metodologia

### Inventário antes do experimento

1. Fixar commit/versão e registrar hash.
2. Ler documentação, manifests, licença, política de segurança, CI e corpus.
3. Separar recursos declarados, cobertos por testes e executados localmente.
4. Definir subset sem ambiguidades, incluindo overflow, chamadas e erros.
5. Registrar critérios de exclusão e baseline.

### Execução e análise

- Builds release/dev separados quando pertinente.
- No mínimo 30 observações independentes por configuração como ponto inicial; aumentar conforme variância e poder estatístico.
- Randomizar/balancear ordem; separar aquecimento, cold start e execução.
- Reportar mediana, quantis, intervalo de confiança e tamanho do efeito, não só média.
- Medir latência, throughput, bytes alocados, pico de memória e fuel.
- Repetir em duas classes de hardware antes de alegar desempenho/portabilidade ampla.
- Armazenar saídas brutas e script de análise.

### Segurança

Corpus adversarial, testes de propriedade, fuzzing de parser/decoder/verifier, sanitizadores quando aplicáveis, revisão de toda fronteira unsafe futura, testes de resource exhaustion e revisão independente antes de alegações de segurança.

## 5. Workloads

- **W01 Chamadas:** local, inter-módulo, entre frontends; só tipos comuns definidos.
- **W02 Controle:** branch, loop limitado, chamada recursiva limitada.
- **W03 Compostos:** depois do contrato de ownership/codec; tamanhos e mutabilidade variados.
- **W04 Recursos:** quotas, entradas malformadas, módulos grandes, chamadas profundas.
- **W05 Caso aplicado:** programa representativo definido antes da medição; não escolher só caso favorável.

## 6. Resultados

Cada experimento recebe ID, hipótese/versionamento, commit, ambiente, comando, corpus, dados, análise e limitações. Classificação final: **corroborada no escopo testado**, **refutada** ou **inconclusiva**; evitar “comprovada”.

## 7. Primeiro experimento

**EXP-RVM-001 — Semântica e custo de chamadas entre módulos.** Começar com funções puras i64/bool; comparar chamada local e entre módulos; validar valores, erros e quotas. Não construir ABI geral antes de observar resultados. Lift/Rio entram no corpus somente após auditoria e subset explícito.
