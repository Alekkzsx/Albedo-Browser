---
trigger: always_on
---

# Workspace do Agente – Plan-Mode

## 1. Princípio: Planejar antes de agir
Antes de iniciar qualquer implementação, o agente deve ativar o **plan-mode**. Neste modo, toda ação é precedida por um planejamento explícito, construído a partir de perguntas direcionadas ao usuário. O objetivo é garantir que o trabalho seja realizado com clareza, alinhamento e sem surpresas.

## 2. Processo de planejamento

### 2.1. Entender o objetivo da tarefa
- Receber a tarefa ou solicitação do usuário.
- Identificar o que se deseja alcançar, incluindo expectativas implícitas.
- Aplicar o processo de **understand-task** para análise quadridimensional.

### 2.2. Formular perguntas para estruturar o plano
O agente deve elaborar perguntas que cubram:

- **Escopo:** O que está incluído? O que está excluído?
- **Critérios de sucesso:** Como saberemos que a tarefa foi concluída com sucesso?
- **Restrições:** Existem limitações de tempo, recursos, tecnologia, performance, segurança?
- **Dependências:** A tarefa depende de algo externo (APIs, pessoas, ambientes)?
- **Prioridades:** Há partes mais críticas que devem ser entregues primeiro?
- **Aceitação:** Quem validará o resultado? Existe um processo de revisão?

As perguntas devem ser específicas, não genéricas. Exemplos:
- "A nova funcionalidade deve ser visível apenas para administradores ou para todos os usuários?"
- "Qual o tempo máximo de resposta aceitável para essa consulta?"
- "Precisamos manter compatibilidade com versões anteriores da API?"

### 2.3. Analisar riscos e modos de falha
Antes de finalizar o plano, aplicar **failure-analysis** simplificada:
- Identificar os 3-5 principais riscos da implementação.
- Para cada risco: probabilidade, impacto e mitigação.
- Cenários "E se...?" para os paths mais críticos.
- Plano de contingência: o que fazer se a abordagem principal falhar.

### 2.4. Pesquisar referências técnicas
Para features que envolvem WebAPIs ou funcionalidades de browser:
- Citar a spec WHATWG/W3C relevante com link.
- Verificar como Servo/Chromium/Firefox implementam.
- Documentar o que será adaptado e por quê.

### 2.5. Construir o plano completo
Com base nas respostas, o agente deve produzir um `implementation_plan.md` contendo:

- **Objetivo claro** (parágrafo conciso).
- **Subtarefas detalhadas** (seguindo o princípio **divide-action**), cada uma com:
  - Descrição da atividade.
  - Critérios de conclusão.
  - Dependências entre elas.
- **Análise de riscos:** Top 3-5 riscos com mitigação.
- **Referências técnicas:** Specs e implementações consultadas.
- **Estratégia de validação:** Como será testado (aspectos funcional, técnico, sistêmico, segurança).
- **Estimativa** (opcional, mas recomendada) de esforço ou tempo.

### 2.6. Submeter o plano para aprovação
- O plano deve ser apresentado ao usuário de forma clara e estruturada.
- Solicitar confirmação ou ajustes antes de iniciar a execução.
- Somente após a aprovação explícita o agente avançará para a fase de implementação.

## 3. Comportamento durante o planejamento

- **Iteração:** O agente pode fazer perguntas em sequência, refinando o plano conforme as respostas.
- **Transparência:** O plano deve ser suficientemente detalhado para que o usuário compreenda exatamente o que será feito.
- **Base técnica:** O planejamento deve estar fundamentado na análise prévia do projeto (workspace **understand-first**), respeitando arquitetura, padrões e convenções.
- **Múltiplas perspectivas:** Considerar ao menos 2 abordagens antes de escolher (de **deep-reasoning**).

## 4. Anti-padrões de planejamento (PROIBIDOS)
- ❌ **Plano vago:** "Vou implementar a feature e testar." — sem detalhes.
- ❌ **Plano sem riscos:** Todo plano significativo tem riscos — omiti-los é negligência.
- ❌ **Plano sem testes:** A estratégia de validação é parte integrante do plano.
- ❌ **Implementar antes de aprovar:** Começar a codificar antes da aprovação do plano.
- ❌ **Plano engessado:** Não adaptar o plano quando novas informações surgem.

## 5. Exceções e ajustes

- **Tarefas triviais:** Se a tarefa for extremamente simples (ex.: corrigir um typo em um texto), o plano pode ser resumido a uma única etapa, mas ainda assim deve ser apresentado e aprovado.
- **Planejamento em andamento:** Se durante a execução surgir necessidade de desvio do plano, o agente deve interromper, comunicar e, se necessário, revisar o plano com novas perguntas.

## 6. Integração com outros workspaces
- **understand-first:** Fornece o contexto técnico necessário para um planejamento preciso.
- **understand-task:** A análise quadridimensional alimenta o plano.
- **divide-action:** A estrutura de subtarefas é aplicada dentro do plano.
- **deep-reasoning:** O plano é o produto do raciocínio profundo — múltiplas perspectivas obrigatórias.
- **failure-analysis:** A análise de riscos é integrada ao plano.
- **context-mastery:** Specs e referências são pesquisadas durante o planejamento.
- **make-full:** O plano já prevê entregas completas, sem pendências.
- **review-full:** O plano inclui a estratégia de validação que será executada ao final.