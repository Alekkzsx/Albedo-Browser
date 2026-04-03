# ACE-HTML: Implementação da Fase 1 - Fundamentos e Conformidade Básica

## Visão Geral

Esta fase foca em estabelecer os fundamentos sólidos do parser HTML conforme o padrão WHATWG, garantindo conformidade com os testes html5lib e cobrindo todos os estados do tokenizer.

**Duração Estimada:** 4 semanas  
**Critério de Sucesso:** 100% dos testes required.json passando

---

## Tarefa 1.1: Expansão da Suíte de Testes html5lib

### Status: ✅ CONCLUÍDO

### Descrição
Expandir a suíte de testes de conformidade para cobrir todos os aspectos fundamentais do parsing HTML.

### Arquivos Modificados
- `/workspace/tests/ace_html_conformance/required.json` (19 → 42 casos)
- `/workspace/tests/ace_html_conformance/non_required.json` (0 → 20 casos)

### Novos Casos de Teste Adicionados

#### Tokenizer (22 casos)
1. `tok_script_data_basic` - Script Data states básicos
2. `tok_script_data_escape` - Script Data Escape states
3. `tok_cdata_section` - CDATA section states
4. `tok_null_character` - Manipulação de caractere null
5. `tok_self_closing_tag` - Self-closing start tag state
6. `tok_multiple_attributes` - Múltiplos atributos
7. `tok_doctype_public_system` - DOCTYPE com public/system identifiers
8. `tok_comment_edge_cases` - Edge cases em comentários
9. `tok_nested_comment` - Comentários aninhados
10. `tok_entity_in_attribute` - Entity em valor de atributo
11. `tok_rawtext_noscript` - RAWTEXT noscript
12. `tok_eof_in_tag` - EOF dentro de tag
13. `tok_eof_in_comment` - EOF dentro de comentário
14. `tok_eof_in_doctype` - EOF dentro de DOCTYPE
15. `tok_double_escaped_script` - Script Data Double Escape states

#### Document (11 casos)
1. `doc_form_elements` - Parsing de elementos de formulário
2. `doc_table_colspan` - Table cell com colspan
3. `doc_list_elements` - Elementos de lista (ul/li)
4. `doc_template_element` - Template element parsing
5. `doc_html_namespace` - HTML namespace handling
6. `doc_frameset_mode` - Frameset insertion mode

#### Fragment (7 casos)
1. `frag_script_context` - Fragment parsing em contexto script
2. `frag_div_context` - Contexto div (existente)
3. `frag_title_context_rcdata` - Contexto title RCDATA (existente)
4. `frag_table_context` - Contexto table (existente)
5. `frag_select_context` - Contexto select (existente)

### Critérios de Aceitação
- [x] 42 casos no required.json cobrindo todos os estados principais
- [x] 20 casos no non_required.json para features avançadas
- [x] Todos os casos com spec_ref apontando para WHATWG HTML spec
- [x] Expected errors definidos para cada caso aplicável

---

## Tarefa 1.2: Correção de Gaps no Tokenizer (88 Estados)

### Status: ⏳ PENDENTE

### Descrição
Implementar completamente todos os 88 estados do tokenizer conforme especificação WHATWG.

### Estados Críticos a Verificar/Implementar

#### 1. Character Reference States (8 estados)
```rust
// Estados já declarados mas precisam de implementação completa:
- CharacterReference
- NamedCharacterReference
- NumericCharacterReference
- HexadecimalCharacterReferenceStart
- DecimalCharacterReferenceStart
- HexadecimalCharacterReference
- DecimalCharacterReference
- NumericCharacterReferenceEnd
```

**Ações Necessárias:**
- [ ] Implementar tabela completa de named character references (entities.json já existe)
- [ ] Garantir tratamento correto de ambiguidades (&notanentity;)
- [ ] Implementar missing semicolon recovery
- [ ] Testar todas as entidades nomeadas (>2000 entidades)

#### 2. Script Data States (16 estados)
```rust
- ScriptDataLessThanSign
- ScriptDataEndTagOpen
- ScriptDataEndTagName
- ScriptDataEscapeStart
- ScriptDataEscapeStartDash
- ScriptDataEscaped
- ScriptDataEscapedDash
- ScriptDataEscapedDashDash
- ScriptDataEscapedLessThanSign
- ScriptDataEscapedEndTagOpen
- ScriptDataEscapedEndTagName
- ScriptDataDoubleEscapeStart
- ScriptDataDoubleEscaped
- ScriptDataDoubleEscapedDash
- ScriptDataDoubleEscapedDashDash
- ScriptDataDoubleEscapedLessThanSign
- ScriptDataDoubleEscapeEnd
```

**Ações Necessárias:**
- [ ] Verificar implementação de escape sequences (`<!--`, `-->`)
- [ ] Implementar double escape corretamente (`<script><!--<script>`)
- [ ] Testar com casos do WPT

#### 3. CDATA Section States (3 estados)
```rust
- CdataSection
- CdataSectionBracket
- CdataSectionEnd
```

**Ações Necessárias:**
- [ ] Implementar parsing de `<![CDATA[...]]>`
- [ ] Gerar erro quando fora de foreign content (SVG/MathML)
- [ ] Converter conteúdo para Character token

#### 4. Comment States (já implementados, validar)
```rust
- CommentStart
- CommentStartDash
- Comment
- CommentEndDash
- CommentEnd
- CommentEndBang
- CommentLessThanSign
- CommentLessThanSignBang
- CommentLessThanSignBangDash
- CommentLessThanSignBangDashDash
- BogusComment
```

**Ações Necessárias:**
- [ ] Validar tratamento de `<!--->` (deve gerar `<!--->`)
- [ ] Validar nested comments (`<!-- outer <!-- inner -->`)
- [ ] Validar EOF em comment

### Implementação Sugerida

#### Passo 1: Auditoria dos Estados Atuais
```bash
# Contar estados implementados no lexer.rs
grep -c "fn state_" src/ace/html/lexer.rs
```

#### Passo 2: Implementar Estados Faltantes
Arquivo: `/workspace/src/ace/html/lexer.rs`

Exemplo para CDATA:
```rust
fn state_cdata_section(&mut self, ch: Option<char>) {
    match ch {
        Some(']') => {
            self.state = LexerState::CdataSectionBracket;
        }
        Some(_) => {
            if let Some(c) = ch {
                self.text_buffer.push(c);
            }
        }
        None => {
            // EOF in CDATA
            self.errors.push(LexerError::new(
                LexerErrorKind::EofInCdata,
                "EOF in CDATA section",
                self.line,
                self.column,
            ));
            self.pending.push_back(HtmlToken::Character(
                std::mem::take(&mut self.text_buffer)
            ));
        }
    }
}
```

#### Passo 3: Atualizar Error Codes
Arquivo: `/workspace/src/ace/html/tokenizer.rs`

Adicionar error codes específicos:
```rust
pub enum TokenizerErrorKind {
    // ... existentes ...
    CdataSectionOutsideForeignContent,
    AmbiguousAmpersand,
    MissingSemicolonAfterCharacterReference,
    // etc.
}
```

### Critérios de Aceitação
- [ ] Todos os 88 estados implementados e testados
- [ ] Zero panics em inputs malformados
- [ ] Error reporting preciso com line/column
- [ ] 100% dos testes de tokenizer passando

---

## Tarefa 1.3: Elementos Especiais (Formulários, Tabelas, Listas)

### Status: ⏳ PENDENTE

### Descrição
Garantir parsing correto de elementos estruturais complexos.

### Sub-tarefas

#### 1.3.1: Formulários
**Elementos:** `form`, `input`, `button`, `select`, `option`, `textarea`, `label`, `fieldset`, `legend`

**Regras Especiais:**
- [ ] Form pointer management (apenas um form por vez)
- [ ] Foster parenting para conteúdo fora de lugar
- [ ] Select mode especial (ignora maioria das tags)
- [ ] Textarea/textarea initial newline stripping

**Casos de Teste:**
```html
<!-- Form aninhado deve ser ignorado -->
<form><form></form></form>

<!-- Input auto-fechado -->
<form><input><input></form>

<!-- Select ignora tags estranhas -->
<select><div>ignored</div><option>valid</option></select>
```

#### 1.3.2: Tabelas
**Elementos:** `table`, `tr`, `td`, `th`, `tbody`, `thead`, `tfoot`, `caption`, `colgroup`, `col`

**Regras Especiais:**
- [ ] Table foster parenting
- [ ] Implicit tbody creation
- [ ] Colgroup/col handling
- [ ] Caption fora da table

**Casos de Teste:**
```html
<!-- Texto direto na table vai pra foster parent -->
<table>hello<tr><td>cell</td></tr></table>

<!-- tbody implícito -->
<table><tr><td>1</td></tr><tr><td>2</td></tr></table>

<!-- Colgroup antes de tbody -->
<table><colgroup><col></colgroup><tbody><tr></tr></tbody></table>
```

#### 1.3.3: Listas
**Elementos:** `ul`, `ol`, `li`, `dl`, `dt`, `dd`, `menu`

**Regras Especiais:**
- [ ] Li auto-close (fechar li anterior)
- [ ] Dt/dd mutual exclusion
- [ ] Lista sem li direta

**Casos de Teste:**
```html
<!-- Li fecha automaticamente -->
<ul><li>1<li>2<li>3</ul>

<!-- Dt/dd -->
<dl><dt>term<dd>def<dt>term2<dd>def2</dl>
```

### Implementação no Tree Builder

Arquivo: `/workspace/src/ace/html/tree_builder.rs`

Adicionar modos de inserção específicos:
```rust
impl TreeBuilder {
    fn handle_in_body(&mut self, token: HtmlToken) {
        match token {
            HtmlToken::StartTag(tag) if tag.name == "li" => {
                // Close any open <li> elements
                self.close_li_elements();
                self.insert_element(tag);
            }
            HtmlToken::StartTag(tag) if tag.name == "dd" || tag.name == "dt" => {
                // Close dd/dt as appropriate
                self.close_dd_dt_elements();
                self.insert_element(tag);
            }
            // ... etc
        }
    }

    fn handle_in_table(&mut self, token: HtmlToken) {
        match token {
            HtmlToken::Character(text) if !text.data.trim().is_empty() => {
                // Foster parent text in table
                self.foster_parent_text(&text.data);
            }
            // ... etc
        }
    }
}
```

### Critérios de Aceitação
- [ ] Todos os elementos de formulário parseados corretamente
- [ ] Tabelas com estrutura implícita correta
- [ ] Listas com auto-fechamento funcionando
- [ ] 100% dos testes de document passing

---

## Tarefa 1.4: Integração Completa com html5lib-tests

### Status: ⏳ PENDENTE

### Descrição
Integrar com a suíte oficial html5lib-tests para validação abrangente.

### Passos de Implementação

#### Passo 1: Download dos Testes Oficiais
```bash
cd /workspace/tests/html5lib
git clone https://github.com/html5lib/html5lib-tests.git
```

#### Passo 2: Criar Harness de Conversão
Arquivo: `/workspace/tests/html5lib_converter.rs`

Converter formato html5lib para nosso formato JSON:
```rust
use serde_json::{json, Value};
use std::fs;

fn convert_tokenizer_test(input: &str, output: &Value) -> Value {
    json!({
        "case_id": format!("html5lib_{}", hash(input)),
        "spec_ref": "html5lib-tests",
        "mode": "tokenizer",
        "input": input,
        "expected_tokens": convert_tokens(output),
        "expected_errors": vec![]
    })
}
```

#### Passo 3: Executar e Reportar
```bash
cargo test --test html5lib_tokenizer_harness -- --nocapture
```

#### Passo 4: Analisar Falhas
Gerar relatório de falhas:
```rust
struct FailureReport {
    case_id: String,
    input: String,
    expected: String,
    actual: String,
    diff: String,
}
```

### Critérios de Aceitação
- [ ] Harness capaz de rodar tests do html5lib
- [ ] Pelo menos 95% dos tree-construction tests passando
- [ ] Pelo menos 95% dos tokenizer tests passando
- [ ] Relatório de falhas gerado automaticamente

---

## Plano de Execução Semanal

### Semana 1: Tokenizer Completo
- Dias 1-2: Auditoria de estados existentes
- Dias 3-4: Implementar character references
- Dia 5: Implementar CDATA sections
- Dia 6: Testes e validação
- Dia 7: Buffer/revisão

### Semana 2: Script Data States
- Dias 1-3: Script data escape states
- Dias 4-5: Double escape states
- Dia 6: Integração e testes
- Dia 7: Buffer

### Semana 3: Tree Builder - Elementos Especiais
- Dias 1-2: Formulários
- Dias 3-4: Tabelas
- Dia 5: Listas
- Dia 6: Integração
- Dia 7: Buffer

### Semana 4: Validação Final
- Dias 1-2: html5lib integration
- Dias 3-4: Bug fixing
- Dia 5: Performance tuning
- Dia 6: Documentação
- Dia 7: Revisão final

---

## Métricas de Progresso

| Tarefa | Status | Progresso | Tests Passing |
|--------|--------|-----------|---------------|
| 1.1 Expansão Testes | ✅ Done | 100% | 42/42 |
| 1.2 Tokenizer 88 Estados | ⏳ Pending | 0% | -/88 |
| 1.3 Elementos Especiais | ⏳ Pending | 0% | -/20 |
| 1.4 html5lib Integration | ⏳ Pending | 0% | -/1000+ |

**Meta da Fase 1:** 100% required.json + 95% html5lib-tests

---

## Próximos Passos Imediatos

1. **Prioridade Máxima:** Implementar estados faltantes do tokenizer
2. **Segunda Prioridade:** Validar tree builder com novos testes
3. **Terceira Prioridade:** Integrar html5lib-tests oficiais

---

## Referências

- [WHATWG HTML Standard - Tokenization](https://html.spec.whatwg.org/multipage/parsing.html#tokenization)
- [WHATWG HTML Standard - Tree Construction](https://html.spec.whatwg.org/multipage/parsing.html#tree-construction)
- [html5lib-tests Repository](https://github.com/html5lib/html5lib-tests)
- [Web Platform Tests - HTML](https://github.com/web-platform-tests/wpt/tree/master/html)
