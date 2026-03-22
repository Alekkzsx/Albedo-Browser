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
- Para o AlbedoBrowser: mapear a pipeline `Network → Parser → DOM → CSS → Layout → Rendering → Display ↔ JS/JIT`.

### 2.4. Consultar fontes de contexto
- **PLAN.txt:** Ler o roadmap do projeto para entender prioridades e direção.
- **Knowledge Items:** Consultar KIs existentes antes de investigar do zero.
- **Specs oficiais:** Identificar quais padrões WHATWG/W3C/ECMAScript governam a área.
- **Implementações de referência:** Verificar como Servo, Chromium ou Firefox abordam o problema.

### 2.5. Analisar funcionalidades existentes
- Revisar código de features principais para entender o domínio.
- Identificar testes existentes e sua cobertura.
- Verificar documentação interna (README, docs/, comentários relevantes).

### 2.6. Levantar convenções e estilo
- Identificar ferramentas de formatação (`cargo fmt`, `clippy`).
- Observar padrões de nomenclatura, organização de arquivos, tratamento de erros.
- Verificar se há guias de contribuição ou padrões definidos em `.editorconfig`, `rustfmt.toml`, `clippy.toml`, etc.

## 3. Documentação obrigatória da compreensão
Após a análise inicial, o agente **deve** produzir um resumo estruturado, que incluirá:

- **Stack identificada**
- **Estrutura de diretórios relevante**
- **Arquitetura e padrões utilizados** (com referência à pipeline do browser)
- **Principais funcionalidades e fluxos**
- **Convenções e boas práticas detectadas**
- **Specs e referências relevantes** (links WHATWG/W3C quando aplicável)
- **Possíveis pontos de atenção ou gaps de conhecimento**

Esse resumo será compartilhado com o usuário antes de qualquer implementação, para validação do entendimento.

## 4. Antes de executar qualquer tarefa
Para cada nova tarefa ou solicitação, o agente deve:

1. **Verificar se já possui o contexto completo do projeto.** Caso contrário, executar a análise inicial.
2. **Mapear como a tarefa se relaciona com o código existente.** Identificar arquivos, funções, módulos afetados.
3. **Verificar KIs e histórico de decisões.** Respeitar decisões arquiteturais anteriores.
4. **Listar hipóteses e dependências.** Se houver ambiguidade, levantar questões ao usuário antes de prosseguir.
5. **Propor uma abordagem** (se necessário) e aguardar confirmação antes de alterar código.

## 5. Comportamento durante a implementação
- Manter a análise atualizada: ao perceber novos padrões ou estruturas, atualizar o contexto.
- Evitar alterações que fujam das convenções estabelecidas.
- Priorizar a manutenção da integridade dos testes existentes.
- Documentar decisões relevantes (em comentários ou no resumo do workspace).

## 6. Anti-padrões de entendimento (PROIBIDOS)
- ❌ **Agir sem ler:** Propor mudanças sem ter lido o código afetado.
- ❌ **Ignorar PLAN.txt:** Implementar sem verificar o roadmap do projeto.
- ❌ **Pular KIs:** Pesquisar do zero quando existe KI sobre o tema.
- ❌ **Assumir arquitetura:** Supor a organização sem explorar a estrutura real.
- ❌ **Análise superficial:** Ler apenas os nomes dos arquivos, sem abrir o código.

## 7. Exceções
A única exceção ao fluxo de "understand-first" é quando o próprio usuário solicitar explicitamente uma ação sem análise prévia (ex.: "ignore a análise e faça X"). Mesmo nesse caso, o agente deve alertar sobre os riscos e, se possível, fazer uma análise rápida.

## 8. Integração com outros workspaces
- **context-mastery:** Fornece as fontes de contexto obrigatórias (specs, KIs, referências).
- **architecture-guard:** A análise inicial mapeia boundaries que devem ser respeitadas.
- **pattern-consistency:** A análise identifica convenções que serão seguidas.
- **evidence-based:** Entender o projeto é o primeiro ato de coleta de evidências.
- **knowledge-synthesis:** A análise é o primeiro exercício de síntese de conhecimento.