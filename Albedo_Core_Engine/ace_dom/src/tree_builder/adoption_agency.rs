//! # Algoritmo da Agência de Adoção (Adoption Agency Algorithm - WHATWG §12.2.6.4.7)
//!
//! Implementação canônica completa dos 16 passos normativos do WHATWG para reestruturação
//! cirúrgica de tags de formatação entrelaçadas ou malformadas (ex: `<b>1<p>2</b>3</p>`).

use crate::tree::Document;
use crate::tree_builder::active_formatting::ActiveFormattingElements;
use crate::tree_builder::open_elements::StackOfOpenElements;
use ace_core::id::NodeId;

/// Executa o Adoption Agency Algorithm (AAA) estrito de 16 passos (WHATWG §12.2.6.4.7).
pub fn run_adoption_agency_algorithm(
    doc: &mut Document,
    open_elements: &mut StackOfOpenElements,
    active_formatting: &mut ActiveFormattingElements,
    subject: &str,
) {
    // Passo 1: Se o current_node tiver a tag subject e não estiver em active_formatting, pop e return
    if let Some(curr_id) = open_elements.current_node() {
        if let Some(curr_node) = doc.get_node(curr_id) {
            if let Some(tag) = curr_node.tag_name() {
                if tag.eq_ignore_ascii_case(subject) && !active_formatting.contains(curr_id) {
                    open_elements.pop();
                    return;
                }
            }
        }
    }

    // Passo 2 & 3: Loop externo (máximo de 8 iterações normativas)
    for _ in 0..8 {
        // Passo 3.1: Encontra o formatting_element mais recente após o último marcador
        let formatting_element_id = match active_formatting.find_element_after_last_marker(doc, subject) {
            Some(id) => id,
            None => return,
        };

        // Passo 3.2: Se não estiver na pilha, remove da lista e retorna
        if !open_elements.contains(formatting_element_id) {
            active_formatting.remove(formatting_element_id);
            return;
        }

        // Passo 3.3: Se não estiver no escopo padrão, retorna
        if !open_elements.has_element_in_scope(doc, subject) {
            return;
        }

        // Passo 3.4: Localiza a posição do formatting element na pilha
        let stack = open_elements.as_slice();
        let fmt_pos = match stack.iter().position(|&id| id == formatting_element_id) {
            Some(p) => p,
            None => return,
        };

        // Passo 3.7: Encontra o furthest block (primeiro elemento especial abaixo do formatting element na pilha)
        let mut furthest_block_id = None;
        for &node_id in &stack[fmt_pos + 1..] {
            if let Some(node) = doc.get_node(node_id) {
                if let Some(tag) = node.tag_name() {
                    if StackOfOpenElements::is_special(tag.as_str()) {
                        furthest_block_id = Some(node_id);
                        break;
                    }
                }
            }
        }

        // Passo 3.8: Se não houver furthest block, fecha até o formatting element
        let furthest_block = match furthest_block_id {
            Some(id) => id,
            None => {
                open_elements.pop_until_id(formatting_element_id);
                active_formatting.remove(formatting_element_id);
                return;
            }
        };

        // Passo 3.9: Common ancestor é o elemento imediatamente acima do formatting element na pilha
        let common_ancestor = stack[fmt_pos - 1];

        // Passo 3.10: Bookmark é a posição do formatting element em active_formatting
        let mut bookmark = active_formatting
            .position_of(formatting_element_id)
            .unwrap_or(active_formatting.len());

        // Passo 3.11: Inicializa node e last_node
        let mut node = furthest_block;
        let mut last_node = furthest_block;

        // Passo 3.12: Inner loop
        let mut inner_loop_counter = 0;
        loop {
            inner_loop_counter += 1;

            // 3.12.2: node = elemento imediatamente acima de node na pilha
            let node_pos = match open_elements.position_of(node) {
                Some(p) if p > 0 => p - 1,
                _ => break,
            };
            node = open_elements.as_slice()[node_pos];

            // 3.12.3: Se node == formatting_element, encerra o inner loop
            if node == formatting_element_id {
                break;
            }

            // 3.12.4: Se inner_loop > 3 e node em active_formatting, remove
            if inner_loop_counter > 3 && active_formatting.contains(node) {
                active_formatting.remove(node);
            }

            // 3.12.5: Se node não estiver em active_formatting, remove da pilha e continua
            if !active_formatting.contains(node) {
                open_elements.remove_id(node);
                continue;
            }

            // 3.12.6: Cria novo clone do node e substitui na pilha e em active_formatting
            let new_node = clone_element(doc, node);
            active_formatting.replace(node, new_node);
            open_elements.replace(node, new_node);
            node = new_node;

            // 3.12.7: Se last_node == furthest_block, atualiza bookmark
            if last_node == furthest_block {
                if let Some(pos) = active_formatting.position_of(node) {
                    bookmark = pos + 1;
                }
            }

            // 3.12.8: Reparenta last_node sob node
            if let Some(last_parent) = doc.get_node(last_node).and_then(|n| n.parent) {
                let _ = doc.remove_child(last_parent, last_node);
            }
            let _ = doc.append_child(node, last_node);

            // 3.12.9: last_node = node
            last_node = node;
        }

        // Passo 3.13: Reparenta last_node sob common_ancestor
        if let Some(last_parent) = doc.get_node(last_node).and_then(|n| n.parent) {
            let _ = doc.remove_child(last_parent, last_node);
        }
        let _ = doc.append_child(common_ancestor, last_node);

        // Passo 3.14: Cria new_element clonando formatting_element
        let new_element = clone_element(doc, formatting_element_id);

        // Passo 3.15: Move todos os filhos de furthest_block para new_element
        let mut children_to_move = Vec::new();
        for (child_id, _) in doc.children(furthest_block) {
            children_to_move.push(child_id);
        }
        for child_id in children_to_move {
            let _ = doc.append_child(new_element, child_id);
        }

        // Passo 3.16: Anexa new_element ao furthest_block
        let _ = doc.append_child(furthest_block, new_element);

        // Passo 3.17: Atualiza active_formatting e stack_of_open_elements
        active_formatting.remove(formatting_element_id);
        active_formatting.insert_at(bookmark, new_element);

        open_elements.remove_id(formatting_element_id);
        open_elements.insert_after(furthest_block, new_element);
    }
}

fn clone_element(doc: &mut Document, source_id: NodeId) -> NodeId {
    if let Some(node) = doc.get_node(source_id) {
        if let Some(el) = node.as_element() {
            let tag = el.tag_name.clone();
            let ns = el.namespace;
            let attrs = el.attributes.clone();
            let new_id = doc.create_element(tag, ns);
            if let Some(new_node) = doc.get_node_mut(new_id) {
                if let Some(new_el) = new_node.as_element_mut() {
                    new_el.attributes = attrs;
                }
            }
            return new_id;
        }
    }
    doc.create_element("div", crate::node::element::Namespace::Html)
}
