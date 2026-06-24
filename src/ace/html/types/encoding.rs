use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Windows1252,
    Utf16Le,
    Utf16Be,
}
