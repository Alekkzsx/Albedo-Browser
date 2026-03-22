---
trigger: always_on
---

## 1. Princípio: Nada é entregue sem validação
Antes de finalizar e entregar qualquer tarefa, o agente **deve** testar o código produzido para garantir que não contém erros e atende aos critérios definidos. A validação será conduzida em **três aspectos complementares**. Somente após a aprovação em todos eles a tarefa será considerada concluída.

## 2. Os três aspectos da validação

### 2.1. Aspecto 1 – Correção funcional
- **Objetivo:** Verificar se o código implementa exatamente o que foi solicitado, sem desvios de escopo.
- **Atividades:**
  - Executar testes automatizados (unitários, integração) existentes e novos criados para a tarefa.
  - Realizar testes manuais ou exploratórios nos fluxos principais e bordas (edge cases).
  - Validar os critérios de aceite documentados na tarefa.
- **Critério de aprovação:** Todos os testes passam; comportamento está de acordo com a especificação.

### 2.2. Aspecto 2 – Qualidade técnica
- **Objetivo:** Garantir que o código é limpo, seguro e mantível.
- **Atividades:**
  - Executar ferramentas de análise estática (linters, formatadores) e corrigir quaisquer alertas.
  - Revisar possíveis vulnerabilidades de segurança (ex.: injeção, exposição de dados sensíveis).
  - Verificar aderência às convenções de código do projeto.
  - Avaliar complexidade ciclomática e boas práticas de design.
- **Critério de aprovação:** Zero alertas críticos; código segue os padrões estabelecidos.

### 2.3. Aspecto 3 – Impacto sistêmico
- **Objetivo:** Assegurar que a alteração não introduz regressões e se integra corretamente ao restante do sistema.
- **Atividades:**
  - Executar testes de regressão (suíte completa ou focada nas áreas afetadas).
  - Verificar logs e rastreabilidade para confirmar que não houve efeitos colaterais.
  - Avaliar impacto em performance (tempo de resposta, uso de recursos) e escalabilidade, quando aplicável.
  - Se houver dependências externas (APIs, banco de dados), confirmar que a integração permanece íntegra.
- **Critério de aprovação:** Nenhuma regressão identificada; sistema se comporta conforme esperado.

## 3. Processo de revisão

### 3.1. Execução dos testes
- O agente deve executar as verificações de cada aspecto de forma sistemática, documentando os resultados.
- Em caso de falha em qualquer aspecto, o agente deve:
  - Corrigir os problemas identificados.
  - Reexecutar os testes (ciclo até aprovação).

### 3.2. Documentação da revisão
Após a conclusão bem-sucedida, o agente deve gerar um relatório sucinto contendo:
- **Resultado por aspecto:** status (aprovado/reprovado) e evidências (ex.: testes passaram, linter ok).
- **Observações relevantes:** pontos de atenção futuros, melhorias sugeridas.
- **Confirmação de finalização:** tarefa marcada como concluída.

## 4. Exceções – quando não é possível testar integralmente
Se, por restrições técnicas ou contextuais, não for possível executar um ou mais aspectos (ex.: ambiente de teste indisponível, falta de testes automatizados, impossibilidade de testar regressão), o agente **deve**:

1. **Explicitar claramente ao usuário** quais aspectos não puderam ser validados e o motivo.
2. **Propor alternativas** (ex.: testes manuais guiados, análise de impacto teórica, verificação em ambiente de homologação posterior).
3. **Solicitar orientação** sobre como prosseguir, antes de considerar a tarefa finalizada.

Nesses casos, a tarefa não será marcada como “finalizada” sem a anuência explícita do usuário.