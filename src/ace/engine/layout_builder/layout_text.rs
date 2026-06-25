{
            let transformed_text =
                crate::ace::engine::layout::inline::apply_text_transform(text.as_ref(), &style.text_transform);
            // Text nodes need an intrinsic size estimate to be visible
            let font_size = style.font_size;
            let line_height = font_size * 1.2;

            // Usar o TextMeasurer global para dimensões reais
            let letter_spacing = crate::ace::engine::style::css_values::resolve_length(
                &style.letter_spacing,
                font_size,
                16.0,
                vw,
                vh,
            );
            let word_spacing = crate::ace::engine::style::css_values::resolve_length(
                &style.word_spacing,
                font_size,
                16.0,
                vw,
                vh,
            );

            let (text_width, text_height) = self.text_measurer.measure_text(
                &transformed_text,
                font_size,
                line_height,
                Some(&style.font_family),
                cosmic_text::Weight::NORMAL,
                None,
                letter_spacing,
                word_spacing,
            );

            let width = text_width;
            let height = text_height;

            taffy_style.size.width = taffy::prelude::Dimension::Points(width);
            taffy_style.size.height = taffy::prelude::Dimension::Points(height);

            // Se o texto for curto, não queremos que ele "estique" se for um bloco
            taffy_style.max_size.width = taffy::prelude::Dimension::Points(width);

            let taffy_node = if let Some(node) = existing_node {
                if is_strictly_dirty {
                    let _ = taffy.set_style(node, taffy_style);
                }
                node
            } else {
                let node = taffy.new_leaf(taffy_style).expect("Albedo Engine: internal invariant violated");
                self.node_to_taffy.lock().unwrap_or_else(|e| e.into_inner()).insert(node_idx, node);
                node
            };

            node_map.insert(taffy_node, node_idx);
            return vec![taffy_node];
}
