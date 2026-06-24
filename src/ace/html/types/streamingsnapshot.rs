use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingSnapshot {
    pub raw_bytes: Vec<u8>,
    pub buffer: String,
    pub decided_encoding: Option<Encoding>,
}
