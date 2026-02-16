use rquickjs::{Ctx, Class, Result, Value};
use crate::engine::dom::{AceDOM, AceNodeType, AceNode};
use std::sync::{Arc, Mutex};
use crate::runtime::bindings::html::element::Element;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct DocumentFragment {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    #[qjs(skip_trace)]
    pub index: usize,
    #[qjs(skip_trace)]
    pub mutations: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub stylesheet_dirty: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub primitives: Arc<Mutex<Vec<crate::engine::ACEPrimitive>>>,
}

#[rquickjs::methods]
impl DocumentFragment {
    #[qjs(rename = "appendChild")]
    pub fn append_child<'js>(&self, ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
        let child_idx = child.borrow().index;
        let parent_idx = self.index;
        if let Ok(mut dom) = self.dom.lock() {
            dom.append_child(parent_idx, child_idx);
        }
        Ok(child)
    }

    #[qjs(get, rename = "children")]
    pub fn children<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        // Reuse hierarchy logic if possible, or implement here
        crate::runtime::bindings::html::element::hierarchy::children(&self.as_element_stub(), ctx)
    }
}

impl DocumentFragment {
    pub fn new(dom: Arc<Mutex<AceDOM>>, mutations: Arc<Mutex<bool>>, stylesheet_dirty: Arc<Mutex<bool>>, primitives: Arc<Mutex<Vec<crate::engine::ACEPrimitive>>>) -> Self {
        let mut d = dom.lock().unwrap();
        let index = d.nodes.len();
        d.nodes.push(AceNode {
            node_type: AceNodeType::DocumentFragment,
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
        });
        Self { dom: dom.clone(), index, mutations, stylesheet_dirty, primitives }
    }

    // Helper to reuse element logic
    fn as_element_stub(&self) -> Element {
        Element {
            dom: self.dom.clone(),
            index: self.index,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
            primitives: self.primitives.clone(),
        }
    }
}
