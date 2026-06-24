//! AceDOM Virtual DOM & Diff/Patch Engine
//! Implementação otimizada para frameworks reativos (React, Solid, Svelte)
//! Objetivo: Updates 50x mais rápidos que re-renderização completa

use std::collections::HashMap;
use std::rc::Rc;
use crate::ace::engine::dom::{AceDOM, NodeId};

/// Representação leve de um nó no Virtual DOM

pub mod vnode; pub use vnode::*;
pub mod patchop; pub use patchop::*;
pub mod diffresult; pub use diffresult::*;
pub mod virtualdom_; pub use virtualdom_::*;
pub mod virtualdom; pub use virtualdom::*;
pub mod test_diff_text_change; pub use test_diff_text_change::*;
