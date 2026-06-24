use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::mem;
use std::rc::{Rc, Weak};

use fxhash::FxHashSet;
use html5ever::tendril::StrTendril;
use html5ever::{Attribute, QualName};
use markup5ever::interface::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use smallvec::SmallVec;

use super::types::{
    DoctypeToken, HtmlDocument, HtmlElement, HtmlNode, Namespace, ParseError,
    ParseErrorKind, ParseErrorSource, ParseResult, ParseStats, ParserOptions,
};


pub mod handle; pub use handle::*;
pub mod weakhandle; pub use weakhandle::*;
pub mod acesinknodedata; pub use acesinknodedata::*;
pub mod acesinknode; pub use acesinknode::*;
pub mod append_node; pub use append_node::*;
pub mod append_to_existing_text; pub use append_to_existing_text::*;
pub mod get_parent_and_index; pub use get_parent_and_index::*;
pub mod detach_from_parent; pub use detach_from_parent::*;
pub mod acetreesink; pub use acetreesink::*;
pub mod convert_document; pub use convert_document::*;
pub mod convert_node; pub use convert_node::*;
pub mod convert_children; pub use convert_children::*;
pub mod map_namespace; pub use map_namespace::*;
pub mod attribute_name_to_string; pub use attribute_name_to_string::*;
pub mod serialize_children_as_text; pub use serialize_children_as_text::*;
pub mod acetreesink_impl_1; pub use acetreesink_impl_1::*;
pub mod acetreesink_impl_2; pub use acetreesink_impl_2::*;
pub mod acetreesink_impl_3; pub use acetreesink_impl_3::*;
