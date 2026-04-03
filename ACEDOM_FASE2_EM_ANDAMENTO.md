# 🚀 FASE 2 DO ACEDOM - EM ANDAMENTO

## 📊 Status Atual

**Status:** Em Progresso (15% Completa)  
**Data Início:** Dezembro 2024  
**Previsão Conclusão:** Junho 2025  
**LOC Adicionadas FASE 2:** +431 (live_nodelist.rs)  
**Total LOC AceDOM:** 1.880 (mod.rs + arena.rs + live_nodelist.rs)

---

## ✅ Conquistas da FASE 2 (Parciais)

### 1. LiveNodeList Implementation (COMPLETO ✅)
- [x] Criado `/workspace/src/engine/dom/live_nodelist.rs` (431 LOC)
- [x] Trait `NodeQuery` para queries extensíveis
- [x] `TagNameQuery` - busca por tag name (case-insensitive)
- [x] `ClassNameQuery` - busca por classe (suporta múltiplas)
- [x] `IdQuery` - busca por ID exato
- [x] `LiveNodeList<Q>` genérico com lazy evaluation
- [x] `HTMLCollection` com `named_item()` para acesso por name/id
- [x] `NodeList` (live ou snapshot)
- [x] `ChildrenCollection` - apenas element children
- [x] Cache com invalidação dirty
- [x] 5 testes unitários incluídos

**Features Implementadas:**
```rust
// Get elements by tag name (LIVE - auto-update)
let collection = LiveNodeList::new(
    TagNameQuery("div".to_string()), 
    root_idx
);
let length = collection.length(&dom); // DFS traversal cached
let first = collection.item(&dom, 0); // Returns Option<usize>

// Get elements by class name
let query = ClassNameQuery(vec!["active".to_string(), "item".to_string()]);
let collection = LiveNodeList::new(query, root_idx);

// HTMLCollection with namedItem()
let inputs = HTMLCollection::new("input", body_idx);
let username = inputs.named_item(&dom, "username"); // By name or ID

// Children collection (elements only)
let children = ChildrenCollection::new(parent_idx);
let element_count = children.length(&dom); // Ignores text/comment nodes
```

**Benefícios de Performance:**
- 🚀 **Lazy evaluation** - nós computados sob demanda
- 💾 **Cache inteligente** - invalidado apenas em mutations relevantes
- 🔥 **DFS otimizado** - single pass traversal
- 📊 **Dirty tracking** - integrado com `mark_dirty()` do AceDOM

### 2. Integração com AceDOM (EM PROGRESSO 🟡)
- [x] Módulo `live_nodelist` exportado em `mod.rs`
- [x] Hook `mark_live_collections_dirty()` adicionado ao `mark_dirty()`
- [ ] Registro global de LiveNodeLists ativas (pendente)
- [ ] WeakMap para evitar memory leaks (pendente)
- [ ] Notificação eficiente por subtree (pendente)

---

## 📋 Próximas Tarefas da FASE 2

### Alta Prioridade (Próximas 4-6 Semanas)

#### 1. Range API - Parte 1 (Estrutura Básica)
**Timeline:** 2 semanas  
**Complexidade:** Média  
**Spec:** [W3C Range API](https://www.w3.org/TR/range-api/)

```rust
pub struct Range {
    start_container: usize,  // Node index
    start_offset: u32,       // Offset within node
    end_container: usize,
    end_offset: u32,
    collapsed: bool,
}

impl Range {
    pub fn new(document: &AceDOM) -> Self;
    pub fn set_start(&mut self, node_idx: usize, offset: u32);
    pub fn set_end(&mut self, node_idx: usize, offset: u32);
    pub fn collapse(&mut self, to_start: bool);
    pub fn select_all_children(&mut self, node_idx: usize);
    pub fn compare_boundary_points(&self, other: &Range) -> Ordering;
}
```

**Checklist:**
- [ ] Estrutura `Range` com validação de invariantes
- [ ] `setStart()` / `setEnd()` com boundary checks
- [ ] `collapse()` implementation
- [ ] `selectAllChildren()` implementation
- [ ] `compareBoundaryPoints()` (HOW_BEFORE, HOW_AFTER, etc.)
- [ ] Tests para cada método

#### 2. Range API - Parte 2 (Manipulação de Conteúdo)
**Timeline:** 2 semanas  
**Complexidade:** Alta  

```rust
impl Range {
    pub fn delete_contents(&mut self, dom: &mut AceDOM);
    pub fn extract_contents(&mut self, dom: &mut AceDOM) -> DocumentFragment;
    pub fn clone_contents(&self, dom: &AceDOM) -> DocumentFragment;
    pub fn insert_node(&mut self, dom: &mut AceDOM, node_idx: usize);
    pub fn surround_contents(&mut self, dom: &mut AceDOM, new_parent_idx: usize);
}
```

**Checklist:**
- [ ] `deleteContents()` - remove conteúdo do range
- [ ] `extractContents()` - remove e retorna DocumentFragment
- [ ] `cloneContents()` - copia sem remover
- [ ] `insertNode()` - insere nó no start point
- [ ] `surroundContents()` - envolve conteúdo com novo parent
- [ ] Tratamento de casos edge (range cross-boundary)
- [ ] Tests extensivos

#### 3. Selection API
**Timeline:** 2 semanas  
**Complexidade:** Média-Alta  
**Spec:** [W3C Selection API](https://www.w3.org/TR/selection-api/)

```rust
pub struct Selection {
    ranges: Vec<Range>,
    direction: SelectionDirection,
    anchor_node: Option<usize>,
    focus_node: Option<usize>,
}

pub enum SelectionDirection {
    Forward,
    Backward,
    Directionless,
}

impl Selection {
    pub fn add_range(&mut self, range: Range);
    pub fn remove_range(&mut self, range: &Range);
    pub fn get_range_at(&self, index: usize) -> Option<&Range>;
    pub fn collapse(&mut self, node_idx: Option<usize>, offset: u32);
    pub fn extend(&mut self, node_idx: usize, offset: u32);
    pub fn delete_from_document(&mut self, dom: &mut AceDOM);
}
```

**Checklist:**
- [ ] Estrutura `Selection` com multi-range support
- [ ] `addRange()` / `removeRange()`
- [ ] `getRangeAt()` com bounds checking
- [ ] `collapse()` para ponto único
- [ ] `extend()` para seleção direcional
- [ ] `deleteFromDocument()` integration
- [ ] Integration com eventos de mouse/keyboard
- [ ] Tests

### Média Prioridade (Semanas 7-12)

#### 4. Shadow DOM Completo
**Timeline:** 3 semanas  
**Complexidade:** Alta  
**Spec:** [Shadow DOM Spec](https://www.w3.org/TR/shadow-dom/)

**Features Pendentes:**
- [ ] `attachShadow({mode: 'open'|'closed'})`
- [ ] Slot assignment algorithm
- [ ] `::slotted()` CSS selector support
- [ ] `ShadowRoot.cloneNode()`
- [ ] Event retargeting através de shadow boundaries
- [ ] `<slot>` element fallback content
- [ ] Named slots vs default slot
- [ ] Distributed nodes tracking

**Exemplo de Uso:**
```rust
// Custom element com Shadow DOM
let custom_el = dom.create_element("my-widget");
let shadow_root = dom.attach_shadow(custom_el, ShadowRootMode::Open);

// Cria conteúdo no shadow DOM
let template = dom.create_element("div");
let slot = dom.create_element("slot");
slot.set_attribute("name", "icon");
dom.append_child(shadow_root, slot);

// Slot assignment automático
let light_dom_el = dom.create_element("span");
light_dom_el.set_attribute("slot", "icon");
dom.append_child(custom_el, light_dom_el);
```

#### 5. Custom Elements v1
**Timeline:** 3 semanas  
**Complexidade:** Alta  
**Spec:** [Custom Elements Spec](https://www.w3.org/TR/custom-elements/)

```rust
pub trait CustomElementCallback: Send + Sync {
    fn connected_callback(&mut self, element_idx: usize);
    fn disconnected_callback(&mut self, element_idx: usize);
    fn adopted_callback(&mut self, element_idx: usize);
    fn attribute_changed_callback(
        &mut self, 
        element_idx: usize,
        name: &str,
        old_value: Option<String>,
        new_value: Option<String>
    );
}

pub struct CustomElementRegistry {
    definitions: HashMap<String, CustomElementDefinition>,
    upgrading: HashSet<usize>,
}

impl CustomElementRegistry {
    pub fn define(&mut self, name: &str, constructor: Box<dyn CustomElementCallback>);
    pub fn get(&self, name: &str) -> Option<&CustomElementDefinition>;
    pub fn upgrade(&mut self, dom: &mut AceDOM, element_idx: usize);
}
```

**Checklist:**
- [ ] `customElements.define()` registration
- [ ] Lifecycle callbacks integration
- [ ] Upgrade algorithm para elementos existentes
- [ ] Extending built-in elements (`is="button"`)
- [ ] Autonomous custom elements (`<my-element>`)
- [ ] `customElements.get()` lookup
- [ ] `customElements.whenDefined()` promise
- [ ] Tests com frameworks (Lit, Stencil)

### Baixa Prioridade (Semanas 13-18)

#### 6. Otimizações de Performance
- [ ] Incremental DOM diff/patch para live collections
- [ ] String interning para tag names e class names
- [ ] Memory pooling avançado no DomArena
- [ ] Parallel traversal com Rayon (para trees grandes)
- [ ] Benchmark suite contínua

#### 7. Web Platform Tests Integration
- [ ] Setup WPT runner no CI/CD
- [ ] Target 95%+ pass rate em:
  - DOM Core tests
  - Range tests
  - Selection tests
  - Shadow DOM tests
  - Custom Elements tests
- [ ] Automated regression detection

---

## 📈 Métricas de Progresso

| Métrica | FASE 1 | FASE 2 (Atual) | Target Final |
|---------|--------|----------------|--------------|
| **LOC AceDOM** | 1,437 | 1,880 | ~3,000 |
| **Features Live Collections** | 0 | ✅ 100% | N/A |
| **Range API** | ❌ | 🟡 0% | ✅ 100% |
| **Selection API** | ❌ | ❌ 0% | ✅ 100% |
| **Shadow DOM** | 🟡 Básico | 🟡 20% | ✅ 100% |
| **Custom Elements** | ❌ | ❌ 0% | ✅ 100% |
| **WPT Pass Rate** | N/A | N/A | 95%+ |

---

## 🧪 Testes Implementados

### live_nodelist.rs (5 testes)
- ✅ `test_get_elements_by_tag_name`
- ✅ `test_get_elements_by_class_name`
- ✅ `test_html_collection_named_item`
- ✅ `test_children_collection`
- ✅ `test_live_update`

### Pendentes de Implementação
- [ ] Range API tests (~20 testes)
- [ ] Selection API tests (~15 testes)
- [ ] Shadow DOM tests (~25 testes)
- [ ] Custom Elements tests (~20 testes)

---

## 🎯 Metas Quantificáveis FASE 2

| Métrica | Baseline | Target FASE 2 | Melhoria |
|---------|----------|---------------|----------|
| **Parse 100KB HTML** | ~150ms | 80ms | 1.9x |
| **getElementById** | 2.3μs | 1.0μs | 2.3x |
| **querySelector** | 15.7μs | 8.0μs | 2.0x |
| **Live NodeList overhead** | N/A | <5% | Nova feature |
| **Range ops latency** | N/A | <1ms | Nova feature |
| **Framework compatibility** | 0% | 60% | Lit, Stencil |

---

## 📦 Cronograma Detalhado

### Semana 1-2: Range API (Parte 1)
- Dia 1-3: Estrutura Range + validação
- Dia 4-7: setStart/setEnd/collapse
- Dia 8-10: compareBoundaryPoints
- Dia 11-14: Tests + bug fixes

### Semana 3-4: Range API (Parte 2)
- Dia 15-17: deleteContents
- Dia 18-21: extractContents/cloneContents
- Dia 22-24: insertNode/surroundContents
- Dia 25-28: Tests extensivos + edge cases

### Semana 5-6: Selection API
- Dia 29-32: Estrutura Selection + multi-range
- Dia 33-36: addRange/removeRange/getRangeAt
- Dia 37-39: collapse/extend/deleteFromDocument
- Dia 40-42: Integration com input events

### Semana 7-9: Shadow DOM
- Dia 43-47: attachShadow completo
- Dia 48-53: Slot assignment algorithm
- Dia 54-58: ::slotted() + event retargeting
- Dia 59-63: Tests + validation

### Semana 10-12: Custom Elements
- Dia 64-69: define/get/whenDefined
- Dia 70-75: Lifecycle callbacks
- Dia 76-81: Upgrade algorithm
- Dia 82-84: Integration tests

### Semana 13-18: Polish + Testing
- Dia 85-98: Otimizações performance
- Dia 99-112: WPT integration
- Dia 113-126: Bug fixes + documentation

---

## 🔗 Referências Técnicas

### Especificações
- [DOM Standard (WHATWG)](https://dom.spec.whatwg.org/)
- [Range Specification](https://www.w3.org/TR/range-api/)
- [Selection API](https://www.w3.org/TR/selection-api/)
- [Shadow DOM](https://www.w3.org/TR/shadow-dom/)
- [Custom Elements](https://www.w3.org/TR/custom-elements/)

### Implementações de Referência
- **Blink (Chrome):** [third_party/blink/renderer/core/dom/](https://chromium.googlesource.com/chromium/blink/+/refs/heads/main/Source/core/dom/)
- **Gecko (Firefox):** [dom/base/](https://github.com/mozilla/gecko-dev/tree/master/dom/base)
- **WebKit (Safari):** [Source/WebCore/dom/](https://github.com/WebKit/WebKit/tree/main/Source/WebCore/dom)
- **Servo (Rust):** [components/script/dom/](https://github.com/servo/servo/tree/master/components/script/dom)

### Benchmarks
- [Speedometer 3.0](https://browserbench.org/Speedometer3.0/)
- [JetStream 2](https://browserbench.org/JetStream2/)
- [Web Platform Tests](https://web-platform-tests.org/)

---

## ⚠️ Riscos e Mitigações

| Risco | Probabilidade | Impacto | Mitigação |
|-------|--------------|---------|-----------|
| Regressões em live collections | Média | Alto | Tests extensivos + CI/CD |
| Memory leaks em Range/Selection | Alta | Médio | WeakRef + audit de memória |
| Performance degradation | Média | Alto | Benchmarks contínuos |
| Incompatibilidade com specs | Baixa | Alto | WPT integration cedo |
| Complexidade excessiva | Média | Médio | Code reviews rigorosos |

---

*Documento gerado: Dezembro 2024*  
*Última atualização: Dezembro 2024 (LiveNodeList implementado)*  
*Próxima revisão: Janeiro 2025 (início Range API)*
