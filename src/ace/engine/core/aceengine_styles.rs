    pub fn recompute_dirty_styles(&mut self) {
        if !self.styles_dirty.load(std::sync::atomic::Ordering::SeqCst) {
            // Check if any individual node is STYLE dirty
            let is_any_node_style_dirty = if let Some(ref dom_arc) = self.dom {
                let dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
                dom.nodes
                    .iter()
                    .any(|n| n.dirty.contains(crate::ace::engine::dom::NodeDirtyFlags::STYLE))
            } else {
                false
            };
            if !is_any_node_style_dirty {
                return;
            }
        }

        tracing::debug!("Recomputing dirty styles");

        // Recompilar estilos para toda a árvore DOM
        if let Some(ref dom_arc) = self.dom {
            let dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
            let stylesheet = self.stylesheet.lock().unwrap_or_else(|e| e.into_inner());

            let now = unix_timestamp_secs_f64();
            let mut am = self.animation_manager.lock().unwrap_or_else(|e| e.into_inner());

            // Map index -> ComputedStyle for inheritance
            let mut style_cache: std::collections::HashMap<usize, ComputedStyle> =
                std::collections::HashMap::new();

            // Recompilar estilos com novos estados de hover/focus/active
            for idx in 0..dom.nodes.len() {
                let parent_style = if let Some(parent_idx) = dom.nodes[idx].parent {
                    style_cache.get(&parent_idx)
                } else {
                    None
                };

                let computed = stylesheet.calculate_style(
                    &dom,
                    idx,
                    parent_style,
                    None,
                    self.hovered_element,
                    self.focused_element,
                    self.active_element,
                    Some(&am),
                    now,
                    800.0,
                    600.0,
                    "light",
                );

                // Start Keyframe Animations
                if let crate::ace::engine::dom::AceNodeType::Element(_) = &dom.nodes[idx].node_type {
                    for anim_def in &computed.animations {
                        if let Some(keyframes) = stylesheet.keyframes.get(&anim_def.name) {
                            let timing = crate::ace::engine::style::animation::TimingFunction::Ease;
                            am.start_keyframe_animation(
                                idx,
                                anim_def.name.clone(),
                                keyframes.clone(),
                                anim_def.duration_ms as f64 / 1000.0,
                                timing,
                                now,
                            );
                        }
                    }
                }

                style_cache.insert(idx, computed);
            }

            // Persistir estilos computados no cache da engine
            {
                let mut engine_styles = self.element_styles.lock().unwrap_or_else(|e| e.into_inner());
                *engine_styles = style_cache;
            }
        }

        self.styles_dirty
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
