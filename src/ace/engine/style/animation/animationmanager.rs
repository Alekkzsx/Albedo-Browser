use super::*;
use super::css_values::{CssColor, CssLength};
use std::collections::HashMap;



#[derive(Clone, Debug)]
pub struct AnimationManager {
    pub transitions: HashMap<(usize, String), ActiveTransition>,
    pub animations: HashMap<usize, Vec<ActiveKeyframeAnimation>>,
}

impl AnimationManager {
    /// TODO: add docs
    pub fn new() -> Self {
        Self {
            transitions: HashMap::new(),
            animations: HashMap::new(),
        }
    }

    /// TODO: add docs
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

    /// TODO: add docs
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

    /// TODO: add docs
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

    /// TODO: add docs
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
