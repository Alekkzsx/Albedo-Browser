---
trigger: always_on
---

# Gestão de Contexto e Referências Técnicas

## 1. Princípio: Contexto é soberania
O agente **deve** dominar o contexto do projeto antes de agir. Contexto não é apenas "ler o código" — é entender as decisões, as specs, as referências e o histórico que moldaram o sistema. Código sem contexto é código cego.

## 2. Fontes de Contexto Obrigatórias

### 2.1. Specs e padrões oficiais
Para QUALQUER implementação de WebAPI ou funcionalidade de browser:
- **WHATWG HTML Living Standard:** Para parser, DOM, semântica de elementos.
- **W3C CSS Specifications:** Para propriedades, valores, cascata, herança.
- **ECMAScript Specification:** Para comportamento de JavaScript.
- **W3C DOM Standard:** Para interfaces, eventos, manipulação de árvore.
- **Fetch Standard:** Para requisições de rede, CORS, redirecionamentos.
- Citar o link da spec no `implementation_plan.md` — sem exceção.

### 2.2. Implementações de referência
Ao implementar funcionalidade complexa, estudar como outros browsers resolvem:
- **Servo (Rust):** Referência principal — mesma linguagem, arquitetura similar.
- **Chromium/Blink (C++):** Referência para comportamento e otimizações.
- **Firefox/Gecko (C++/Rust):** Referência para standards compliance.
- **WebKit (C++):** Referência alternativa, especialmente para rendering.
- Documentar qual implementação foi consultada e o que foi adaptado.

### 2.3. Knowledge Items (KIs)
- **ANTES** de pesquisar algo novo, verificar se já existe um KI sobre o tema.
- KIs contêm decisões arquiteturais, investigações e lições aprendidas.
- Ao tomar uma decisão arquitetural significativa: criar ou atualizar um KI.
- KIs são a **memória de longo prazo** do projeto — tratá-los com respeito.

## 3. Processo de Contexto

### 3.1. Antes de implementar qualquer feature
1. **Identificar specs relevantes** — quais padrões governam essa feature?
2. **Ler o código existente** — como o projeto já aborda algo similar?
3. **Consultar KIs** — existe histórico de decisão sobre isso?
4. **Estudar referência** — como Servo/Chromium/Firefox implementam?
5. **Sintetizar** — combinar todas as fontes em uma compreensão unificada.

### 3.2. Mapeamento de dependências antes de mudanças
Para TODA alteração, antes de tocar no código:
- Listar **todos os módulos** que serão afetados (direto e indireto).
- Identificar **interfaces** que serão modificadas.
- Verificar **testes existentes** para a área afetada.
- Mapear **consumidores** de APIs que serão alteradas.

### 3.3. Rastreabilidade de decisões
Toda decisão arquitetural significativa deve ter:
- **Contexto:** Por que essa decisão foi necessária.
- **Alternativas consideradas:** O que mais foi avaliado.
- **Justificativa:** Por que essa abordagem foi escolhida.
- **Registro:** Em KI, comentário de código ou implementation plan.

## 4. Gestão de PLAN.txt
- `PLAN.txt` é o roadmap do projeto — consultá-lo regularmente.
- Ao completar itens do PLAN, comunicar ao usuário.
- Ao identificar que um item do PLAN precisa ser atualizado, sugerir.

## 5. Anti-padrões de contexto (PROIBIDOS)
- ❌ **Implementar sem spec:** Criar funcionalidade web sem consultar a spec.
- ❌ **Ignorar KIs:** Pesquisar do zero quando existe KI relevante.
- ❌ **Código cego:** Alterar código sem ler o módulo completo primeiro.
- ❌ **Decisão sem registro:** Tomar decisão arquitetural sem documentar.
- ❌ **Copy-paste de referência:** Copiar código de Servo/Chromium sem adaptar.

## 6. Integração com outros workspaces
- **understand-first:** A análise inicial é o primeiro exercício de gestão de contexto.
- **knowledge-synthesis:** O contexto é matéria-prima para a síntese de conhecimento.
- **evidence-based:** Specs e referências são as evidências mais fortes.
- **architecture-guard:** O contexto informa quais boundaries devem ser respeitadas.
