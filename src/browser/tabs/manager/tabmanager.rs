use super::*;
use super::collection::TabCollection;
use super::tab::TabMode;
use crate::ace::engine::AceEngine;
use crate::network::resources::ResourceManager;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::mpsc;


#[derive(Clone)]
pub struct TabManager {
    collection: Rc<RefCell<TabCollection>>,
}
