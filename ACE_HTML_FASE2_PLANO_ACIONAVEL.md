# Plano de Implementação Fase 2: Recursos Avançados ACE-HTML

## Status Atual Verificado

### ✅ O Que Já Existe (Surpresa Positiva!)

1. **InsertionMode::InTemplate** - JÁ IMPLEMENTADO
   - Linha 28: `InTemplate` definido no enum
   - Linha 312: Dispatch para `handle_in_template()`
   - Linhas 2136-2186: Implementação completa de `handle_in_template()`
   - Stack de template insertion modes já existe

2. **Foreign Content** - JÁ IMPLEMENTADO PARCIALMENTE
   - Namespace enum com Html, Svg, MathMl
   - `handle_foreign_content()` implementado (linha 2396)
   - `is_mathml_text_integration_point()` (linha 2474)
   - `is_html_integration_point()` implementado
   - SVG tag name adjustment (linha 445)
   - MathML attribute adjustment (linha 488)
   - SVG attribute adjustment (linha 499)
   - Foreign attribute adjustment (linha 567)

3. **Elementos SVG/MathML** - PARCIALMENTE IMPLEMENTADO
   - Teste existente: `test_svg_foreign_content()` (linha 2652)
   - foreignObject handling
   - Self-closing tags em SVG

### ❌ O Que Falta Implementar

1. **Template Element - Estruturas de Dados**
   - [ ] DocumentFragment structure
   - [ ] TemplateElement com content field
   - [ ] Shadow root preparation field

2. **Shadow DOM Infrastructure**
   - [ ] Slot element parsing
   - [ ] ShadowRoot struct
   - [ ] Custom elements validation

3. **SVG Completo**
   - [ ] Lista completa de 100+ elementos SVG
   - [ ] Case-sensitive attributes completo
   - [ ] Mais integration points

4. **MathML Completo**
   - [ ] Lista completa de elementos MathML3
   - [ ] Text integration points completos
   - [ ] Annotation-xml handling

---

## Implementação Prioritária - Semana 1

### Dia 1: Estruturas de Dados para Template

**Arquivo:** `src/ace/html/dom.rs` (ou criar `src/ace/html/template.rs`)

```rust
// Adicionar ao módulo DOM
#[derive(Debug, Clone)]
pub struct DocumentFragment {
    pub nodes: Vec<usize>, // NodeHandles
    pub document: usize,   // DocumentHandle
    pub host: Option<usize>,
}

impl DocumentFragment {
    pub fn new(document: usize) -> Self {
        Self {
            nodes: Vec::new(),
            document,
            host: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TemplateElement {
    pub base: HTMLElement,
    pub content: DocumentFragment,
    pub shadow_root: Option<usize>, // ShadowRoot handle - para fase 2.2
}
```

### Dia 2: Integrar Template com Tree Builder

**Arquivo:** `src/ace/html/tree_builder.rs`

Modificar inserção de elemento template:
```rust
// Quando inserting "template" element
if tag.name == "template" {
    let template = TemplateElement {
        base: self.create_html_element("template"),
        content: DocumentFragment::new(self.document),
        shadow_root: None,
    };
    // Insert template...
    
    // Push current insertion mode to stack
    self.template_insertion_modes.push(self.insertion_mode);
    self.insertion_mode = InsertionMode::InTemplate;
}
```

### Dia 3-4: Slot Elements

**Adicionar ao DOM:**
```rust
#[derive(Debug, Clone)]
pub struct SlotElement {
    pub base: HTMLElement,
    pub name: Option<String>,
    pub assigned_nodes: Vec<usize>,
    pub is_manual: bool,
}

// No tree builder, adicionar handling para <slot>
fn handle_slot_element(&mut self, attrs: HashMap<String, String>) {
    let name = attrs.get("name").cloned();
    
    let slot = SlotElement {
        base: self.create_html_element("slot"),
        name,
        assigned_nodes: Vec::new(),
        is_manual: false,
    };
    
    self.insert_element(slot);
}
```

### Dia 5: Shadow Root Infrastructure

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShadowRootMode {
    Open,
    Closed,
}

#[derive(Debug, Clone)]
pub struct ShadowRoot {
    pub mode: ShadowRootMode,
    pub host: usize,
    pub nodes: Vec<usize>,
    pub slots: Vec<usize>,
}

// Adicionar ao Node ou Element
pub enum NodeShadow {
    None,
    Open(ShadowRoot),
    Closed(ShadowRoot),
}
```

---

## Implementação Semana 2: SVG Expandido

### Lista Completa de Elementos SVG

**Arquivo:** `src/ace/html/tree_builder.rs`

```rust
const SVG_ELEMENTS: &[&str] = &[
    // Containers básicos (já existem)
    "svg", "g", "defs", "symbol", "use", "image",
    
    // Formas geométricas
    "circle", "ellipse", "line", "polygon", "polyline", 
    "rect", "path",
    
    // Texto
    "text", "tspan", "textPath", "altGlyph", "textArea",
    
    // Gradientes e padrões
    "linearGradient", "radialGradient", "pattern", "stop",
    
    // Filtros (20+ elementos)
    "filter", "feBlend", "feColorMatrix", "feComponentTransfer",
    "feComposite", "feConvolveMatrix", "feDiffuseLighting",
    "feDisplacementMap", "feDropShadow", "feFlood",
    "feFuncA", "feFuncB", "feFuncG", "feFuncR", "feGaussianBlur",
    "feImage", "feMerge", "feMergeNode", "feMorphology",
    "feOffset", "feSpecularLighting", "feTile", "feTurbulence",
    
    // Clipping e masking
    "clipPath", "mask",
    
    // Outros
    "a", "view", "script", "style", "marker", "mesh",
    "hatch", "solidColor", "unknown",
    
    // Animação (SMIL)
    "animate", "animateColor", "animateMotion", "animateTransform",
    "set", "mpath",
    
    // Descritivos
    "title", "desc", "metadata",
    
    // Cursor e fonte
    "cursor", "font", "font-face", "font-face-format",
    "font-face-name", "font-face-src", "font-face-uri",
    "hkern", "missing-glyph", "vkern",
    
    // Malha e pintura
    "meshgradient", "meshpatch", "meshrow",
    
    // Solid color
    "solidcolor",
];
```

### Atributos Case-Sensitive SVG

```rust
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
    "yChannelSelector", "zoomAndPan",
];
```

---

## Implementação Semana 3: MathML

### Elementos MathML3

```rust
const MATHML_ELEMENTS: &[&str] = &[
    // Elementos de script
    "msup", "msub", "msubsup", "munder", "mover", "munderover",
    "mroot", "msqrt",
    
    // Layout
    "mrow", "mfrac", "mmultiscripts", "mtable", "mtr", "mtd",
    "mlabeledtr",
    
    // Token
    "mi", "mn", "mo", "mtext", "mspace", "ms", "mglyph",
    
    // Especiais
    "mphantom", "mfenced", "menclose", "semantics",
    "annotation", "annotation-xml",
    
    // Canvas 3D (MathML 3.0+)
    "mstack", "mscarries", "mscarry", "msgroup", "mstack",
    "msline", "msrow", "msvert",
];
```

---

## Testes a Criar

### Arquivo: `tests/ace_html_conformance/phase2_tests.json`

```json
{
  "tests": [
    {
      "case_id": "template_basic",
      "category": "template",
      "input": "<template><div>Hello</div></template>",
      "expected": {
        "type": "Document",
        "children": [
          {
            "type": "Element",
            "name": "template",
            "content": {
              "type": "DocumentFragment",
              "children": [
                {
                  "type": "Element",
                  "name": "div",
                  "children": [
                    {"type": "Text", "data": "Hello"}
                  ]
                }
              ]
            }
          }
        ]
      }
    },
    {
      "case_id": "template_nested",
      "category": "template",
      "input": "<template><template><span>Nested</span></template></template>",
      "expected": {
        "type": "Document",
        "children": [
          {
            "type": "Element",
            "name": "template",
            "content": {
              "type": "DocumentFragment",
              "children": [
                {
                  "type": "Element",
                  "name": "template",
                  "content": {
                    "type": "DocumentFragment",
                    "children": [
                      {
                        "type": "Element",
                        "name": "span",
                        "children": [{"type": "Text", "data": "Nested"}]
                      }
                    ]
                  }
                }
              ]
            }
          }
        ]
      }
    },
    {
      "case_id": "slot_named",
      "category": "shadow_dom",
      "input": "<slot name=\"header\">Default Header</slot>",
      "expected": {
        "type": "Element",
        "name": "slot",
        "attributes": {"name": "header"},
        "children": [{"type": "Text", "data": "Default Header"}]
      }
    },
    {
      "case_id": "svg_all_elements",
      "category": "svg",
      "input": "<svg><circle/><rect/><path/><text/><tspan/><linearGradient/><filter/><feGaussianBlur/></svg>",
      "expected": {
        "type": "Element",
        "name": "svg",
        "namespace": "Svg",
        "children_count": 8
      }
    },
    {
      "case_id": "svg_case_attrs",
      "category": "svg",
      "input": "<svg><linearGradient gradientUnits=\"userSpaceOnUse\" viewBox=\"0 0 100 100\"/></svg>",
      "expected": {
        "attributes_preserved_case": ["gradientUnits", "viewBox"]
      }
    },
    {
      "case_id": "mathml_complete",
      "category": "mathml",
      "input": "<math><mi>x</mi><mo>=</mo><msqrt><mn>2</mn></msqrt></math>",
      "expected": {
        "type": "Element",
        "name": "math",
        "namespace": "MathMl",
        "children_count": 3
      }
    }
  ]
}
```

---

## Critérios de Aceitação Fase 2

### Template Element
- ✅ DocumentFragment estrutura criada
- ✅ Template content separado do main document
- ✅ Templates aninhados funcionam
- ✅ Template em tables funciona
- ✅ 100% testes html5lib template

### Shadow DOM
- ✅ Slot elements parsed
- ✅ Named vs default slots
- ✅ ShadowRoot infrastructure
- ✅ Custom element name validation

### SVG
- ✅ 100+ elementos SVG suportados
- ✅ Atributos camel-case preservados
- ✅ foreignObject completo
- ✅ Self-closing tags

### MathML
- ✅ Todos elementos MathML3
- ✅ Text integration points
- ✅ Annotation-xml

---

## Próximos Passos Imediatos

1. **Hoje:** Criar estruturas DocumentFragment e TemplateElement
2. **Amanhã:** Integrar com tree builder
3. **Dia 3:** Implementar slot elements
4. **Dia 4:** Shadow root infrastructure
5. **Dia 5:** Primeiros testes

**Comandos para começar:**
```bash
# Verificar estrutura atual do DOM
find src/ace/html -name "*.rs" -exec grep -l "struct.*Element" {} \;

# Verificar onde estão as structs principais
grep -n "pub struct Html" src/ace/html/*.rs | head -10
```
