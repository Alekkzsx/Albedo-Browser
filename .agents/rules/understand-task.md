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
- Classificar a tarefa: feature, bugfix, refatoração, otimização, documentação.

### 2.2. Formulação de perguntas de refinamento
Sempre que houver qualquer incerteza, o agente **deve** elaborar perguntas específicas para:
- Esclarecer o escopo (o que está dentro/fora).
- Entender critérios de sucesso e aceitação.
- Confirmar impactos em outras áreas do sistema.
- Alinhar expectativas sobre qualidade, performance, segurança, etc.

As perguntas devem ser direcionadas, evitando generalidades. Exemplo:
- "Essa nova funcionalidade deve estar disponível apenas para usuários autenticados ou também para visitantes?"
- "Há alguma restrição de desempenho para essa consulta? (ex.: tempo máximo de resposta)"
- "Essa mudança afeta a pipeline de rendering ou apenas a camada de parsing?"

### 2.3. Revisão de robustez com análise quadridimensional
Após coletar as respostas (ou se não houver perguntas pendentes), o agente deve revisar a tarefa sob **quatro dimensões**:

1. **Dimensão Funcional**
   - A tarefa cobre todos os casos de uso esperados?
   - Existem bordas (edge cases) não tratados?
   - A lógica proposta é coerente com o domínio?
   - Para browsers: HTML malformado, CSS inválido e JS adversarial foram considerados?

2. **Dimensão Técnica / Arquitetural**
   - A solução se encaixa na arquitetura existente? (boundaries respeitados?)
   - Introduz dívida técnica ou viola padrões estabelecidos?
   - Dependências externas são justificadas e gerenciáveis?
   - Respeita a pipeline do AlbedoBrowser? (Network → Parser → DOM → CSS → Layout → Rendering)

3. **Dimensão de Qualidade**
   - A tarefa prevê testes (unitários, integração)?
   - Há impactos em performance, segurança ou acessibilidade?
   - A manutenibilidade está garantida (código claro, documentação)?
   - Atende aos padrões de `code-quality` e `error-handling`?

4. **Dimensão de Segurança**
   - A mudança expõe novas superfícies de ataque?
   - Input externo é validado adequadamente?
   - Same-Origin Policy e CSP são respeitados?
   - Dados sensíveis estão protegidos?

Para cada dimensão, o agente deve listar pontos fortes, riscos e recomendações.

### 2.4. Documentação da análise
O agente deve produzir um resumo contendo:
- **Objetivo claro** da tarefa.
- **Perguntas feitas e respostas obtidas** (se houver).
- **Análise quadridimensional** com conclusões.
- **Specs/referências** WHATWG/W3C relevantes.
- **Próximos passos** validados (ex.: "tarefa robusta o suficiente para iniciar implementação" ou "necessário ajustes no escopo").

Esse resumo será compartilhado com o usuário para alinhamento antes da execução.

## 3. Comportamento durante a execução
- Se durante a implementação surgir nova ambiguidade ou desvio, o agente deve interromper e retornar ao processo de entendimento.
- Manter o histórico de decisões para rastreabilidade.
- Após concluir a tarefa, verificar se os critérios definidos foram atendidos.

## 4. Anti-padrões de entendimento de tarefa (PROIBIDOS)
- ❌ **Assumir escopo:** Começar a implementar sem esclarecer ambiguidades.
- ❌ **Perguntas genéricas:** "O que devo fazer?" — as perguntas devem ser específicas e técnicas.
- ❌ **Ignorar segurança:** Não considerar a dimensão de segurança na análise.
- ❌ **Análise incompleta:** Pular dimensões da análise por pressa.
- ❌ **Over-analysis:** Passar tempo excessivo analisando uma tarefa trivial.

## 5. Exceções
Caso o usuário indique que não deseja o processo de entendimento detalhado (ex.: "implemente diretamente"), o agente pode reduzir as perguntas, mas ainda deve fazer uma análise mínima para evitar erros graves.

## 6. Integração com outros workspaces
- **understand-first:** Fornece o contexto do projeto onde a tarefa será executada.
- **deep-reasoning:** A análise quadridimensional exige raciocínio profundo.
- **failure-analysis:** A identificação de riscos alimenta a FMEA.
- **security-first:** A dimensão de segurança aplica os princípios de security-first.
- **plan-mode:** O entendimento da tarefa é pré-requisito para o planejamento.