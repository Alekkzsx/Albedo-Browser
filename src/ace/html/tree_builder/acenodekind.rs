use super::*;
use crate::ace::html::tokenizer_v2::{AceTokenKind, AceTokenizer};
use crate::ace::util::allocator::AceAllocator;
use fxhash::FxHashMap;
use smol_str::SmolStr;



#[derive(Debug, Clone)]
pub enum AceNodeKind<'a> {
    Document,
    Element {
        name: SmolStr,
        attributes: FxHashMap<SmolStr, SmolStr>,
    },
    Text(&'a str),
    Comment(&'a str),
}
