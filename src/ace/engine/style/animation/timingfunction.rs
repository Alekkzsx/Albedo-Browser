use super::*;
use super::css_values::{CssColor, CssLength};
use std::collections::HashMap;


#[derive(Clone, Debug, PartialEq)]
pub enum TimingFunction {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f32, f32, f32, f32),
}

impl TimingFunction {
    /// TODO: add docs
    pub fn transform(&self, t: f32) -> f32 {
        match self {
            TimingFunction::Linear => t,
            TimingFunction::Ease => cubic_bezier(0.25, 0.1, 0.25, 1.0, t),
            TimingFunction::EaseIn => cubic_bezier(0.42, 0.0, 1.0, 1.0, t),
            TimingFunction::EaseOut => cubic_bezier(0.0, 0.0, 0.58, 1.0, t),
            TimingFunction::EaseInOut => cubic_bezier(0.42, 0.0, 0.58, 1.0, t),
            TimingFunction::CubicBezier(x1, y1, x2, y2) => cubic_bezier(*x1, *y1, *x2, *y2, t),
        }
    }
}
