use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceType {
    Stylesheet,
    Script,
    ModulePreload,
    Image,
    Fetch,
    Other,
}
