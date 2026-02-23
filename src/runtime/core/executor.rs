use super::runtime::JsRuntime;
use rquickjs::{Value, Ctx};
use std::collections::HashMap;

pub fn run_pending(rt: &JsRuntime) -> (bool, bool) {
    let mut executed = false;
    
    // 0. Deliver pending postMessage messages (async event loop delivery).
    // Drain the queue first (only holds event_loop lock briefly), then dispatch
    // into the JS context without holding any lock - safe and deadlock-free.
    {
        let messages: std::collections::VecDeque<_> = {
            rt.event_loop.lock().unwrap().take_pending_messages()
        };
        if !messages.is_empty() {
            rt.with_context(|ctx| {
                ctx.with(|ctx| {
                    for msg in messages {
                        let safe_data = msg.data_json.replace('\'', "\\'");
                        let safe_origin = msg.origin.replace('\'', "\\'");
                        let script = format!(
                            "globalThis.dispatchEvent(new MessageEvent('message', {{ data: {}, origin: '{}' }}))",
                            safe_data, safe_origin
                        );
                        let _ = ctx.eval::<(), _>(script);
                        executed = true;
                    }
                })
            });
        }
    }

    // 0b. Check for DOM mutations that happened since last pulse

    {
        let mut mutated_flag = rt.mutations.lock().unwrap();
        if *mutated_flag {
            if let Some(dom_arc) = rt.dom.lock().unwrap().as_ref() {
                let mut dom = dom_arc.lock().unwrap();
                let pending = dom.take_pending_mutations();
                if !pending.is_empty() {
                    rt.with_context(|ctx| {
                        ctx.with(|ctx| {
                            let registry = rt.observer_registry.lock().unwrap();
                            for (callback_id, records) in pending {
                                if let Some(cb_persistent) = registry.get(&callback_id) {
                                    if let Ok(callback) = cb_persistent.clone().restore(&ctx) {
                                        // Convert records to JS array
                                        let arr = rquickjs::Array::new(ctx.clone()).unwrap();
                                        for (i, rec) in records.into_iter().enumerate() {
                                            let obj = rquickjs::Object::new(ctx.clone()).unwrap();
                                            let _ = obj.set("type", rec.type_);
                                            let _ = obj.set("attributeName", rec.attribute_name);
                                            let _ = obj.set("oldValue", rec.old_value);
                                            
                                            // Wrap target
                                            let target = wrap_element(rt, rec.target, &ctx);
                                            let _ = obj.set("target", target);

                                            // addedNodes
                                            let added_arr = rquickjs::Array::new(ctx.clone()).unwrap();
                                            for (idx, &node_idx) in rec.added_nodes.iter().enumerate() {
                                                let node = wrap_element(rt, node_idx, &ctx);
                                                let _ = added_arr.set(idx, node);
                                            }
                                            let _ = obj.set("addedNodes", added_arr);

                                            // removedNodes
                                            let removed_arr = rquickjs::Array::new(ctx.clone()).unwrap();
                                            for (idx, &node_idx) in rec.removed_nodes.iter().enumerate() {
                                                let node = wrap_element(rt, node_idx, &ctx);
                                                let _ = removed_arr.set(idx, node);
                                            }
                                            let _ = obj.set("removedNodes", removed_arr);

                                            // Siblings
                                            if let Some(prev) = rec.previous_sibling {
                                                let _ = obj.set("previousSibling", wrap_element(rt, prev, &ctx));
                                            } else {
                                                let _ = obj.set("previousSibling", rquickjs::Value::new_null(ctx.clone()));
                                            }

                                            if let Some(next) = rec.next_sibling {
                                                let _ = obj.set("nextSibling", wrap_element(rt, next, &ctx));
                                            } else {
                                                let _ = obj.set("nextSibling", rquickjs::Value::new_null(ctx.clone()));
                                            }

                                            let _ = arr.set(i, obj);
                                        }
                                        let _: rquickjs::Result<Value> = callback.call((arr,));
                                        executed = true;
                                    }
                                }
                            }
                        });
                    });
                }
            }
            *mutated_flag = false;
        }
    }

    // 1. Run QuickJS pending jobs (Promises/microtasks)
    {
        let ctx = rt.context.lock().unwrap();
        ctx.with(|ctx| {
            while ctx.execute_pending_job() {
                println!("[JS] Microtask/Promise executed.");
                executed = true;
            }
        });
    }

    // 2. Handle Async Bridge (Fetch, etc)
    let async_results = {
        let mut el = rt.event_loop.lock().unwrap();
        el.receive_async_results()
    };

    if !async_results.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                let mut el = rt.event_loop.lock().unwrap();
                for res in async_results {
                    if let Some(resolution) = el.take_resolution(res.id) {
                        match res.result {
                            Ok((status, body)) => {
                                if let Ok(resolve) = resolution.resolve.0.clone().restore(&ctx) {
                                    use crate::runtime::bindings::webapi::fetch::Response;
                                    let response = Response { status, body, headers: crate::runtime::bindings::webapi::fetch::Headers::new() };
                                    if let Ok(instance) = rquickjs::Class::instance(ctx.clone(), response) {
                                        let _: rquickjs::Result<()> = resolve.call((instance,));
                                        executed = true;
                                    }
                                }
                            }
                            Err(err) => {
                                if let Ok(reject) = resolution.reject.0.clone().restore(&ctx) {
                                    let _: rquickjs::Result<()> = reject.call((err,));
                                    executed = true;
                                }
                            }
                        }
                    }
                }
                // Run jobs again as resolutions might trigger then() callbacks
                while ctx.execute_pending_job() {
                    executed = true;
                }
            })
        });
    }

    // 3. Run EventLoop tasks (timers, etc)
    let (timers, macros) = {
        let mut el = rt.event_loop.lock().unwrap();
        el.take_pending_tasks()
    };
    
    if !timers.is_empty() {
            rt.with_context(|ctx| {
            ctx.with(|ctx| {
                for timer in timers {
                    if let Ok(func) = timer.callback.0.clone().restore(&ctx) {
                        let _: rquickjs::Result<Value> = func.call(());
                        executed = true;
                    }
                }
                
                // Run pending jobs AGAIN after timers might have resolved promises
                while ctx.execute_pending_job() {
                    executed = true;
                }
            })
            });
    }
            
    for task in macros {
        task();
        executed = true;
    }

    // 4. Stylesheet dirty check
    let mut stylesheet_dirty = false;
    if let Ok(mut sd) = rt.stylesheet_dirty.lock() {
        if *sd {
            stylesheet_dirty = true;
            *sd = false;
        }
    }

    // 5. Layout Observers (Resize & Intersection)
    check_layout_observers(rt);
    
    (executed, stylesheet_dirty)
}

fn check_layout_observers(rt: &JsRuntime) {
    let mut resize_notifications = Vec::new();
    let mut intersection_notifications = Vec::new();

    {
        let geometry = rt.element_geometry.lock().unwrap();
        let iframe_projected_geometry = rt.iframe_projected_geometry.lock().unwrap(); // Added
        let mut layout_states = rt.layout_states.lock().unwrap();
        let resize_registry = rt.resize_registry.lock().unwrap();
        let intersection_registry = rt.intersection_registry.lock().unwrap();

        // Obtém o viewport do Runtime e calcula o retângulo da Câmera (janela de visualização) no mundo
        // Offset (viewport_y) do Browser_View Slint é tipicamente esticado pro negativo ou positivo 
        let current_viewport_y = *rt.viewport_y.lock().unwrap();
        let viewport = (0.0, -current_viewport_y, 1024.0, 768.0); // Câmera 1024x768 ajustada via Y Scroll

        let mut current_rects = HashMap::new();
        // Diferente do "Primitive", Geometry armazena TODOS IDs validos da page mesmo "invisíveis" 
        for (node_idx, geom) in geometry.iter() {
            current_rects.insert(*node_idx, (geom.x, geom.y, geom.width, geom.height));
        }

        // Check ResizeObservers
        for (&node_idx, callbacks) in resize_registry.iter() {
            if let Some(&(cx, cy, cw, ch)) = current_rects.get(&node_idx) {
                let prev = layout_states.get(&node_idx).cloned();
                if prev.is_none() || (prev.unwrap().2 != cw || prev.unwrap().3 != ch) {
                    for cb in callbacks {
                        resize_notifications.push((cb.clone(), node_idx, cw, ch));
                    }
                }
            }
        }

        // Check IntersectionObservers (first pass: normal elements)
        for (&node_idx, observers) in intersection_registry.iter() {
            if let Some(&(cx, cy, cw, ch)) = current_rects.get(&node_idx) {
                // Calculate intersection area baseando-se no viewport_y Dinâmico
                let x_overlap = (cx.max(viewport.0)).min(cx + cw).min(viewport.0 + viewport.2) - (cx.max(viewport.0));
                let y_overlap = (cy.max(viewport.1)).min(cy + ch).min(viewport.1 + viewport.3) - (cy.max(viewport.1));
                
                let intersection_area = (x_overlap * y_overlap).max(0.0);
                let total_area = cw * ch;
                let ratio = if total_area > 0.0 { intersection_area / total_area } else { 0.0 };

                for (cb, threshold) in observers {
                    if ratio >= *threshold {
                        intersection_notifications.push((cb.clone(), node_idx, ratio));
                    }
                }
            }
        }

        // Check IntersectionObservers (second pass: elementos em subframes projetados)
        // iframe_projected_geometry tem chave u64 = (iframe_idx * 1_000_000 + elem_idx).
        // O intersection_registry usa elem_idx (usize) como chave.
        // Iteramos sobre as projeções e buscamos observers que correspondam ao elem_idx.
        for (&proj_key, proj_geom) in iframe_projected_geometry.iter() {
            // Extrair elem_idx da chave composta
            let elem_idx = (proj_key % 1_000_000) as usize;

            if let Some(observers) = intersection_registry.get(&elem_idx) {
                let (gx, gy, gw, gh) = (proj_geom.x, proj_geom.y, proj_geom.width, proj_geom.height);

                // Interseção com o viewport do frame pai
                let x_overlap = (gx.max(viewport.0)).min(gx + gw).min(viewport.0 + viewport.2) - (gx.max(viewport.0));
                let y_overlap = (gy.max(viewport.1)).min(gy + gh).min(viewport.1 + viewport.3) - (gy.max(viewport.1));
                let intersection_area = (x_overlap * y_overlap).max(0.0);
                let total_area = gw * gh;
                let ratio = if total_area > 0.0 { intersection_area / total_area } else { 0.0 };

                for (cb, threshold) in observers {
                    if ratio >= *threshold {
                        intersection_notifications.push((cb.clone(), elem_idx, ratio));
                    }
                }
            }
        }

        // Update layout states for next check
        for (&node_idx, rect) in current_rects.iter() {
            layout_states.insert(node_idx, *rect);
        }
    }

    // Trigger callbacks
    if !resize_notifications.is_empty() || !intersection_notifications.is_empty() {
        rt.with_context(|ctx| {
            ctx.with(|ctx| {
                // Handle Resize Notifications
                for (cb_persistent, node_idx, w, h) in resize_notifications {
                    if let Ok(cb) = cb_persistent.clone().restore(&ctx) {
                        let entry = rquickjs::Object::new(ctx.clone()).unwrap();
                        let rect = rquickjs::Object::new(ctx.clone()).unwrap();
                        let _ = rect.set("width", w);
                        let _ = rect.set("height", h);
                        let _ = entry.set("contentRect", rect);
                        
                        let target = wrap_element(rt, node_idx, &ctx);
                        let _ = entry.set("target", target);
                        
                        let arr = rquickjs::Array::new(ctx.clone()).unwrap();
                        let _ = arr.set(0, entry);
                        let _: rquickjs::Result<Value> = cb.call((arr,));
                    }
                }

                // Handle Intersection Notifications
                for (cb_persistent, node_idx, ratio) in intersection_notifications {
                    if let Ok(cb) = cb_persistent.clone().restore(&ctx) {
                        let entry = rquickjs::Object::new(ctx.clone()).unwrap();
                        let _ = entry.set("intersectionRatio", ratio);
                        let _ = entry.set("isIntersecting", ratio > 0.0);
                        
                        let target = wrap_element(rt, node_idx, &ctx);
                        let _ = entry.set("target", target);

                        let arr = rquickjs::Array::new(ctx.clone()).unwrap();
                        let _ = arr.set(0, entry);
                        let _: rquickjs::Result<Value> = cb.call((arr,));
                    }
                }
            });
        });
    }
}

fn wrap_element<'js>(rt: &JsRuntime, node_idx: usize, ctx: &Ctx<'js>) -> Value<'js> {
    use crate::runtime::bindings::html::element::Element;
    let element = Element {
        dom: rt.dom.lock().unwrap().as_ref().unwrap().clone(),
        index: node_idx,
        mutations: rt.mutations.clone(),
        stylesheet_dirty: rt.stylesheet_dirty.clone(),
        primitives: rt.primitives.clone(),
        canvas_contexts: rt.canvas_contexts.clone(),
        pending_scroll: rt.pending_scroll.clone(),
        element_geometry: rt.element_geometry.clone(),
        element_scroll: rt.element_scroll.clone(),
    };
    if let Ok(instance) = rquickjs::Class::instance(ctx.clone(), element) {
        instance.into_value()
    } else {
        Value::new_null(ctx.clone())
    }
}
