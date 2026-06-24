{
                crate::ace::engine::types::FormInputType::Color => {
                    // Premium Color Button: Rounded and with a "chip" look
                    if let Ok(color) = crate::utils::color::parse_hex_color(&prim.input_value) {
                        let mut paint = Paint::default();
                        paint.set_color(color);
                        paint.anti_alias = true;

                        let inset = 4.0;
                        let _corner_radius = 4.0;

                        // Draw shadow/depth effect
                        let mut shadow_paint = Paint::default();
                        shadow_paint.set_color(Color::from_rgba8(0, 0, 0, 40));
                        if let Some(shadow_rect) = Rect::from_xywh(
                            local_x + inset,
                            local_y + inset + 1.0,
                            (prim.width - inset * 2.0).max(0.0),
                            (prim.height - inset * 2.0).max(0.0),
                        ) {
                            let shadow_path = PathBuilder::from_rect(shadow_rect);
                            pixmap.fill_path(
                                &shadow_path,
                                &shadow_paint,
                                tiny_skia::FillRule::Winding,
                                Transform::from_scale(scale_factor, scale_factor),
                                None,
                            );
                        }

                        if let Some(inner_rect) = Rect::from_xywh(
                            local_x + inset,
                            local_y + inset,
                            (prim.width - inset * 2.0).max(0.0),
                            (prim.height - inset * 2.0).max(0.0),
                        ) {
                            let path = PathBuilder::from_rect(inner_rect);
                            pixmap.fill_path(
                                &path,
                                &paint,
                                tiny_skia::FillRule::Winding,
                                Transform::from_scale(scale_factor, scale_factor),
                                None,
                            );

                            // Highlight on hover
                            if prim.is_hovered {
                                let mut highlight = Paint::default();
                                highlight.set_color(Color::from_rgba8(255, 255, 255, 60));
                                pixmap.fill_path(
                                    &path,
                                    &highlight,
                                    tiny_skia::FillRule::Winding,
                                    Transform::from_scale(scale_factor, scale_factor),
                                    None,
                                );
                            }
                        }
                    }
}
