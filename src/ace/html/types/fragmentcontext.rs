use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentContext {
    pub tag_name: String,
    pub namespace: Namespace,
    pub scripting_enabled: bool,
}

impl FragmentContext {
    /// TODO: add docs
    pub fn new(tag_name: &str) -> Self {
        Self {
            tag_name: tag_name.to_string(),
            namespace: Namespace::Html,
            scripting_enabled: true,
        }
    }

    /// TODO: add docs
    pub fn with_scripting(mut self, enabled: bool) -> Self {
        self.scripting_enabled = enabled;
        self
    }
}
