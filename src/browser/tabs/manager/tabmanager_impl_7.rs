use super::*;
use super::collection::TabCollection;
use super::tab::TabMode;
use crate::ace::engine::AceEngine;
use crate::network::resources::ResourceManager;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::mpsc;



impl TabManager {

    /// TODO: add docs
    pub fn take_pending_nav(&self) -> Option<String> {
        self.with_collection_mut(|col| col.pending_nav.take())
    }
}
