//! # Fila e Níveis de Prioridade de Recursos (`PriorityLevel`)
//!
//! Alinhado com a semântica de escalonamento de recursos do Chromium (PrioritizedDispatcher)
//! e WHATWG Fetch Resource Priority.

use std::cmp::Ordering;

/// Níveis de prioridade de escalonamento de requisições de rede.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PriorityLevel {
    /// Menor prioridade: Prefetch de páginas futuras, analytics em background, beacons.
    Lowest = 0,
    /// Baixa prioridade: Imagens fora da viewport, preloads especulativos não críticos.
    Low = 1,
    /// Prioridade média: Imagens visíveis na viewport inicial, scripts assíncronos (`async`).
    #[default]
    Medium = 2,
    /// Alta prioridade: Folhas de estilo bloqueantes de renderização, fontes web (`@font-face`), scripts síncronos.
    High = 3,
    /// Prioridade máxima: Documento HTML principal da navegação, relatórios de violação de CSP.
    VeryHigh = 4,
}

impl PriorityLevel {
    /// Retorna representação textual para logs e DevTools.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lowest => "Lowest",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::VeryHigh => "VeryHigh",
        }
    }

    /// Retorna true se a prioridade for considerada bloqueante para a primeira renderização (FCP).
    pub const fn is_render_blocking(self) -> bool {
        matches!(self, Self::High | Self::VeryHigh)
    }

    /// Converte o nível de prioridade no cabeçalho padronizado RFC 9218 (`Priority: u=..., i`).
    pub fn to_rfc9218_header(self) -> http::HeaderValue {
        match self {
            Self::VeryHigh => http::HeaderValue::from_static("u=0"),
            Self::High => http::HeaderValue::from_static("u=1"),
            Self::Medium => http::HeaderValue::from_static("u=3, i"),
            Self::Low => http::HeaderValue::from_static("u=5, i"),
            Self::Lowest => http::HeaderValue::from_static("u=7, i"),
        }
    }
}

/// Item ordenável por prioridade para processamento em filas de prioridade.
#[derive(Debug)]
pub struct PrioritizedItem<T> {
    pub priority: PriorityLevel,
    pub sequence_id: u64,
    pub item: T,
}

impl<T> PartialEq for PrioritizedItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence_id == other.sequence_id
    }
}

impl<T> Eq for PrioritizedItem<T> {}

impl<T> PartialOrd for PrioritizedItem<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for PrioritizedItem<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Prioridade maior sai primeiro
        self.priority.cmp(&other.priority)
            // Se mesma prioridade, menor sequence_id (FIFO) sai primeiro
            .then_with(|| other.sequence_id.cmp(&self.sequence_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BinaryHeap;

    #[test]
    fn test_priority_ordering() {
        assert!(PriorityLevel::VeryHigh > PriorityLevel::High);
        assert!(PriorityLevel::High > PriorityLevel::Medium);
        assert!(PriorityLevel::Medium > PriorityLevel::Low);
        assert!(PriorityLevel::Low > PriorityLevel::Lowest);
    }

    #[test]
    fn test_binary_heap_prioritized_dispatch() {
        let mut heap = BinaryHeap::new();

        heap.push(PrioritizedItem {
            priority: PriorityLevel::Low,
            sequence_id: 1,
            item: "image.png",
        });

        heap.push(PrioritizedItem {
            priority: PriorityLevel::VeryHigh,
            sequence_id: 2,
            item: "index.html",
        });

        heap.push(PrioritizedItem {
            priority: PriorityLevel::High,
            sequence_id: 3,
            item: "styles.css",
        });

        heap.push(PrioritizedItem {
            priority: PriorityLevel::High,
            sequence_id: 4,
            item: "script.js",
        });

        assert_eq!(heap.pop().unwrap().item, "index.html"); // VeryHigh
        assert_eq!(heap.pop().unwrap().item, "styles.css"); // High (FIFO id 3)
        assert_eq!(heap.pop().unwrap().item, "script.js");  // High (FIFO id 4)
        assert_eq!(heap.pop().unwrap().item, "image.png");  // Low
    }
}
