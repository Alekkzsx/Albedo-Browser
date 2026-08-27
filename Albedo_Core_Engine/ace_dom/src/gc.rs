//! # Arquitetura de Heap Unificado e Tracing GC (Blink Oilpan Pattern)
//!
//! Fornece traits de rastreabilidade (`Traceable`, `GcTracer`) e modelo de raízes (`GcRoot`)
//! para integração direta com a coleta de lixo geracional e quebra de ciclos de `ace_js`.

use crate::node::{ElementData, ElementRareData, NodeData, NodeKind};
use crate::tree::Document;
use ace_core::id::NodeId;
use std::collections::HashSet;

/// Interface para o coletor de lixo que visita referências ativas (Tri-color marking).
pub trait GcTracer {
    /// Registra um nó do DOM como vivo/alcançável.
    fn trace_node(&mut self, node_id: NodeId);
}

/// Trait implementado por todas as estruturas que contêm referências a nós ou objetos JS.
pub trait Traceable {
    /// Rastreia e visita todas as referências diretas de nós e recursos gerenciados.
    fn trace(&self, tracer: &mut dyn GcTracer);
}

impl Traceable for NodeData {
    fn trace(&self, tracer: &mut dyn GcTracer) {
        if let Some(parent) = self.parent {
            tracer.trace_node(parent);
        }
        if let Some(child) = self.first_child {
            tracer.trace_node(child);
        }
        if let Some(child) = self.last_child {
            tracer.trace_node(child);
        }
        if let Some(prev) = self.prev_sibling {
            tracer.trace_node(prev);
        }
        if let Some(next) = self.next_sibling {
            tracer.trace_node(next);
        }

        match &self.kind {
            NodeKind::Element(el) => el.trace(tracer),
            NodeKind::ShadowRoot(s) => tracer.trace_node(s.host),
            _ => {}
        }
    }
}

impl Traceable for ElementData {
    fn trace(&self, tracer: &mut dyn GcTracer) {
        if let Some(ref rare) = self.rare_data {
            rare.trace(tracer);
        }
    }
}

impl Traceable for ElementRareData {
    fn trace(&self, tracer: &mut dyn GcTracer) {
        if let Some(sr) = self.shadow_root {
            tracer.trace_node(sr);
        }
        if let Some(tc) = self.template_content {
            tracer.trace_node(tc);
        }
        if let Some(fo) = self.form_owner {
            tracer.trace_node(fo);
        }
    }
}

impl Traceable for Document {
    fn trace(&self, tracer: &mut dyn GcTracer) {
        tracer.trace_node(self.root());
        if let Some(doctype) = self.doctype {
            tracer.trace_node(doctype);
        }
        if let Some(doc_el) = self.document_element {
            tracer.trace_node(doc_el);
        }
        if let Some(head) = self.head {
            tracer.trace_node(head);
        }
        if let Some(body) = self.body {
            tracer.trace_node(body);
        }
    }
}

/// Coletor básico de marcação para inspeção e validação de alcançabilidade de raízes.
#[derive(Debug, Default)]
pub struct MarkTracer {
    pub visited_nodes: HashSet<NodeId>,
}

impl MarkTracer {
    pub fn new() -> Self {
        Self {
            visited_nodes: HashSet::new(),
        }
    }

    /// Retorna `true` se o nó foi visitado durante o ciclo de marcação do GC.
    #[inline]
    pub fn is_marked(&self, node_id: NodeId) -> bool {
        self.visited_nodes.contains(&node_id)
    }

    /// Rastreia recursivamente todos os nós alcançáveis a partir das raízes do documento.
    pub fn trace_document(&mut self, doc: &Document) {
        doc.trace(self);

        let mut queue: Vec<NodeId> = self.visited_nodes.iter().copied().collect();
        while let Some(node_id) = queue.pop() {
            if let Some(node) = doc.get_node(node_id) {
                let mut local_tracer = MarkTracer::new();
                node.trace(&mut local_tracer);
                for target in local_tracer.visited_nodes {
                    if self.visited_nodes.insert(target) {
                        queue.push(target);
                    }
                }
            }
        }
    }
}

impl GcTracer for MarkTracer {
    fn trace_node(&mut self, node_id: NodeId) {
        self.visited_nodes.insert(node_id);
    }
}
