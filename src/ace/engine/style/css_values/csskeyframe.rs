use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub struct CssKeyframe {
    pub percentage: f32, // 0.0 to 100.0
    pub declarations: std::collections::HashMap<String, String>,
}
