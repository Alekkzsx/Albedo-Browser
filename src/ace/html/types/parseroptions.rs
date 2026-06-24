use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParserOptions {
    pub base_url: Option<String>,
    pub encoding_hint: Option<Encoding>,
    pub scripting_enabled: bool,
    pub fragment_context: Option<FragmentContext>,
}

impl Default for ParserOptions {
pub(crate) fn default() -> Self {
        Self {
            base_url: None,
            encoding_hint: None,
            scripting_enabled: true,
            fragment_context: None,
        }
    }
}
