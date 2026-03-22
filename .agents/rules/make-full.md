---
trigger: always_on
---

## 1. Princípio: Entregar sempre 100% completo
Toda tarefa assumida deve ser executada **de forma integral**, atendendo a todos os requisitos, critérios de aceite e expectativas. Nada de soluções parciais, “esqueletos” ou entregas que deixem pendências para depois. O objetivo é que, ao final, a tarefa esteja pronta para ser considerada concluída sem necessidade de retrabalho imediato.

## 2. O que significa “100% completo”
Uma tarefa é considerada completa quando:

- **Requisitos atendidos:** Todos os objetivos funcionais e não funcionais definidos na tarefa (e nas subtarefas) foram implementados.
- **Testes realizados:** A tarefa passou pela validação completa do workspace **review-full** (três aspectos: funcional, qualidade técnica, impacto sistêmico).
- **Documentação atualizada:** Qualquer documentação afetada (código, README, comentários, API docs) foi ajustada para refletir as mudanças.
- **Sem pendências:** Não existem `TODO`, `FIXME`, comentários de código não finalizado, ou funcionalidades pela metade.
- **Integração verificada:** A alteração se integra corretamente ao restante do sistema, sem regressões.
- **Pronto para entrega:** A solução pode ser disponibilizada (commit, PR, deploy) sem necessidade de intervenção manual adicional.

## 3. Processo para garantir completude

### 3.1. Antes de iniciar
- Revisar a tarefa e suas subtarefas (via **divide-action**) para confirmar que o escopo completo está coberto.
- Identificar critérios de aceite e condições de “pronto” (definition of done).

### 3.2. Durante a execução
- Trabalhar nas subtarefas de forma incremental, mas sempre as concluindo integralmente antes de avançar para a próxima.
- A cada subtarefa finalizada, verificar se os critérios de conclusão foram atingidos.
- Nunca deixar “marcadores” de trabalho futuro sem que sejam explicitamente tratados como parte da tarefa atual.

### 3.3. Antes de finalizar
- Executar o processo **review-full** para validar os três aspectos.
- Verificar a lista de subtarefas: todas devem estar marcadas como concluídas.
- Realizar uma auto-inspeção para garantir que nada foi esquecido (ex.: mensagens de log, tratamento de erros, validações de entrada).

### 3.4. Comunicação da conclusão
- Informar ao usuário que a tarefa está 100% completa, anexando evidências (ex.: testes passando, documentação atualizada).
- Se houver qualquer impedimento que impeça a completude, comunicar imediatamente e propor alternativas.

## 4. Comportamento diante de impedimentos
- **Bloqueios externos:** Se uma dependência (ex.: API externa, decisão do usuário) impedir a conclusão, o agente deve:
  - Interromper o trabalho.
  - Explicar claramente o que falta e o que é necessário para desbloquear.
  - Aguardar orientação antes de prosseguir.
- **Complexidade inesperada:** Se a tarefa se revelar maior que o previsto, o agente deve:
  - Revisar a divisão em subtarefas.
  - Comunicar o aumento de escopo e solicitar confirmação.
  - Nunca entregar uma solução incompleta sob o argumento de “já fiz a parte principal”.

## 5. Integração com outros workspaces
- **Divide-Action:** Fornece a decomposição que permite acompanhar a completude por partes.
- **Review-Full:** Garante que a validação de qualidade e impacto seja realizada antes de considerar a tarefa finalizada.
- **Understand-Task / Understand-First:** Fornecem o contexto necessário para saber o que é considerado “completo”.

## 6. Exceções justificadas
- Em situações extremas, onde o ambiente ou ferramentas não permitem a conclusão plena (ex.: falta de acesso a banco de dados de teste), o agente deve:
  - Listar explicitamente os itens que não puderam ser concluídos.
  - Explicar o motivo.
  - Propor um plano para conclusão posterior.
  - Obter consentimento do usuário antes de entregar algo incompleto.
