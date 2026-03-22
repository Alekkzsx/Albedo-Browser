use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_height: f32,
}

#[derive(Clone)]
pub struct TextMeasurer {
    pub font_system: Arc<Mutex<FontSystem>>,
    pub measure_cache: Arc<Mutex<std::collections::HashMap<String, (f32, f32)>>>,
}

impl TextMeasurer {
    pub fn new(font_system: Arc<Mutex<FontSystem>>) -> Self {
        Self {
            font_system,
            measure_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn measure_text(
        &self,
        text: &str,
        font_size: f32,
        line_height: f32,
        family: Option<&str>,
        weight: cosmic_text::Weight,
        max_width: Option<f32>,
        letter_spacing: f32,
        word_spacing: f32,
    ) -> (f32, f32) {
        if text.is_empty() {
            return (0.0, 0.0);
        }

        let cache_key = format!(
            "{}_{}_{}_{:?}_{:?}_{:?}_{}_{}",
            text, font_size, line_height, family, weight.0, max_width, letter_spacing, word_spacing
        );

        {
            let cache = self.measure_cache.lock().unwrap();
            if let Some(&metrics) = cache.get(&cache_key) {
                return metrics;
            }
        }

        let mut font_system = self.font_system.lock().unwrap();

        let mut attrs = Attrs::new().weight(weight);
        if let Some(f) = family {
            attrs = attrs.family(Family::Name(f));
        } else {
            attrs = attrs.family(Family::SansSerif);
        }

        if letter_spacing == 0.0 && word_spacing == 0.0 {
            // Fast Path
            let metrics = Metrics::new(font_size, line_height);
            let mut buffer = Buffer::new(&mut font_system, metrics);
            buffer.set_size(&mut font_system, max_width, None);
            buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
            buffer.shape_until_scroll(&mut font_system, false);

            let mut width: f32 = 0.0;
            let mut height: f32 = 0.0;
            for run in buffer.layout_runs() {
                width = width.max(run.line_w);
                height += run.line_height;
            }
            let result = (width, height);
            self.measure_cache.lock().unwrap().insert(cache_key, result);
            return result;
        }

        // Spaced Path (Tokenized Greedy Wrapping)
        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut font_system, metrics);

        let mut current_x = 0.0;
        let mut current_y = line_height;
        let mut max_x: f32 = 0.0;
        let mw = max_width.unwrap_or(f32::INFINITY);

        let mut space_w = 0.0;
        buffer.set_text(&mut font_system, " ", attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut font_system, false);
        if let Some(run) = buffer.layout_runs().next() {
            space_w = run.line_w;
        }

        let mut first_word_in_line = true;

        for word in text.split_inclusive(|c: char| c.is_whitespace()) {
            let is_space_ended = word.ends_with(|c: char| c.is_whitespace());
            let (word_trim, trailing) = if is_space_ended {
                let bytes = word.len() - word.chars().last().unwrap().len_utf8();
                (&word[..bytes], &word[bytes..])
            } else {
                (word, "")
            };

            let mut word_w = 0.0;
            if letter_spacing != 0.0 {
                for c in word_trim.chars() {
                    let mut b = [0; 4];
                    let char_str = c.encode_utf8(&mut b);
                    buffer.set_text(&mut font_system, char_str, attrs, Shaping::Advanced);
                    buffer.shape_until_scroll(&mut font_system, false);
                    let char_w = buffer.layout_runs().next().map_or(0.0, |r| r.line_w);
                    // No tracking after the last character logically in CSS but actually typically letter spacing applies to all chars
                    word_w += char_w + letter_spacing;
                }
            } else {
                buffer.set_text(&mut font_system, word_trim, attrs, Shaping::Advanced);
                buffer.shape_until_scroll(&mut font_system, false);
                word_w = buffer.layout_runs().next().map_or(0.0, |r| r.line_w);
            }

            if !first_word_in_line && current_x + word_w > mw {
                current_x = 0.0;
                current_y += line_height;
            }

            current_x += word_w;
            max_x = max_x.max(current_x);

            if is_space_ended {
                let spc_adv = if letter_spacing != 0.0 {
                    let mut b = [0; 4];
                    let c = trailing.chars().next().unwrap();
                    let char_str = c.encode_utf8(&mut b);
                    buffer.set_text(&mut font_system, char_str, attrs, Shaping::Advanced);
                    buffer.shape_until_scroll(&mut font_system, false);
                    buffer.layout_runs().next().map_or(0.0, |r| r.line_w)
                } else {
                    space_w
                };
                current_x += spc_adv + word_spacing + letter_spacing;
            }

            first_word_in_line = false;
            let _ = first_word_in_line; // Silenciar aviso de atribuição não lida na última iteração
        }

        let result = (max_x, current_y);
        self.measure_cache.lock().unwrap().insert(cache_key, result);
        result
    }
}
