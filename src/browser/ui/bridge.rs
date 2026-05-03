use crate::browser::tabs::manager::TabManager;
use crate::ui::AppWindow;
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

pub fn bytes_to_slint_buffer(
    data: &[u8],
    width: u32,
    height: u32,
) -> SharedPixelBuffer<Rgba8Pixel> {
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

        let max_y = primitives
            .get_all_layers_sorted()
            .iter()
            .flat_map(|layer| layer.display_items.iter())
            .fold(0.0f32, |max, p| max.max(p.y + p.height));
        let content_h = max_y.max(vh);

        let physical_w = (vw * scale_factor).ceil() as u32;
        let physical_h = (content_h * scale_factor).ceil() as u32;
        let framebuffer = engine.framebuffer.clone();

        let mut dirty_rects = {
            let mut im = engine.invalidation_manager.lock().unwrap();
            let rects = im.dirty_rects.clone();
            im.clear();
            rects
        };

        let mut fb_size_changed = false;
        {
            let fb = framebuffer.lock().unwrap();
            if let Some(ref p) = *fb {
                if p.width() != physical_w || p.height() != physical_h {
                    fb_size_changed = true;
                }
            } else {
                fb_size_changed = true;
            }
        }

        if dirty_rects.is_empty() && !fb_size_changed {
            let ui_clone = ui.as_weak();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_clone.upgrade() {
                    ui.set_content_height(content_h);
                    ui.set_viewport_y(viewport_y);
                }
            });
            return;
        }

        if fb_size_changed {
            dirty_rects.clear();
            if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 0.0, logical_width, content_h) {
                dirty_rects.push(rect);
            }
        }

        let ui_clone = ui.as_weak();
        let font_system_arc = engine.font_system.clone();
        let swash_cache_arc = engine.swash_cache.clone();

        tokio::spawn(async move {
            // 1. Try WGPU Compositor firsthand
            let mut gpu_success = false;
            let mut final_pixels = None;

            {
                let mut comp_opt = engine.gpu_compositor.lock().await;
                if let Some(comp) = comp_opt.as_mut() {
                    // Call the async compositor rendering pass
                    if let Some(pixels) = comp
                        .render_tree(
                            &primitives,
                            physical_w,
                            physical_h,
                            scale_factor,
                            font_system_arc.clone(),
                            swash_cache_arc.clone(),
                        )
                        .await
                    {
                        final_pixels = Some(pixels);
                        gpu_success = true;
                    }
                }
            }

            // 2. Fallback: CPU Rasterization (Tiny-Skia)
            if !gpu_success {
                let mut font_system_lock = font_system_arc.lock().unwrap();
                let mut swash_cache_lock = swash_cache_arc.lock().unwrap();
                let mut fb_lock = framebuffer.lock().unwrap();
                for layer in primitives.get_all_layers_sorted() {
                    let tiles = layer.build_tiles(512);
                    for tile in tiles {
                        crate::renderer::paint_layout_tree(
                            &tile,
                            physical_w,
                            physical_h,
                            scale_factor,
                            &mut font_system_lock,
                            &mut swash_cache_lock,
                            &mut fb_lock,
                            &dirty_rects,
                        );
                    }
                }

                if let Some(ref pixmap) = *fb_lock {
                    final_pixels = Some(pixmap.data().to_vec());
                }
            }

            if let Some(pixels) = final_pixels {
                let pixel_buffer = bytes_to_slint_buffer(&pixels, physical_w, physical_h);

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_clone.upgrade() {
                        let slint_image = Image::from_rgba8(pixel_buffer);
                        ui.set_web_content_buffer(slint_image);
                        ui.set_content_height(content_h);
                        ui.set_viewport_y(viewport_y);
                    }
                });
            }
        });
    }
}
