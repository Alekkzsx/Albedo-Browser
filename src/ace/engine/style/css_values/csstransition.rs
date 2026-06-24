use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub struct CssTransition {
    pub property: String,
    pub duration_ms: u32,
    pub timing_function: String, // e.g., "ease", "linear"
    pub delay_ms: u32,
}
