use super::*;
use std::fmt;



/// Gradient type
#[derive(Debug, Clone, PartialEq)]
pub enum Gradient {
    Linear {
        angle: f32, // degrees
        stops: Vec<GradientStop>,
    },
    Radial {
        shape: String, // circle or ellipse
        stops: Vec<GradientStop>,
    },
}
