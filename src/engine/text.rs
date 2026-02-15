use cosmic_text::{FontSystem, Buffer, Metrics, Attrs, Family, Shaping};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TextMeasurer {
    pub font_system: Arc<Mutex<FontSystem>>,
}

impl TextMeasurer {
    pub fn new(font_system: Arc<Mutex<FontSystem>>) -> Self {
        Self { font_system }
    }

    pub fn measure_text(&self, text: &str, font_size: f32, line_height: f32, family: Option<&str>, weight: cosmic_text::Weight, max_width: Option<f32>) -> (f32, f32) {
        let mut font_system = self.font_system.lock().unwrap();
        
        // Font settings
        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        
        let mut attrs = Attrs::new();
        attrs = attrs.weight(weight);
        
        if let Some(f) = family {
            attrs = attrs.family(Family::Name(f));
        } else {
            attrs = attrs.family(Family::SansSerif);
        }

        buffer.set_size(&mut font_system, max_width, None);
        buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
        
        buffer.shape_until_scroll(&mut font_system, false);

        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;

        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height += run.line_height;
        }

        (width, height)
    }
}
