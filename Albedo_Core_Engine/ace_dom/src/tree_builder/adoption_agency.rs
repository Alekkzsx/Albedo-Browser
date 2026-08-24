//! # Algoritmo da Agência de Adoção (Adoption Agency Algorithm - WHATWG §12.2.6.4.7)
//!
//! Reestrutura tags de formatação entrelaçadas ou malformadas (ex: `<b>1<p>2</b>3</p>`).

use crate::tree::Document;
use crate::tree_builder::active_formatting::ActiveFormattingElements;
use crate::tree_builder::open_elements::StackOfOpenElements;

/// Executa o Adoption Agency Algorithm (AAA) para uma tag de fechamento de formatação.
pub fn run_adoption_agency_algorithm(
    doc: &mut Document,
    open_elements: &mut StackOfOpenElements,
    active_formatting: &mut ActiveFormattingElements,
    subject: &str,
) {
    // Passo 1: Loop externo (máximo de 8 iterações normativas)
    for _ in 0..8 {
        // Passo 2: Encontra o elemento de formatação na lista
        let formatting_element_id = match active_formatting.find_element_after_last_marker(doc, subject) {
            Some(id) => id,
            None => return, // Não há elemento a adotar
        };

        // Passo 3: Verifica se o elemento está na pilha de elementos abertos
        if !open_elements.contains(formatting_element_id) {
            active_formatting.remove(formatting_element_id);
            return;
        }

        // Passo 4: Verifica se está no escopo
        if !open_elements.has_element_in_scope(doc, subject) {
            return;
        }

        // Passo 5: Encontra o furthest block (bloco mais distante abaixo do formatting element na pilha)
        let stack = open_elements.as_slice();
        let fmt_pos = match stack.iter().position(|&id| id == formatting_element_id) {
            Some(p) => p,
            None => return,
        };

        let mut furthest_block_id = None;
        for &node_id in &stack[fmt_pos + 1..] {
            if let Some(node) = doc.get_node(node_id) {
                if let Some(tag) = node.tag_name() {
                    // Elementos de categoria especial (blocos)
                    if matches!(
                        tag.as_str(),
                        "address"
                            | "article"
                            | "aside"
                            | "blockquote"
                            | "center"
                            | "details"
                            | "dialog"
                            | "dir"
                            | "div"
                            | "dl"
                            | "fieldset"
                            | "figcaption"
                            | "figure"
                            | "footer"
                            | "header"
                            | "hgroup"
                            | "main"
                            | "menu"
                            | "nav"
                            | "ol"
                            | "p"
                            | "section"
                            | "summary"
                            | "ul"
                            | "table"
                            | "form"
                            | "pre"
                    ) {
                        furthest_block_id = Some(node_id);
                        break;
                    }
                }
            }
        }

        // Passo 6: Se não houver furthest block, fecha até o formatting element
        let furthest_block = match furthest_block_id {
            Some(id) => id,
            None => {
                open_elements.pop_until_id(formatting_element_id);
                active_formatting.remove(formatting_element_id);
                return;
            }
        };

        // Passo 7: Common ancestor é o nó imediatamente acima do formatting element na pilha
        let _common_ancestor_id = stack[fmt_pos - 1];

        // Passo 8: Clona o formatting element e insere os filhos
        if let Some(fmt_node) = doc.get_node(formatting_element_id) {
            if let Some(el_data) = fmt_node.as_element() {
                let tag_name = el_data.tag_name.clone();
                let ns = el_data.namespace;
                let new_el = doc.create_element(tag_name, ns);

                // Move todos os filhos do furthest block para o novo elemento
                let mut children_to_move = Vec::new();
                for (child_id, _) in doc.children(furthest_block) {
                    children_to_move.push(child_id);
                }
                for child_id in children_to_move {
                    let _ = doc.append_child(new_el, child_id);
                }

                let _ = doc.append_child(furthest_block, new_el);
            }
        }

        // Remove do active formatting e desempilha o original
        active_formatting.remove(formatting_element_id);
        open_elements.pop_until_id(formatting_element_id);
    }
}
