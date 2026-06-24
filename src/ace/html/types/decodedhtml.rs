use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedHtml {
    pub content: String,
    pub encoding: Encoding,
}
