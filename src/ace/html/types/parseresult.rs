use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseResult {
    pub document: HtmlDocument,
    pub errors: Vec<ParseError>,
    pub parse_errors: Vec<ParseError>,
    pub preload_requests: Vec<PreloadRequest>,
    pub stats: ParseStats,
}

impl ParseResult {
    /// TODO: add docs
    pub fn parse_errors(&self) -> &[ParseError] {
        &self.parse_errors
    }
}
