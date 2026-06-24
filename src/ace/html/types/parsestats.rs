use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParseStats {
    pub total_errors: usize,
    pub total_preloads: usize,
    pub parse_time_us: u128,
    pub input_bytes: usize,
    pub fast_path_used: bool,
}
