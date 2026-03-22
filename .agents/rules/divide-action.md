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

### 2.3. Estruturar a lista de subtarefas
- Criar uma lista ordenada ou priorizada.
- Para cada subtarefa, definir:
  - **Objetivo** (o que será feito)
  - **Critérios de conclusão** (como saber que está pronto)
  - **Dependências** (se houver)
  - **Estimativa relativa** (opcional)

### 2.4. Validação da divisão
- Verificar se a soma das subtarefas cobre integralmente o escopo da tarefa original.
- Confirmar que não há lacunas ou sobreposições desnecessárias.
- Se necessário, ajustar a granularidade (dividir mais ou agrupar) para manter equilíbrio.

## 3. Comportamento durante a execução

### 3.1. Execução incremental
- As subtarefas devem ser executadas em sequência, respeitando dependências.
- Após concluir cada subtarefa, o agente deve:
  - Verificar os critérios de conclusão.
  - Executar verificações rápidas (ex.: testes relacionados) para garantir estabilidade.
  - Comunicar o progresso ao usuário (se apropriado).

### 3.2. Adaptação dinâmica
- Se durante a execução uma subtarefa se mostrar muito grande ou complexa, deve ser novamente dividida.
- Se novas dependências surgirem, a lista de subtarefas deve ser reordenada.

## 4. Integração com outros workspaces
- **Understand-First:** A análise inicial do projeto fornece contexto para uma divisão adequada.
- **Understand-Task:** O refinamento da tarefa ajuda a esclarecer escopo e critérios, facilitando a decomposição.
- **Review-Full:** A validação final será aplicada ao conjunto completo das subtarefas (ou incrementalmente, conforme definido).

## 5. Exceções e considerações
- Em casos excepcionais onde a tarefa é extremamente simples (ex.: alterar uma linha de configuração), a divisão pode ser mínima (uma única subtarefa), mas o princípio de clareza ainda se aplica: documentar o que será feito.
- Se o usuário solicitar explicitamente que não haja divisão, o agente deve acatar, mas pode sugerir uma divisão para garantir robustez.
