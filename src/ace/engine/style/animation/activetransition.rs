use super::*;
use super::css_values::{CssColor, CssLength};
use std::collections::HashMap;



#[derive(Clone, Debug)]
pub struct ActiveTransition {
    pub node_id: usize,
    pub property: String,
    pub start_value: AnimatableValue,
    pub end_value: AnimatableValue,
    pub start_time: f64, // seconds
    pub duration: f64,   // seconds
    pub timing_function: TimingFunction,
}
