---
trigger: always_on
---

## 1. Princípio: Dividir para conquistar
Toda tarefa recebida, seja simples ou complexa, **deve** ser decomposta em subtarefas menores e atômicas. A divisão maximiza a clareza, facilita a validação, reduz riscos e permite entregas incrementais. Nenhuma tarefa será executada como um bloco monolítico sem antes ser quebrada em partes gerenciáveis.

## 2. Processo de decomposição

### 2.1. Identificar a natureza da tarefa
- Analisar o escopo e os requisitos da tarefa.
- Classificar o tipo: nova funcionalidade, correção de bug, refatoração, ajuste de configuração, documentação, etc.

### 2.2. Quebrar em subtarefas lógicas
- **Critério de atomicidade:** Cada subtarefa deve representar uma unidade de trabalho com um objetivo claro e entregável.
- **Nível de granularidade:** As subtarefas devem ser pequenas o suficiente para serem concluídas em poucas interações (idealmente menos de 1 hora de trabalho), mas sem se tornarem triviais ou fragmentadas demais.
- **Dependências:** Identificar relações de precedência (subtasks que precisam ser concluídas antes de outras).
- **Exemplos de divisão:**
  - *Funcionalidade:* Criar rota → Implementar controller → Escrever testes → Atualizar documentação.
  - *Correção:* Reproduzir o erro → Identificar causa → Implementar correção → Adicionar teste de regressão.
  - *Refatoração:* Extrair função → Atualizar referências → Executar testes → Limpar código morto.

### 2.3. Decomposição por pipeline (específico AlbedoBrowser)
Para features que cruzam a pipeline do browser, decompor por camada:
- **Network:** Requisição, resposta, cache.
- **Parser:** Tokenização, construção de árvore.
- **DOM:** Nós, atributos, manipulação.
- **CSS:** Parsing, cascata, estilo computado.
- **Layout:** Box model, posicionamento.
- **Rendering:** Rasterização, composição.
- **JS/JIT:** Bindings, compilação, execução.

Cada camada é uma subtarefa natural — devem ser implementadas e testadas independentemente.

### 2.4. Estruturar a lista de subtarefas
- Criar uma lista ordenada ou priorizada.
- Para cada subtarefa, definir:
  - **Objetivo** (o que será feito)
  - **Critérios de conclusão** (como saber que está pronto)
  - **Dependências** (se houver)
  - **Verificação** (qual teste ou check confirma conclusão)
  - **Estimativa relativa** (opcional)

### 2.5. Validação da divisão
- Verificar se a soma das subtarefas cobre integralmente o escopo da tarefa original.
- Confirmar que não há lacunas ou sobreposições desnecessárias.
- Aplicar princípio de **surgical-precision**: cada subtarefa toca o mínimo de módulos.
- Se necessário, ajustar a granularidade (dividir mais ou agrupar) para manter equilíbrio.

## 3. Comportamento durante a execução

### 3.1. Execução incremental
- As subtarefas devem ser executadas em sequência, respeitando dependências.
- Após concluir cada subtarefa, o agente deve:
  - Verificar os critérios de conclusão.
  - Executar o ciclo de verificação do **smart-automation** (`cargo check` → `clippy` → `test`).
  - Comunicar o progresso ao usuário (se apropriado).

### 3.2. Adaptação dinâmica
- Se durante a execução uma subtarefa se mostrar muito grande ou complexa, deve ser novamente dividida.
- Se novas dependências surgirem, a lista de subtarefas deve ser reordenada.
- Se uma subtarefa falha repetidamente, aplicar o protocolo de **self-correction**.

## 4. Anti-padrões de decomposição (PROIBIDOS)
- ❌ **Bloco monolítico:** Executar tudo de uma vez sem dividir.
- ❌ **Fragmentação excessiva:** Criar 20 subtarefas para algo que deveria ser 5.
- ❌ **Dependências circulares:** Subtarefa A depende de B que depende de A.
- ❌ **Subtarefas vagas:** "Implementar a feature" sem detalhes.
- ❌ **Pular verificação intermediária:** Concluir 5 subtarefas sem verificar nenhuma.

## 5. Exceções e considerações
- Em casos excepcionais onde a tarefa é extremamente simples (ex.: alterar uma linha de configuração), a divisão pode ser mínima (uma única subtarefa), mas o princípio de clareza ainda se aplica: documentar o que será feito.
- Se o usuário solicitar explicitamente que não haja divisão, o agente deve acatar, mas pode sugerir uma divisão para garantir robustez.

## 6. Integração com outros workspaces
- **understand-first:** A análise inicial do projeto fornece contexto para uma divisão adequada.
- **understand-task:** O refinamento da tarefa ajuda a esclarecer escopo e critérios, facilitando a decomposição.
- **surgical-precision:** Cada subtarefa deve tocar o mínimo de módulos possível.
- **smart-automation:** O ciclo de verificação é executado após cada subtarefa.
- **self-correction:** Subtarefas que falham repetidamente ativam o protocolo de correção.
- **review-full:** A validação final será aplicada ao conjunto completo das subtarefas.
