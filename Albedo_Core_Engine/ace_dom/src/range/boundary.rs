//! # Pontos de Contorno DOM (Boundary Points — WHATWG DOM §5)

use ace_core::id::NodeId;

/// Ponto de contorno na árvore DOM definido por um nó e um deslocamento numérico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryPoint {
    /// O nó que contém o ponto de contorno.
    pub node: NodeId,
    /// O deslocamento (índice de filho ou offset de caractere).
    pub offset: usize,
}

impl BoundaryPoint {
    /// Cria um novo ponto de contorno.
    #[inline]
    pub fn new(node: NodeId, offset: usize) -> Self {
        Self { node, offset }
    }
}

/// Constantes de comparação entre pontos de contorno de um Range (WHATWG DOM §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeComparison {
    /// Compara o início de `source` com o início de `this`.
    StartToStart = 0,
    /// Compara o início de `source` com o fim de `this`.
    StartToEnd = 1,
    /// Compara o fim de `source` com o fim de `this`.
    EndToEnd = 2,
    /// Compara o fim de `source` com o início de `this`.
    EndToStart = 3,
}
