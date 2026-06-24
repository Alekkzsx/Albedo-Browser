// AceDOM - Range API Implementation
// FASE 2: Range API completa (W3C DOM Range spec)
// Status: 100% implementado e documentado

use crate::ace::engine::dom::{AceDOM, NodeId, NodeType};

/// Ponto de limite (boundary point) no Range

pub mod boundarypoint; pub use boundarypoint::*;
pub mod range; pub use range::*;
pub mod range_impl_1; pub use range_impl_1::*;
pub mod range_impl_2; pub use range_impl_2::*;
pub mod range_impl_3; pub use range_impl_3::*;
pub mod get_all_text_content; pub use get_all_text_content::*;
pub mod test_range_new_is_collapsed; pub use test_range_new_is_collapsed::*;
