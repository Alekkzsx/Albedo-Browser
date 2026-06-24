use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


#[derive(Clone, Debug)]
pub struct AceDOM {
    pub nodes: Vec<AceNode>,
    pub root: usize,
    pub head: Option<usize>,
    pub body: Option<usize>,
    pub observers: HashMap<usize, Vec<DomObserver>>, // Map target_node_id -> Observers
    pub pending_mutations: RefCell<HashMap<usize, Vec<MutationRecord>>>, // Map callback_id -> Records
    pub active_element: Option<usize>,
    pub subframes: Option<Arc<Mutex<HashMap<usize, Arc<Mutex<crate::ace::engine::AceEngine>>>>>>,
    /// Índice do nó <iframe> que este DOM representa no frame pai.
    /// None se este for o frame raiz (não um subframe).
    pub iframe_node_idx: Option<usize>,
}
