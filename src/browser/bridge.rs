use crate::ui::AppWindow;
use crate::browser::tabs::manager::TabManager;
use slint::{Image, SharedPixelBuffer, Rgba8Pixel, ComponentHandle};
use tiny_skia::Pixmap;

/// Converts a 100% Rust memory buffer (tiny-skia Pixmap) into a natively compatible
/// Slint Image via SharedPixelBuffer.
pub fn skia_to_slint_buffer(pixmap: Pixmap) -> SharedPixelBuffer<Rgba8Pixel> {
    let width = pixmap.width();
    let height = pixmap.height();
    let data = pixmap.data();

    let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    let slint_pixels = pixel_buffer.make_mut_slice();
    
    if data.len() == slint_pixels.len() * 4 {
        for (i, pixel) in data.chunks_exact(4).enumerate() {
            slint_pixels[i] = Rgba8Pixel {
                r: pixel[0],
                g: pixel[1],
                b: pixel[2],
                a: pixel[3],
            };
        }
    }

    pixel_buffer
}

pub fn sync_ace_visuals(ui: &AppWindow, tm: &TabManager) {
    if let Some((_, Some(engine), _)) = tm.get_active_tab_native_data() {
        let scale_factor = ui.window().scale_factor();
        let physical_size = ui.window().size();

        let logical_width = physical_size.width as f32 / scale_factor;
        let logical_height = physical_size.height as f32 / scale_factor;

        let vw = logical_width.max(800.0);
        let vh = logical_height.max(600.0);
        
        let primitives = engine.render_visual(vw, vh);
        let viewport_y = engine.viewport_y;
        
        let max_y = primitives.iter().fold(0.0f32, |max, p| max.max(p.y + p.height));
        let content_h = max_y.max(vh);
        
        let physical_w = (vw * scale_factor).ceil() as u32;
        let physical_h = (content_h * scale_factor).ceil() as u32;

        let ui_clone = ui.as_weak();
        
        tokio::spawn(async move {
            let mut font_system = cosmic_text::FontSystem::new();
            let pixmap = crate::renderer::paint_layout_tree(&primitives, physical_w, physical_h, scale_factor, &mut font_system);
            let pixel_buffer = skia_to_slint_buffer(pixmap);
            
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_clone.upgrade() {
                    let slint_image = Image::from_rgba8(pixel_buffer);
                    ui.set_web_content_buffer(slint_image);
                    ui.set_content_height(content_h);
                    ui.set_viewport_y(viewport_y);
                }
            });
        });
    }
}

