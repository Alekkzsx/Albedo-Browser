use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};

#[cfg(feature = "ace_html_parser")]
use crate::ace::html::build_document_with_errors;
// kuchiki removido - usando ACE-HTML parser proprietário
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub mod arena;
pub mod live_nodelist;
pub mod range;
pub mod selection;
pub mod shadow;
pub mod custom_elements;
pub mod a11y;
pub mod virtual_dom;
pub mod string_intern;
pub mod wpt_harness;
pub mod benchmarks;

pub use arena::{DomArena, ArenaNode};
pub use live_nodelist::{LiveNodeList, HTMLCollection, NodeList, ChildrenCollection, NodeQuery, TagNameQuery, ClassNameQuery, IdQuery};
pub use range::Range;
pub use selection::{Selection, SelectionDirection, SelectionType};
pub use shadow::{ShadowRoot, ShadowRootInit, ShadowRootMode, SlotAssignment, EventPath};
pub use custom_elements::{CustomElementsRegistry, CustomElementDefinition, LifecycleCallbacks, CustomElementError};
pub use a11y::{AccessibilityTree, AccessibilityNode, AriaRole, AriaStates, AriaProperties, ImplicitRoleMap, AccessibleNameComputer};
pub use virtual_dom::{VNode, PatchOp, DiffResult, VirtualDom};
pub use string_intern::global as string_interning;
pub use wpt_harness::{WPTRunner, WPTBuilder, TestStatus, SuiteResult};
pub use benchmarks::{AceDOMBenchmarks, BenchmarkResult};

pub type NodeId = usize;
