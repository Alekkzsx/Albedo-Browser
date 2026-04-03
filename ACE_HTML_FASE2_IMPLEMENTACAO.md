# Implementação Fase 2: Recursos Avançados ACE-HTML

## Visão Geral

**Objetivo:** Implementar recursos avançados de parsing HTML para atingir compatibilidade completa com navegadores modernos.

**Duração Estimada:** 4 semanas
**Status:** 🟢 INICIANDO

---

## 2.1 Template Element Completo

### Especificação WHATWG Referenciada
- Seção 13.2.5.4.7: "In template" insertion mode
- Seção 13.5.2: Template element behavior
- HTML5.3+ Template specification

### Sub-tarefas

#### 2.1.1 Template Insertion Mode [SEMANA 1]
- [ ] Implementar estado `InsertionMode::InTemplate`
- [ ] Stack de template insertion modes (já existe, expandir)
- [ ] Handle de start tags dentro de template
- [ ] Handle de end tags dentro de template
- [ ] Handle de character tokens dentro de template
- [ ] Handle de comments dentro de template
- [ ] Handle de DOCTYPE dentro de template
- [ ] Handle de EOF dentro de template

**Arquivos para Modificar:**
- `src/ace/html/tree_builder.rs` - Adicionar InsertionMode::InTemplate
- `src/ace/html/tree_builder.rs` - Implementar handle_in_template()

**Código Esperado:**
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InsertionMode {
    // ... existing modes ...
    InTemplate,
}

impl TreeBuilder {
    fn handle_in_template(&mut self, token: Token) {
        match token {
            Token::StartTag { name, .. } => {
                // Handle all start tags according to spec
                self.handle_start_tag_in_template(name);
            }
            Token::EndTag { name, .. } => {
                // Handle end tags
                self.handle_end_tag_in_template(name);
            }
            Token::Character(data) => {
                // Process characters in template
                self.insert_character(data);
            }
            // ... other tokens
        }
    }
}
```

#### 2.1.2 Template Content DocumentFragment [SEMANA 1-2]
- [ ] Criar estrutura TemplateContent como DocumentFragment
- [ ] Separar parsing do conteúdo do template
- [ ] Implementar serialização de template content
- [ ] Manter pointer para template owner

**Estrutura de Dados:**
```rust
pub struct TemplateElement {
    pub base: HTMLElement,
    pub content: DocumentFragment,
    pub shadow_root: Option<ShadowRoot>, // Para Fase 2.2
}

pub struct DocumentFragment {
    pub nodes: Vec<NodeHandle>,
    pub document: DocumentHandle,
    pub host: Option<NodeHandle>,
}
```

#### 2.1.3 Edge Cases de Template [SEMANA 2]
- [ ] Templates aninhados (`<template><template></template></template>`)
- [ ] Templates dentro de tables (foster parenting interaction)
- [ ] Templates em foreign content (SVG/MathML)
- [ ] Adoption agency algorithm com templates
- [ ] Fragment parsing com contexto template

**Casos de Teste:**
```html
<!-- Template aninhado -->
<template id="outer">
  <template id="inner">Content</template>
</template>

<!-- Template em table -->
<table>
  <template>
    <tr><td>Cell</td></tr>
  </template>
</table>

<!-- Template em SVG -->
<svg>
  <foreignObject>
    <template>HTML in SVG</template>
  </foreignObject>
</svg>
```

### Critérios de Aceitação
- ✅ Parse correto de `<template>` em qualquer contexto
- ✅ Conteúdo do template não é renderizado (apenas armazenado)
- ✅ Templates aninhados funcionam corretamente
- ✅ 100% dos testes html5lib para template
- ✅ Serialização/deserialização preservam conteúdo

---

## 2.2 Shadow DOM Parsing Support

### Especificação Referenciada
- Shadow DOM v1 Specification (W3C)
- WHATWG DOM Standard - Shadow trees
- Custom Elements v1 Specification

### Sub-tarefas

#### 2.2.1 Slot Elements [SEMANA 2-3]
- [ ] Implementar `<slot>` element parsing
- [ ] Distinguir named slots vs default slots
- [ ] Slot assignment preparation (data structures)
- [ ] Slot distribution algorithm (preparação)

**Implementação:**
```rust
pub struct SlotElement {
    pub base: HTMLElement,
    pub name: Option<String>, // None = default slot
    pub assigned_nodes: Vec<NodeHandle>,
    pub is_manual: bool, // slot="manual"
}

impl TreeBuilder {
    fn handle_slot_element(&mut self, attrs: Vec<Attribute>) {
        let name = attrs.iter()
            .find(|a| a.name == "name")
            .map(|a| a.value.clone());
        
        let slot = SlotElement {
            base: self.create_html_element("slot"),
            name,
            assigned_nodes: Vec::new(),
            is_manual: false,
        };
        
        self.insert_element(slot);
    }
}
```

#### 2.2.2 Shadow Root Infrastructure [SEMANA 3]
- [ ] Estruturas de dados para shadow roots
- [ ] Distinguir open vs closed shadow roots
- [ ] Shadow host pointer
- [ ] prepare para attachShadow() API

**Estruturas:**
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShadowRootMode {
    Open,
    Closed,
}

pub struct ShadowRoot {
    pub mode: ShadowRootMode,
    pub host: NodeHandle,
    pub nodes: Vec<NodeHandle>,
    pub slots: Vec<NodeHandle>,
    pub style_sheets: Vec<StyleSheetHandle>,
    pub adopted_style_sheets: Vec<StyleSheetHandle>,
}

pub enum NodeShadow {
    None,
    Open(ShadowRoot),
    Closed(ShadowRoot), // Only accessible via JS
}
```

#### 2.2.3 Custom Elements Integration [SEMANA 3-4]
- [ ] is="type" attribute support
- [ ] Autonomous custom elements (tag names com hyphen)
- [ ] Preparation para upgradedCallback
- [ ] Reserved names validation

**Regras de Parsing:**
```rust
fn is_valid_custom_element_name(name: &str) -> bool {
    // Must contain a hyphen
    if !name.contains('-') {
        return false;
    }
    
    // Must start with lowercase ASCII letter
    if !name.chars().next().unwrap().is_ascii_lowercase() {
        return false;
    }
    
    // Cannot be any of the forbidden names
    const FORBIDDEN: &[&str] = &[
        "annotation-xml", "color-profile", "font-face", 
        "font-face-src", "font-face-uri", "font-face-format",
        "font-face-name", "missing-glyph"
    ];
    
    !FORBIDDEN.contains(&name)
}
```

### Critérios de Aceitação
- ✅ `<slot>` elements parsed corretamente
- ✅ Named e default slots distinguidos
- ✅ Shadow root infrastructure pronta
- ✅ Custom element names validados
- ✅ Preparação completa para Shadow DOM runtime

---

## 2.3 Foreign Content Avançado

### Especificações Referenciadas
- SVG 2.0 Specification
- MathML3 Specification
- WHATWG HTML - Foreign content section

### Sub-tarefas

#### 2.3.1 SVG Completo [SEMANA 3-4]
- [ ] Todos os elementos SVG 2.0 (100+ elementos)
- [ ] Atributos SVG com case-sensitive handling
- [ ] SVG namespace inheritance correto
- [ ] Mixed HTML/SVG content (foreignObject completo)
- [ ] Self-closing tags em SVG

**Lista de Elementos SVG Prioritários:**
```rust
const SVG_ELEMENTS: &[&str] = &[
    // Basic containers
    "svg", "g", "defs", "symbol", "use", "image",
    
    // Shapes
    "circle", "ellipse", "line", "polygon", "polyline", 
    "rect", "path",
    
    // Text
    "text", "tspan", "textPath", "altGlyph", "textArea",
    
    // Gradients and patterns
    "linearGradient", "radialGradient", "pattern", "stop",
    
    // Filters
    "filter", "feBlend", "feColorMatrix", "feComponentTransfer",
    "feComposite", "feConvolveMatrix", "feDiffuseLighting",
    "feDisplacementMap", "feDropShadow", "feFlood",
    "feFuncA", "feFuncB", "feFuncG", "feFuncR", "feGaussianBlur",
    "feImage", "feMerge", "feMergeNode", "feMorphology",
    "feOffset", "feSpecularLighting", "feTile", "feTurbulence",
    
    // Clipping and masking
    "clipPath", "mask",
    
    // Other
    "a", "view", "script", "style", "marker", "mesh",
    "hatch", "solidColor", "unknown"
];
```

**Case-Sensitive Attributes:**
```rust
// SVG attributes must preserve case
const SVG_CAMEL_CASE_ATTRS: &[&str] = &[
    "attributeName", "attributeType", "baseFrequency",
    "baseProfile", "calcMode", "clipPathUnits",
    "contentScriptType", "contentStyleType", "diffuseConstant",
    "edgeMode", "externalResourcesRequired", "filterRes",
    "filterUnits", "glyphRef", "gradientTransform",
    "gradientUnits", "kernelMatrix", "kernelUnitLength",
    "keyPoints", "keySplines", "keyTimes", "lengthAdjust",
    "limitingConeAngle", "markerHeight", "markerUnits",
    "markerWidth", "maskContentUnits", "maskUnits",
    "numOctaves", "pathLength", "patternContentUnits",
    "patternTransform", "patternUnits", "pointsAtX",
    "pointsAtY", "pointsAtZ", "preserveAlpha",
    "preserveAspectRatio", "primitiveUnits", "refX", "refY",
    "repeatCount", "repeatDur", "requiredExtensions",
    "requiredFeatures", "specularConstant", "specularExponent",
    "spreadMethod", "startOffset", "stdDeviation",
    "stitchTiles", "surfaceScale", "systemLanguage",
    "tableValues", "targetX", "targetY", "textLength",
    "viewBox", "viewTarget", "xChannelSelector",
    "yChannelSelector", "zoomAndPan"
];
```

#### 2.3.2 MathML Completo [SEMANA 4]
- [ ] Todos os elementos MathML3
- [ ] MathML attributes adjustment
- [ ] MathML text integration points
- [ ] Annotation-xml handling
- [ ] Mixed MathML/HTML content

**Elementos MathML Principais:**
```rust
const MATHML_ELEMENTS: &[&str] = &[
    // Script elements
    "msup", "msub", "msubsup", "munder", "mover", "munderover",
    "mroot", "msqrt",
    
    // Layout elements
    "mrow", "mfrac", "mmultiscripts", "mtable", "mtr", "mtd",
    
    // Token elements
    "mi", "mn", "mo", "mtext", "mspace", "ms",
    
    // Special elements
    "mphantom", "mfenced", "menclose", "semantics",
    "annotation", "annotation-xml"
];
```

#### 2.3.3 Foreign Content Error Handling [SEMANA 4]
- [ ] HTML tags em foreign content (correto handling)
- [ ] Doctype em foreign content (erro)
- [ ] Comments em foreign content
- [ ] Character references em foreign content
- [ ] Missing namespace errors

**Regras de Error Handling:**
```rust
impl TreeBuilder {
    fn handle_in_foreign_content(&mut self, token: Token) {
        match token {
            Token::StartTag { name, .. } => {
                // Check if tag should switch back to HTML
                if self.is_html_integration_point() {
                    self.parse_in_html_mode(token);
                } else if self.is_mathml_text_integration_point() {
                    self.parse_in_mathml_mode(token);
                } else {
                    // Keep in foreign content
                    self.insert_foreign_element(name, Namespace::Svg or MathML);
                }
            }
            Token::EndTag { name, .. } => {
                // Handle closing foreign element
                self.pop_until_foreign(name);
            }
            Token::DOCTYPE => {
                self.report_error(ParseError::UnexpectedDoctypeInForeignContent);
                // Ignore DOCTYPE
            }
            // ... other tokens
        }
    }
}
```

### Critérios de Aceitação
- ✅ Todos os elementos SVG 2.0 suportados
- ✅ Atributos SVG com case preservation
- ✅ Todos os elementos MathML3 suportados
- ✅ MathML text integration points corretos
- ✅ Error handling em foreign content
- ✅ 95%+ html5lib foreign content tests

---

## Plano de Implementação por Semana

### Semana 1: Template Element
- Dia 1-2: Insertion mode implementation
- Dia 3-4: DocumentFragment structure
- Dia 5: Edge cases e testes

### Semana 2: Shadow DOM Base
- Dia 1-2: Slot elements
- Dia 3-4: Shadow root infrastructure
- Dia 5: Custom elements prep

### Semana 3: SVG Completo
- Dia 1-2: Element list expansion
- Dia 3: Attribute case handling
- Dia 4-5: Foreign object e mixed content

### Semana 4: MathML + Validação
- Dia 1-2: MathML elements
- Dia 3: Integration points
- Dia 4-5: Testing e bug fixes

---

## Arquivos para Modificação

### Principais
1. `src/ace/html/tree_builder.rs` - Inserção modes, element handling
2. `src/ace/html/dom.rs` - Novas estruturas (Template, ShadowRoot, Slot)
3. `src/ace/html/namespace.rs` - Expansão de namespace handling
4. `src/ace/html/error.rs` - Novos error codes

### Secundários
1. `src/ace/html/tokenizer.rs` - Foreign content detection
2. `src/ace/html/lexer.rs` - SVG/MathML specific tokenization
3. `tests/ace_html_conformance/phase2_tests.json` - Novos testes

---

## Métricas de Sucesso

### Quantitativas
- ✅ 100% template element tests passing
- ✅ 100% slot element tests passing
- ✅ 95%+ SVG foreign content tests
- ✅ 95%+ MathML tests
- ✅ Zero crashes em documentos complexos

### Qualitativas
- ✅ Código documentado com referências WHATWG
- ✅ Testes unitários para cada feature
- ✅ Performance não degradada (>90% baseline)
- ✅ Integração smooth com existing code

---

## Riscos e Mitigações

### Risco: Complexidade do Template Insertion Mode
**Mitigação:** Seguir especificação passo-a-passo, testes incrementais

### Risco: Performance com muitos templates
**Mitigação:** Lazy initialization de template content

### Risco: SVG/MathML edge cases
**Mitigação:** Usar html5lib-tests como referência principal

### Risco: Shadow DOM muito complexo para esta fase
**Mitigação:** Focar apenas em parsing, deixar runtime para fase posterior

---

## Próximos Passos Imediatos

1. **Hoje:** Criar estruturas de dados para Template
2. **Amanhã:** Implementar InsertionMode::InTemplate
3. **Dia 3:** Implementar DocumentFragment
4. **Dia 4-5:** Testes e validação

**Comando para iniciar:**
```bash
# Verificar estrutura atual do tree_builder
grep -n "InsertionMode" src/ace/html/tree_builder.rs | head -20

# Verificar namespaces existentes
cat src/ace/html/namespace.rs
```

---

## Referências

1. [WHATWG HTML Living Standard - Templates](https://html.spec.whatwg.org/multipage/scripting.html#the-template-element)
2. [Shadow DOM v1 Spec](https://www.w3.org/TR/shadow-dom/)
3. [SVG 2.0 Specification](https://svgwg.org/svg2-draft/)
4. [MathML3 Specification](https://www.w3.org/TR/MathML3/)
5. [html5lib-tests - Template tests](https://github.com/html5lib/html5lib-tests/tree/master/tests)
