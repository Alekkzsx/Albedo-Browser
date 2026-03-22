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

### 2.2. Formular perguntas para estruturar o plano
O agente deve elaborar perguntas que cubram:

- **Escopo:** O que está incluído? O que está excluído?
- **Critérios de sucesso:** Como saberemos que a tarefa foi concluída com sucesso?
- **Restrições:** Existem limitações de tempo, recursos, tecnologia, performance, segurança?
- **Dependências:** A tarefa depende de algo externo (APIs, pessoas, ambientes)?
- **Prioridades:** Há partes mais críticas que devem ser entregues primeiro?
- **Aceitação:** Quem validará o resultado? Existe um processo de revisão?

As perguntas devem ser específicas, não genéricas. Exemplos:
- “A nova funcionalidade deve ser visível apenas para administradores ou para todos os usuários?”
- “Qual o tempo máximo de resposta aceitável para essa consulta?”
- “Precisamos manter compatibilidade com versões anteriores da API?”

### 2.3. Construir o plano completo
Com base nas respostas, o agente deve produzir um documento de planejamento contendo:

- **Objetivo claro** (parágrafo conciso).
- **Subtarefas detalhadas** (seguindo o princípio **divide-action**), cada uma com:
  - Descrição da atividade.
  - Critérios de conclusão.
  - Dependências entre elas.
- **Estratégia de validação:** Como será testado (aspectos funcional, técnico, sistêmico).
- **Riscos e contingências:** Possíveis problemas e como mitigá-los.
- **Estimativa** (opcional, mas recomendada) de esforço ou tempo.

### 2.4. Submeter o plano para aprovação
- O plano deve ser apresentado ao usuário de forma clara e estruturada.
- Solicitar confirmação ou ajustes antes de iniciar a execução.
- Somente após a aprovação explícita o agente avançará para a fase de implementação (que seguirá os demais workspaces: **divide-action**, **make-full**, **review-full**).

## 3. Comportamento durante o planejamento

- **Iteração:** O agente pode fazer perguntas em sequência, refinando o plano conforme as respostas.
- **Transparência:** O plano deve ser suficientemente detalhado para que o usuário compreenda exatamente o que será feito.
- **Base técnica:** O planejamento deve estar fundamentado na análise prévia do projeto (workspace **understand-first**), respeitando arquitetura, padrões e convenções.

## 4. Integração com outros workspaces

- **Understand-First:** Fornece o contexto técnico necessário para um planejamento preciso.
- **Divide-Action:** A estrutura de subtarefas é aplicada dentro do plano.
- **Make-Full:** O plano já prevê entregas completas, sem pendências.
- **Review-Full:** O plano inclui a estratégia de validação que será executada ao final.

## 5. Exceções e ajustes

- **Tarefas triviais:** Se a tarefa for extremamente simples (ex.: corrigir um typo em um texto), o plano pode ser resumido a uma única etapa, mas ainda assim deve ser apresentado e aprovado.
- **Planejamento em andamento:** Se durante a execução surgir necessidade de desvio do plano, o agente deve interromper, comunicar e, se necessário, revisar o plano com novas perguntas.