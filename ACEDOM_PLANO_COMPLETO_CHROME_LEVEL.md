# 🚀 AceDOM 2.0 - Plano de Implementação para Nível Chrome/Firefox

## 📊 Análise do Estado Atual (Dezembro 2025)

### Código Existente
- **1.121 linhas** em `/workspace/src/engine/dom/mod.rs`
- **Dependência crítica:** `kuchiki 0.8` (parser HTML externo)
- **Estrutura:** `Vec<AceNode>` com índices numéricos
- **Features implementadas:**
  - ✅ Árvore DOM completa (parent, children, siblings)
  - ✅ MutationObserver API
  - ✅ Shadow DOM básico iniciado
  - ✅ Dirty flags system
  - ✅ Serialização HTML/texto
  - ✅ Manipulação de atributos
  - ✅ insertAdjacentHTML
  - ✅ Fragmentos HTML
  - ✅ Subframes/iframes

### Problemas Críticos
🔴 **kuchiki dependency** - Viola princípio "100% próprio"  
🔴 **Sem Range/Selection API** - Essencial para editores  
🔴 **Sem Accessibility Tree** - Leitura de tela impossível  
🔴 **Sem Custom Elements** - Web Components não funcionam  
🔴 **Performance não otimizada** - Vec<usize> não é cache-friendly  
🔴 **Memória ineficiente** - Sem string interning ou pooling  

---

## 🎯 Metas Quantificáveis (24 Meses)

| Métrica | Atual | Meta 24M | Melhoria |
|---------|-------|----------|----------|
| Parse 100KB HTML | ~150ms | **30ms** | 5x |
| getElementById | 2.3μs | **0.4μs** | 5.7x |
| querySelector | 15.7μs | **2.0μs** | 7.8x |
| DOM memory (10K nodes) | ~2.5MB | **0.6MB** | 4x menos |
| WPT Pass Rate | N/A | **95%+** | Production-ready |
| Custom Elements | ❌ | ✅ Full v1 | Nova feature |
| ARIA Tree | ❌ | ✅ 1.2 Complete | Nova feature |

---

## 📋 Roadmap em 5 Fases (18-24 Meses)

### FASE 1: Fundação de Performance (Meses 1-4)
**Objetivo:** Remover kuchiki + Otimizações básicas de memória/performance

#### Tarefas:
1. **[CRÍTICO] Remover kuchiki completamente**
   - Migrar `from_kuchiki()` → `from_html_document()` (ACE-HTML parser)
   - Remover `use kuchiki::NodeRef` e `use kuchiki::traits::TendrilSink`
   - Eliminar `set_inner_html_from_kuchiki()` → usar apenas `set_inner_html_from_nodes()`
   - Feature flag `ace_html_parser` torna-se default permanente

2. **Implementar DomArena própria**
   - Alocação em blocos de 4KB (não realloc por node)
   - Memory pooling para nodes frequentes (div, span, text)
   - Cache-line alignment (64 bytes)

3. **Small String Optimization (SSO)**
   - Strings ≤23 chars inline (sem alocação heap)
   - Interning para tag names (div, span, p, a, etc.)
   - Arc<str> mantido apenas para texto longo

4. **Live NodeLists**
   - `HTMLCollection` (live, auto-update)
   - `NodeList` (snapshot ou live)
   - `getElementsByClassName()` live
   - `getElementsByTagName()` live

5. **Índices especializados**
   - HashMap<ID, usize> para `getElementById`
   - BTreeMap para queries por nome/classe
   - Atualização incremental (não rebuild completo)

#### Entregáveis:
- [ ] Zero dependência kuchiki no código
- [ ] DomArena com arena.rs próprio
- [ ] SSO implementado em AceNodeType
- [ ] LiveNodeList<T> genérico
- [ ] Benchmarks: 2-5x ganho vs atual

#### Riscos:
- ⚠️ Regressões de funcionalidade durante migração
- ⚠️ Bugs em live collections (infinit loops)
- ⚠️ Memory leaks se pooling mal implementado

---

### FASE 2: APIs Web Completas (Meses 5-9)
**Objetivo:** Suporte completo a Web Components v1

#### Tarefas:
1. **Range API Completa** (WHATWG DOM Spec)
   ```rust
   pub struct Range {
       start_container: usize,
       start_offset: u32,
       end_container: usize,
       end_offset: u32,
       collapsed: bool,
   }
   
   // Métodos essenciais:
   - setStart(node, offset)
   - setEnd(node, offset)
   - deleteContents()
   - extractContents() → DocumentFragment
   - cloneContents() → DocumentFragment
   - insertNode(node)
   - surroundContents(newParent)
   - compareBoundaryPoints()
   - getBoundingClientRect()
   ```

2. **Selection API Multi-Range**
   ```rust
   pub struct Selection {
       ranges: Vec<Range>,
       anchor_node: usize,
       anchor_offset: u32,
       focus_node: usize,
       focus_offset: u32,
       direction: SelectionDirection,
   }
   
   // Métodos:
   - addRange(range)
   - removeRange(range)
   - getRangeAt(index)
   - removeAllRanges()
   - collapse(node, offset)
   - extend(node, offset)
   - selectAllChildren(node)
   - deleteFromDocument()
   - toString()
   ```

3. **Custom Elements v1**
   ```rust
   pub struct CustomElementRegistry {
       definitions: HashMap<String, CustomElementDefinition>,
       upgrading: HashSet<usize>,
   }
   
   pub struct CustomElementDefinition {
       name: String,
       local_name: String,
       namespace: Namespace,
       constructor: JsFunction,
       observed_attributes: Vec<String>,
       lifecycle_callbacks: LifecycleCallbacks,
   }
   
   // Callbacks:
   - connectedCallback()
   - disconnectedCallback()
   - adoptedCallback()
   - attributeChangedCallback(name, oldVal, newVal)
   ```

4. **Shadow DOM Completo**
   - Slot assignment algorithm (Flattened DOM tree)
   - `attachShadow({mode, delegatesFocus})`
   - `shadowRoot` property (null se closed)
   - `<slot>` element com named slots
   - CSS scoping (:host, ::slotted, :host-context)
   - Event retargeting através de shadow boundaries
   - Declarative Shadow DOM (`<template shadowrootmode="open">`)

5. **Lifecycle Integration**
   - Queue microtasks para callbacks
   - Upgrade steps durante parsing
   - Pre-upgrade registry (define antes de parse)

#### Entregáveis:
- [ ] Range API 100% spec-compliant
- [ ] Selection API multi-range
- [ ] Custom Elements v1 completo
- [ ] Shadow DOM com slot assignment
- [ ] Lit.dev funcional
- [ ] Stencil funcional

#### Testes:
- Web Platform Tests: custom-elements, shadow-dom, range
- Meta: 90%+ pass rate

---

### FASE 3: Acessibilidade & Events (Meses 10-14)
**Objetivo:** A11y completa + Event system integrado

#### Tarefas:
1. **Accessibility Tree (ARIA 1.2)**
   ```rust
   pub struct AccessibilityNode {
       dom_node: usize,
       role: AriaRole,
       name: Option<String>,      // accessible name
       description: Option<String>,
       value: Option<String>,
       states: AriaStates,        // bitmask
       properties: AriaProperties,
       children: Vec<usize>,      // AX children (differs from DOM!)
       parent: Option<usize>,
   }
   
   pub enum AriaRole {
       // Landmarks
       Banner, Navigation, Main, Complement, ContentInfo,
       // Widgets
       Button, Checkbox, Combobox, Listbox, Menu, Menubar,
       MenuItem, Radio, Radiogroup, Slider, Spinbutton, Tab,
       Tablist, Tabpanel, Textbox, Tooltip, Tree, Treeitem,
       // Structures
       Article, Cell, Columnheader, Definition, Directory,
       Document, Feed, Figure, Group, Heading, Img, List,
       Listitem, Math, None, Note, Presentation, Row,
       Rowgroup, Rowheader, Separator, Table, Term, Toolbar,
       // Live Regions
       Alert, Log, Marquee, Status, Timer,
       // Windows
       Alertdialog, Dialog,
   }
   ```

2. **Accessible Name Computation (AccName Spec)**
   - Priority order:
     1. `aria-labelledby` (referenced elements)
     2. `aria-label` (direct string)
     3. Native HTML (alt, title, label for, etc.)
     4. Fallback (textContent)
   - Recursion detection (cycles)
   - Hidden element handling

3. **Implicit Role Mapping**
   ```rust
   fn get_implicit_role(tag: &str, attrs: &Attrs) -> AriaRole {
       match tag {
           "a" => if attrs.has("href") { Link } else { Generic },
           "button" => Button,
           "input" => match attrs.get("type") {
               Some("checkbox") => Checkbox,
               Some("radio") => Radio,
               Some("text") => Textbox,
               _ => Generic,
           },
           "h1".."h6" => Heading,
           "img" => if attrs.has("alt") { Img } else { Presentation },
           // ... 100+ mappings
       }
   }
   ```

4. **Event System Integrado**
   ```rust
   pub struct EventTarget {
       listeners: HashMap<String, Vec<EventListener>>,
   }
   
   pub struct EventListener {
       callback: JsFunction,
       capture: bool,
       once: bool,
       passive: bool,
       signal: Option<AbortSignal>,
   }
   
   // Phases:
   1. Capture phase (top-down)
   2. At-target phase
   3. Bubble phase (bottom-up)
   
   // Special events:
   - DOMContentLoaded
   - load, unload
   - focus, blur (non-bubbling)
   - focusin, focusout (bubbling)
   - click, dblclick
   - keydown, keyup, keypress
   - mouseenter, mouseleave (non-bubbling)
   - input, change
   ```

5. **Event Delegation Otimizada**
   - Single listener no parent
   - Filter por selector durante dispatch
   - Stop propagation support

#### Entregáveis:
- [ ] Accessibility Tree completa
- [ ] AccName computation 100% spec
- [ ] 100+ implicit role mappings
- [ ] Event system com 3 phases
- [ ] axe-core 100% pass
- [ ] NVDA/VoiceOver tested

#### Testes:
- WPT: accessibility, aria, events
- axe-core automated tests
- Manual testing com screen readers

---

### FASE 4: Otimizações Avançadas (Meses 15-18)
**Objetivo:** Performance de produção para frameworks

#### Tarefas:
1. **Incremental DOM Updates**
   ```rust
   pub fn diff(old_tree: &Dom, new_tree: &Dom) -> Vec<Patch> {
       // Algoritmo tipo React/Solid
       - Keyed reconciliation
       - LIS (Longest Increasing Subsequence) para reordering
       - Minimal moves
   }
   
   pub enum Patch {
       Insert { parent: usize, node: AceNode, before: Option<usize> },
       Remove { node: usize },
       Replace { old: usize, new: AceNode },
       UpdateText { node: usize, text: String },
       UpdateAttributes { node: usize, changes: AttrChanges },
   }
   ```

2. **Memory Pooling Avançado**
   - Pool por tipo de node (element, text, comment)
   - Pool por tag name (div pool, span pool, etc.)
   - Generational GC integration
   - Dealloc em batch (não individual)

3. **Parallel DOM Operations**
   ```rust
   use rayon::prelude::*;
   
   // Parallel querySelectorAll
   pub fn query_selector_all_parallel(&self, selector: &str) -> Vec<usize> {
       self.nodes.par_iter()
           .filter(|node| matches_selector(node, selector))
           .map(|node| node.id)
           .collect()
   }
   
   // Parallel style recalc
   pub fn recalc_styles_parallel(&mut self) {
       let dirty_nodes = self.collect_dirty();
       dirty_nodes.par_iter().for_each(|idx| {
           self.compute_style(*idx);
       });
   }
   ```

4. **Cache Systems**
   - Selector cache (querySelector results)
   - Style cache (computed styles)
   - Layout cache (bounding rects)
   - Invalidation strategies (LRU, time-based)

5. **Zero-Copy Text Nodes**
   - Mmap de grandes textos
   - Rope data structure para edições
   - Copy-on-write para clones

#### Entregáveis:
- [ ] Incremental DOM diff/patch
- [ ] Memory pools com 80% hit rate
- [ ] Parallel operations (Rayon)
- [ ] Selector cache com LRU
- [ ] Solid.js benchmark: dentro de 20% do Chrome

#### Benchmarks:
- JS Framework Benchmark (keyed)
- TodoMVC (React, Vue, Svelte, Solid)
- Memory footprint comparison

---

### FASE 5: Testing & Compliance (Meses 19-24)
**Objetivo:** Validação production-ready

#### Tarefas:
1. **Web Platform Tests Integration**
   ```bash
   # Rodar suites específicas
   ./wpt run --dom
   ./wpt run --custom-elements
   ./wpt run --shadow-dom
   ./wpt run --range
   ./wpt run --selection
   ./wpt run --aria
   ./wpt run --events
   ```

2. **CI/CD Pipeline**
   - GitHub Actions rodando WPT daily
   - Performance regression detection
   - Memory leak detection (valgrind, ASan)
   - Fuzzing contínuo (libFuzzer)

3. **Cross-Browser Testing**
   - Test suites do Chromium/Gecko/WebKit
   - Bug compatibility (quirks mode)
   - Edge cases documentados

4. **Documentation**
   - API docs (rustdoc)
   - Architecture docs
   - Performance tuning guide
   - Migration guide (kuchiki → ACE)

5. **Developer Tools**
   - DOM inspector integration
   - Accessibility tree viewer
   - Event listener debugger
   - Memory profiler hooks

#### Entregáveis:
- [ ] 95%+ WPT pass rate (DOM, Selectors, Range, Shadow DOM)
- [ ] CI pipeline com WPT automated
- [ ] Documentation completa
- [ ] DevTools integration
- [ ] Release 1.0 stable

---

## 🔧 Cronograma Detalhado - Primeiros 90 Dias

### Semana 1-2: Auditoria & Planejamento
- [ ] Audit todo uso de kuchiki no código
- [ ] Identificar todos os call sites
- [ ] Criar testes de regressão
- [ ] Setup de benchmarks baseline

### Semana 3-6: Remoção do kuchiki
- [ ] Remover `from_kuchiki()` (manter como deprecated)
- [ ] Migrar `from_html()` para ACE-HTML parser
- [ ] Eliminar `set_inner_html_from_kuchiki()`
- [ ] Remover imports do kuchiki
- [ ] Remover kuchiki do Cargo.toml
- [ ] Testes: todos passing

### Semana 7-10: DomArena + Otimizações
- [ ] Implementar DomArena com blocos 4KB
- [ ] Adicionar SSO para strings curtas
- [ ] Criar string interning para tags
- [ ] Implementar LiveNodeList
- [ ] Índices especializados (ID, class, tag)
- [ ] Benchmarks: validar 2-5x ganho

### Semana 11-14: Range API
- [ ] Estrutura Range
- [ ] setStart/setEnd
- [ ] deleteContents
- [ ] extractContents/cloneContents
- [ ] insertNode
- [ ] WPT range tests: 90%+ pass

### Semana 15-18: Shadow DOM + Custom Elements
- [ ] Slot assignment algorithm
- [ ] attachShadow() completo
- [ ] Custom Element registry
- [ ] Lifecycle callbacks
- [ ] attributeChangedCallback
- [ ] Lit.dev test: funcional

---

## 📦 Estrutura de Arquivos Proposta

```
src/engine/dom/
├── mod.rs              # AceDOM principal (refatorado, ~800 LOC)
├── arena.rs            # DomArena própria (novo, ~400 LOC)
├── node.rs             # AceNode + tipos (novo, ~300 LOC)
├── collection.rs       # LiveNodeList, HTMLCollection (novo, ~250 LOC)
├── range.rs            # Range API (novo, ~500 LOC)
├── selection.rs        # Selection API (novo, ~300 LOC)
├── custom_elements.rs  # Custom Elements v1 (novo, ~400 LOC)
├── shadow_dom.rs       # Shadow DOM completo (novo, ~600 LOC)
├── accessibility.rs    # ARIA tree (novo, ~700 LOC)
├── events.rs           # Event system (novo, ~500 LOC)
├── indices.rs          # HashMaps especializados (novo, ~200 LOC)
└── tests/
    ├── dom_tests.rs
    ├── range_tests.rs
    ├── shadow_dom_tests.rs
    └── wpt_runner.rs
```

---

## 🎯 Critérios de Sucesso

### Técnicos:
- ✅ Zero dependências externas para parsing HTML
- ✅ 95%+ WPT pass rate nas suites relevantes
- ✅ Performance 2-5x superior ao estado atual
- ✅ Memória 4x mais eficiente
- ✅ Custom Elements + Shadow DOM funcionais

### Funcionais:
- ✅ Lit.dev carrega e funciona
- ✅ Stencil apps funcionais
- ✅ Screen readers (NVDA, VoiceOver) operacionais
- ✅ Editores de texto ricos (Range/Selection)
- ✅ Web Components de bibliotecas modernas

### Processo:
- ✅ CI/CD com WPT automated
- ✅ Benchmarks contínuos
- ✅ Documentação completa
- ✅ Exemplos e tutorials

---

## 🚨 Riscos e Mitigações

| Risco | Probabilidade | Impacto | Mitigação |
|-------|--------------|---------|-----------|
| Regressões durante migração kuchiki→ACE | Alta | Alto | Testes extensivos, feature flags, rollback plan |
| Performance pior que o esperado | Média | Alto | Benchmarks contínuos, profiling semanal |
| Complexidade de Shadow DOM subestimada | Alta | Médio | Dividir em subtarefas menores,参考 Blink code |
| WPT fail rate alto | Média | Alto | Priorizar bugs críticos, iterative fixes |
| Burnout (projeto longo) | Alta | Crítico | Milestones curtos, celebrate wins, community |

---

## 📚 Referências

### Specs:
- [DOM Standard](https://dom.spec.whatwg.org/)
- [Shadow DOM](https://w3c.github.io/webcomponents/spec/shadow/)
- [Custom Elements](https://html.spec.whatwg.org/multipage/custom-elements.html)
- [Range](https://w3c.github.io/range-api/)
- [Selection](https://w3c.github.io/selection-api/)
- [ARIA 1.2](https://www.w3.org/TR/wai-aria-1.2/)
- [AccName 1.2](https://www.w3.org/TR/accname-1.2/)

### Code References:
- [Blink DOM](https://chromium.googlesource.com/chromium/blink/+/refs/heads/main/Source/core/dom/)
- [Gecko DOM](https://github.com/mozilla/gecko-dev/tree/master/dom/base)
- [WebKit DOM](https://github.com/WebKit/WebKit/tree/main/Source/WebCore/dom)
- [Servo DOM](https://github.com/servo/servo/tree/master/components/script/dom)

### Testing:
- [Web Platform Tests](https://github.com/web-platform-tests/wpt)
- [axe-core](https://github.com/dequelabs/axe-core)
- [JS Framework Benchmark](https://github.com/krausest/js-framework-benchmark)

---

## 💡 Notas Finais

Este plano é **ambicioso mas alcançável** com:
- **Foco incremental:** Uma feature por vez
- **Testes primeiro:** WPT como guia
- **Performance contínua:** Benchmarks em cada PR
- **Comunidade:** Engajar contributors early

**Visão:** AceDOM 2.0 será **mais rápido**, **mais leve**, e **mais compatível** que as implementações atuais, mantendo soberania tecnológica total (zero deps externas).

**Tempo estimado:** 18-24 meses (1 dev full-time)  
**LOC estimadas:** ~5.000 (vs 1.121 atuais)  
**Complexidade:** Alta, mas gerenciável com milestones claros

---

*Documento criado: Dezembro 2025*  
*Próxima revisão: Após FASE 1 completa (Meses 1-4)*
