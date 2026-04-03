# Fase 2: Recursos Avançados - Plano de Implementação

## Status Atual do ACE-HTML

### ✅ Já Implementado
1. **Template Insertion Mode** - Completo (tree_builder.rs:2136-2188)
   - Todos os casos de tags especiais dentro de template
   - Stack de insertion modes para templates aninhados
   - EOF handling correto

2. **Foreign Content Infrastructure** - Parcialmente Implementado
   - Namespaces: Html, Svg, MathML (mod.rs:41-45)
   - SVG tag name adjustment (tree_builder.rs:445-479)
   - MathML attribute adjustment (tree_builder.rs:488-497)
   - SVG attribute adjustment (tree_builder.rs:499-565)
   - Foreign attribute adjustment (tree_builder.rs:567-594)
   - handle_foreign_content() (tree_builder.rs:2396-2472)
   - Integration points (tree_builder.rs:2474-2504)
   - Teste básico de SVG foreign content (tree_builder.rs:2652-2677)

3. **Error Handling** - Robusto
   - 58 error codes definidos
   - Follows WHATWG specification

### ❌ Gaps Identificados para Fase 2

#### 1. Shadow DOM Parsing Support (CRÍTICO)
- [ ] Slot elements (`<slot>` e `<slot name="...">`)
- [ ] Atributo `slot` em elementos HTML
- [ ] Detecção de shadow root via atributo especial
- [ ] Serialização de conteúdo com slot assignment

#### 2. DocumentFragment Parsing
- [ ] API `parse_fragment()` existe mas não suporta contexto completo
- [ ] Faltam casos de teste para fragmentos
- [ ] Context element inheritance não implementado

#### 3. Custom Elements
- [ ] Validação de nomes (deve conter hífen)
- [ ] Is attribute handling
- [ ] Potential custom element detection durante parsing

#### 4. SVG Completo
- [ ] Lista completa de SVG elements (atualmente parcial)
- [ ] Mais attribute adjustments específicos de SVG
- [ ] Testes abrangentes de SVG

#### 5. MathML Completo  
- [ ] Lista completa de MathML elements
- [ ] Mais attribute adjustments específicos de MathML
- [ ] Testes abrangentes de MathML

#### 6. Template Element Expansão
- [ ] Template em contextos especiais (select, table)
- [ ] Template com shadow DOM
- [ ] Testes de templates aninhados complexos

---

## Plano de Implementação (4 semanas)

### Semana 1: Shadow DOM e Slot Elements
**Objetivo:** Implementar suporte completo a Shadow DOM parsing

#### Tarefas:
1. **Slot Element Handler**
   - Adicionar `<slot>` como elemento especial
   - Implementar `name` attribute parsing
   - Criar estrutura para slot assignment

2. **Slot Attribute**
   - Parse `slot="..."` em qualquer elemento
   - Armazenar slot assignment no HtmlElement
   - Validar slot names

3. **Shadow Root Detection**
   - Suporte para atributo `shadowrootmode` (declarative shadow DOM)
   - Criar nó de documento separado para shadow content
   - Anexar shadow root ao host element

4. **Testes**
   - Slot sem nome (default slot)
   - Slot nomeado
   - Múltiplos slots
   - Nested shadow roots
   - Slot assignment com fallback content

**Critério de Conclusão:** 
- 100% dos testes de slot passing
- Declarative shadow DOM funcional
- Zero regressões em testes existentes

### Semana 2: DocumentFragment e Custom Elements
**Objetivo:** Fragment parsing robusto e custom element detection

#### Tarefas:
1. **DocumentFragment API Completa**
   - Expandir `parse_fragment()` com contexto rico
   - Suportar namespace do contexto
   - Herdar form flags do contexto

2. **Custom Element Validation**
   - Validar nomes durante parsing (must contain hyphen)
   - Detectar potential custom elements
   - Marcar elementos para upgrade posterior

3. **Is Attribute**
   - Parse `is="x-button"` syntax
   - Armazenar is value no HtmlElement
   - Validar against custom element registry (futuro)

4. **Testes**
   - Fragment parsing com diferentes contextos
   - Custom element names válidos/inválidos
   - Is attribute handling

**Critério de Conclusão:**
- Fragment parsing 100% compatível com html5lib
- Custom element detection funcional
- Is attribute parsing correto

### Semana 3: SVG e MathML Completos
**Objetivo:** Listas completas e attribute adjustments

#### Tarefas:
1. **SVG Element List**
   - Adicionar todos os 140+ SVG elements
   - Categorizar por functionality
   - Adicionar ao tree builder

2. **SVG Attribute Adjustments**
   - Implementar todos os namespace adjustments
   - Handle xlink:, xmlns: prefixes
   - Case conversion específico de SVG

3. **MathML Element List**
   - Adicionar todos os MathML elements
   - Separar presentation vs content markup
   - Adicionar ao tree builder

4. **MathML Attribute Adjustments**
   - Implementation de mathvariant, mathsize, etc.
   - Displaystyle handling
   - Scriptlevel adjustments

5. **Testes Abrangentes**
   - SVG: shapes, paths, text, gradients, patterns
   - MathML: fractions, scripts, tables, symbols
   - Mixed content (HTML + SVG + MathML)

**Critério de Conclusão:**
- 95%+ WPT svg/ tests passing
- 95%+ WPT mathml/ tests passing
- Zero crashes em conteúdo misto

### Semana 4: Template Expansion e Integração Final
**Objetivo:** Templates robustos e integração completa

#### Tarefas:
1. **Template em Contextos Especiais**
   - Template dentro de select
   - Template dentro de table elements
   - Template em foreign content

2. **Nested Templates**
   - Múltiplos níveis de nesting
   - Correct insertion mode stack management
   - EOF handling complexo

3. **Template + Shadow DOM**
   - Template content com shadow roots
   - Slot assignment em template content
   - Cloning de templates com shadow DOM

4. **Integração e Refinamento**
   - Code cleanup e documentação
   - Performance optimization
   - Error messages melhoradas

5. **Testes Finais**
   - Suite completa de conformidade
   - Regression testing
   - Performance benchmarks

**Critério de Conclusão:**
- 100% dos testes de template passing
- Templates funcionam em todos os contextos
- Documentação completa

---

## Métricas de Sucesso da Fase 2

### Quantitativas:
- [ ] 95%+ html5lib tree-construction tests (todos os tipos)
- [ ] 90%+ WPT html/syntax/tests/tree-construction/
- [ ] 95%+ WPT svg/ tests
- [ ] 90%+ WPT mathml/ tests
- [ ] 100% shadow DOM parsing tests
- [ ] Zero crashes em páginas reais

### Qualitativas:
- [ ] Shadow DOM parsing completo
- [ ] Slot elements funcionais
- [ ] Custom element detection
- [ ] DocumentFragment robusto
- [ ] SVG/MathML lists completas
- [ ] Templates em todos os contextos

---

## Riscos e Mitigações

### Risco 1: Complexidade do Shadow DOM
**Mitigação:** Implementar incrementalmente, começando com slots simples

### Risco 2: SVG/MathML são enormes
**Mitigação:** Focar nos elements mais comuns primeiro, expandir gradualmente

### Risco 3: Regressões em código existente
**Mitigação:** Manter suite de testes robusta, rodar após cada mudança

### Risco 4: Spec changes
**Mitigação:** Seguir WHATWG HTML Living Standard rigorosamente

---

## Próximos Passos Imediatos

1. ✅ Auditoria do código atual concluída
2. 🔄 Criar estrutura para Slot elements
3. ⏳ Implementar shadowrootmode parsing
4. ⏳ Expandir listas de SVG/MathML elements
5. ⏳ Adicionar testes abrangentes

**Tempo Estimado:** 4 semanas
**Complexidade:** Alta
**Prioridade:** Crítica para conformidade com navegadores modernos
