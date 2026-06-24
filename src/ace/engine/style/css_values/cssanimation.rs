use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub struct CssAnimation {
    pub name: String,
    pub duration_ms: u32,
    pub timing_function: String,
    pub delay_ms: u32,
    pub iteration_count: String, // "infinite" or number
    pub direction: String,       // "normal", "reverse", "alternate"...
    pub fill_mode: String,       // "none", "forwards", "backwards", "both"
}
