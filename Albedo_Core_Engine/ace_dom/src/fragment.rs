//! # Parser de Fragmentos de HTML Contextuais (WHATWG §13.4)
//!
//! Permite o parsing de trechos soltos de HTML dentro do contexto de um elemento existente
//! (essencial para setters de `innerHTML` e `createContextualFragment`).

use crate::node::element::Namespace;
use crate::tokenizer::{HTMLTokenizer, TokenizerState};
use crate::tree::Document;
use crate::tree_builder::{HTMLTreeBuilder, InsertionMode};
use ace_core::id::NodeId;
use ace_core::text::SegmentedString;

/// Faz o parsing de um fragmento HTML considerando o contexto do elemento pai (WHATWG §13.4).
pub fn parse_fragment(
    doc: &mut Document,
    context_node_id: Option<NodeId>,
    html: &str,
) -> NodeId {
    let frag_id = doc.create_document_fragment();

    let mut builder = HTMLTreeBuilder::new(None);
    let mut tokenizer = HTMLTokenizer::new();

    // Cria nó raiz implícito no documento temporário
    let root_id = builder.doc.root();
    let html_id = builder.doc.create_element("html", Namespace::Html);
    let _ = builder.doc.append_child(root_id, html_id);
    builder.open_elements.push(html_id);
    builder.doc.document_element = Some(html_id);

    // Ajusta o modo inicial de inserção e o estado do tokenizer conforme o elemento de contexto
    if let Some(ctx_id) = context_node_id {
        if let Some(ctx_node) = doc.get_node(ctx_id) {
            if let Some(el) = ctx_node.as_element() {
                let tag = el.tag_name.as_str();

                match tag {
                    "title" | "textarea" => {
                        tokenizer.state = TokenizerState::RCDATA;
                        builder.mode = InsertionMode::InBody;
                    }
                    "style" | "xmp" | "iframe" | "noembed" | "noframes" => {
                        tokenizer.state = TokenizerState::RAWTEXT;
                        builder.mode = InsertionMode::InBody;
                    }
                    "script" => {
                        tokenizer.state = TokenizerState::ScriptData;
                        builder.mode = InsertionMode::InBody;
                    }
                    "plaintext" => {
                        tokenizer.state = TokenizerState::PLAINTEXT;
                        builder.mode = InsertionMode::InBody;
                    }
                    "table" => {
                        builder.mode = InsertionMode::InTable;
                    }
                    "tbody" | "thead" | "tfoot" => {
                        builder.mode = InsertionMode::InTableBody;
                    }
                    "tr" => {
                        builder.mode = InsertionMode::InRow;
                    }
                    "td" | "th" => {
                        builder.mode = InsertionMode::InCell;
                    }
                    "template" => {
                        builder.mode = InsertionMode::InTemplate;
                    }
                    _ => {
                        let body_id = builder.doc.create_element("body", Namespace::Html);
                        let _ = builder.doc.append_child(html_id, body_id);
                        builder.open_elements.push(body_id);
                        builder.doc.body = Some(body_id);
                        builder.mode = InsertionMode::InBody;
                    }
                }
            }
        }
    } else {
        let body_id = builder.doc.create_element("body", Namespace::Html);
        let _ = builder.doc.append_child(html_id, body_id);
        builder.open_elements.push(body_id);
        builder.doc.body = Some(body_id);
        builder.mode = InsertionMode::InBody;
    }

    // Executa a tokenização e construção no documento temporário
    let mut input = SegmentedString::from_preprocessed_str(html);
    tokenizer.tokenize(&mut input, &mut builder);

    let temp_doc = builder.finish();

    // Encontra os nós construídos e clona-os para o fragmento do documento de destino
    let search_root = temp_doc.body.or(temp_doc.document_element).unwrap_or(temp_doc.root());
    let mut cloned_nodes = Vec::new();

    for (child_id, _) in temp_doc.children(search_root) {
        if let Ok(cloned) = clone_node_into_doc(&temp_doc, child_id, doc) {
            cloned_nodes.push(cloned);
        }
    }

    for child in cloned_nodes {
        let _ = doc.append_child(frag_id, child);
    }

    frag_id
}

/// Helper para clonar um nó recursivamente de um documento para outro.
fn clone_node_into_doc(
    src_doc: &Document,
    src_node_id: NodeId,
    dest_doc: &mut Document,
) -> Result<NodeId, crate::error::DomError> {
    let src_node = src_doc.get_node(src_node_id).ok_or(crate::error::DomError::InvalidNodeId(src_node_id))?;
    let new_kind = src_node.kind.clone();

    let dummy_id = NodeId::new();
    let mut new_node = crate::node::NodeData::new(dummy_id, new_kind);
    new_node.flags = src_node.flags;

    let arena_id = dest_doc.arena.alloc(new_node);
    let new_id = arena_id.to_node_id();
    if let Some(n) = dest_doc.arena.get_mut(arena_id) {
        n.id = new_id;
    }

    for (child_id, _) in src_doc.children(src_node_id) {
        let cloned_child = clone_node_into_doc(src_doc, child_id, dest_doc)?;
        dest_doc.append_child(new_id, cloned_child)?;
    }

    Ok(new_id)
}
