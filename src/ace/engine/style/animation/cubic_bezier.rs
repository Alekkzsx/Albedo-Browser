use super::*;
use super::css_values::{CssColor, CssLength};
use std::collections::HashMap;



// Simple Cubic Bezier implementation (approximate for MVP)
pub(crate) fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
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
