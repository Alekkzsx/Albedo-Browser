# Plano de Implementação: ACE-HTML 100% Completo

## Visão Geral

Este plano visa levar o parser HTML do Albedo Browser (ACE-HTML) ao nível de completude dos navegadores modernos como Chrome (Blink) e Firefox (Gecko), garantindo conformidade total com a especificação WHATWG HTML Living Standard.

**Estado Atual:**
- Lexer/Tokenizer: ~3.106 linhas
- Tree Builder: ~2.678 linhas  
- Tokenizer wrapper: ~281 linhas
- Total: ~6.065 linhas de código
- Funcionalidades básicas implementadas: tokenização, construção de árvore DOM, namespaces SVG/MathML básicos, algoritmo adoption agency, foster parenting, template support básico

**Gap Principal:** Faltam recursos avançados, cobertura completa de testes de conformidade, otimizações de produção e integrações completas com CSS/JS engine.

---

## FASE 1: Fundamentos e Conformidade Básica (Semanas 1-4)

### 1.1 Expansão da Suíte de Testes de Conformidade
**Objetivo:** Alcançar 95%+ de aprovação nos testes html5lib

#### 1.1.1 Integração Completa html5lib-tests
- [ ] Baixar suite completa html5lib-tests do GitHub
- [ ] Implementar harness de testes para:
  - `tree-construction` (todos os arquivos .dat)
  - `tokenizer` (todos os arquivos .json)
  - `fragment` (contextos múltiplos)
- [ ] Criar sistema de relatório de falhas categorizadas
- [ ] Estabelecer baseline de conformidade atual

#### 1.1.2 Testes Específicos WHATWG
- [ ] Implementar testes para todos os estados do tokenizer (88 estados)
- [ ] Testes de erros de parse (todos os error codes)
- [ ] Testes de encoding detection básico

**Critério de Conclusão:** 90%+ aprovação em html5lib tree-construction tests

---

### 1.2 Correção de Gaps no Tokenizer

#### 1.2.1 Estados Faltantes ou Incompletos
- [ ] **CDATA sections em foreign content**: Implementar suporte completo
- [ ] **RCDATA/RAWTEXT edge cases**: Testar todos os cenários de escape
- [ ] **Script data states**: 
  - Script data state
  - Script data escape start state
  - Script data escape start dash state
  - Script data escaped state
  - Script data escaped dash state/dash dash state
  - Script data escaped double quote/single quote states
- [ ] **Markup declaration open state**: Detectar automaticamente DOCTYPE/comment/CDATA

#### 1.2.2 Entidades de Caracteres
- [ ] Verificar completude do entities.json (todas as 2.231 entidades)
- [ ] Implementar fallback correto para entidades desconhecidas
- [ ] Tratar casos históricos de ampersand ambíguo corretamente

#### 1.2.3 Tratamento de EOF
- [ ] EOF em todos os estados possíveis
- [ ] EOF em tags abertas
- [ ] EOF em comentários
- [ ] EOF em CDATA
- [ ] EOF em doctype

**Critério de Conclusão:** Zero falhas em testes de tokenização html5lib

---

### 1.3 Tree Builder: Casos Especiais de Elementos

#### 1.3.1 Elementos de Formulário Completos
- [ ] `<form>`: nesting rules, form pointer, association algorithms
- [ ] `<input>`: todos os tipos, atributos, estado de validação
- [ ] `<select>`: insertion mode específico, handling de options
- [ ] `<option>`/`<optgroup>`: parsing em contextos especiais
- [ ] `<textarea>`: RCDATA handling, value sanitization
- [ ] `<button>`: nesting restrictions, form association

#### 1.3.2 Elementos de Tabela Avançados
- [ ] `<colgroup>`/`<col>`: column group handling completo
- [ ] `<caption>`: caption insertion mode
- [ ] `<tfoot>`: foot insertion mode, reordering
- [ ] Algoritmo de "foster parenting" completo para todos os casos

#### 1.3.3 Elementos de Lista e Definição
- [ ] `<ul>`/`<ol>`/`<li>`: implicit opening/closing
- [ ] `<dl>`/`<dt>`/`<dd>`: definition list parsing
- [ ] `<menu>`/`<dir>`: legacy handling

**Critério de Conclusão:** Todos os elementos HTML5 suportados com comportamento correto

---

## FASE 2: Recursos Avançados (Semanas 5-8)

### 2.1 Template Element Completo

#### 2.1.1 Template Insertion Mode
- [ ] Implementar "in template" insertion mode completo
- [ ] Stack of template insertion modes (já existe, verificar completude)
- [ ] Fragment parsing com contexto template

#### 2.1.2 Template Content DocumentFragment
- [ ] Criar documento separado para conteúdo do template
- [ ] Serialização correta de templates
- [ ] Interação com JavaScript (preparação para binding)

#### 2.1.3 Edge Cases de Template
- [ ] Templates aninhados
- [ ] Templates dentro de tables
- [ ] Templates em foreign content (SVG/MathML)
- [ ] Adoption agency algorithm com templates

**Critério de Conclusão:** Suporte completo a `<template>` conforme spec

---

### 2.2 Shadow DOM Parsing Support

#### 2.2.1 Slot Elements
- [ ] `<slot>` element parsing
- [ ] Named slots vs default slots
- [ ] Slot assignment preparation (não rendering ainda)

#### 2.2.2 Shadow Root Preparation
- [ ] Estruturas de dados para shadow roots
- [ ] attachShadow() preparation na DOM bindings
- [ ] Closed vs open shadow roots

#### 2.2.3 Custom Elements Integration
- [ ] is="type" attribute support
- [ ] Autonomous custom elements (tag names com hyphen)
- [ ] Preparation para upgradedCallback

**Critério de Conclusão:** Infraestrutura pronta para Shadow DOM v1

---

### 2.3 Foreign Content Avançado

#### 2.3.1 SVG Completo
- [ ] Todos os elementos SVG 2.0 (100+ elementos)
- [ ] Atributos SVG com case-sensitive handling
- [ ] SVG namespace inheritance correto
- [ ] Mixed HTML/SVG content (foreignObject completo)
- [ ] Self-closing tags em SVG

#### 2.3.2 MathML Completo
- [ ] Todos os elementos MathML3
- [ ] MathML attributes adjustment
- [ ] MathML text integration points
- [ ] Annotation-xml handling
- [ ] Mixed MathML/HTML content

#### 2.3.3 Foreign Content Error Handling
- [ ] HTML tags em foreign content (correto handling)
- [ ] Doctype em foreign content (erro)
- [ ] Comments em foreign content
- [ ] Character references em foreign content

**Critério de Conclusão:** Parse correto de documentos complexos SVG/MathML

---

## FASE 3: Encoding e Internacionalização (Semanas 9-10)

### 3.1 Detecção Automática de Encoding

#### 3.1.1 BOM Detection
- [ ] UTF-8 BOM
- [ ] UTF-16LE BOM
- [ ] UTF-16BE BOM

#### 3.1.2 Meta Tag Detection
- [ ] `<meta charset="...">`
- [ ] `<meta http-equiv="Content-Type" content="text/html; charset=...">`
- [ ] Priority order correto (BOM > HTTP > meta > prescan)

#### 3.1.3 Prescan Algorithm
- [ ] Implementar encoding prescan (first 1024 bytes)
- [ ] Buscar meta tags no stream inicial
- [ ] Detectar patterns de encoding comum

#### 3.1.4 Fallback Encoding
- [ ] Encoding por região/localização
- [ ] Heurísticas para encodings legados
- [ ] windows-1252 fallback para ISO-8859-1

**Critério de Conclusão:** Detecção de encoding compatível com browsers

---

### 3.2 Unicode e Normalização

#### 3.2.1 Normalização de Nomes
- [ ] ASCII lowercase para tag names
- [ ] Attribute name normalization
- [ ] Character reference processing completo

#### 3.2.2 surrogate pairs
- [ ] UTF-16 surrogate handling
- [ ] Invalid codepoint replacement (U+FFFD)
- [ ] Noncharacter detection

#### 3.2.3 Null Character Handling
- [ ] U+0000 em diferentes contextos
- [ ] Replacement com U+FFFD onde necessário

**Critério de Conclusão:** Manipulação Unicode 100% correta

---

## FASE 4: Performance e Otimização (Semanas 11-12)

### 4.1 Otimizações de Parser

#### 4.1.1 Streaming Parser
- [ ] Parse incremental (chunk-based)
- [ ] Pause/resume parsing
- [ ] Network-driven parsing integration

#### 4.1.2 Memory Efficiency
- [ ] Arena allocator optimizations
- [ ] String interning para tag names comuns
- [ ] Small string optimization para attributes

#### 4.1.3 Fast Paths
- [ ] Fast path para HTML simples (sem errors)
- [ ] Lookup tables otimizadas para entity decoding
- [ ] SIMD para operações de string (quando disponível)

**Critério de Conclusão:** Performance competitiva com parsers modernos

---

### 4.2 Preload Scanner Avançado

#### 4.2.1 Resource Detection
- [ ] `<link rel="preload">`
- [ ] `<link rel="stylesheet">`
- [ ] `<script src>`
- [ ] `<img src>` / `<picture>` / `<source>`
- [ ] `<video>` / `<audio>` sources
- [ ] CSS `@import` detection (básico)

#### 4.2.2 Priorization
- [ ] Resource priority hints
- [ ] Fetchpriority attribute
- [ ] Async/defer script handling

**Critério de Conclusão:** Preload scanner funcional integrado com network layer

---

## FASE 5: Integração e APIs (Semanas 13-16)

### 5.1 DOM Bindings Completos

#### 5.1.1 Document APIs
- [ ] `document.write()` / `document.writeln()`
- [ ] `document.open()` / `document.close()`
- [ ] Dynamic document manipulation durante parsing

#### 5.1.2 Element Query APIs
- [ ] `querySelector()` / `querySelectorAll()` otimizadas
- [ ] `getElementsByClassName()` live collections
- [ ] `getElementsByTagName()` live collections
- [ ] `getElementById()` fast path

#### 5.1.3 Mutation Observers
- [ ] Observer durante parsing
- [ ] ChildList mutations
- [ ] Attribute mutations
- [ ] Subtree observers

**Critério de Conclusão:** DOM APIs completas e performáticas

---

### 5.2 Parser Integration com Engine

#### 5.2.1 CSS Integration
- [ ] Parse CSS embedded em `<style>`
- [ ] Parse CSS em attributes style=""
- [ ] Link external stylesheets detection
- [ ] Critical CSS extraction preparation

#### 5.2.2 JavaScript Integration
- [ ] Inline script execution preparation
- [ ] External script loading coordination
- [ ] Defer/async/timeline handling
- [ ] Module scripts detection

#### 5.2.3 Incremental Rendering Prep
- [ ] Frame request coordination
- [ ] Layout scheduling signals
- [ ] Paint preparation hooks

**Critério de Conclusão:** Parser totalmente integrado com rendering pipeline

---

## FASE 6: Validação Final e Polimento (Semanas 17-20)

### 6.1 Testes de Conformidade Extensivos

#### 6.1.1 Web Platform Tests (WPT)
- [ ] Integrar WPT html/syntax/ tree-construction
- [ ] Rodar todos os testes de parsing
- [ ] Alcançar 95%+ pass rate

#### 6.1.2 Testes de Regressão
- [ ] Criar suite de regressão própria
- [ ] Testes de sites reais (top 1000 Alexa)
- [ ] Fuzz testing do parser

#### 6.1.3 Performance Benchmarks
- [ ] Speedometer HTML parsing component
- [ ] JetStream parsing benchmarks
- [ ] Custom benchmarks com páginas reais

**Critério de Conclusão:** 95%+ WPT pass rate, performance competitiva

---

### 6.2 Documentação e Developer Experience

#### 6.2.1 Documentação Técnica
- [ ] API documentation completa (rustdoc)
- [ ] Architecture documentation
- [ ] Contributing guidelines
- [ ] Parser state machine diagrams

#### 6.2.2 Debugging Tools
- [ ] Parse tree visualization
- [ ] Error reporting detalhado
- [ ] Performance profiling tools
- [ ] Token stream inspector

**Critério de Conclusão:** Documentação completa para contributors

---

### 6.3 Error Handling e Reporting

#### 6.3.1 Error Categorization
- [ ] Classificação de erros por severidade
- [ ] Error recovery strategies documentadas
- [ ] Compatibilidade com quirks mode

#### 6.3.2 Developer Warnings
- [ ] Console warnings para erros comuns
- [ ] Deprecation warnings
- [ ] Accessibility hints

**Critério de Conclusão:** Error handling robusto e informativo

---

## METRICAS DE SUCESSO

### Conformidade
- ✅ 95%+ html5lib tree-construction tests
- ✅ 95%+ html5lib tokenizer tests  
- ✅ 90%+ WPT html/syntax tests
- ✅ Zero crashes em páginas reais

### Performance
- ✅ Parse de HTML5 spec em < 100ms (hardware moderno)
- ✅ Memory usage < 2x tamanho do HTML original
- ✅ Streaming parse com latency < 10ms por chunk

### Features
- ✅ Todos os elementos HTML5.3+
- ✅ SVG 2.0 completo
- ✅ MathML3 completo
- ✅ Template/Shadow DOM ready
- ✅ Encoding detection automático
- ✅ Full Unicode support

### Integração
- ✅ DOM bindings completos
- ✅ CSS integration
- ✅ JS execution coordination
- ✅ Preload scanner funcional

---

## CRONOGRAMA RESUMIDO

| Fase | Duração | Entregáveis Principais |
|------|---------|------------------------|
| 1. Fundamentos | 4 semanas | 90% html5lib pass, tokenizer completo |
| 2. Recursos Avançados | 4 semanas | Template, Shadow DOM prep, SVG/MathML full |
| 3. Encoding/i18n | 2 semanas | Encoding auto-detect, Unicode completo |
| 4. Performance | 2 semanas | Streaming parser, otimizações |
| 5. Integração | 4 semanas | DOM APIs, CSS/JS integration |
| 6. Validação | 4 semanas | 95% WPT, docs, polish |

**Total Estimado: 20 semanas (~5 meses)**

---

## PRIORIDADES CRÍTICAS

### Alta Prioridade (Must Have)
1. Conformidade html5lib 95%+
2. Encoding detection automático
3. Template element completo
4. SVG foreign content completo
5. Error handling robusto

### Média Prioridade (Should Have)
1. Shadow DOM infrastructure
2. Preload scanner avançado
3. Performance optimizations
4. MathML completo

### Baixa Prioridade (Nice to Have)
1. Quirks mode completo (legado)
2. XML/ XHTML mode
3. Experimental features

---

## RISCOS E MITIGAÇÕES

### Risco: Complexidade do Adoption Agency Algorithm
- **Mitigação:** Usar referência do specification diretamente, testes extensivos

### Risco: Performance em páginas grandes
- **Mitigação:** Profiling contínuo, otimizações incrementais

### Risco: Edge cases de encoding
- **Mitigação:** Testes com corpus real de páginas internacionais

### Risco: Integração com JS engine
- **Mitigação:** API clara, documentada, testes de integração

---

## RECURSOS NECESSÁRIOS

### Humanos
- 1-2 engenheiros Rust senior (tempo integral)
- 1 engenheiro de QA/testes
- Revisão periódica de especialistas em browsers

### Infraestrutura
- CI/CD para rodar suites de testes grandes
- Servidores para fuzz testing
- Acesso a ferramentas de profiling

### Dependências Externas
- html5lib-tests suite
- Web Platform Tests
-WHATWG HTML Living Standard (referência)

---

## CONCLUSÃO

Este plano cobre todos os aspectos necessários para elevar o ACE-HTML ao nível de completude dos browsers modernos. A abordagem é incremental, começando com fundamentos de conformidade, passando por recursos avançados, e culminando em integração completa com a engine.

**Próximo Passo Imediato:** Iniciar Fase 1.1 - Integrar suite completa html5lib-tests e estabelecer baseline de conformidade atual.
