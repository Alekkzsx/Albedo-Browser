//! # Driver de Reestilização Reativa em Tempo Real (`RestyleDriver`)
//!
//! Conecta mutações do DOM diretamente a invalidações cirúrgicas de regras CSS
//! e atualizações incrementais da `RenderTree` com latência de microssegundos.

use crate::engine::invalidation::DOMMutation;
use crate::engine::style_engine::{RenderTree, StyleEngine};
use ace_core::id::NodeId;
use ace_dom::tree::Document;

/// Driver reativo de reestilização incremental do DOM.
#[derive(Default)]
pub struct RestyleDriver {
    /// Motor de estilo com MPC e regras indexadas
    pub engine: StyleEngine,
    /// Última versão ativa da árvore de renderização
    pub current_render_tree: RenderTree,
}

impl RestyleDriver {
    /// Cria um novo driver a partir de um StyleEngine e compila a RenderTree inicial do Documento.
    pub fn new(mut engine: StyleEngine, doc: &Document) -> Self {
        let current_render_tree = engine.compute_parallel(doc);
        Self {
            engine,
            current_render_tree,
        }
    }

    /// Processa uma mutação estrutural ou de atributos no DOM, executando invalidação cirúrgica
    /// e recalculando a RenderTree de forma incremental.
    pub fn on_mutation(
        &mut self,
        doc: &Document,
        node_id: NodeId,
        mutation: &DOMMutation,
    ) -> usize {
        let invalidation_res = self.engine.invalidate_mutation(doc, node_id, mutation);
        if invalidation_res.invalidated_count > 0 {
            self.current_render_tree = self.engine.compute_styles(doc);
        }
        invalidation_res.invalidated_count
    }

    /// Retorna uma referência à RenderTree atualmente ativa.
    pub fn render_tree(&self) -> &RenderTree {
        &self.current_render_tree
    }
}
