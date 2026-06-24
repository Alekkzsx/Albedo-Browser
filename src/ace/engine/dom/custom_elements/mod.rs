// Custom Elements v1 Implementation - WHATWG Spec
//
// Este módulo implementa:
// - customElements.define()
// - Lifecycle callbacks (connected, disconnected, adopted, attributeChanged)
// - Upgrade algorithm
// - Built-in element extension

use std::collections::HashMap;
use std::sync::Arc;
use crate::ace::engine::dom::{AceDOM, AceNodeType, AceNode, AceElement, NodeDirtyFlags};

/// Registry global de Custom Elements

pub mod customelementsregistry; pub use customelementsregistry::*;
pub mod customelementdefinition; pub use customelementdefinition::*;
pub mod elementdefinitionoptions; pub use elementdefinitionoptions::*;
pub mod customelementerror; pub use customelementerror::*;
pub mod lifecyclecallbacks; pub use lifecyclecallbacks::*;
pub mod acedom; pub use acedom::*;
pub mod test_define_valid_name; pub use test_define_valid_name::*;
