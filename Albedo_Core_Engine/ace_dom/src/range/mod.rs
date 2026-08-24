//! # Motor de Faixas e Seleção DOM (Range Engine — WHATWG DOM §5)
//!
//! Representa um fragmento contíguo de conteúdo de um documento delimitado por dois pontos de contorno.

pub mod boundary;

pub use boundary::{BoundaryPoint, RangeComparison};

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
}
