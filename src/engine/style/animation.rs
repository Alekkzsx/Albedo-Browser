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

// Simple Cubic Bezier implementation (approximate for MVP)
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;

    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;

    // Sample x at t
    let _solve_x = |t: f32| ((ax * t + bx) * t + cx) * t;
    // Sample y at t
    let solve_y = |t: f32| ((ay * t + by) * t + cy) * t;

    // Newton's method to find t for x is complex, using direct approximation for now
    // For CSS animations, t is usually time-based progress, which is X axis.
    // We want the Y value for that time.
    // But CSS cubic-bezier maps Time(X) -> Progression(Y).
    // So if input t is linear time, we need to solve curve(T).x = t for T, then result = curve(T).y

    // Approximation:
    // Since we don't have a solver yet, let's just use the Y formulation directly on t
    // This is wrong for non-linear time, but creates *some* easing effect.
    // TODO: Implement proper Newton-Raphson solver for X -> T
    solve_y(t)
}

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

    pub fn parse(val: &str) -> Option<AnimatableValue> {
        let val = val.trim();
        if val.ends_with("px") {
            if let Ok(n) = val[..val.len() - 2].parse::<f32>() {
                return Some(AnimatableValue::Length(CssLength::Px(n)));
            }
        } else if val.ends_with("%") {
            if let Ok(n) = val[..val.len() - 1].parse::<f32>() {
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

#[derive(Clone, Debug)]
pub struct AnimationManager {
    pub transitions: HashMap<(usize, String), ActiveTransition>,
    pub animations: HashMap<usize, Vec<ActiveKeyframeAnimation>>,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            transitions: HashMap::new(),
            animations: HashMap::new(),
        }
    }

    pub fn tick(&mut self, current_time: f64) -> bool {
        let mut finished_transitions = Vec::new();
        let mut has_changes = false;

        // Transitions
        for (key, trans) in self.transitions.iter() {
            if current_time >= trans.start_time + trans.duration {
                finished_transitions.push(key.clone());
            } else {
                has_changes = true;
            }
        }

        for key in finished_transitions {
            self.transitions.remove(&key);
            has_changes = true;
        }

        // Animations (Stub logic for now, just checking expiry)
        let mut finished_anims = Vec::new();
        for (node_id, anims) in self.animations.iter_mut() {
            let mut active_anims = Vec::new();
            for anim in anims.drain(..) {
                let elapsed = current_time - anim.start_time;
                let iter_duration = anim.duration;
                let total_duration = if anim.iteration_count.is_infinite() {
                    f64::INFINITY
                } else {
                    iter_duration * anim.iteration_count as f64
                };

                if elapsed < total_duration {
                    active_anims.push(anim);
                    has_changes = true;
                } else {
                    has_changes = true; // Animation ended
                }
            }
            *anims = active_anims;
            if anims.is_empty() {
                finished_anims.push(*node_id);
            }
        }

        for node_id in finished_anims {
            self.animations.remove(&node_id);
        }

        has_changes
    }

    pub fn get_animated_value(
        &self,
        node_id: usize,
        property: &str,
        current_time: f64,
    ) -> Option<AnimatableValue> {
        // 1. Check Transitions (Higher priority than animations usually, but spec says animations > transitions? actually cascade depends)
        // Usually Animations override Transitions if both apply to same property.

        // Simple Transition Check
        if let Some(trans) = self.transitions.get(&(node_id, property.to_string())) {
            let elapsed = current_time - trans.start_time;
            let progress = (elapsed / trans.duration).clamp(0.0, 1.0) as f32;
            let eased_progress = trans.timing_function.transform(progress);
            return Some(
                trans
                    .start_value
                    .interpolate(&trans.end_value, eased_progress),
            );
        }

        if let Some(anims) = self.animations.get(&node_id) {
            for anim in anims {
                // Calculate progress
                let elapsed = current_time - anim.start_time;
                let cycle = elapsed / anim.duration;
                let progress = (cycle % 1.0) as f32;

                // Apply general timing function (optional, usually keyframe animations have timing per keyframe)
                // CSS says `animation-timing-function` applies between keyframes.
                // For MVP, apply to whole progress for simplicity or per interval?
                // Let's assume linear between keyframes but eased globally if we want?
                // Actually spec says: animation-timing-function applies to each keyframe interval.

                // Sort keyframes by percentage
                let mut frames = anim.keyframes.clone();
                frames.sort_by(|a, b| a.percentage.total_cmp(&b.percentage));

                // Find interval
                let mut start_frame = &frames[0];
                let mut end_frame = &frames[frames.len() - 1];

                // Need 0% and 100% implicitly if not present?
                // Assuming they exist for now.

                for i in 0..frames.len() - 1 {
                    if progress >= frames[i].percentage && progress <= frames[i + 1].percentage {
                        start_frame = &frames[i];
                        end_frame = &frames[i + 1];
                        break;
                    }
                }

                // Check if property exists in these frames
                let start_val_str = start_frame.declarations.get(property);
                let end_val_str = end_frame.declarations.get(property);

                if let (Some(s), Some(e)) = (start_val_str, end_val_str) {
                    if let (Some(v1), Some(v2)) =
                        (AnimatableValue::parse(s), AnimatableValue::parse(e))
                    {
                        // Local progress in interval
                        let interval_len = end_frame.percentage - start_frame.percentage;
                        let local_progress = if interval_len > 0.0 {
                            (progress - start_frame.percentage) / interval_len
                        } else {
                            0.0
                        };

                        // Apply ease
                        let eased = anim.timing_function.transform(local_progress);
                        return Some(v1.interpolate(&v2, eased));
                    }
                }
            }
        }

        None
    }

    pub fn start_transition(
        &mut self,
        node_id: usize,
        property: String,
        start: AnimatableValue,
        end: AnimatableValue,
        duration: f64,
        timing: TimingFunction,
        now: f64,
    ) {
        self.transitions.insert(
            (node_id, property.clone()),
            ActiveTransition {
                node_id,
                property,
                start_value: start,
                end_value: end,
                start_time: now,
                duration,
                timing_function: timing,
            },
        );
    }

    pub fn start_keyframe_animation(
        &mut self,
        node_id: usize,
        name: String,
        keyframes: Vec<super::css_values::CssKeyframe>,
        duration: f64,
        timing: TimingFunction,
        now: f64,
    ) {
        let anims = self.animations.entry(node_id).or_insert(Vec::new());
        // Check if already running
        if anims.iter().any(|a| a.name == name) {
            return;
        }

        anims.push(ActiveKeyframeAnimation {
            node_id,
            name,
            start_time: now,
            duration,
            iteration_count: f32::INFINITY, // Default to infinite loop for now
            direction: "normal".to_string(),
            timing_function: timing,
            keyframes,
        });
    }
}
