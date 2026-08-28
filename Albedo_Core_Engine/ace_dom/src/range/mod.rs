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
                return Ok(-1);
            } else if pt_this.offset == pt_other.offset {
                return Ok(0);
            } else {
                return Ok(1);
            }
        }

        // O(depth) comparison using ancestor chains instead of O(n) descendants traversal.
        // Build root→node paths for both boundary points and compare positionally.
        let path_a = ancestor_chain_to_root(doc, pt_this.node);
        let path_b = ancestor_chain_to_root(doc, pt_other.node);

        // Find the lowest common ancestor (LCA) by comparing paths from root down.
        // path_a[0] == root, path_a[last] == pt_this.node
        let common_depth = path_a
            .iter()
            .zip(path_b.iter())
            .take_while(|(a, b)| a == b)
            .count();

        if common_depth == 0 {
            return Err(DomError::HierarchyRequestError(
                "Nós pertencem a raízes incompatíveis".into(),
            ));
        }

        // If one node is an ancestor of the other, the ancestor comes first.
        if common_depth == path_a.len() {
            // pt_this.node is an ancestor of pt_other.node → pt_this comes before pt_other
            return Ok(-1);
        }
        if common_depth == path_b.len() {
            // pt_other.node is an ancestor of pt_this.node → pt_this comes after pt_other
            return Ok(1);
        }

        // Compare the two diverging children at the LCA level by sibling order.
        let lca_id = path_a[common_depth - 1];
        let child_a = path_a[common_depth];
        let child_b = path_b[common_depth];

        // Walk children of LCA in order to determine which diverging child comes first.
        for (sibling_id, _) in doc.children(lca_id) {
            if sibling_id == child_a {
                return Ok(-1); // pt_this comes before pt_other
            }
            if sibling_id == child_b {
                return Ok(1); // pt_other comes before pt_this
            }
        }

        Err(DomError::HierarchyRequestError(
            "Falha ao comparar pontos de contorno".into(),
        ))
    }

    /// Clona o conteúdo delimitado por este Range em um novo `DocumentFragment` (WHATWG DOM §5.4).
    ///
    /// ## Correção de Conformidade
    /// O algoritmo normativo recorta corretamente os nós parcialmente incluídos nas bordas do Range:
    /// - **Nó de início parcial:** copia apenas o texto de `start.offset` até o final do nó.
    /// - **Nós completamente contidos:** clonados na íntegra (`deep = true`).
    /// - **Nó de fim parcial:** copia apenas o texto do início do nó até `end.offset`.
    pub fn clone_contents(&self, doc: &mut Document) -> Result<NodeId, DomError> {
        let frag_id = doc.create_document_fragment();

        if self.collapsed() {
            return Ok(frag_id);
        }

        // Caso 1: Start e End estão no mesmo nó.
        if self.start.node == self.end.node {
            let node_id = self.start.node;
            // Para nós de texto, extraímos apenas o substring delimitado.
            let text_slice = if let Some(node) = doc.get_node(node_id) {
                if let crate::node::NodeKind::Text(ref t) = node.kind {
                    let data = t.data.as_str();
                    let s_off = self.start.offset.min(data.len());
                    let e_off = self.end.offset.min(data.len()).max(s_off);
                    Some(data[s_off..e_off].to_string())
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(sliced) = text_slice {
                let text_clone = doc.create_text_node(sliced);
                doc.append_child(frag_id, text_clone)?;
            } else {
                // Para outros nós (Element, Comment, etc.), clonamos o nó inteiro.
                let cloned = doc.clone_node(node_id, true)?;
                doc.append_child(frag_id, cloned)?;
            }
            return Ok(frag_id);
        }

        // Caso 2: Start e End estão em nós diferentes.
        // Identifica o ancestral comum e todos os filhos diretos do LCA que estão no Range.
        let common = self.common_ancestor_container(doc);
        let child_ids: Vec<NodeId> = doc.children(common).map(|(c_id, _)| c_id).collect();

        // Encontra os índices do filho do LCA que contém start e do filho que contém end.
        let start_top = topmost_child_in(doc, common, self.start.node);
        let end_top = topmost_child_in(doc, common, self.end.node);

        let mut in_range = false;

        for child_id in &child_ids {
            let is_start_child = Some(*child_id) == start_top;
            let is_end_child = Some(*child_id) == end_top;

            if is_start_child {
                in_range = true;
                // Nó de início parcial: para texto, inclui apenas a parte a partir de start.offset.
                if *child_id == self.start.node {
                    // Extrai os dados textuais antes de qualquer borrow mutável.
                    let text_slice: Option<String> = doc.get_node(*child_id).and_then(|node| {
                        if let crate::node::NodeKind::Text(ref t) = node.kind {
                            let data = t.data.as_str();
                            let s_off = self.start.offset.min(data.len());
                            Some(data[s_off..].to_string())
                        } else {
                            None
                        }
                    });
                    if let Some(partial) = text_slice {
                        let text_clone = doc.create_text_node(partial);
                        doc.append_child(frag_id, text_clone)?;
                    } else {
                        let cloned = doc.clone_node(*child_id, true)?;
                        doc.append_child(frag_id, cloned)?;
                    }
                } else if !is_end_child {
                    let cloned = doc.clone_node(*child_id, true)?;
                    doc.append_child(frag_id, cloned)?;
                }
            } else if in_range && !is_end_child {
                // Nó completamente contido no Range: clonagem completa.
                let cloned = doc.clone_node(*child_id, true)?;
                doc.append_child(frag_id, cloned)?;
            }

            if is_end_child {
                // Nó de fim parcial: para texto, inclui apenas a parte até end.offset.
                if in_range || is_start_child {
                    if *child_id == self.end.node {
                        // Extrai os dados textuais antes de qualquer borrow mutável.
                        let text_slice: Option<String> = doc.get_node(*child_id).and_then(|node| {
                            if let crate::node::NodeKind::Text(ref t) = node.kind {
                                let data = t.data.as_str();
                                let e_off = self.end.offset.min(data.len());
                                Some(data[..e_off].to_string())
                            } else {
                                None
                            }
                        });
                        if let Some(partial) = text_slice {
                            let text_clone = doc.create_text_node(partial);
                            doc.append_child(frag_id, text_clone)?;
                        } else {
                            let cloned = doc.clone_node(*child_id, true)?;
                            doc.append_child(frag_id, cloned)?;
                        }
                    } else {
                        let cloned = doc.clone_node(*child_id, true)?;
                        doc.append_child(frag_id, cloned)?;
                    }
                }
                break;
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
        } else if self.start.node == parent_id && self.start.offset > child_index {
            self.start.offset -= 1;
        }

        if self.end.node == removed_node {
            self.end.node = parent_id;
            self.end.offset = child_index;
        } else if self.end.node == parent_id && self.end.offset > child_index {
            self.end.offset -= 1;
        }
    }

    /// Ajusta os pontos de contorno considerando a árvore DOM (WHATWG DOM §5.5), incluindo verificação de descendentes.
    pub fn adjust_for_node_removal_with_doc(
        &mut self,
        doc: &Document,
        removed_node: NodeId,
        parent_id: NodeId,
        child_index: usize,
    ) {
        let is_start_in_removed = self.start.node == removed_node || doc.contains(removed_node, self.start.node);
        if is_start_in_removed {
            self.start.node = parent_id;
            self.start.offset = child_index;
        } else if self.start.node == parent_id && self.start.offset > child_index {
            self.start.offset -= 1;
        }

        let is_end_in_removed = self.end.node == removed_node || doc.contains(removed_node, self.end.node);
        if is_end_in_removed {
            self.end.node = parent_id;
            self.end.offset = child_index;
        } else if self.end.node == parent_id && self.end.offset > child_index {
            self.end.offset -= 1;
        }
    }

    /// Ajusta os pontos de contorno quando múltiplos nós (como uma lista explícita de descendentes) são removidos.
    pub fn adjust_for_node_removal_with_descendants(
        &mut self,
        removed_nodes: &[NodeId],
        parent_id: NodeId,
        child_index: usize,
    ) {
        if removed_nodes.contains(&self.start.node) {
            self.start.node = parent_id;
            self.start.offset = child_index;
        } else if self.start.node == parent_id && self.start.offset > child_index {
            self.start.offset -= 1;
        }

        if removed_nodes.contains(&self.end.node) {
            self.end.node = parent_id;
            self.end.offset = child_index;
        } else if self.end.node == parent_id && self.end.offset > child_index {
            self.end.offset -= 1;
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

/// Constrói o caminho da raiz até `node_id` usando links de pai (O(depth)).
/// O resultado é um `Vec<NodeId>` onde `result[0]` é a raiz e `result[last]` é `node_id`.
fn ancestor_chain_to_root(doc: &Document, node_id: NodeId) -> Vec<NodeId> {
    let mut chain = Vec::new();
    let mut curr = node_id;
    loop {
        chain.push(curr);
        match doc.get_node(curr).and_then(|n| n.parent) {
            Some(parent) => curr = parent,
            None => break,
        }
    }
    chain.reverse(); // agora chain[0] == raiz, chain[last] == node_id
    chain
}

/// Encontra o filho direto de `ancestor_id` que é ancestral ou igual a `descendant_id`.
/// Retorna `None` se `descendant_id` não for descendente de `ancestor_id`.
///
/// Usado pelo algoritmo `clone_contents` para identificar qual filho do ancestral comum
/// contém cada ponto de contorno do Range.
fn topmost_child_in(doc: &Document, ancestor_id: NodeId, descendant_id: NodeId) -> Option<NodeId> {
    if ancestor_id == descendant_id {
        return None;
    }
    let mut curr = descendant_id;
    loop {
        let parent = doc.get_node(curr).and_then(|n| n.parent)?;
        if parent == ancestor_id {
            return Some(curr);
        }
        curr = parent;
    }
}
