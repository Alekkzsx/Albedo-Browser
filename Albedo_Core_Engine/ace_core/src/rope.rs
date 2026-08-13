// ============================================================================
// Albedo Core Engine (ACE)
// File: rope.rs
// Description: Text Ropes. Estrutura em Árvore Binária para Textos Gigantes.
//              Permite edições e concatenações O(1) (Zero-Copy) essenciais para
//              manipulação pesada do DOM (ex: innerHTML massivo).
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::sync::Arc;
use crate::string::AceString;

#[derive(Clone, Debug, PartialEq)]
pub enum RopeNode {
    Leaf(AceString),
    Concat {
        left: Arc<RopeNode>,
        right: Arc<RopeNode>,
        length: usize, // Cache do comprimento para consultas O(1)
    },
}

impl RopeNode {
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            RopeNode::Leaf(s) => s.len(),
            RopeNode::Concat { length, .. } => *length,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A "Corda" de Texto. Embrulha a raiz da árvore e oculta o estado imutável de `Arc`.
#[derive(Clone, Debug, PartialEq)]
pub struct Rope {
    root: Arc<RopeNode>,
}

impl Rope {
    /// Cria uma Rope vazia apontando para uma AceString pequena otimizada em stack (SSO).
    #[inline]
    pub fn new() -> Self {
        Self {
            root: Arc::new(RopeNode::Leaf(AceString::new())),
        }
    }

    /// Constrói uma Rope a partir de uma String nativa, comprimindo usando AceString.
    pub fn from_str(s: &str) -> Self {
        Self {
            root: Arc::new(RopeNode::Leaf(AceString::from_str(s))),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.root.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// O coração da Rope: Concatenação Híper-Rápida O(1).
    /// Não há cópia de memória (`memcpy`), apenas alocação de 1 nó de controle (24 bytes).
    pub fn concat(left: &Rope, right: &Rope) -> Rope {
        if left.is_empty() {
            return right.clone();
        }
        if right.is_empty() {
            return left.clone();
        }

        let length = left.len() + right.len();

        let new_root = RopeNode::Concat {
            left: left.root.clone(),
            right: right.root.clone(),
            length,
        };

        Rope { root: Arc::new(new_root) }
    }

    /// Achata a árvore e extrai a String completa.
    /// Em ambientes reais de renderização, criaremos um Iterador `Chars` em vez disso
    /// para desenhar letras na tela sem alocar uma buffer contíguo.
    pub fn to_string(&self) -> String {
        let mut result = String::with_capacity(self.len());
        self.collect_to_string(&self.root, &mut result);
        result
    }

    fn collect_to_string(&self, node: &RopeNode, buf: &mut String) {
        match node {
            RopeNode::Leaf(s) => buf.push_str(&s.to_string()),
            RopeNode::Concat { left, right, .. } => {
                self.collect_to_string(left, buf);
                self.collect_to_string(right, buf);
            }
        }
    }
}

impl Default for Rope {
    fn default() -> Self {
        Self::new()
    }
}
