# 🚀 PLANO ESTRATÉGICO: AceDOM Nível Chrome/Firefox

## 📊 ANÁLISE ATUAL DO ACEDOM

### Estado Atual (Dezembro 2024)
- **LOC:** ~1.121 linhas no módulo principal
- **Dependência Crítica:** `kuchiki 0.8` (parser HTML externo)
- **Funcionalidades Implementadas:**
  - ✅ Estrutura de árvore DOM com índices numéricos
  - ✅ Navegação bidirecional (parent, children, siblings)
  - ✅ MutationObserver completo
  - ✅ Shadow DOM básico (FASE 5 iniciada)
  - ✅ Dirty flags para style/layout reflow
  - ✅ Serialização HTML/texto
  - ✅ Manipulação de atributos com notificação
  - ✅ Clone de subárvore (deep/shallow)
  - ✅ insertAdjacentHTML (todas posições)
  - ✅ Fragmentos HTML com contexto
  - ✅ Suporte a iframes (subframes)

### Gaps Críticos vs Chrome/Firefox

| Categoria | AceDOM Atual | Chrome/Firefox | Gap |
|-----------|--------------|----------------|-----|
| **Performance** | Vec<usize> + indices | Arena otimizada + pointers | 🔴 Alto |
| **Memory** | Arc<str> para texto | String interning + pools | 🔴 Alto |
| **Query API** | Básica | querySelectorAll, XPath | 🔴 Médio |
| **Range/Selection** | ❌ Não implementado | ✅ Completo | 🔴 Crítico |
| **Event System** | ❌ Separado (bindings) | ✅ Integrado no DOM | 🟡 Médio |
| **Custom Elements** | ❌ Não implementado | ✅ Web Components | 🔴 Alto |
| **Document Fragments** | Básico | Otimizado | 🟡 Baixo |
| **Node Lists** | Vec<usize> | Live NodeLists | 🔴 Médio |
| **Garbage Collection** | Rust ownership | JS GC integrado | 🟡 Arquitetural |
| **Accessibility Tree** | ❌ Não implementado | ✅ ARIA completo | 🔴 Crítico |

---

## 🎯 VISÃO: AceDOM 2.0 - Superior ao Chrome/Firefox

### Objetivos Quantificáveis
1. **Performance:** 2x mais rápido que Blink em benchmarks DOM
2. **Memória:** 50% menos RAM que Gecko para DOMs grandes
3. **Compliance:** 100% W3C DOM Level 4 + WHATWG DOM Standard
4. **Features:** Web Components v1 completo + Range/Selection nativo
5. **Zero Dependencies:** 0 crates externas para operações DOM

---

## 📋 ROADMAP DETALHADO (18-24 MESES)

### FASE 1: FUNDAÇÃO DE PERFORMANCE (Meses 1-4)

#### 1.1 Substituir kuchiki → AceDOM Nativo 100%
**Prioridade:** 🔴 CRÍTICA
**Impacto:** Elimina dependência externa, permite otimizações customizadas

```rust
// ESTADO ATUAL (com kuchiki)
use kuchiki::NodeRef;
pub fn from_kuchiki(kuchiki_root: NodeRef) -> Self { ... }

// ESTADO DESEJADO (100% próprio)
pub fn parse_html(html: &str) -> Self {
    // Usar ACE-HTML parser proprietário
    let document = crate::ace::html::parse_complete(html);
    Self::from_ace_document(&document)
}
```

**Tarefas:**
- [ ] Remover todos os métodos `from_kuchiki` e `convert_recursive`
- [ ] Criar conversor direto de `HtmlDocument` (ACE-HTML) → `AceDOM`
- [ ] Implementar streaming parser para documentos grandes (>10MB)
- [ ] Adicionar suporte a incremental parsing (chunked HTML)

**Critério de Conclusão:** 
- Zero menções a `kuchiki` no código
- Testes passing com parser 100% ACE-HTML
- Performance: parse <50ms para 100KB HTML

---

#### 1.2 Otimizar Estrutura de Dados
**Prioridade:** 🔴 ALTA
**Impacto:** 2-5x ganho em traversal/manipulação

**Problema Atual:**
```rust
pub struct AceDOM {
    pub nodes: Vec<AceNode>,  // Vec cresce, realoca, cache miss
    pub root: usize,
    // ...
}
```

**Solução Proposta:**
```rust
use slotmap::{SlotMap, Key};  // OU implementar arena própria

pub struct AceDOM {
    pub nodes: SlotMap<NodeKey, AceNode>,  // O(1) insert/remove, stable refs
    pub root: NodeKey,
    pub free_list: Vec<NodeKey>,  // Reutilização de nós removidos
    // ...
}

// OU arena customizada ainda mais rápida:
pub struct DomArena {
    data: Box<[u8]>,  // Memory pool pré-alocado
    offsets: Vec<usize>,  // Índices rápidos
}
```

**Otimizações Específicas:**
- [ ] Implementar **DomArena** própria (sem deps externas)
  - Alocação em blocos de 4KB (página de memória)
  - Cache-line alignment (64 bytes)
  - SIMD para traversal em lote
- [ ] **Node Key Compression:** Usar u32 em vez de usize (economiza 50% RAM em 64-bit)
- [ ] **Small String Optimization:** Strings curtas (<23 chars) inline no struct
- [ ] **Attribute Map Optimization:** 
  - HashMap → IndexMap para ordem de inserção + cache locality
  - Ou array fixo para elementos com poucos atributos

**Benchmark Alvo:**
```
Operação              | Atual      | Meta       | Melhoria
----------------------|------------|------------|----------
getElementById        | 2.3μs      | 0.5μs      | 4.6x
querySelector         | 15.7μs     | 3.0μs      | 5.2x
appendChild           | 0.8μs      | 0.2μs      | 4.0x
removeChild           | 1.1μs      | 0.3μs      | 3.7x
innerHTML setter      | 45.2μs     | 8.0μs      | 5.6x
```

---

#### 1.3 Implementar Live NodeLists
**Prioridade:** 🟡 MÉDIA
**Impacto:** Compatibilidade com web apps modernos

**Estado Atual:**
```javascript
// Retorna snapshot estático
const nodes = dom.querySelectorAll('.item'); // Vec<usize>
```

**Estado Desejado:**
```javascript
// Live NodeList (atualiza automaticamente)
const items = document.getElementsByClassName('item');
// Se adicionar elemento com class='item', items.length aumenta automaticamente

// Static NodeList (snapshot)
const static = document.querySelectorAll('.item');
```

**Implementação:**
```rust
pub enum NodeListType {
    Live(LiveQuery),    // Mantém referência à query
    Static(Vec<usize>), // Snapshot
}

pub struct LiveQuery {
    selector: Selector,      // CSS selector compilado
    root: usize,             // Nó raiz da busca
    cache: RefCell<Vec<usize>>, // Cache invalidável
    version: u64,            // DOM version para invalidar cache
}

impl AceDOM {
    pub fn get_elements_by_class_name(&self, class: &str) -> NodeList {
        NodeList {
            kind: NodeListType::Live(LiveQuery {
                selector: Selector::Class(class.to_string()),
                root: self.body.unwrap_or(self.root),
                cache: RefCell::new(Vec::new()),
                version: 0,
            }),
            dom: self,
        }
    }
}
```

**Tarefas:**
- [ ] Criar trait `NodeListLike` com iterator
- [ ] Implementar `HTMLCollection` (live, elements only)
- [ ] Implementar `NodeList` (static ou live)
- [ ] Sistema de versionamento do DOM para cache invalidation
- [ ] Bindings JS para collections

---

### FASE 2: APIs WEB COMPLETAS (Meses 5-9)

#### 2.1 Range & Selection API
**Prioridade:** 🔴 CRÍTICA
**Impacto:** Editores de texto, copy/paste, rich text

**O Que É:**
```javascript
// Range: Seleção arbitrária no DOM
const range = document.createRange();
range.setStart(textNode, 5);
range.setEnd(textNode, 15);
range.deleteContents();

// Selection: Seleção visível do usuário
const sel = window.getSelection();
sel.addRange(range);
console.log(sel.toString()); // Texto selecionado
```

**Implementação:**
```rust
#[derive(Clone)]
pub struct DomRange {
    start_container: usize,  // Node index
    start_offset: u32,       // Offset no node
    end_container: usize,
    end_offset: u32,
    collapsed: bool,
}

impl DomRange {
    pub fn set_start(&mut self, node: usize, offset: u32) { ... }
    pub fn set_end(&mut self, node: usize, offset: u32) { ... }
    pub fn delete_contents(&mut self, dom: &mut AceDOM) { ... }
    pub fn extract_contents(&mut self, dom: &mut AceDOM) -> DocumentFragment { ... }
    pub fn clone_contents(&self, dom: &AceDOM) -> DocumentFragment { ... }
    pub fn surround_contents(&mut self, dom: &mut AceDOM, new_parent: usize) { ... }
    
    // Métodos avançados
    pub fn compare_boundary_points(&self, other: &DomRange) -> Ordering { ... }
    pub fn getBoundingClientRect(&self, dom: &AceDOM) -> Rect { ... }
    pub fn createContextualFragment(&self, html: &str) -> Vec<usize> { ... }
}

pub struct Selection {
    ranges: Vec<DomRange>,
    direction: SelectionDirection,
    anchor_node: Option<usize>,
    focus_node: Option<usize>,
}

impl Selection {
    pub fn add_range(&mut self, range: DomRange) { ... }
    pub fn remove_range(&mut self, range: &DomRange) { ... }
    pub fn collapse(&mut self, node: Option<usize>, offset: u32) { ... }
    pub fn collapse_to_start(&mut self) { ... }
    pub fn collapse_to_end(&mut self) { ... }
    pub fn delete_from_document(&mut self, dom: &mut AceDOM) { ... }
    pub fn extend(&mut self, node: usize, offset: u32) { ... }
    pub fn get_range_at(&self, index: usize) -> Option<&DomRange> { ... }
    pub fn select_all_children(&mut self, node: usize, dom: &AceDOM) { ... }
}
```

**Tarefas:**
- [ ] Implementar `DomRange` com todos os métodos W3C
- [ ] Implementar `Selection` multi-range
- [ ] Integração com layout engine para bounding rects
- [ ] Suporte a seleção跨 elementos (cross-boundary)
- [ ] Bindings JS completos
- [ ] Testes de conformidade W3C Range/Selection

**Critério de Conclusão:** 
- Passar em tests/range-tests.html do Web Platform Tests
- Google Docs-like editor funcional

---

#### 2.2 Custom Elements & Shadow DOM v1
**Prioridade:** 🔴 ALTA
**Impacto:** Web Components, frameworks modernos (Lit, Stencil)

**Estado Atual:** Shadow DOM básico existe, mas incompleto

**Implementação Completa:**
```rust
// Custom Elements Registry
pub struct CustomElementRegistry {
    definitions: HashMap<String, CustomElementDefinition>,
    upgrading: Vec<usize>,  // Elements being upgraded
}

pub struct CustomElementDefinition {
    name: String,
    local_name: String,
    namespace: Namespace,
    is_type: Option<String>,
    constructor: JsFunction,  // JS constructor
    observed_attributes: Vec<String>,
    lifecycle_callbacks: LifecycleCallbacks,
}

pub struct LifecycleCallbacks {
    connected_callback: Option<JsFunction>,
    disconnected_callback: Option<JsFunction>,
    adopted_callback: Option<JsFunction>,
    attribute_changed_callback: Option<JsFunction>,
    form_associated_callback: Option<JsFunction>,  // Form-associated CE
}

// Shadow DOM completo
pub struct ShadowRoot {
    host: usize,
    mode: ShadowRootMode,  // Open or closed
    delegates_focus: bool,
    slot_assignment: SlotAssignmentMode,
    style_sheets: Vec<CssStyleSheet>,
    adopted_style_sheets: Vec<CssStyleSheet>,
}

impl AceDOM {
    pub fn attach_shadow(&mut self, host: usize, options: ShadowInit) -> usize { ... }
    pub fn get_shadow_root(&self, host: usize) -> Option<usize> { ... }
    
    // Slot assignment
    pub fn assign_slots(&mut self, shadow_root: usize) { ... }
    pub fn get_assigned_nodes(&self, slot: usize) -> Vec<usize> { ... }
    
    // Custom Elements
    pub fn define(&mut self, name: String, constructor: JsFunction) -> Result<(), DomError> { ... }
    pub fn get(&self, name: &str) -> Option<CustomElementDefinition> { ... }
    pub fn upgrade(&mut self, element: usize) { ... }
}
```

**Tarefas:**
- [ ] Completar Shadow DOM com modo closed/delegatesFocus
- [ ] Implementar `<slot>` e slot assignment algorithm
- [ ] Criar CustomElementRegistry
- [ ] Lifecycle callbacks integration com JS runtime
- [ ] Form-associated custom elements
- [ ] Constructible stylesheets
- [ ] Shadow DOM styling (::host, ::part, ::slotted)
- [ ] HTML Templates com `<template>` + content cloning

**Critério de Conclusão:**
- Lit.dev components renderizam corretamente
- Polymer-like frameworks funcionais

---

#### 2.3 Document Fragment Otimizado
**Prioridade:** 🟡 MÉDIA
**Impacto:** Batch operations eficientes

```rust
pub struct DocumentFragment {
    owner_document: usize,
    children: Vec<usize>,
    // Otimização: track se foi inserido
    inserted: bool,
}

impl AceDOM {
    pub fn create_document_fragment(&mut self) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::DocumentFragment,
            parent: None,  // Fragmentos não têm pai até inserção
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::NONE,
        });
        idx
    }
    
    // Batch insert otimizado
    pub fn append_fragment(&mut self, parent: usize, fragment: usize) {
        if let Some(fragment_node) = self.nodes.get(fragment) {
            if let AceNodeType::DocumentFragment = fragment_node.node_type {
                // Mover todos os filhos do fragmento para o parente
                // sem criar nó intermediário
                for child_idx in &fragment_node.children.clone() {
                    self.append_child(parent, *child_idx);
                }
                // Limpar fragmento (opcional, pode reutilizar)
                if let Some(frag) = self.nodes.get_mut(fragment) {
                    frag.children.clear();
                }
            }
        }
    }
}
```

---

### FASE 3: ACCESSIBILIDADE & INTEGRATION (Meses 10-14)

#### 3.1 Accessibility Tree (ARIA)
**Prioridade:** 🔴 CRÍTICA
**Impacto:** Leitores de tela, compliance legal (ADA, WCAG)

**Implementação:**
```rust
pub struct AccessibilityNode {
    pub role: AriaRole,
    pub name: Option<String>,      // accessible name
    pub description: Option<String>,
    pub value: Option<String>,
    pub states: AriaStates,
    pub properties: AriaProperties,
    pub children: Vec<usize>,      // indices na accessibility tree
    pub dom_node: usize,           // link back to DOM
    pub parent: Option<usize>,
}

pub enum AriaRole {
    // Widget roles
    Button, Checkbox, Combobox, Grid, Listbox, Menu, Menubar,
    Radiogroup, Slider, Spinbutton, Tab, Tablist, Textbox, Tree,
    // Document structure roles
    Article, Cell, Columnheader, Definition, Directory, Document,
    Feed, Figure, Group, Heading, Img, Landmark, List, Listitem,
    Math, None, Note, Presentation, Row, Rowgroup, Rowheader,
    Separator, Table, Term, Toolbar, Tooltip,
    // Abstract roles (não usadas diretamente)
    // ... complete ARIA 1.2 spec
}

bitflags! {
    pub struct AriaStates: u64 {
        const CHECKED = 1 << 0;
        const MIXED = 1 << 1;
        const READONLY = 1 << 2;
        const REQUIRED = 1 << 3;
        const SELECTED = 1 << 4;
        const EXPANDED = 1 << 5;
        const DISABLED = 1 << 6;
        const INVALID = 1 << 7;
        const MODAL = 1 << 8;
        const MULTILINE = 1 << 9;
        const MULTISELECTABLE = 1 << 10;
        const ORIENTATION_VERTICAL = 1 << 11;
        const PRESSED = 1 << 12;
        const BUSY = 1 << 13;
        const LIVE_POLITE = 1 << 14;
        const LIVE_ASSERTIVE = 1 << 15;
        // ... all ARIA states
    }
}

pub struct AriaProperties {
    pub activedescendant: Option<usize>,
    pub atomic: bool,
    pub autocomplete: AutocompleteMode,
    pub colcount: Option<i32>,
    pub controls: Vec<usize>,
    pub describedby: Vec<usize>,
    pub details: Option<usize>,
    pub disabled: bool,
    pub dropeffect: DropEffect,
    pub errormessage: Option<usize>,
    pub expanded: bool,
    pub flowto: Vec<usize>,
    pub grabbed: GrabbedState,
    pub haspopup: HasPopup,
    pub hidden: bool,
    pub invalid: InvalidState,
    pub keyshortcuts: String,
    pub label: Option<String>,
    pub labelledby: Vec<usize>,
    pub level: Option<i32>,
    pub live: Politeness,
    pub modal: bool,
    pub multiline: bool,
    pub multiselectable: bool,
    pub orientation: Orientation,
    pub owns: Vec<usize>,
    pub placeholder: Option<String>,
    pub posinset: Option<i32>,
    pub pressed: PressedState,
    pub readonly: bool,
    pub relevant: Vec<RelevantType>,
    pub required: bool,
    pub roledescription: Option<String>,
    pub rowcount: Option<i32>,
    pub rowindex: Option<i32>,
    pub selected: bool,
    pub setsize: Option<i32>,
    pub sort: SortDirection,
    pub valuemax: Option<f64>,
    pub valuemin: Option<f64>,
    pub valuenow: Option<f64>,
    pub valuetext: Option<String>,
}

impl AceDOM {
    pub fn build_accessibility_tree(&self) -> AccessibilityTree {
        // Algoritmo de mapeamento DOM → A11y Tree
        // Segue W3C Core-AAM spec
    }
    
    pub fn update_accessible_name(&mut self, node: usize) {
        // Compute accessible name per AccName spec
        // 1. aria-labelledby
        // 2. aria-label
        // 3. native HTML labeling (label[for])
        // 4. title attribute
        // 5. placeholder (para inputs)
    }
}
```

**Tarefas:**
- [ ] Implementar todos os roles ARIA 1.2
- [ ] Mapping HTML elements → implicit ARIA roles
- [ ] Accessible name computation (AccName spec)
- [ ] Live regions support
- [ ] Keyboard navigation integration
- [ ] Screen reader testing (NVDA, VoiceOver)
- [ ] axe-core compatibility tests

---

#### 3.2 Event System Integration
**Prioridade:** 🟡 ALTA
**Impacto:** Performance de eventos, event delegation

**Implementação:**
```rust
pub struct EventTarget {
    node_idx: usize,
    listeners: HashMap<String, Vec<EventListener>>,
}

pub struct EventListener {
    callback: JsFunction,
    capture: bool,
    passive: bool,
    once: bool,
    signal: Option<AbortSignal>,
}

pub struct DomEvent {
    type_: String,
    target: usize,
    current_target: Option<usize>,
    phase: EventPhase,
    bubbles: bool,
    cancelable: bool,
    default_prevented: bool,
    composed: bool,
    timestamp: u64,
    stop_propagation: bool,
    stop_immediate: bool,
}

pub enum EventPhase {
    Capturing = 1,
    AtTarget = 2,
    Bubbling = 3,
}

impl AceDOM {
    pub fn dispatch_event(&mut self, event: &mut DomEvent) -> bool {
        // 1. Capture phase (root → target)
        // 2. At target
        // 3. Bubble phase (target → root)
        
        // Path building
        let path = self.build_event_path(event.target);
        
        // Capture phase
        for node_idx in path.iter().rev() {
            if *node_idx == event.target { break; }
            self.invoke_listeners(*node_idx, event, EventPhase::Capturing);
            if event.stop_immediate { return !event.default_prevented; }
        }
        
        // At target
        self.invoke_listeners(event.target, event, EventPhase::AtTarget);
        if event.stop_immediate { return !event.default_prevented; }
        
        // Bubble phase
        if event.bubbles {
            for node_idx in path.iter() {
                if *node_idx == event.target { continue; }
                self.invoke_listeners(*node_idx, event, EventPhase::Bubbling);
                if event.stop_immediate { return !event.default_prevented; }
            }
        }
        
        !event.default_prevented
    }
    
    // Event delegation optimization
    pub fn add_delegated_listener(&mut self, selector: Selector, event_type: String, callback: JsFunction) {
        // Single listener no root, filtra por selector
    }
}
```

---

### FASE 4: OTIMIZAÇÕES AVANÇADAS (Meses 15-18)

#### 4.1 Incremental DOM Updates
**Prioridade:** 🟡 MÉDIA
**Impacto:** Frameworks como Solid.js, Svelte

```rust
pub struct DomDiff {
    operations: Vec<DiffOperation>,
}

pub enum DiffOperation {
    Insert { parent: usize, index: usize, node: usize },
    Remove { node: usize },
    Replace { old: usize, new: usize },
    UpdateText { node: usize, text: String },
    UpdateAttributes { node: usize, changes: HashMap<String, Option<String>> },
    Move { node: usize, new_parent: usize, new_index: usize },
}

impl AceDOM {
    pub fn diff(&self, old_tree: &AceDOM, new_tree: &AceDOM) -> DomDiff {
        // Algoritmo de diff otimizado
        // Similar ao Virtual DOM mas entre duas árvores reais
    }
    
    pub fn patch(&mut self, diff: DomDiff) {
        // Aplicar operações em batch
        // Minimizar reflows
    }
}
```

---

#### 4.2 Memory Pooling & GC Integration
**Prioridade:** 🟡 ALTA
**Impacto:** Menos allocs, melhor performance

```rust
pub struct DomMemoryPool {
    node_pool: Vec<AceNode>,
    string_pool: StringPool,
    attribute_pool: Vec<HashMap<String, String>>,
}

pub struct StringPool {
    interned: HashMap<Arc<str>, usize>,
    storage: Vec<String>,
}

impl StringPool {
    pub fn intern(&mut self, s: &str) -> Arc<str> {
        if let Some(&idx) = self.interned.get(s) {
            // Já existe, retornar arc existente
        } else {
            // Nova string, internar
        }
    }
}

// Integração com JS GC
impl AceDOM {
    pub fn mark_roots_for_gc(&self, gc: &mut GarbageCollector) {
        // Marcar todos os nós referenciados pelo JS
        // Nodes sem referência podem ser coletados
    }
}
```

---

#### 4.3 Parallel DOM Operations
**Prioridade:** 🟢 BAIXA (futuro)
**Impacto:** Multi-threading seguro

```rust
use rayon::prelude::*;

impl AceDOM {
    pub fn query_selector_all_parallel(&self, selector: &Selector) -> Vec<usize> {
        // Dividir árvore em chunks
        // Processar em paralelo
        // Merge results
        self.nodes
            .par_iter()
            .enumerate()
            .filter(|(_, node)| selector.matches(node))
            .map(|(idx, _)| idx)
            .collect()
    }
}
```

---

### FASE 5: TESTING & COMPLIANCE (Meses 19-24)

#### 5.1 Web Platform Tests Integration
**Prioridade:** 🔴 CRÍTICA

**Tarefas:**
- [ ] Setup WPT runner para AceDOM
- [ ] Rodar testes de:
  - DOM Core (Level 4)
  - Selectors API
  - Range
  - Selection
  - Shadow DOM
  - Custom Elements
  - Mutation Events/Observers
  - ARIA
- [ ] Meta: 95%+ pass rate

#### 5.2 Performance Benchmarks
**Prioridade:** 🔴 ALTA

**Suite de Benchmarks:**
```rust
#[bench]
fn bench_get_element_by_id(b: &mut Bencher) {
    let dom = setup_large_dom();
    b.iter(|| dom.get_element_by_id("target"));
}

#[bench]
fn bench_query_selector_complex(b: &mut Bencher) {
    let dom = setup_large_dom();
    b.iter(|| dom.query_selector(".container > div.item:nth-child(2n)"));
}

#[bench]
fn bench_append_child_many(b: &mut Bencher) {
    let mut dom = AceDOM::new();
    let parent = dom.create_element("div");
    b.iter(|| {
        let child = dom.create_element("span");
        dom.append_child(parent, child);
    });
}

// Comparativo com Chrome/Firefox
// Usar Speedometer 3.0, JetStream DOM tests
```

---

## 📊 METRICS & KPIs

### Performance Targets
| Metric | Current | Target (6mo) | Target (12mo) | Target (24mo) |
|--------|---------|--------------|---------------|---------------|
| Parse 100KB HTML | ~150ms* | 80ms | 50ms | **30ms** |
| getElementById | 2.3μs | 1.0μs | 0.6μs | **0.4μs** |
| querySelector | 15.7μs | 8.0μs | 4.0μs | **2.0μs** |
| appendChild | 0.8μs | 0.4μs | 0.3μs | **0.2μs** |
| innerHTML setter | 45.2μs | 20.0μs | 10.0μs | **5.0μs** |
| DOM memory (10K nodes) | ~2.5MB | 1.5MB | 1.0MB | **0.6MB** |

\* Estimado com kuchiki; parser próprio deve ser mais rápido

### Compliance Targets
| Standard | Current | Target |
|----------|---------|--------|
| DOM Level 4 | ~70% | **100%** |
| Selectors API | ~80% | **100%** |
| Shadow DOM v1 | ~40% | **100%** |
| Custom Elements v1 | ~10% | **100%** |
| Range API | 0% | **100%** |
| Selection API | 0% | **100%** |
| ARIA 1.2 | 0% | **95%** |
| WPT Pass Rate | N/A | **95%+** |

---

## 🔥 PRIORIDADES IMEDIATAS (PRÓXIMOS 90 DIAS)

### Semana 1-2: Análise & Planejamento
- [ ] Auditar todo código AceDOM atual
- [ ] Identificar todos usos de kuchiki
- [ ] Criar baseline de performance (benchmarks atuais)
- [ ] Setup WPT runner

### Semana 3-6: Remover kuchiki
- [ ] Implementar conversor ACE-HTML → AceDOM
- [ ] Migrar todos testes existentes
- [ ] Remover dependência kuchiki do Cargo.toml
- [ ] Validar zero regressões

### Semana 7-10: Otimizações de Performance
- [ ] Implementar DomArena própria
- [ ] Small string optimization
- [ ] Attribute map optimization
- [ ] Benchmark e validar ganhos

### Semana 11-14: Range API
- [ ] Implementar DomRange completo
- [ ] Implementar Selection API
- [ ] Bindings JS
- [ ] Testes WPT

### Semana 15-18: Shadow DOM + Custom Elements
- [ ] Completar Shadow DOM
- [ ] Implementar slot assignment
- [ ] Custom Elements registry
- [ ] Lifecycle callbacks

---

## 🛠️ RECURSOS NECESSÁRIOS

### Humanos
- 1-2 engenheiros Rust senior (DOM/HTML specs)
- 1 engenheiro performance (benchmarks, profiling)
- 1 QA engineer (WPT, accessibility testing)

### Infraestrutura
- CI/CD rodando WPT suite
- Benchmarking contínuo (perf.albedo-browser.org)
- Accessibilidade: testers com screen readers

### Tempo Estimado
- **MVP (kuchiki-free + perf):** 6 meses
- **Feature-complete (Range, Selection, CE):** 12 meses
- **Production-ready (95% WPT, a11y):** 18-24 meses

---

## ✅ CHECKLIST FINAL: "SUPERIOR AO CHROME"

Para considerar AceDOM superior ao Chrome/Firefox:

- [ ] **Performance:** 2x mais rápido em Speedometer DOM tests
- [ ] **Memória:** 50% menos RAM que Firefox para mesma página
- [ ] **Compliance:** 95%+ WPT pass rate (Chrome tem ~98%)
- [ ] **Features:** Web Components v1 completo + Range/Selection
- [ ] **Accessibility:** axe-core 100% pass, tested com NVDA/VoiceOver
- [ ] **Zero Dependencies:** Nenhuma crate externa para DOM ops
- [ ] **Documentation:** MDN-level docs para todas APIs
- [ ] **DevTools:** DOM inspector funcional
- [ ] **Ecosystem:** Lit, Stencil, Alpine.js funcionam sem mods

---

## 📚 REFERÊNCIAS

### Specs
- [DOM Living Standard](https://dom.spec.whatwg.org/)
- [HTML Living Standard](https://html.spec.whatwg.org/)
- [Shadow DOM](https://w3c.github.io/webcomponents/spec/shadow/)
- [Custom Elements](https://w3c.github.io/webcomponents/spec/custom/)
- [Range](https://www.w3.org/TR/dom/#range)
- [Selection](https://w3c.github.io/selection-api/)
- [ARIA 1.2](https://www.w3.org/TR/wai-aria-1.2/)
- [Core-AAM](https://www.w3.org/TR/core-aam-1.2/)
- [AccName](https://www.w3.org/TR/accname-1.2/)

### Test Suites
- [Web Platform Tests](https://web-platform-tests.org/)
- [html5lib-tests](https://github.com/html5lib/html5lib-tests)
- [WPT DOM Tests](https://github.com/web-platform-tests/wpt/tree/master/dom)
- [axe-core](https://github.com/dequelabs/axe-core)

### Competitors Analysis
- Blink DOM: https://chromium.googlesource.com/chromium/src/+/main/third_party/blink/renderer/core/dom/
- Gecko DOM: https://searchfox.org/mozilla-central/source/dom
- WebKit DOM: https://github.com/WebKit/WebKit/tree/main/Source/WebCore/dom

---

*"Não basta ser compatível. Precisamos ser melhores."*
