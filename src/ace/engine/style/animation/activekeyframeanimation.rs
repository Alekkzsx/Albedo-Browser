use super::*;
use super::css_values::{CssColor, CssLength};
use std::collections::HashMap;



#[derive(Clone, Debug)]
pub struct ActiveKeyframeAnimation {
    pub node_id: usize,
    pub name: String,
    pub start_time: f64,
    pub duration: f64,
    pub iteration_count: f32, // INFINITY for infinite
    pub direction: String,    // normal, reverse, alternate
    pub timing_function: TimingFunction,
    pub keyframes: Vec<super::css_values::CssKeyframe>,
}
