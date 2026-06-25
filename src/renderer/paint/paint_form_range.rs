                crate::ace::engine::types::FormInputType::Range => {
                    // Modern Slider: Gradient track and circular thumb
                    let track_h = 6.0;
                    let track_y = local_y + (prim.height - track_h) / 2.0;

                    // Track with gradient
                    let mut track_paint = Paint::default();
                    let shader = tiny_skia::LinearGradient::new(
                        tiny_skia::Point::from_xy(local_x, track_y),
                        tiny_skia::Point::from_xy(local_x + prim.width, track_y),
                        vec![
                            tiny_skia::GradientStop::new(
                                0.0,
                                Color::from_rgba8(220, 220, 220, 255),
                            ),
                            tiny_skia::GradientStop::new(
                                1.0,
                                Color::from_rgba8(180, 180, 180, 255),
                            ),
                        ],
                        tiny_skia::SpreadMode::Pad,
                        Transform::identity(),
                    )
                    .expect("Albedo Engine: internal invariant violated");
                    track_paint.shader = shader;
                    track_paint.anti_alias = true;

                    if let Some(track_rect) = Rect::from_xywh(local_x, track_y, prim.width, track_h)
                    {
                        let track_path = PathBuilder::from_rect(track_rect);
                        pixmap.fill_path(
                            &track_path,
                            &track_paint,
                            tiny_skia::FillRule::Winding,
                            Transform::from_scale(scale_factor, scale_factor),
                            None,
                        );
                    }

                    // Logic for positioning
                    let min: f32 = prim.input_min.parse().unwrap_or(0.0);
                    let max: f32 = prim.input_max.parse().unwrap_or(100.0);
                    let val: f32 = prim.input_value.parse().unwrap_or((min + max) / 2.0);
                    let percent = if max > min {
                        (val - min) / (max - min)
                    } else {
                        0.5
                    };

                    let thumb_radius = 9.0;
                    let thumb_x = local_x
                        + (prim.width - thumb_radius * 2.0) * percent.clamp(0.0, 1.0)
                        + thumb_radius;
                    let thumb_y = local_y + prim.height / 2.0;

                    // Thumb Shadow
                    let mut shadow_paint = Paint::default();
                    shadow_paint.set_color(Color::from_rgba8(0, 0, 0, 60));
                    shadow_paint.anti_alias = true;
                    let mut pb = tiny_skia::PathBuilder::new();
                    pb.push_circle(thumb_x, thumb_y + 1.0, thumb_radius);
                    if let Some(shadow_path) = pb.finish() {
                        pixmap.fill_path(
                            &shadow_path,
                            &shadow_paint,
                            tiny_skia::FillRule::Winding,
                            Transform::from_scale(scale_factor, scale_factor),
                            None,
                        );
                    }

                    // Thumb
                    let mut thumb_paint = Paint::default();
                    let thumb_color = if prim.is_hovered {
                        Color::from_rgba8(0, 100, 200, 255)
                    } else {
                        Color::from_rgba8(0, 120, 215, 255)
                    };
                    thumb_paint.set_color(thumb_color);
                    thumb_paint.anti_alias = true;

                    let mut pb = tiny_skia::PathBuilder::new();
                    pb.push_circle(thumb_x, thumb_y, thumb_radius);
                    if let Some(path) = pb.finish() {
                        pixmap.fill_path(
                            &path,
                            &thumb_paint,
                            tiny_skia::FillRule::Winding,
                            Transform::from_scale(scale_factor, scale_factor),
                            None,
                        );

                        // Thumb Border
                        let mut stroke = tiny_skia::Stroke::default();
                        stroke.width = 2.0;
                        let mut stroke_paint = Paint::default();
                        stroke_paint.set_color(Color::WHITE);
                        pixmap.stroke_path(
                            &path,
                            &stroke_paint,
                            &stroke,
                            Transform::from_scale(scale_factor, scale_factor),
                            None,
                        );
                    }
                }
