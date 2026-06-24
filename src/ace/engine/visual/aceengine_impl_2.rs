use super::*;

use crate::ace::engine::layout_types::DisplayItem;
use crate::ace::engine::core::AceEngine;


impl AceEngine {
    /// TODO: add docs
    pub fn tick(&mut self, now: f64) -> bool {
        let mut am = self.animation_manager.lock().unwrap_or_else(|e| e.into_inner());
        let mut has_changes = am.tick(now);

        // Disparar requestAnimationFrame callbacks
        if let Some(ref rt) = self.js_runtime {
            if rt.run_raf_callbacks(now) {
                has_changes = true;
            }
        }

        if has_changes {
            tracing::debug!("tick() has_changes == true, setting styles_dirty");
            self.styles_dirty
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }

        has_changes
    }
    /// TODO: add docs
    pub fn css_color_to_skia(
        css_color: &crate::ace::engine::style::css_values::CssColor,
    ) -> Option<tiny_skia::Color> {
        use crate::ace::engine::style::css_values::CssColor;
        match css_color {
            CssColor::Rgba(r, g, b, a) => {
                Some(tiny_skia::Color::from_rgba8(*r, *g, *b, (*a * 255.0) as u8))
            }
            CssColor::Named(name) => {
                if name.eq_ignore_ascii_case("transparent") {
                    return None;
                }
                if name.starts_with("#") {
                    let hex = name.trim_start_matches('#');
                    if hex.len() == 3 {
                        // Expandir dígito hex: 0xA => 0xAA == A * 17 (zero alocações)
                        let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0) * 17;
                        let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0) * 17;
                        let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0) * 17;
                        Some(tiny_skia::Color::from_rgba8(r, g, b, 255))
                    } else {
                        let r =
                            u8::from_str_radix(if hex.len() >= 2 { &hex[0..2] } else { hex }, 16)
                                .unwrap_or(0);
                        let g =
                            u8::from_str_radix(if hex.len() >= 4 { &hex[2..4] } else { "0" }, 16)
                                .unwrap_or(0);
                        let b =
                            u8::from_str_radix(if hex.len() >= 6 { &hex[4..6] } else { "0" }, 16)
                                .unwrap_or(0);
                        let a = if hex.len() == 8 {
                            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
                        } else {
                            255
                        };
                        Some(tiny_skia::Color::from_rgba8(r, g, b, a))
                    }
                } else {
                    let lower = name.to_ascii_lowercase();
                    match lower.as_str() {
                        "white" => Some(tiny_skia::Color::from_rgba8(255, 255, 255, 255)),
                        "black" | "currentcolor" => {
                            Some(tiny_skia::Color::from_rgba8(0, 0, 0, 255))
                        }
                        "red" => Some(tiny_skia::Color::from_rgba8(255, 0, 0, 255)),
                        "green" => Some(tiny_skia::Color::from_rgba8(0, 128, 0, 255)),
                        "blue" => Some(tiny_skia::Color::from_rgba8(0, 0, 255, 255)),
                        "darkblue" => Some(tiny_skia::Color::from_rgba8(0, 0, 139, 255)),
                        "darkgreen" => Some(tiny_skia::Color::from_rgba8(0, 100, 0, 255)),
                        "darkred" => Some(tiny_skia::Color::from_rgba8(139, 0, 0, 255)),
                        "yellow" => Some(tiny_skia::Color::from_rgba8(255, 255, 0, 255)),
                        "gray" | "grey" => Some(tiny_skia::Color::from_rgba8(128, 128, 128, 255)),
                        "lightslategray" => Some(tiny_skia::Color::from_rgba8(119, 136, 153, 255)),
                        _ => None,
                    }
                }
            }
            _ => None,
        }
    }
}
