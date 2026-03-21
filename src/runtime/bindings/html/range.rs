use crate::runtime::bindings::html::element::Element;
use rquickjs::{Class, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Range {
    pub start_container: Option<usize>,
    pub start_offset: usize,
    pub end_container: Option<usize>,
    pub end_offset: usize,
    pub collapsed: bool,
}

#[rquickjs::methods]
impl Range {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            start_container: None,
            start_offset: 0,
            end_container: None,
            end_offset: 0,
            collapsed: true,
        }
    }

    #[qjs(rename = "setStart")]
    pub fn set_start(&mut self, node: Value<'_>, offset: usize) {
        if let Some(obj) = node.as_object() {
            if let Some(el_class) = Class::<Element>::from_object(&obj) {
                self.start_container = Some(el_class.borrow().index);
                self.start_offset = offset;
                self.collapsed = self.start_container == self.end_container
                    && self.start_offset == self.end_offset;
            }
        }
    }

    #[qjs(rename = "setEnd")]
    pub fn set_end(&mut self, node: Value<'_>, offset: usize) {
        if let Some(obj) = node.as_object() {
            if let Some(el_class) = Class::<Element>::from_object(&obj) {
                self.end_container = Some(el_class.borrow().index);
                self.end_offset = offset;
                self.collapsed = self.start_container == self.end_container
                    && self.start_offset == self.end_offset;
            }
        }
    }

    #[qjs(rename = "collapse")]
    pub fn collapse(&mut self, to_start: Option<bool>) {
        if to_start.unwrap_or(false) {
            self.end_container = self.start_container;
            self.end_offset = self.start_offset;
        } else {
            self.start_container = self.end_container;
            self.start_offset = self.end_offset;
        }
        self.collapsed = true;
    }

    #[qjs(rename = "selectNode")]
    pub fn select_node(&mut self, node: Value<'_>) {
        if let Some(obj) = node.as_object() {
            if let Some(el_class) = Class::<Element>::from_object(&obj) {
                let idx = el_class.borrow().index;
                self.start_container = Some(idx);
                self.end_container = Some(idx);
                self.start_offset = 0;

                // Assuming child nodes length to set end_offset correctly,
                // but since we only have node_idx here, we approximate or leave as 1 for element boundary
                self.end_offset = 1;
                self.collapsed = false;
            }
        }
    }

    #[qjs(rename = "cloneRange")]
    pub fn clone_range(&self) -> Self {
        self.clone()
    }

    #[qjs(rename = "detach")]
    pub fn detach(&self) {}

    #[qjs(get, rename = "startOffset")]
    pub fn get_start_offset(&self) -> usize {
        self.start_offset
    }

    #[qjs(get, rename = "endOffset")]
    pub fn get_end_offset(&self) -> usize {
        self.end_offset
    }

    #[qjs(get, rename = "collapsed")]
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }
}
