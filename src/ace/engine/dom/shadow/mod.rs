//! Shadow DOM Implementation - W3C Shadow DOM v1 Spec
//! 
//! Este módulo implementa:
//! - attachShadow() com modos open/closed
//! - Slot assignment algorithm
//! - Event retargeting através de shadow boundaries
//! - Pseudo-elemento ::slotted()
//! - Host integration

use std::collections::{HashMap, HashSet};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType, NodeDirtyFlags};

/// Configuração para attachShadow()

pub mod shadowrootinit; pub use shadowrootinit::*;
pub mod shadowrootmode; pub use shadowrootmode::*;
pub mod shadowroot; pub use shadowroot::*;
pub mod slotassignment_; pub use slotassignment_::*;
pub mod slotassignment; pub use slotassignment::*;
pub mod eventpath_; pub use eventpath_::*;
pub mod eventpath; pub use eventpath::*;
pub mod acedom; pub use acedom::*;
pub mod test_attach_shadow_open; pub use test_attach_shadow_open::*;
