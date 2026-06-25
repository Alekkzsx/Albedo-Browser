            if el.tag == "iframe" {
                // If this is an iframe, ensure we have a subframe engine for it
                let mut needs_init = false;
                let mut iframe_src = String::new();

                {
                    if dom.subframes.is_none() {
                        dom.subframes = Some(std::sync::Arc::new(std::sync::Mutex::new(
                            std::collections::HashMap::new(),
                        )));
                    }

                    let subframes_arc = dom.subframes.as_ref().expect("Albedo Engine: internal invariant violated").clone();
                    let mut subframes = subframes_arc.lock().unwrap_or_else(|e| e.into_inner());

                    if !subframes.contains_key(&node_idx) {
                        let sub_engine =
                            std::sync::Arc::new(std::sync::Mutex::new(AceEngine::new()));
                        // Marcar qual nó <iframe> este subframe representa no pai
                        {
                            let sub_eng = sub_engine.lock().unwrap_or_else(|e| e.into_inner());
                            if let Some(ref dom_arc) = sub_eng.dom {
                                let mut sub_dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
                                sub_dom.iframe_node_idx = Some(node_idx);
                }
            }
        }
                        subframes.insert(node_idx, sub_engine);
                        needs_init = true;

                        if let Some(src) = el.attributes.get("src") {
                            iframe_src = src.clone();
                        }
                    }
                }

                if needs_init && !iframe_src.is_empty() {
                    if let Some(subframes_arc) = &dom.subframes {
                        let subframes_lock = subframes_arc.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(engine_arc) = subframes_lock.get(&node_idx) {
                            let mut sub_engine = engine_arc.lock().unwrap_or_else(|e| e.into_inner());
                            // Copy over resource manager and store target URL.
                            // JS runtime is initialized lazily when the iframe content
                            // is actually loaded (do NOT call init_js_for_url here as
                            // it runs init_stdlib which uses tokio and may block).
                            sub_engine.resource_manager = self.resource_manager.clone();
                            sub_engine.current_url = iframe_src.clone();
                        }
                    }
                }
