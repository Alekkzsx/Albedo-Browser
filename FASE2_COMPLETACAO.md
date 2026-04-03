# Fase 2: Recursos Avançados - COMPLETAÇÃO

## Status Atual do Código

### ✅ Já Implementado:
1. **Estrutura de dados** para Shadow DOM em `InternalNodeData::Element`:
   - `slot_name: Option<String>`
   - `is_value: Option<String>`
   - `shadow_root_mode: Option<ShadowRootMode>`
   - `shadow_root: Option<Box<HtmlDocument>>`

2. **Extração de atributos especiais** na linha 611-619:
   ```rust
   let slot_name = tag.attributes.remove("slot");
   let is_value = tag.attributes.remove("is");
   let shadow_root_mode = tag.attributes.remove("shadowrootmode")...
   ```

3. **Foreign Content handling** completo (linhas 2415-2482):
   - `handle_foreign_content()` implementado
   - `is_mathml_text_integration_point()` implementado
   - `is_html_integration_point()` implementado

4. **Integration points** para SVG/MathML (linhas 2493-2524)

### ❌ Faltando Implementar:

1. **Função `set_element_special_properties()`** - chamada na linha 628 mas NÃO DEFINIDA
2. **Validação de Custom Elements** - função `validate_custom_element_name()` inexistente
3. **Processamento Declarative Shadow DOM** - mover filhos para shadow root
4. **Handler especializado para elemento `<slot>`**
5. **Listas completas de elementos SVG e MathML**
6. **Serialização dos campos Shadow DOM** na conversão para `HtmlElement`

## Plano de Ação Imediato

### Tarefa 1: Implementar `set_element_special_properties()`
**Local:** `src/ace/html/tree_builder.rs` após linha 647

```rust
fn set_element_special_properties(
    &mut self, 
    node_id: usize, 
    slot_name: Option<String>, 
    is_value: Option<String>, 
    shadow_root_mode: Option<crate::ace::html::ShadowRootMode>
) {
    if let InternalNodeData::Element { 
        ref mut slot_name: ref mut stored_slot,
        ref mut is_value: ref mut stored_is,
        ref mut shadow_root_mode: ref mut stored_shadow,
        .. 
    } = self.arena[node_id].data {
        *stored_slot = slot_name;
        *stored_is = is_value;
        *stored_shadow = shadow_root_mode;
    }
}
```

### Tarefa 2: Implementar validação de Custom Elements
**Local:** Nova função utilitária

```rust
fn validate_custom_element_name(name: &str) -> bool {
    // Regras da especificação Web Components
    if !name.contains('-') { return false; }
    if name.starts_with('-') { return false; }
    if name.ends_with('-') { return false; }
    
    let first_char = name.chars().next().unwrap_or('\0');
    if !first_char.is_ascii_lowercase() && first_char != '_' && first_char != ':' {
        return false;
    }
    
    true
}
```

### Tarefa 3: Processar Declarative Shadow DOM
**Local:** Após inserção de elemento com `shadowrootmode`

Quando um elemento tem `shadowrootmode`, os filhos devem ser movidos para um `HtmlDocument` separado.

### Tarefa 4: Handler para `<slot>`
Elementos `<slot>` precisam de tratamento especial no tree builder.

### Tarefa 5: Listas de elementos SVG/MathML
Definir constantes com todos os elementos válidos.

### Tarefa 6: Serialização completa
Atualizar `convert_to_html_node()` para incluir campos Shadow DOM.

## Próximos Passos

Vou implementar todas essas funções faltantes no arquivo `tree_builder.rs`.
