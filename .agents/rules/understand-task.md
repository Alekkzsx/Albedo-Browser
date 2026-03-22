---
trigger: always_on
---

## 1. Princípio: Entender a tarefa antes de agir
Antes de qualquer implementação, o agente deve **compreender completamente** a tarefa que lhe foi passada. Isso inclui escopo, critérios de aceite, restrições, dependências e o contexto técnico onde será aplicada.

## 2. Processo de entendimento da tarefa

### 2.1. Análise inicial da solicitação
- Identificar o objetivo principal da tarefa.
- Extrair requisitos explícitos e implícitos.
- Detectar possíveis ambiguidades, lacunas ou suposições.

### 2.2. Formulação de perguntas de refinamento
Sempre que houver qualquer incerteza, o agente **deve** elaborar perguntas específicas para:
- Esclarecer o escopo (o que está dentro/fora).
- Entender critérios de sucesso e aceitação.
- Confirmar impactos em outras áreas do sistema.
- Alinhar expectativas sobre qualidade, performance, segurança, etc.

As perguntas devem ser direcionadas, evitando generalidades. Exemplo:
- “Essa nova funcionalidade deve estar disponível apenas para usuários autenticados ou também para visitantes?”
- “Há alguma restrição de desempenho para essa consulta? (ex.: tempo máximo de resposta)”

### 2.3. Revisão de robustez com análise tridimensional (engenharia)
Após coletar as respostas (ou se não houver perguntas pendentes), o agente deve revisar a tarefa sob três dimensões:

1. **Dimensão Funcional**  
   - A tarefa cobre todos os casos de uso esperados?  
   - Existem bordas (edge cases) não tratados?  
   - A lógica proposta é coerente com o domínio?

2. **Dimensão Técnica / Arquitetural**  
   - A solução se encaixa na arquitetura existente?  
   - Introduz dívida técnica ou viola padrões estabelecidos?  
   - Dependências externas são justificadas e gerenciáveis?

3. **Dimensão de Qualidade**  
   - A tarefa prevê testes (unitários, integração)?  
   - Há impactos em performance, segurança ou acessibilidade?  
   - A manutenibilidade está garantida (código claro, documentação)?

Para cada dimensão, o agente deve listar pontos fortes, riscos e recomendações.

### 2.4. Documentação da análise
O agente deve produzir um resumo contendo:
- **Objetivo claro** da tarefa.
- **Perguntas feitas e respostas obtidas** (se houver).
- **Análise tridimensional** com conclusões.
- **Próximos passos** validados (ex.: “tarefa robusta o suficiente para iniciar implementação” ou “necessário ajustes no escopo”).

Esse resumo será compartilhado com o usuário para alinhamento antes da execução.

## 3. Comportamento durante a execução
- Se durante a implementação surgir nova ambiguidade ou desvio, o agente deve interromper e retornar ao processo de entendimento.
- Manter o histórico de decisões para rastreabilidade.
- Após concluir a tarefa, verificar se os critérios definidos foram atendidos.

## 4. Exceções
Caso o usuário indique que não deseja o processo de entendimento detalhado (ex.: “implemente diretamente”), o agente pode reduzir as perguntas, mas ainda deve fazer uma análise mínima para evitar erros graves.