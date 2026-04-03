# 🚀 Iniciando Implementação Fase 2: Recursos Avançados ACE-HTML

## ✅ Análise Completa do Estado Atual

### Estrutura do Projeto ACE-HTML

**Arquivos Principais:**
- `src/ace/html/mod.rs` - Módulo principal com structs básicas
- `src/ace/html/lexer.rs` - Tokenizer de baixo nível (3106 linhas, 80 estados)
- `src/ace/html/tokenizer.rs` - Wrapper do tokenizer
- `src/ace/html/tree_builder.rs` - Construtor da árvore DOM (2678 linhas)
- `src/ace/html/entities.json` - Entidades HTML

### O Que JÁ ESTÁ IMPLEMENTADO (Surpresa!)

#### 1. Template Element - Insertion Mode ✅
```rust
// Já existe em tree_builder.rs:
pub enum InsertionMode {
    // ... 24 modes incluindo:
    InTemplate,  // Linha 28
}

// Handler completo já implementado:
fn handle_in_template(&mut self, token: HtmlToken) -> Option<HtmlToken> {
    // Linhas 2136-2186 - Implementação completa!
    // - Handle start tags (base, link, meta, script, style, template, title)
    // - Handle table-related tags (caption, colgroup, tbody, tfoot, thead, col, tr, td, th)
    // - Handle end tag template
    // - Handle EOF
}
```

#### 2. Foreign Content - SVG/MathML ✅
```rust
// Namespace já definido:
pub enum Namespace {
    Html,
    Svg,
    MathMl,
}

// Handlers já implementados:
- handle_foreign_content()           // Linha 2396
- is_mathml_text_integration_point() // Linha 2474
- is_html_integration_point()        // Implementado
- adjust_svg_tag_name()              // Linha 445
- adjust_mathml_attributes()         // Linha 488
- adjust_svg_attributes()            // Linha 499
- adjust_foreign_attributes()        // Linha 567
```

#### 3. Testes Existentes ✅
```rust
// Teste de foreign content já existe:
fn test_svg_foreign_content() {  // Linha 2652
    // Testa: div -> svg -> circle -> foreignObject -> span
}
```

---

## ❌ O Que REALMENTE Falta (Gaps Identificados)

### Gap 1: Estruturas de Dados para Template Content
**Problema:** O insertion mode existe, mas não há estrutura `DocumentFragment` para armazenar o conteúdo do template separadamente.

**Solução Necessária:**
```rust
// PRECISA SER ADICIONADO ao mod.rs ou arquivo separado
pub struct DocumentFragment {
    pub nodes: Vec<HtmlNode>,
}

// E modificar HtmlElement para templates:
pub struct HtmlElement {
    pub tag: String,
    pub namespace: Namespace,
    pub attributes: HashMap<String, String>,
    pub children: Vec<HtmlNode>,
    pub template_content: Option<DocumentFragment>, // NOVO - apenas para <template>
}
```

### Gap 2: Slot Elements para Shadow DOM
**Problema:** Nenhum handling especial para `<slot>` elements.

**Solução:**
```rust
// Adicionar ao tree_builder.rs
fn handle_slot_element(&mut self, attrs: HashMap<String, String>) {
    // Parsing de <slot name="...">
    // Preparação para slot assignment
}
```

### Gap 3: Lista Completa de Elementos SVG
**Problema:** Apenas elementos básicos SVG estão na lista de ajuste.

**Solução:** Expandir `adjust_svg_tag_name()` para 100+ elementos

### Gap 4: Lista Completa de Elementos MathML
**Problema:** Apenas elementos básicos MathML suportados.

**Solução:** Expandir handling para todos elementos MathML3

### Gap 5: Custom Elements Validation
**Problema:** Nenhuma validação de nomes de custom elements.

**Solução:**
```rust
fn is_valid_custom_element_name(name: &str) -> bool {
    // Must contain hyphen
    // Must start with lowercase ASCII letter
    // Cannot be forbidden names
}
```

---

## 📋 Plano de Ação Imediato

### Dia 1: Estruturas de Dados Template

**Tarefa 1.1:** Criar DocumentFragment
```bash
# Arquivo: src/ace/html/mod.rs
# Adicionar após HtmlElement:
```

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentFragment {
    pub nodes: Vec<HtmlNode>,
}

impl DocumentFragment {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
        }
    }
    
    pub fn with_nodes(nodes: Vec<HtmlNode>) -> Self {
        Self { nodes }
    }
}
```

**Tarefa 1.2:** Modificar HtmlElement para suportar template content
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlElement {
    pub tag: String,
    pub namespace: Namespace,
    pub attributes: HashMap<String, String>,
    pub children: Vec<HtmlNode>,
    pub template_content: Option<DocumentFragment>, // APENAS para <template>
}
```

**Tarefa 1.3:** Atualizar métodos de HtmlElement
```rust
impl HtmlElement {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            namespace: Namespace::Html,
            attributes: HashMap::new(),
            children: Vec::new(),
            template_content: None,
        }
    }
    
    pub fn with_namespace(tag: impl Into<String>, ns: Namespace) -> Self {
        Self {
            tag: tag.into(),
            namespace: ns,
            attributes: HashMap::new(),
            children: Vec::new(),
            template_content: None,
        }
    }
}
```

### Dia 2: Integrar Template Content no Tree Builder

**Tarefa 2.1:** Modificar inserção de elemento template
```rust
// No tree_builder.rs, quando inserting "template":
if tag.name == "template" {
    let mut element = HtmlElement::with_namespace(tag.name, Namespace::Html);
    element.attributes = tag.attributes;
    element.template_content = Some(DocumentFragment::new());
    
    // Insert element...
    
    // Push current insertion mode to stack
    self.template_insertion_modes.push(self.insertion_mode);
    self.insertion_mode = InsertionMode::InTemplate;
}
```

**Tarefa 2.2:** Modificar handle_in_template para usar template_content
```rust
// Quando processando tokens dentro de template, adicionar ao template_content
// em vez de children normais
```

### Dia 3: Slot Elements

**Tarefa 3.1:** Adicionar handling para `<slot>` no tree_builder
```rust
// No handle_in_body ou função específica:
"slot" => {
    self.handle_slot_element(tag.attributes);
}
```

**Tarefa 3.2:** Implementar handle_slot_element
```rust
fn handle_slot_element(&mut self, mut attrs: HashMap<String, String>) {
    // Ajustar nome do slot se necessário
    let slot_name = attrs.remove("name");
    
    // Criar elemento slot
    let mut element = HtmlElement::new("slot");
    element.attributes = attrs;
    
    // Marcar para futuro slot assignment
    // (isso será usado pelo runtime Shadow DOM)
    
    self.insert_element(element);
}
```

### Dia 4: Expansão SVG

**Tarefa 4.1:** Expandir adjust_svg_tag_name
```rust
fn adjust_svg_tag_name(&self, tag: &mut String) {
    match tag.as_str() {
        // Existing adjustments...
        
        // Add all 100+ SVG elements
        "altglyph" => *tag = "altGlyph",
        "altglyphdef" => *tag = "altGlyphDef",
        "altglyphitem" => *tag = "altGlyphItem",
        "animatecolor" => *tag = "animateColor",
        "animatemotion" => *tag = "animateMotion",
        "animatetransform" => *tag = "animateTransform",
        "clippath" => *tag = "clipPath",
        "feblend" => *tag = "feBlend",
        "fecolormatrix" => *tag = "feColorMatrix",
        // ... (continuar com todos)
        _ => {}
    }
}
```

**Tarefa 4.2:** Expandir adjust_svg_attributes
```rust
fn adjust_svg_attributes(&self, attributes: &mut HashMap<String, String>) {
    let mut new_attrs = HashMap::new();
    
    for (key, value) in attributes.drain() {
        let adjusted_key = match key.as_str() {
            "gradientunits" => "gradientUnits",
            "viewbox" => "viewBox",
            "preservealpha" => "preserveAlpha",
            "preserveaspectratio" => "preserveAspectRatio",
            // ... all camelCase attrs
            _ => key.as_str(),
        }.to_string();
        
        new_attrs.insert(adjusted_key, value);
    }
    
    *attributes = new_attrs;
}
```

### Dia 5: Expansão MathML + Testes

**Tarefa 5.1:** Adicionar adjust_mathml_attributes completo
```rust
fn adjust_mathml_attributes(&self, attributes: &mut HashMap<String, String>) {
    // MathML attribute adjustments per spec
}
```

**Tarefa 5.2:** Criar testes para todas features novas
```bash
# Arquivo: tests/ace_html_conformance/phase2_tests.json
```

---

## 🎯 Métricas de Sucesso Imediatas

### Ao Final da Semana 1:
- [ ] DocumentFragment implementado
- [ ] Template content funcionando
- [ ] Slot elements parsed
- [ ] 100% testes template passing

### Ao Final da Semana 2:
- [ ] 100+ elementos SVG suportados
- [ ] Atributos SVG case-sensitive
- [ ] foreignObject completo

### Ao Final da Semana 3:
- [ ] Todos elementos MathML3
- [ ] Integration points completos
- [ ] 95%+ testes html5lib Phase 2

---

## ⚠️ Considerações Importantes

### 1. Backward Compatibility
- Manter compatibilidade com código existente
- Não quebrar testes atuais
- Adicionar campos opcionais quando possível

### 2. Performance
- DocumentFragment só deve ser alocado para `<template>`
- Usar Option<T> para campos especializados
- Evitar cloning desnecessário

### 3. Especificação WHATWG
- Seguir especificação exatamente
- Comentar referências no código
- Manter testes alinhados com html5lib-tests

---

## 🚀 Vamos Começar!

**Primeiro Comando:**
```bash
# Backup do estado atual
cp src/ace/html/mod.rs src/ace/html/mod.rs.backup

# Verificar compilação atual
cargo check --release
```

**Primeira Implementação:** Adicionar DocumentFragment ao mod.rs
