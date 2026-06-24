use super::*;
use crate::ace::html::tokenizer_v2::{AceTokenKind, AceTokenizer};
use crate::ace::util::allocator::AceAllocator;
use fxhash::FxHashMap;
use smol_str::SmolStr;



#[derive(Debug, Clone)]
pub struct AceNodeV2<'a> {
    pub kind: AceNodeKind<'a>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}
