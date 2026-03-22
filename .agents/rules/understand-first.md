---
trigger: always_on
---

## 1. Princípio fundamental: Understand First
Antes de qualquer tarefa, implementação, correção ou sugestão, o agente **deve** analisar e compreender completamente o projeto. Nenhuma ação será tomada sem que o contexto, a estrutura e as convenções do código estejam claros.

## 2. Processo obrigatório de análise inicial
Ao iniciar a interação com um novo projeto (ou após uma grande mudança), o agente executará as seguintes etapas:

### 2.1. Explorar a estrutura do projeto
- Listar diretórios e arquivos relevantes (evitar listar pastas como `node_modules`, `.git`, etc., se for o caso).
- Identificar arquivos de configuração principais: `package.json`, `pyproject.toml`, `Cargo.toml`, `go.mod`, `Dockerfile`, `.env.example`, etc.
- Localizar o ponto de entrada principal (ex.: `main.py`, `index.js`, `src/main.rs`, etc.).

### 2.2. Mapear dependências e stack tecnológica
- Extrair linguagens, frameworks e bibliotecas principais.
- Identificar dependências de desenvolvimento e produção.
- Verificar se há scripts de build, teste ou execução definidos.

### 2.3. Compreender a arquitetura
- Identificar padrões arquiteturais (MVC, clean architecture, microservices, etc.).
- Mapear camadas (controllers, services, repositories, models, etc.).
- Observar como a comunicação entre módulos é feita (imports, injeção de dependência, APIs).

### 2.4. Analisar funcionalidades existentes
- Revisar código de features principais para entender o domínio.
- Identificar testes existentes e sua cobertura.
- Verificar documentação interna (README, docs/, comentários relevantes).

### 2.5. Levantar convenções e estilo
- Identificar ferramentas de formatação (Prettier, Black, ESLint, etc.).
- Observar padrões de nomenclatura, organização de arquivos, tratamento de erros.
- Verificar se há guias de contribuição ou padrões definidos em `.editorconfig`, `.eslintrc`, etc.

## 3. Documentação obrigatória da compreensão
Após a análise inicial, o agente **deve** produzir um resumo estruturado, que incluirá:

- **Stack identificada**
- **Estrutura de diretórios relevante**
- **Arquitetura e padrões utilizados**
- **Principais funcionalidades e fluxos**
- **Convenções e boas práticas detectadas**
- **Possíveis pontos de atenção ou gaps de conhecimento**

Esse resumo será compartilhado com o usuário antes de qualquer implementação, para validação do entendimento.

## 4. Antes de executar qualquer tarefa
Para cada nova tarefa ou solicitação, o agente deve:

1. **Verificar se já possui o contexto completo do projeto.** Caso contrário, executar a análise inicial.
2. **Mapear como a tarefa se relaciona com o código existente.** Identificar arquivos, funções, módulos afetados.
3. **Listar hipóteses e dependências.** Se houver ambiguidade, levantar questões ao usuário antes de prosseguir.
4. **Propor uma abordagem** (se necessário) e aguardar confirmação antes de alterar código.

## 5. Comportamento durante a implementação
- Manter a análise atualizada: ao perceber novos padrões ou estruturas, atualizar o contexto.
- Evitar alterações que fujam das convenções estabelecidas.
- Priorizar a manutenção da integridade dos testes existentes.
- Documentar decisões relevantes (em comentários ou no resumo do workspace).

## 6. Exceções
A única exceção ao fluxo de “understand-first” é quando o próprio usuário solicitar explicitamente uma ação sem análise prévia (ex.: “ignore a análise e faça X”). Mesmo nesse caso, o agente deve alertar sobre os riscos e, se possível, fazer uma análise rápida.