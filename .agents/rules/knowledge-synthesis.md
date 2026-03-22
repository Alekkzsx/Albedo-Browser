---
trigger: always_on
---

# Síntese e Conexão de Conhecimento

## 1. Princípio: Conectar para amplificar
O agente **deve** ativamente conectar conhecimentos de diferentes fontes, módulos e domínios para criar soluções mais robustas. Conhecimento isolado é limitado — conhecimento conectado é poder.

## 2. Fontes de Conhecimento

### 2.1. Internas ao projeto
- **Código existente:** Padrões, decisões de design e convenções já estabelecidas.
- **Histórico de commits e decisões:** Por que algo foi feito de certa forma.
- **Testes:** O que é considerado comportamento correto e quais bordas foram cobertas.
- **Documentação:** README, PLAN.txt, comentários de código, KIs.

### 2.2. Externas ao projeto
- **Specs oficiais:** WHATWG HTML, CSS, DOM specs; ECMAScript spec; W3C.
- **Implementações de referência:** Servo (Rust), Chromium/V8 (C++), Firefox/SpiderMonkey (C++/Rust).
- **Literatura técnica:** Papers, livros, RFCs relevantes.
- **Ecossistema Rust:** Documentação de crates, best practices da comunidade.

## 3. Técnicas de Síntese

### 3.1. Cross-referência
- Ao implementar uma feature, verificar como módulos similares já resolveram problemas análogos no projeto.
- Mapear padrões recorrentes: "O módulo X usa o mesmo padrão que Y."
- Identificar oportunidades de reutilização antes de criar código novo.

### 3.2. Analogias com sistemas de referência
- Ao implementar funcionalidade de browser: "Como o Servo resolve isso?"
- Ao implementar JIT: "Como o V8/SpiderMonkey aborda esse problema?"
- Documentar a analogia e o que foi adaptado para o AlbedoBrowser.

### 3.3. Transferência de aprendizado
- Lições aprendidas em um módulo devem ser aplicadas em outros.
- Se um bug foi encontrado por padrão X no módulo A, verificar se o módulo B tem o mesmo padrão.
- Se uma otimização funcionou em um path, considerar para paths similares.

### 3.4. Visão sistêmica
- Entender como cada peça se encaixa na arquitetura geral.
- Ao alterar um módulo, considerar o impacto na cadeia: parser → DOM → layout → rendering.
- Manter um modelo mental atualizado do sistema como um todo.

## 4. Memória Persistente (Knowledge Items)

### 4.1. Consulta obrigatória
- Antes de pesquisar algo novo, verificar se já existe um KI sobre o tema.
- KIs de conversas anteriores contêm decisões arquiteturais que devem ser respeitadas.

### 4.2. Criação recomendada
Criar/atualizar KIs quando:
- Uma decisão arquitetural significativa é tomada.
- Uma investigação profunda é feita sobre um componente.
- Um padrão recorrente é identificado e documentado.
- Um bug difícil é resolvido e a lição deve ser preservada.

## 5. Ensinar Enquanto Faz
- Explicar o **"porquê"** além do **"como"** em toda implementação.
- Conectar decisões de implementação com princípios de engenharia.
- Quando usar uma técnica não-óbvia, explicar o conceito por trás.
- Exemplos: "Usamos Cow<str> aqui porque permite evitar clone quando o valor já é owned, mas ainda funciona com referências."

## 6. Anti-padrões de síntese (PROIBIDOS)
- ❌ **Pesquisa cega:** Investigar do zero sem verificar KIs e histórico.
- ❌ **Siloing:** Tratar cada módulo como se fosse isolado.
- ❌ **Reinvenção:** Implementar algo que já existe no projeto ou em crates.
- ❌ **Código sem explicação:** Implementar sem explicar o raciocínio.

## 7. Integração com outros workspaces
- **understand-first:** A análise inicial é o primeiro exercício de síntese.
- **context-mastery:** Gestão de contexto fornece as fontes de conhecimento.
- **deep-reasoning:** Raciocínio profundo se alimenta de conhecimento sintetizado.
- **communication-protocol:** A explicação ao usuário é um ato de síntese.
