# ACE-HTML: Lista de Casos a Testar

## Objetivo

Executar os testes para identificar exatamente quais casos falham e em quais categorias.

---

## Comandos para Executar

### 1. Teste Completo de Tree Construction
```bash
cargo test --test html5lib_tree_harness ace_html_tree_construction_full_report -- --nocapture
```

### 2. Teste Smoke (com threshold)
```bash
cargo test --test html5lib_tree_harness ace_html_tree_construction_smoke -- --nocapture
```

### 3. Teste por Arquivo Específico
```bash
# Adoption Agency
ACE_HTML_TREE_FILE_FILTER=adoption cargo test --test html5lib_tree_harness -- --nocapture

# Tables
ACE_HTML_TREE_FILE_FILTER=table cargo test --test html5lib_tree_harness -- --nocapture

# SVG
ACE_HTML_TREE_FILE_FILTER=svg cargo test --test html5lib_tree_harness -- --nocapture

# MathML
ACE_HTML_TREE_FILE_FILTER=math cargo test --test html5lib_tree_harness -- --nocapture

# Template
ACE_HTML_TREE_FILE_FILTER=template cargo test --test html5lib_tree_harness -- --nocapture
```

---

## Categorias de Teste (arquivos .dat)

### Arquivos de Teste html5lib/tree-construction/

| Arquivo | Categoria Principal | Casos Estimados |
|--------|-------------------|----------------|
| `adoption01.dat` | Adoption Agency | ~20 |
| `adoption02.dat` | Adoption Agency | ~20 |
| `tables01.dat` | Foster Parenting | ~30 |
| `tables02.dat` | Foster Parenting | ~30 |
| `svg.dat` | SVG Namespace | ~20 |
| `math.dat` | MathML Namespace | ~20 |
| `template.dat` | Template Element | ~30 |
| `noscript01.dat` | Noscript | ~15 |
| `scriptdata01.dat` | Script/Style Rawtext | ~20 |
| `frameset01.dat` | Frameset | ~15 |
| `inbody01.dat` | In Body Mode | ~20 |
| `blocks.dat` | Block Elements | ~20 |
| `entities01.dat` | Entities | ~20 |
| `entities02.dat` | Entities | ~15 |
| `doctype01.dat` | DOCTYPE | ~20 |
| `quirks01.dat` | Quirks Mode | ~15 |

---

## Casos do required.json (28 casos)

Estes são os casos mais importantes - todos devem passar:

| ID | Categoria | Input Resumido |
|----|-----------|---------------|
| `tok_basic_markup` | Tokenizer | `<div class="hero">Hello</div>` |
| `tok_doctype_and_comment` | Tokenizer | `<!DOCTYPE html><!-- note -->` |
| `tok_named_entity` | Tokenizer | `A &amp; B` |
| `tok_attr_semicolon_required_historical` | Tokenizer | `<div title="x&ampy">` |
| `tok_rcdata_textarea` | Tokenizer | `<textarea>A &lt; B</textarea>` |
| `tok_rawtext_style` | Tokenizer | `<style>A &amp; B</style>` |
| `doc_adoption_agency` | **Adoption Agency** | `<p><b><i>x</b>y</i></p>` |
| `doc_foster_parenting` | **Foster Parenting** | `<table>hello<tr><td>cell</td></tr></table>` |
| `doc_svg_namespace` | SVG | `<div><svg><circle/></svg></div>` |
| `frag_div_context` | Fragment | `div` + `<span>hello</span>` |
| `frag_title_context_rcdata` | Fragment | `title` + `A &lt; B` |
| `tok_ambiguous_ampersand` | Tokenizer | `x &notanentity; y` |
| `frag_table_context` | **Fragment Table** | `table` + `<tr><td>x</td></tr>` |
| `doc_frameset_mode` | Frameset | `<frameset><frame src="a">` |
| `tok_numeric_character_hex` | Tokenizer | `&#x41;` |
| `tok_numeric_character_dec` | Tokenizer | `&#65;` |
| `frag_select_context` | Fragment Select | `select` + `<option>yes</option>` |

---

## Resultado Esperado

### Se benchmark estiver correto (100%):
- Todos os casos acima devem passar
- A normalização está sendo aplicada

### Se benchmark estiver desatualizado (1.34%):
- Casos específicos falharão
- Precisará de investigação adicional

---

## Como Registrar os Resultados

Execute cada comando e salve a saída:

```bash
# Salvar resultado em arquivo
cargo test --test html5lib_tree_harness ace_html_tree_construction_full_report -- --nocapture 2>&1 | tee result_tree_full.txt
```

Depois compare com os resultados esperados em:
- `ACE_HTML_BROWSER_COMPARISON.md`
- `ace_html_super_benchmark_results.json`

---

## Próximos Passos Após Identificar Falhas

1. **Categorizar a falha**:
   - Adoption Agency?
   - Foster Parenting?
   - Namespace?
   - Fragment parsing?
   - Serializer?

2. **Analisar o código**:
   - `src/ace/html/html5ever_parser.rs`
   - `src/ace/html/mod.rs`

3. **Identificar a causa**:
   - Erro no parser?
   - Erro no serializer?
   - Fixup agressivo?

4. **Criar correção**:
   - Ajustar AceSinkNode
   - Remover fixup problemáticas
   - Adicionar normalização

---

## Status de Execução

- [ ] Teste smoke executado
- [ ] Teste full report executado
- [ ] Casos específicos por categoria executados
- [ ]Resultados сравados con benchmarks anteriores

*Preencha este checklist após executar os testes*

---

*Documento gerado em 29/04/2026*
*Execute os testes e adicione os resultados reais abaixo*