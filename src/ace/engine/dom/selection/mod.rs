// AceDOM - Selection API Implementation
// FASE 2: Selection API completa (WHATWG Selection spec)
// Status: 100% implementado e documentado

use std::cell::RefCell;
use std::rc::Rc;
use crate::ace::engine::dom::{NodeId, Range};

/// Direção da seleção

pub mod selectiondirection; pub use selectiondirection::*;
pub mod boundarypoint; pub use boundarypoint::*;
pub mod selection; pub use selection::*;
pub mod selection_impl_1; pub use selection_impl_1::*;
pub mod selection_impl_2; pub use selection_impl_2::*;
pub mod selection_impl_3; pub use selection_impl_3::*;
pub mod selectiontype; pub use selectiontype::*;
pub mod test_selection_new_is_empty; pub use test_selection_new_is_empty::*;
pub mod arrowkey; pub use arrowkey::*;
