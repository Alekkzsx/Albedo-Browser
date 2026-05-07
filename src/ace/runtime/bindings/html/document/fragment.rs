use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use crate::ace::runtime::bindings::html::element::Element;
use rquickjs::{Class, Ctx, Result, Value};
use std::sync::{Arc, Mutex};

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
    pub primitives: Arc<Mutex<Vec<crate::ace::engine::ACEPrimitive>>>,
    #[qjs(skip_trace)]
    pub canvas_contexts:
        Arc<Mutex<std::collections::HashMap<usize, crate::ace::engine::graphics::canvas2d::Canvas2D>>>,
    #[qjs(skip_trace)]
    pub pending_scroll: Arc<Mutex<Option<usize>>>,
    #[qjs(skip_trace)]
    pub element_geometry:
        Arc<Mutex<std::collections::HashMap<usize, crate::ace::engine::ElementGeometry>>>,
    #[qjs(skip_trace)]
    pub element_scroll: Arc<Mutex<std::collections::HashMap<usize, (f32, f32)>>>,
}

#[rquickjs::methods]
impl DocumentFragment {
    #[qjs(rename = "appendChild")]
    pub fn append_child<'js>(
        &self,
        _ctx: Ctx<'js>,
        child: Class<'js, Element>,
    ) -> Result<Class<'js, Element>> {
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
        crate::ace::runtime::bindings::html::element::hierarchy::children(&self.as_element_stub(), ctx)
    }
}

impl DocumentFragment {
    pub fn new(
        dom: Arc<Mutex<AceDOM>>,
        mutations: Arc<Mutex<bool>>,
        stylesheet_dirty: Arc<Mutex<bool>>,
        primitives: Arc<Mutex<Vec<crate::ace::engine::ACEPrimitive>>>,
        canvas_contexts: Arc<
            Mutex<std::collections::HashMap<usize, crate::ace::engine::graphics::canvas2d::Canvas2D>>,
        >,
        pending_scroll: Arc<Mutex<Option<usize>>>,
        element_geometry: Arc<
            Mutex<std::collections::HashMap<usize, crate::ace::engine::ElementGeometry>>,
        >,
        element_scroll: Arc<Mutex<std::collections::HashMap<usize, (f32, f32)>>>,
    ) -> Self {
        let mut d = dom.lock().unwrap();
        let index = d.nodes.len();
        d.nodes.push(AceNode {
            node_type: AceNodeType::DocumentFragment,
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: crate::ace::engine::dom::NodeDirtyFlags::LAYOUT
                | crate::ace::engine::dom::NodeDirtyFlags::STYLE,
        });

        Self {
            dom: dom.clone(),
            index,
            mutations,
            stylesheet_dirty,
            primitives,
            canvas_contexts,
            pending_scroll,
            element_geometry,
            element_scroll,
        }
    }

    // Helper to reuse element logic
    fn as_element_stub(&self) -> Element {
        Element {
            dom: self.dom.clone(),
            index: self.index,
            mutations: self.mutations.clone(),
            stylesheet_dirty: self.stylesheet_dirty.clone(),
            primitives: self.primitives.clone(),
            canvas_contexts: self.canvas_contexts.clone(),
            pending_scroll: self.pending_scroll.clone(),
            element_geometry: self.element_geometry.clone(),
            element_scroll: self.element_scroll.clone(),
        }
    }
}
