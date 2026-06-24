use super::*;
use super::css_values::{CssColor, CssLength};
use std::collections::HashMap;



#[derive(Clone, Debug, PartialEq)]
pub enum AnimatableValue {
    Length(CssLength),
    Color(CssColor),
    Float(f32),
    None,
}

impl AnimatableValue {
    /// TODO: add docs
    pub fn interpolate(&self, other: &AnimatableValue, progress: f32) -> AnimatableValue {
        match (self, other) {
            (AnimatableValue::Float(a), AnimatableValue::Float(b)) => {
                AnimatableValue::Float(a + (b - a) * progress)
            }
            (AnimatableValue::Color(c1), AnimatableValue::Color(c2)) => {
                // Simple RGBA interpolation
                // Assuming c1 and c2 can be converted to rgba float or u8
                // For MVP, if types match, interpolate. Else, discrete step.
                // TODO: Implement proper color interpolation
                if progress < 0.5 {
                    AnimatableValue::Color(c1.clone())
                } else {
                    AnimatableValue::Color(c2.clone())
                }
            }
            (AnimatableValue::Length(l1), AnimatableValue::Length(l2)) => {
                // If units match, interpolate values.
                // Examples: Px(a), Px(b).
                match (l1, l2) {
                    (CssLength::Px(a), CssLength::Px(b)) => {
                        AnimatableValue::Length(CssLength::Px(a + (b - a) * progress))
                    }
                    (CssLength::Percent(a), CssLength::Percent(b)) => {
                        AnimatableValue::Length(CssLength::Percent(a + (b - a) * progress))
                    }
                    (CssLength::Vw(a), CssLength::Vw(b)) => {
                        AnimatableValue::Length(CssLength::Vw(a + (b - a) * progress))
                    }
                    (CssLength::Vh(a), CssLength::Vh(b)) => {
                        AnimatableValue::Length(CssLength::Vh(a + (b - a) * progress))
                    }
                    (CssLength::Rem(a), CssLength::Rem(b)) => {
                        AnimatableValue::Length(CssLength::Rem(a + (b - a) * progress))
                    }
                    (CssLength::Em(a), CssLength::Em(b)) => {
                        AnimatableValue::Length(CssLength::Em(a + (b - a) * progress))
                    }
                    (CssLength::Zero, CssLength::Px(b)) => {
                        AnimatableValue::Length(CssLength::Px(b * progress))
                    }
                    (CssLength::Px(a), CssLength::Zero) => {
                        AnimatableValue::Length(CssLength::Px(a * (1.0 - progress)))
                    }
                    _ => {
                        if progress < 0.5 {
                            AnimatableValue::Length(l1.clone())
                        } else {
                            AnimatableValue::Length(l2.clone())
                        }
                    }
                }
            }
            _ => {
                if progress < 0.5 {
                    self.clone()
                } else {
                    other.clone()
                }
            }
        }
    }

    /// TODO: add docs
    pub fn parse(val: &str) -> Option<AnimatableValue> {
        let trimmed_value = val.trim();
        if trimmed_value.ends_with("px") {
            if let Ok(n) = trimmed_value[..trimmed_value.len() - 2].parse::<f32>() {
                return Some(AnimatableValue::Length(CssLength::Px(n)));
            }
        } else if trimmed_value.ends_with("%") {
            if let Ok(n) = trimmed_value[..trimmed_value.len() - 1].parse::<f32>() {
                return Some(AnimatableValue::Length(CssLength::Percent(n)));
            }
        } else if val.starts_with("#") {
            // Simple hex parse
            if val.len() == 7 {
                let r = u8::from_str_radix(&val[1..3], 16).ok()?;
                let g = u8::from_str_radix(&val[3..5], 16).ok()?;
                let b = u8::from_str_radix(&val[5..7], 16).ok()?;
                return Some(AnimatableValue::Color(CssColor::Rgba(r, g, b, 1.0)));
            }
        } else if let Ok(n) = val.parse::<f32>() {
            return Some(AnimatableValue::Float(n));
        }

        // Conversão de cores nomeadas básicas
        match val {
            "red" => Some(AnimatableValue::Color(CssColor::Rgba(255, 0, 0, 1.0))),
            "blue" => Some(AnimatableValue::Color(CssColor::Rgba(0, 0, 255, 1.0))),
            "green" => Some(AnimatableValue::Color(CssColor::Rgba(0, 128, 0, 1.0))),
            "white" => Some(AnimatableValue::Color(CssColor::Rgba(255, 255, 255, 1.0))),
            "black" => Some(AnimatableValue::Color(CssColor::Rgba(0, 0, 0, 1.0))),
            "transparent" => Some(AnimatableValue::Color(CssColor::Transparent)),
            _ => None,
        }
    }
}
