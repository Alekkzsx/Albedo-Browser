//! # Motor de Faixas e Seleção DOM (Range Engine — WHATWG DOM §5)
//!
//! Representa um fragmento contíguo de conteúdo de um documento delimitado por dois pontos de contorno.

pub mod boundary;
pub mod registry;

pub use boundary::{BoundaryPoint, RangeComparison};
pub use registry::{LiveRangeHandle, LiveRangeRegistry};

use crate::error::DomError;
use crate::tree::Document;
use ace_core::id::NodeId;

/// Objeto `Range` normativo representando um intervalo entre dois pontos na árvore DOM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    pub start: BoundaryPoint,
    pub end: BoundaryPoint,
}

impl Range {
    /// Cria um novo `Range` colapsado na raiz do documento.
    pub fn new(root_id: NodeId) -> Self {
        Self {
            start: BoundaryPoint::new(root_id, 0),
            end: BoundaryPoint::new(root_id, 0),
        }
    }

    /// Cria um `Range` com início e fim explícitos.
    pub fn from_points(start: BoundaryPoint, end: BoundaryPoint) -> Self {
        Self { start, end }
    }

    /// Retorna `true` se o início e o fim coincidirem no mesmo ponto.
    #[inline]
    pub fn collapsed(&self) -> bool {
        self.start == self.end
    }

    /// Define o ponto inicial do Range.
    pub fn set_start(&mut self, node: NodeId, offset: usize) {
        self.start = BoundaryPoint::new(node, offset);
    }

    /// Define o ponto final do Range.
    pub fn set_end(&mut self, node: NodeId, offset: usize) {
        self.end = BoundaryPoint::new(node, offset);
    }

    /// Retorna o ancestral comum mais próximo entre o início e o fim do Range.
    pub fn common_ancestor_container(&self, doc: &Document) -> NodeId {
        if self.start.node == self.end.node {
            return self.start.node;
        }

        let mut start_ancestors = Vec::new();
        start_ancestors.push(self.start.node);
        for (a_id, _) in doc.ancestors(self.start.node) {
            start_ancestors.push(a_id);
        }

        let mut end_ancestors = Vec::new();
        end_ancestors.push(self.end.node);
        for (a_id, _) in doc.ancestors(self.end.node) {
            end_ancestors.push(a_id);
        }

        for s_id in &start_ancestors {
            if end_ancestors.contains(s_id) {
                return *s_id;
            }
        }

        doc.root()
    }

    /// Compara os pontos de contorno entre este Range e outro Range (WHATWG DOM §5.3).
    pub fn compare_boundary_points(
        &self,
        doc: &Document,
        how: RangeComparison,
        other: &Range,
    ) -> Result<i8, DomError> {
        let (pt_this, pt_other) = match how {
            RangeComparison::StartToStart => (self.start, other.start),
            RangeComparison::StartToEnd => (self.end, other.start),
            RangeComparison::EndToEnd => (self.end, other.end),
            RangeComparison::EndToStart => (self.start, other.end),
        };

        if pt_this.node == pt_other.node {
            if pt_this.offset < pt_other.offset {
                Ok(-1)
            } else if pt_this.offset == pt_other.offset {
                Ok(0)
            } else {
                Ok(1)
            }
        } else {
            // Compara ordem de pré-ordem na árvore
            let mut this_pos = None;
            let mut other_pos = None;
            for (idx, (n_id, _)) in doc.descendants(doc.root()).enumerate() {
                if n_id == pt_this.node {
                    this_pos = Some(idx);
                }
                if n_id == pt_other.node {
                    other_pos = Some(idx);
                }
                if this_pos.is_some() && other_pos.is_some() {
                    break;
                }
            }

            match (this_pos, other_pos) {
                (Some(t), Some(o)) => {
                    if t < o {
                        Ok(-1)
                    } else {
                        Ok(1)
                    }
                }
                _ => Err(DomError::HierarchyRequestError("Nós pertencem a raízes incompatíveis".into())),
            }
        }
    }

    /// Clona o conteúdo delimitado por este Range em um novo `DocumentFragment`.
    pub fn clone_contents(&self, doc: &mut Document) -> Result<NodeId, DomError> {
        let frag_id = doc.create_document_fragment();

        if self.collapsed() {
            return Ok(frag_id);
        }

        if self.start.node == self.end.node {
            let node_id = self.start.node;
            let text_slice_opt = if let Some(node) = doc.get_node(node_id) {
                if let crate::node::NodeKind::Text(ref t) = node.kind {
                    let s_off = self.start.offset.min(t.data.len());
                    let e_off = self.end.offset.min(t.data.len()).max(s_off);
                    Some(t.data[s_off..e_off].to_string())
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(sliced_text) = text_slice_opt {
                let text_clone = doc.create_text_node(sliced_text);
                doc.append_child(frag_id, text_clone)?;
                return Ok(frag_id);
            }

            let cloned = doc.clone_node(self.start.node, true)?;
            doc.append_child(frag_id, cloned)?;
        } else {
            let common = self.common_ancestor_container(doc);
            let child_ids: Vec<NodeId> = doc.children(common).map(|(c_id, _)| c_id).collect();
            for child_id in child_ids {
                let cloned = doc.clone_node(child_id, true)?;
                doc.append_child(frag_id, cloned)?;
            }
        }

        Ok(frag_id)
    }

    /// Remove o conteúdo contido no intervalo do Range mantendo a árvore íntegra (WHATWG DOM §5.4).
    pub fn delete_contents(&mut self, doc: &mut Document) -> Result<(), DomError> {
        if self.collapsed() {
            return Ok(());
        }

        if self.start.node == self.end.node {
            let node_id = self.start.node;
            if let Some(node) = doc.get_node_mut(node_id) {
                if let crate::node::NodeKind::Text(ref mut t) = node.kind {
                    let s_off = self.start.offset.min(t.data.len());
                    let e_off = self.end.offset.min(t.data.len()).max(s_off);
                    let mut s = t.data.to_string();
                    s.replace_range(s_off..e_off, "");
                    t.data = smol_str::SmolStr::new(s);
                    self.end.offset = self.start.offset;
                    return Ok(());
                }
            }
            if let Some(parent_id) = doc.get_node(node_id).and_then(|n| n.parent) {
                let _ = doc.remove_child(parent_id, node_id);
                self.end = self.start;
            }
        } else {
            let common = self.common_ancestor_container(doc);
            let mut to_remove = Vec::new();
            for (child_id, _) in doc.children(common) {
                if child_id != self.start.node && child_id != self.end.node {
                    to_remove.push(child_id);
                }
            }
            for child_id in to_remove {
                let _ = doc.remove_child(common, child_id);
            }
            self.end = self.start;
        }

        Ok(())
    }

    /// Extrai o conteúdo contido no Range, removendo-o da árvore e retornando em um DocumentFragment.
    pub fn extract_contents(&mut self, doc: &mut Document) -> Result<NodeId, DomError> {
        let frag_id = self.clone_contents(doc)?;
        self.delete_contents(doc)?;
        Ok(frag_id)
    }

    /// Insere um nó no ponto inicial do Range (WHATWG DOM §5.4).
    pub fn insert_node(&mut self, doc: &mut Document, node_id: NodeId) -> Result<(), DomError> {
        let start_node = self.start.node;
        if let Some(parent_id) = doc.get_node(start_node).and_then(|n| n.parent) {
            doc.insert_before(parent_id, node_id, Some(start_node))?;
        } else {
            doc.append_child(start_node, node_id)?;
        }
        Ok(())
    }

    /// Envelopa o conteúdo do Range dentro de um novo elemento pai.
    pub fn surround_contents(&mut self, doc: &mut Document, new_parent: NodeId) -> Result<(), DomError> {
        let extracted = self.extract_contents(doc)?;
        doc.append_child(new_parent, extracted)?;
        self.insert_node(doc, new_parent)?;
        Ok(())
    }

    /// Ajusta os pontos de contorno quando um nó de texto é dividido em dois (WHATWG DOM §5.5).
    pub fn adjust_for_split_text(&mut self, orig_node: NodeId, new_node: NodeId, split_offset: usize) {
        if self.start.node == orig_node && self.start.offset > split_offset {
            self.start.node = new_node;
            self.start.offset -= split_offset;
        }
        if self.end.node == orig_node && self.end.offset > split_offset {
            self.end.node = new_node;
            self.end.offset -= split_offset;
        }
    }

    /// Ajusta os pontos de contorno quando um nó é removido (WHATWG DOM §5.5).
    pub fn adjust_for_node_removal(&mut self, removed_node: NodeId, parent_id: NodeId, child_index: usize) {
        if self.start.node == removed_node {
            self.start.node = parent_id;
            self.start.offset = child_index;
        }
        if self.end.node == removed_node {
            self.end.node = parent_id;
            self.end.offset = child_index;
        }
    }

    /// Ajusta os pontos de contorno quando um nó é inserido (WHATWG DOM §5.5).
    pub fn adjust_for_node_insertion(&mut self, parent_id: NodeId, insertion_index: usize) {
        if self.start.node == parent_id && self.start.offset >= insertion_index {
            self.start.offset += 1;
        }
        if self.end.node == parent_id && self.end.offset >= insertion_index {
            self.end.offset += 1;
        }
    }
}
