use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub code: String,
    pub source: ParseErrorSource,
    pub kind: ParseErrorKind,
    pub line: usize,
    pub column: usize,
    pub message: String,
}
