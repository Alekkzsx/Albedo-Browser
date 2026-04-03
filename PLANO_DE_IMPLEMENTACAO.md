# 🎯 PLANO DE IMPLEMENTAÇÃO TÉCNICA: ACE-HTML
## Implementação Cirúrgica para Conformidade WHATWG 100%

---

## ⚠️ Premissas Fundamentais

1. **Conformidade > Performance**: Otimização SOMENTE após 100% html5lib verde
2. **Testes Primeiro**: Cada feature requer testes ANTES da implementação
3. **Differential Testing**: Comparação constante com Blink/Gecko
4. **Zero Panics**: Fuzzing contínuo como guard-rail
5. **Documentação Obrigatória**: Código sem docs = código incompleto

---

## 📊 ESTADO ATUAL (Linha de Base)

### ✅ Implementado (Fases 1-3)
- Tokenizer: 80/88 estados (91%)
- Error handling: 58 codes (100%)
- Shadow DOM v1, Custom Elements, Slots
- SVG (39) + MathML (25) elementos
- Encoding: 52 codificações, BOM, HTTP headers
- Preload scanner (básico, instável)

### 🔴 Gaps Críticos (Prioridade 0)
1. **`preload_scanner.rs` quebrado** - FIX IMEDIATO
2. **Entidades históricas em atributos** - html5lib falhando
3. **Ambiguous ampersand** - casos edge não cobertos
4. **Adoption Agency Algorithm** - implementação incompleta
5. **Foster parenting** - edge cases faltando
6. **Foreign content SVG/MathML** - integration points frágeis
7. **Fragment parsing em contexto de tabela** - falhas conhecidas
8. **Frameset insertion mode** - não implementado
9. **`<template>` completo** - comportamento parcial

### 📈 Métricas Atuais
- html5lib tokenizer: ~91%
- html5lib tree-construction: <50% (estimado)
- html5lib fragment: <30% (estimado)
- Panics em fuzzing: >0 (inaceitável)
- Top 1K sites errors: Alto (não medido sistematicamente)

---

## 🎯 FASE 0: ESTABILIZAÇÃO (Semanas 1-2)

### Objetivo: Consolidar base antes de avançar

#### Tarefa 0.1: Fix `preload_scanner.rs` (URGENTE)
**Responsável**: Core team
**Duração**: 2-3 dias
**Critério de aceite**:
- [ ] Compila sem warnings
- [ ] Passa em todos testes existentes
- [ ] Integration test com tokenizer estável
- [ ] Documentação atualizada

**Passos**:
```rust
// 1. Identificar erro exato
cd /workspace && cargo check --all-targets

// 2. Rodar testes do módulo
cargo test preload_scanner -- --nocapture

// 3. Fix compilation errors
// 4. Adicionar testes de regressão
// 5. Code review obrigatório
```

#### Tarefa 0.2: Setup Differential Testing
**Responsável**: DevOps + Core
**Duração**: 3-4 dias
**Critério de aceite**:
- [ ] Pipeline CI comparando ACE vs Blink output
- [ ] Dashboard público com métricas diárias
- [ ] Alertas automáticos de regressão (>2% divergência)

**Ferramentas**:
- Puppeteer para capturar DOM do Chrome
- Script Python para diff estrutural
- GitHub Actions rodando diariamente
- Dashboard em Grafana ou similar

#### Tarefa 0.3: Catalogar TODAS Falhas html5lib
**Responsável**: QA + Core
**Duração**: 3-4 dias
**Critério de aceite**:
- [ ] Lista completa de testes falhando
- [ ] Issues criadas no tracker (uma por falha)
- [ ] Priorização (P0, P1, P2, P3)
- [ ] Planilha pública de progresso

**Processo**:
```bash
# 1. Rodar suíte completa
cargo test html5lib -- --nocapture > html5lib_results.json

# 2. Parsear resultados
python scripts/parse_html5lib_results.py html5lib_results.json

# 3. Criar issues automaticamente
python scripts/create_issues_from_failures.py

# 4. Atualizar dashboard
python scripts/update_dashboard.py
```

#### Tarefa 0.4: Infraestrutura de Fuzzing
**Responsável**: Security team
**Duração**: 4-5 dias
**Critério de aceite**:
- [ ] cargo-fuzz configurado
- [ ] 1M+ inputs rodando em CI
- [ ] Zero panics como gate de merge
- [ ] Coverage report (>80% tokenizer, >70% tree builder)

**Setup**:
```toml
# Cargo.toml (dev-dependencies)
[dependencies]
libfuzzer-sys = "0.4"
arbitrary = { version = "1.3", features = ["derive"] }
```

```rust
// fuzz/fuzz_targets/html_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use ace::html::{Tokenizer, TreeBuilder};

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize();
        
        let mut tree_builder = TreeBuilder::new();
        let _ = tree_builder.build(tokens);
        // Se chegar aqui sem panic = sucesso
    }
});
```

---

## 🎯 FASE 1: TOKENIZER COMPLETO (Semanas 3-8)

### Objetivo: 100% html5lib tokenizer

#### Sprint 1.1: Entidades e Character References (Semanas 3-4)

**Tarefa 1.1.1: Entidades Históricas em Atributos**
- **Spec**: WHATWG §13.2.5.73 Named character reference state
- **Issue**: #FIXME (criar após catalogação)
- **Testes**: 15-20 casos do html5lib
- **Implementação**:
  ```rust
  // tokenizer.rs
  fn consume_named_character_reference_in_attribute(
      &mut self,
      additional_allowed: Option<char>
  ) -> Option<String> {
      // Implementar lógica histórica
      // Se encontrar &entity sem ; mas seguida de alnum
      // → consumir apenas até onde for entidade válida
      // → retornar entidade parcial + remaining chars
  }
  ```
- **Testes obrigatórios**:
  - `<div title="x&ampy">` → `x&ampy` (não `x&y`)
  - `<div title="x&amp;y">` → `x&y`
  - `<div title="&foo">` → `&foo` (entidade inválida)
  - Casos com unicode, emojis, null bytes

**Tarefa 1.1.2: Ambiguous Ampersand**
- **Spec**: WHATWG §13.2.5.73
- **Testes**: 10-15 casos edge
- **Implementação**:
  ```rust
  fn handle_ambiguous_ampersand(&mut self) -> TokenizerResult {
      // & seguido de alnum sem ; em contexto de texto
      // → emitir erro ambiguous-ampersand
      // → consumir como texto literal
      match self.peek_context() {
          Context::Attribute => { /* regra específica */ }
          Context::Data => { /* regra específica */ }
          Context::RCDATA => { /* regra específica */ }
      }
  }
  ```

**Tarefa 1.1.3: Numeric Character References**
- **Spec**: WHATWG §13.2.5.74 Numeric character reference state
- **Testes**: 20+ casos (decimal, hex, invalid, surrogate, noncharacter)
- **Implementação**:
  ```rust
  fn consume_numeric_char_ref(&mut self) -> Result<char, EntityError> {
      // Parse decimal (&#123;) ou hex (&#x7B;)
      // Validar ranges proibidos:
      // - Surrogates: U+D800..U+DFFF
      // - Noncharacters: U+FDD0..U+FDEF, U+FFFE, U+FFFF, etc.
      // - Control chars exceto whitespace
      // → mapear para replacement char () se inválido
  }
  ```

**Critério de aceite Sprint 1.1**:
- [ ] 100% testes de entidade passando
- [ ] Zero erros em ambiguous ampersand
- [ ] Coverage >95% entity handling code
- [ ] Documentação completa de comportamentos edge

---

#### Sprint 1.2: Estados Especiais de Tokenização (Semanas 5-6)

**Tarefa 1.2.1: RCDATA States Completo**
- **Espec**: WHATWG §13.2.5 RCDATA states
- **Elementos**: `<textarea>`, `<title>`
- **Comportamento**:
  - Entity parsing habilitado
  - Tags ignoradas exceto closing tag do elemento
  - Character references processadas
  
**Tarefa 1.2.2: RAWTEXT States**
- **Espec**: WHATWG §13.2.5 RAWTEXT states
- **Elementos**: `<style>`, `<script>`, `<xmp>`, `<iframe>`, `<noembed>`, `<noframes>`
- **Comportamento**:
  - Sem entity parsing
  - Apenas closing tag reconhecida
  
**Tarefa 1.2.3: Script Data States**
- **Espec**: WHATWG §13.2.5 Script data states
- **Contexto**: Dentro de `<script>`
- **Estados**:
  - Script data state
  - Script data double escape start/end
  - Script data escaped states
  - Script data escaped dash/dash-dash states

**Tarefa 1.2.4: Markup Declaration Open State**
- **Espec**: WHATWG §13.2.5 Markup declaration open state
- **Detectar**:
  - `<!--` → Comment
  - `<!DOCTYPE` → Doctype
  - `<![CDATA[` → CDATA (foreign content apenas)
  - `--!>` → Comment (legacy)

**Critério de aceite Sprint 1.2**:
- [ ] Todos estados especiais implementados
- [ ] Transições entre estados corretas
- [ ] Tests html5lib: 100% passando
- [ ] Fuzzing: 1M inputs, zero panics

---

#### Sprint 1.3: Edge Cases e Error Recovery (Semanas 7-8)

**Tarefa 1.3.1: EOF Handling em Todos Estados**
- Testar EOF em cada um dos 88 estados
- Garantir error emission correta
- Validar token final emitido

**Tarefa 1.3.2: Null Byte Handling**
- `\0` em diferentes contextos
- Emissão de parse errors apropriados
- Substitution por U+FFFD quando necessário

**Tarefa 1.3.3: Invalid Unicode**
- Sequências UTF-8 inválidas
- Surrogate pairs
- Noncharacters
- Behavior idêntico a Blink

**Critério de aceite Fase 1**:
- [ ] **html5lib tokenizer: 100%**
- [ ] Coverage: >95%
- [ ] Fuzzing: 5M inputs, zero panics
- [ ] Differential testing: <1% divergência vs Blink

---

## 🎯 FASE 2: TREE BUILDER COMPLETO (Semanas 9-20)

### Objetivo: 100% html5lib tree-construction

#### Sprint 2.1: Adoption Agency Algorithm (Semanas 9-12)

**Tarefa 2.1.1: AAA Implementação Completa**
- **Spec**: WHATWG §13.2.6.4.7 Adoption agency algorithm
- **Complexidade**: Alta (um dos algoritmos mais complexos do HTML)
- **Casos de teste**: 30+ do html5lib

**Algoritmo** (simplificado):
```rust
fn adoption_agency_algorithm(
    &mut self,
    target_tag: &Atom,
    current_node: NodeId,
    formatting_element: NodeId
) -> Result<(), TreeBuilderError> {
    
    // Loop máximo 8 vezes
    for _ in 0..8 {
        // 1. Encontrar formatting element na lista de ativos
        
        // 2. Se não encontrado ou não na stack → erro, abortar
        
        // 3. Se formatting element não é current node → foster parenting
        
        // 4. Se formatting element é current node:
        //    a. Encontrar furthest block ancestor
        //    b. Se não existe → remover formatting element, abortar
        //    c. Encontrar common ancestor
        //    d. Criar bookmark
        //    e. Inner loop: mover nodes do furthest block
        //    f. Reinsertar formatting element
        //    g. Remover formatting element original
        //    h. Insert new formatting element no common ancestor
        
        // 5. Continuar outer loop se formatting element ainda ativo
    }
    
    Ok(())
}
```

**Testes Críticos**:
```html
<!-- Caso clássico -->
<p><b><i>x</b>y</i></p>
<!-- Expected: <p><b><i>x</i></b><i>y</i></p> -->

<!-- Nested formattings -->
<div><b><i><u>a</b>b</i>c</u>d</div>

<!-- With blocks -->
<b>bold <div>block</div> more bold</b>

<!-- Edge cases -->
<a href="#"><b><i>nested</a></i></b>
```

**Tarefa 2.1.2: Formatting Element List Management**
- Manter lista de active formatting elements
- Reconhecer scope markers (`<table>`, `<td>`, etc.)
- Clear list appropriately

**Critério de aceite Sprint 2.1**:
- [ ] 100% testes AAA passando
- [ ] Code review por especialista em parsers
- [ ] Documentação detalhada do algoritmo
- [ ] Performance benchmark (AAA é hot path)

---

#### Sprint 2.2: Foster Parenting (Semanas 13-14)

**Tarefa 2.2.1: Foster Parenting Rules**
- **Spec**: WHATWG §13.2.6.4.1 Appropriate place for inserting a node
- **Quando**: Content fora de `<table>`, `<tbody>`, etc.

**Implementação**:
```rust
fn foster_parent(&mut self, node: Node) {
    // 1. Procurar last <table> na stack
    // 2. Se encontrado:
    //    a. Se table tem parent → insert after table
    //    b. Senão → insert before table
    // 3. Se não tem table → insert no current node
    
    let table_pos = self.stack.iter().rposition(|n| n.is_table());
    
    match table_pos {
        Some(pos) => {
            let table = &self.stack[pos];
            if let Some(parent) = table.parent() {
                parent.insert_after(table, node);
            } else {
                self.stack[pos - 1].insert_before(node);
            }
        }
        None => {
            self.current_node().append(node);
        }
    }
}
```

**Testes**:
```html
<table>hello<tr><td>cell</td></tr></table>
<!-- "hello" vai para foster parent (antes da table) -->

<div><table><tr><td><div>nested</table></div></td></tr></table>
<!-- nested div fostered -->
```

**Critério de aceite**:
- [ ] 100% testes foster parenting
- [ ] Integração com AAA correta
- [ ] Edge cases com tables aninhadas

---

#### Sprint 2.3: Insertion Modes (Semanas 15-18)

**Tarefa 2.3.1: Todos Insertion Modes**
Implementar/completar:

1. **Initial** - antes do DOCTYPE
2. **Before HTML** - após DOCTYPE
3. **Before Head** - criando `<head>`
4. **In Head** - processando head content
5. **In Head Noscript** - quando `<noscript>` no head
6. **After Head** - transição para body
7. **In Body** - principal mode (MAIS COMPLEXO)
8. **Text** - dentro de textarea/title
9. **In Table** - conteúdo de table
10. **In Table Text** - text direto em table
11. **In Caption** - dentro de `<caption>`
12. **In Column Group** - `<colgroup>`
13. **In Row Group** - `<tbody>`, `<thead>`, `<tfoot>`
14. **In Row** - `<tr>`
15. **In Cell** - `<td>`, `<th>`
16. **In Select** - `<select>`
17. **In Select In Table** - select dentro de table
18. **In Template** - dentro de `<template>`
19. **After Body** - após body
20. **In Frameset** - `<frameset>` (legacy)
21. **After Frameset** - após frameset
22. **After After Body** - pós-documento
23. **After After Frameset** - pós-frameset

**Tarefa 2.3.2: In Body Mode (Detalhado)**
```rust
fn in_body(&mut self, token: Token) -> Result<()> {
    match token {
        Token::Character(c) if is_whitespace(c) => {
            self.reconstruct_active_formatting_elements();
            self.insert_text(c);
        }
        Token::StartTag("a") => {
            // Verificar se já tem <a> aberto → erro
            // Run AAA se necessário
            // Insert novo <a>
        }
        Token::StartTag("nobr") => {
            // Similar a <a>
        }
        Token::StartTag("button") => {
            // Fechar button existente se houver
        }
        Token::EndTag(_) => {
            // Handle end tags por categoria
        }
        // ... 50+ casos
    }
}
```

**Critério de aceite Sprint 2.3**:
- [ ] Todos 23 insertion modes implementados
- [ ] Transições entre modes corretas
- [ ] html5lib tree-construction: ≥80%

---

#### Sprint 2.4: Foreign Content (Semanas 19-20)

**Tarefa 2.4.1: SVG Integration**
- **Spec**: WHATWG §13.2.6.5 Parsing tokens in foreign content
- **Regras**:
  - Case sensitivity de tags SVG
  - Attributes case-sensitive
  - Self-closing tags permitidas
  - Namespace switching

**Tarefa 2.4.2: MathML Integration**
- Mesmas regras gerais
- MathML-specific attributes
- Namespace handling

**Tarefa 2.4.3: HTML-in-SVG/SVG-in-HTML**
- `<foreignObject>` em SVG
- `<svg>` em HTML
- Transições de namespace

**Testes**:
```html
<div><svg><circle cx="50" cy="50" r="40"/></svg></div>
<svg><foreignObject><div>HTML in SVG</div></foreignObject></svg>
<math><mi>x</mi></math>
```

**Critério de aceite Fase 2**:
- [ ] **html5lib tree-construction: 100%**
- [ ] Foreign content: 100% testes passando
- [ ] Fuzzing: 10M inputs, zero panics
- [ ] Differential: <2% divergência vs Blink

---

## 🎯 FASE 3: FRAGMENT PARSING & TEMPLATE (Semanas 21-26)

### Objetivo: 100% html5lib fragment tests

#### Sprint 3.1: Fragment Parsing API (Semanas 21-22)

**Tarefa 3.1.1: Fragment Parser**
```rust
pub fn parse_fragment(
    input: &str,
    context: &str,  // e.g., "div", "table", "tbody"
    namespace: Namespace,
) -> Result<DocumentFragment> {
    // 1. Criar context element
    // 2. Configurar tree builder com context
    // 3. Set insertion mode baseado no context
    // 4. Parse tokens
    // 5. Retornar children do context element
}
```

**Tarefa 3.1.2: Context-Specific Parsing**
- Table contexts: `<table>`, `<tbody>`, `<tr>`, `<td>`
- Select contexts: `<select>`, `<optgroup>`, `<option>`
- Foreign contexts: `<svg>`, `<math>`

**Testes**:
```javascript
// innerHTML simulation
const div = document.createElement('div');
div.innerHTML = '<tr><td>cell</td></tr>';
// Deve criar tbody implicitamente

const table = document.createElement('table');
table.innerHTML = '<tr><td>cell</td></tr>';
// Também cria tbody
```

#### Sprint 3.2: Template Element (Semanas 23-24)

**Tarefa 3.2.1: Template Parsing**
- Conteúdo vai para DocumentFragment
- Não renderizado inicialmente
- `content` attribute access

**Tarefa 3.2.2: Template Nesting**
```html
<template>
    <div>
        <template>
            <span>Nested template</span>
        </template>
    </div>
</template>
```

**Tarefa 3.2.3: Template em Contextos Especiais**
- Template em table
- Template em select
- Template em foreign content

#### Sprint 3.3: Frameset & Legacy (Semanas 25-26)

**Tarefa 3.3.1: Frameset Insertion Mode**
- Legacy, mas necessário para compatibilidade
- Interação com `<frame>`, `<frameset>`

**Tarefa 3.3.2: After After Body/Frameset**
- Trailing content handling
- Trailing whitespace
- Comments após documento

**Critério de aceite Fase 3**:
- [ ] **html5lib fragment: 100%**
- [ ] Template: todos testes passando
- [ ] Fuzzing: 25M inputs, zero panics
- [ ] Top 10K sites: ≤5 errors/site

---

## 🎯 FASE 4: PRELOAD SCANNER & ENCODING (Semanas 27-34)

### Objetivo: Preload scanner maduro + encoding detection robusto

#### Sprint 4.1: Preload Scanner Robusto (Semanas 27-30)

**Tarefa 4.1.1: Refatorar `preload_scanner.rs`**
- Arquitetura limpa e testável
- Streaming parsing support
- Error recovery

**Tarefa 4.1.2: Resource Detection**
Detectar:
- Scripts (sync, async, defer, module)
- Stylesheets (link, @import)
- Images (img, picture, srcset)
- Videos/audio (video, audio, source)
- Fonts (@font-face, link rel=preload)
- Prefetch/preconnect/dns-prefetch

**Tarefa 4.1.3: Speculation & Hints**
```rust
pub struct PreloadHint {
    pub resource_type: PreloadResourceType,
    pub url: Url,
    pub crossorigin: Option<CrossOrigin>,
    pub integrity: Option<IntegrityHash>,
    pub media: Option<MediaQuery>,
    pub fetch_priority: FetchPriority,
}
```

**Testes**:
```html
<link rel="preload" href="style.css" as="style">
<script src="app.js" defer></script>
<img src="hero.jpg" srcset="hero@2x.jpg 2x" loading="lazy">
```

#### Sprint 4.2: Encoding Detection (Semanas 31-34)

**Tarefa 4.2.1: BOM Detection**
- UTF-8 BOM
- UTF-16 LE/BE BOM
- Priority sobre outros métodos

**Tarefa 4.2.2: HTTP Header Detection**
- Parse `Content-Type: charset=utf-8`
- Priority sobre meta tags

**Tarefa 4.2.3: Meta Tag Detection**
```html
<meta charset="utf-8">
<meta http-equiv="Content-Type" content="text/html; charset=iso-8859-1">
```

**Tarefa 4.2.4: Heuristic Detection**
- Detectar por byte patterns
- Fallback para windows-1252 (comportamento histórico)
- Compatibilidade com legacy web

**Critério de aceite Fase 4**:
- [ ] Preload scanner: estável, integrado
- [ ] Encoding: 100% testes WHATWG
- [ ] Fuzzing: 50M inputs, zero panics
- [ ] Top 100K sites: ≤1 error/site

---

## 🎯 FASE 5: PERFORMANCE & PRODUCTION READY (Semanas 35-52)

### Objetivo: Performance competitiva + production hardening

#### Sprint 5.1: SIMD Optimizations (Semanas 35-38)

**Tarefa 5.1.1: Tokenizer SIMD**
- Usar `simd-json` como inspiração
- Processar 16-32 bytes por ciclo
- Branchless parsing onde possível

```rust
#[target_feature(enable = "avx2")]
unsafe fn tokenize_simd_chunk(chunk: &[u8]) -> Vec<Token> {
    // AVX2 implementation
    // 32 bytes por vez
}
```

**Tarefa 5.1.2: Memory Pooling**
- Arena allocator para nodes
- Reuse de buffers
- Zero-copy parsing onde possível

#### Sprint 5.2: Parallel Parsing (Semanas 39-44)

**Tarefa 5.2.1: Incremental Parsing**
- Parse chunks sob demanda
- Update eficiente do DOM
- Streaming network integration

**Tarefa 5.2.2: Multi-core Utilization**
- Preload scanner em thread separada
- Background parsing
- Worker threads para heavy operations

#### Sprint 5.3: Production Hardening (Semanas 45-52)

**Tarefa 5.3.1: Security Audits**
- OSS-Fuzz integration contínua
- Penetration testing
- Memory safety verification (Miri)

**Tarefa 5.3.2: Documentation**
- 100% APIs documentadas
- Examples para cada feature
- Migration guides

**Tarefa 5.3.3: Release Process**
- Semantic versioning
- Changelog automático
- Rollback procedures

**Critério de aceite Fase 5**:
- [ ] **Performance: ≥70% do Blink**
- [ ] **Memory: ≤120% do Blink**
- [ ] **Fuzzing: 100M inputs, zero panics**
- [ ] **Top 100K sites: ≤0.5 errors/site**
- [ ] **Security audit: zero critical issues**

---

## 📋 CHECKLIST DE QUALIDADE (Obrigatório por PR)

### Para Cada Pull Request

- [ ] **Tests**: Todos testes passando (local + CI)
- [ ] **Coverage**: Nova cobertura >80%
- [ ] **Fuzzing**: 100K inputs sem panic
- [ ] **Docs**: Documentação atualizada
- [ ] **Changelog**: Entry adicionada
- [ ] **Review**: Pelo menos 1 reviewer aprovado
- [ ] **Benchmark**: Sem regressão >5%
- [ ] **Differential**: <1% nova divergência vs Blink

### Para Cada Release

- [ ] **html5lib**: Suite completa passando
- [ ] **Fuzzing**: 1M+ inputs sem panic
- [ ] **Pages reais**: Top 10K testados
- [ ] **Security**: Miri + valgrind clean
- [ ] **Docs**: 100% atualizadas
- [ ] **Examples**: Todos funcionando

---

## 📊 MÉTRICAS DE PROGRESSO (Atualização Semanal)

### Dashboard Público

| Métrica | Meta | Atual | Status |
|---------|------|-------|--------|
| html5lib tokenizer | 100% | ~91% | 🟡 Em progresso |
| html5lib tree-construction | 100% | <50% | 🔴 Crítico |
| html5lib fragment | 100% | <30% | 🔴 Crítico |
| Panics (1M fuzz) | 0 | >0 | 🔴 Bloqueante |
| Top 1K sites errors | ≤0.5 | Alto | 🔴 Crítico |
| Performance vs Blink | ≥70% | ? | ⚪ Não medido |
| Coverage tokenizer | ≥95% | ? | ⚪ Não medido |
| Coverage tree builder | ≥90% | ? | ⚪ Não medido |

**Atualização**: Semanal (toda sexta-feira)
**Local**: Dashboard público (GitHub Pages)

---

## 🔧 FERRAMENTAS & INFRAESTRUTURA

### Desenvolvimento

```bash
# Setup inicial
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly
rustup component add rustfmt clippy miri

# Build
cargo build --release

# Tests
cargo test --all-targets
cargo test html5lib -- --test-threads=1

# Fuzzing
cargo install cargo-fuzz
cargo fuzz run html_parser

# Coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# Benchmark
cargo install cargo-criterion
cargo criterion
```

### CI/CD

```yaml
# .github/workflows/ci.yml
name: ACE-HTML CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-action@nightly
      - run: cargo test --all-targets
      
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo fuzz run html_parser -- -max_total_time=3600
      
  differential:
    runs-on: ubuntu-latest
    steps:
      - name: Compare with Blink
        run: python scripts/differential_test.py
        
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - name: Run benchmarks
        run: cargo criterion
```

---

## 📚 RECURSOS & REFERÊNCIAS

### Especificações

- [WHATWG HTML Standard](https://html.spec.whatwg.org/multipage/)
- [HTML5 Parsing Algorithm](https://html.spec.whatwg.org/multipage/parsing.html)
- [W3C DOM Specification](https://dom.spec.whatwg.org/)

### Test Suites

- [html5lib-tests](https://github.com/html5lib/html5lib-tests)
- [W3C Validation Suite](https://validator.w3.org/)
- [Web Platform Tests](https://web-platform-tests.org/)

### Implementações de Referência

- [Chromium HTML Parser](https://chromium.googlesource.com/chromium/src/+/main/third_party/blink/renderer/core/html/)
- [Gecko HTML Parser](https://github.com/mozilla/gecko-dev/tree/master/parser/html)
- [Ladybird LibHTML](https://github.com/LadybirdBrowser/ladybird/tree/master/Libraries/LibHTML)

### Literatura

- "Parsing Techniques: A Practical Guide" - Dick Grune
- "Crafting Interpreters" - Robert Nystrom (online gratuito)
- "High Performance Browser Networking" - Ilya Grigorik

---

## 🎯 PRÓXIMOS PASSOS IMEDIATOS

### Semana 1

1. **Dia 1-2**: Fix `preload_scanner.rs` (URGENTE)
2. **Dia 3-4**: Setup differential testing pipeline
3. **Dia 5-7**: Catalogar todas falhas html5lib

### Semana 2

1. **Dia 1-3**: Infraestrutura de fuzzing
2. **Dia 4-5**: Dashboard público de métricas
3. **Dia 6-7**: Criar issues detalhadas para gaps críticos

### Semana 3

1. **Início Sprint 1.1**: Entidades e character references
2. **Code review** de arquitetura do tokenizer
3. **Recruitar** contributors para tasks bem definidas

---

## 💬 NOTA FINAL

**Este plano é executável.** Requer:

- ✅ Disciplina férrea (seguir specs à risca)
- ✅ Humildade (aprender com erros da web real)
- ✅ Transparência (métricas públicas sempre)
- ✅ Persistência (maratona, não sprint)
- ✅ Comunidade (ninguém faz isso sozinho)

**Não há atalhos.** Conformidade exige trabalho duro, testes exaustivos e paciência. Mas cada linha nos torna menos dependentes de monopólios tecnológicos.

**Vamos construir.**

---

*Última atualização: $(date +%Y-%m-%d)*
*Próxima revisão: Weekly (sextas-feiras)*
