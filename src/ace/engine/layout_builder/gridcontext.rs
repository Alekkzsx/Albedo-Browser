use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::style::Stylesheet;
use crate::ace::engine::layout::ElementGeometry;
use crate::utils::time::unix_timestamp_secs_f64;
use taffy::geometry::MinMax;
use crate::ace::engine::core::AceEngine;


#[derive(Clone, Debug, Default)]
pub struct GridContext {
    pub column_names: std::collections::HashMap<String, Vec<i16>>,
    pub row_names: std::collections::HashMap<String, Vec<i16>>,
    pub areas: std::collections::HashMap<String, (usize, usize, usize, usize)>,
    pub col_offset: i16,
    pub row_offset: i16,
}
