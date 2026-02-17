use super::css_values::{CssLength, CssColor, ComputedStyle, CssDisplay};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum AnimatableValue {
    Length(CssLength),
    Color(CssColor),
    Float(f32),
    None,
}

impl AnimatableValue {
    pub fn interpolate(&self, other: &AnimatableValue, progress: f32) -> AnimatableValue {
        match (self, other) {
            (AnimatableValue::Float(a), AnimatableValue::Float(b)) => {
                AnimatableValue::Float(a + (b - a) * progress)
            },
            (AnimatableValue::Color(c1), AnimatableValue::Color(c2)) => {
                // Simple RGBA interpolation
                // Assuming c1 and c2 can be converted to rgba float or u8
                // For MVP, if types match, interpolate. Else, discrete step.
                // TODO: Implement proper color interpolation
                if progress < 0.5 { AnimatableValue::Color(c1.clone()) } else { AnimatableValue::Color(c2.clone()) }
            },
            (AnimatableValue::Length(l1), AnimatableValue::Length(l2)) => {
                // If units match, interpolate values.
                // Examples: Px(a), Px(b).
                match (l1, l2) {
                    (CssLength::Px(a), CssLength::Px(b)) => AnimatableValue::Length(CssLength::Px(a + (b - a) * progress)),
                    (CssLength::Percent(a), CssLength::Percent(b)) => AnimatableValue::Length(CssLength::Percent(a + (b - a) * progress)),
                    (CssLength::Vw(a), CssLength::Vw(b)) => AnimatableValue::Length(CssLength::Vw(a + (b - a) * progress)),
                    (CssLength::Vh(a), CssLength::Vh(b)) => AnimatableValue::Length(CssLength::Vh(a + (b - a) * progress)),
                    (CssLength::Rem(a), CssLength::Rem(b)) => AnimatableValue::Length(CssLength::Rem(a + (b - a) * progress)),
                    (CssLength::Em(a), CssLength::Em(b)) => AnimatableValue::Length(CssLength::Em(a + (b - a) * progress)),
                    (CssLength::Zero, CssLength::Px(b)) => AnimatableValue::Length(CssLength::Px(b * progress)),
                    (CssLength::Px(a), CssLength::Zero) => AnimatableValue::Length(CssLength::Px(a * (1.0 - progress))),
                    _ => if progress < 0.5 { AnimatableValue::Length(l1.clone()) } else { AnimatableValue::Length(l2.clone()) }
                }
            },
            _ => if progress < 0.5 { self.clone() } else { other.clone() }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActiveTransition {
    pub node_id: usize,
    pub property: String,
    pub start_value: AnimatableValue,
    pub end_value: AnimatableValue,
    pub start_time: f64, // seconds
    pub duration: f64,   // seconds
    pub timing_function: String,
}

#[derive(Clone, Debug)]
pub struct AnimationManager {
    pub transitions: HashMap<(usize, String), ActiveTransition>,
    // Future: keyframe_animations: HashMap<usize, Vec<ActiveKeyframeAnimation>>,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            transitions: HashMap::new(),
        }
    }

    pub fn tick(&mut self, current_time: f64) -> bool {
        let mut finished = Vec::new();
        let mut has_changes = false;

        for (key, parsing) in self.transitions.iter() {
            if current_time >= parsing.start_time + parsing.duration {
                finished.push(key.clone());
            } else {
                has_changes = true;
            }
        }

        for key in finished {
            self.transitions.remove(&key);
            // In a real engine, we might want to keep the final state or let CSS re-resolve take over
            has_changes = true; 
        }
        
        has_changes
    }

    pub fn get_animated_value(&self, node_id: usize, property: &str, current_time: f64) -> Option<AnimatableValue> {
        if let Some(anim) = self.transitions.get(&(node_id, property.to_string())) {
            let elapsed = current_time - anim.start_time;
            let progress = (elapsed / anim.duration).clamp(0.0, 1.0) as f32;
            
            // Apply timing function (linear for now)
            let eased_progress = progress; 
            
            return Some(anim.start_value.interpolate(&anim.end_value, eased_progress));
        }
        None
    }

    pub fn start_transition(&mut self, node_id: usize, property: String, start: AnimatableValue, end: AnimatableValue, duration: f64, now: f64) {
        self.transitions.insert((node_id, property.clone()), ActiveTransition {
            node_id,
            property,
            start_value: start,
            end_value: end,
            start_time: now,
            duration,
            timing_function: "ease".to_string(),
        });
    }
}
