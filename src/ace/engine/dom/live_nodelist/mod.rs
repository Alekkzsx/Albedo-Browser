//! LiveNodeList - Coleções de nós que se atualizam automaticamente
//! 
//! Implementa HTMLCollection e NodeList conforme especificação WHATWG DOM.
//! Diferente de Vec<usize>, estas coleções são "live" - refletem mudanças no DOM
//! sem necessidade de re-query.

use std::cell::RefCell;
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};

/// Trait para tipos de queries suportados por LiveNodeList

pub mod nodequery; pub use nodequery::*;
pub mod tagnamequery; pub use tagnamequery::*;
pub mod classnamequery; pub use classnamequery::*;
pub mod idquery; pub use idquery::*;
pub mod livenodelist; pub use livenodelist::*;
pub mod htmlcollection; pub use htmlcollection::*;
pub mod nodelist; pub use nodelist::*;
pub mod childrencollection; pub use childrencollection::*;
pub mod test_get_elements_by_tag_name; pub use test_get_elements_by_tag_name::*;
