use crate::engine::dom::AceDOM;
use crate::engine::graphics::canvas2d::Canvas2D;
use rquickjs::{class::Trace, methods};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Trace)]
#[rquickjs::class]
pub struct CanvasRenderingContext2D {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    #[qjs(skip_trace)]
    pub index: usize,
    #[qjs(skip_trace)]
    pub canvas_contexts: Arc<Mutex<HashMap<usize, Canvas2D>>>,
}

#[methods]
impl CanvasRenderingContext2D {
    // --- Estilos ---
    #[qjs(get, rename = "fillStyle")]
    pub fn get_fill_style(&self) -> String {
        "black".to_string()
    }

    #[qjs(set, rename = "fillStyle")]
    pub fn set_fill_style(&self, color: String) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.set_fill_style(&color);
        }
    }

    #[qjs(get, rename = "strokeStyle")]
    pub fn get_stroke_style(&self) -> String {
        "black".to_string()
    }

    #[qjs(set, rename = "strokeStyle")]
    pub fn set_stroke_style(&self, color: String) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.set_stroke_style(&color);
        }
    }

    #[qjs(get, rename = "lineWidth")]
    pub fn get_line_width(&self) -> f32 {
        1.0
    }

    #[qjs(set, rename = "lineWidth")]
    pub fn set_line_width(&self, width: f32) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.set_line_width(width);
        }
    }

    #[qjs(get, rename = "globalAlpha")]
    pub fn get_global_alpha(&self) -> f32 {
        1.0
    }

    #[qjs(set, rename = "globalAlpha")]
    pub fn set_global_alpha(&self, alpha: f32) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.set_global_alpha(alpha);
        }
    }

    // --- Retângulos ---
    #[qjs(rename = "fillRect")]
    pub fn fill_rect(&self, x: f32, y: f32, w: f32, h: f32) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.fill_rect(x, y, w, h);
        }
    }

    #[qjs(rename = "strokeRect")]
    pub fn stroke_rect(&self, x: f32, y: f32, w: f32, h: f32) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.begin_path();
            ctx.rect(x, y, w, h);
            ctx.stroke();
        }
    }

    #[qjs(rename = "clearRect")]
    pub fn clear_rect(&self, x: f32, y: f32, w: f32, h: f32) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.clear_rect(x, y, w, h);
        }
    }

    // --- Caminhos (Paths) ---
    #[qjs(rename = "beginPath")]
    pub fn begin_path(&self) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.begin_path();
        }
    }

    #[qjs(rename = "closePath")]
    pub fn close_path(&self) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.close_path();
        }
    }

    #[qjs(rename = "moveTo")]
    pub fn move_to(&self, x: f32, y: f32) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.move_to(x, y);
        }
    }

    #[qjs(rename = "lineTo")]
    pub fn line_to(&self, x: f32, y: f32) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.line_to(x, y);
        }
    }

    #[qjs(rename = "quadraticCurveTo")]
    pub fn quadratic_curve_to(&self, cx: f32, cy: f32, x: f32, y: f32) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.quadratic_curve_to(cx, cy, x, y);
        }
    }

    #[qjs(rename = "bezierCurveTo")]
    pub fn bezier_curve_to(&self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.bezier_curve_to(cx1, cy1, cx2, cy2, x, y);
        }
    }

    #[qjs(rename = "fill")]
    pub fn fill(&self) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.fill();
        }
    }

    #[qjs(rename = "stroke")]
    pub fn stroke(&self) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.stroke();
        }
    }

    // --- Estado ---
    #[qjs(rename = "save")]
    pub fn save(&self) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.save();
        }
    }

    #[qjs(rename = "restore")]
    pub fn restore(&self) {
        let mut contexts = self.canvas_contexts.lock().unwrap();
        if let Some(ctx) = contexts.get_mut(&self.index) {
            ctx.restore();
        }
    }
}
