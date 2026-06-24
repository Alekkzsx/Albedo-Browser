use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreloadRequest {
    pub url: String,
    pub resource_type: ResourceType,
    pub priority: RequestPriority,
    pub crossorigin: Option<String>,
    pub rel: Option<String>,
    pub as_attribute: Option<String>,
    pub fetchpriority: Option<String>,
    pub loading: Option<String>,
    pub is_module: bool,
    pub is_async: bool,
    pub is_defer: bool,
}
