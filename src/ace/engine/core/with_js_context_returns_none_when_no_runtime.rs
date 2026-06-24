use super::*;
use crate::ace::engine::graphics::compositor;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::style::css_values::ComputedStyle;
use crate::ace::engine::style::Stylesheet;
use crate::ace::engine::text::TextMeasurer;
use crate::ace::engine::layout::{ElementGeometry, ACEPrimitive, InvalidationManager};
use crate::utils::time::unix_timestamp_secs_f64;
use std::sync::{Arc, Mutex};



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
pub(crate) fn with_js_context_returns_none_when_no_runtime() {
        let engine = AceEngine::new();
        assert!(engine.with_js_context(|_, _| true).is_none());
    }

    #[test]
pub(crate) fn with_js_context_returns_none_when_no_dom() {
        let mut engine = AceEngine::new();
        engine.dom = None;
        assert!(engine.with_js_context(|_, _| true).is_none());
    }
}
