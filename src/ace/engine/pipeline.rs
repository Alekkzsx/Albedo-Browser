
use crate::ace::engine::dom::AceDOM;
use std::sync::{Arc, Mutex};
use crate::ace::engine::core::AceEngine;

impl AceEngine {
    pub fn load_url(&mut self, url: String) {
        println!("[AceEngine] Initiating load for URL: {}", url);
        self.current_url = url.clone();

        if let Some(rm) = &self.resource_manager {
            rm.fetch(url, crate::network::resources::ResourceType::Html, None);
        }
    }
    pub fn load_html(&mut self, html: &str) {
        println!("[AceEngine] Parsing HTML...");
        let ace_dom = AceDOM::from_html(html);
        println!(
            "[AceEngine] DOM Tree created with {} nodes",
            ace_dom.nodes.len()
        );

        self.dom = Some(Arc::new(Mutex::new(ace_dom)));

        // Compilação do CSS da Página e injeção do Author CSS em self.stylesheet
        self.update_stylesheet();

        // Calcular estilos de todos os nós para habilitar o Display e Box Model
        self.recompute_dirty_styles();

        // Force layout computation immediately (this creates subframe slots for iframes)
        self.recompute_layout();

        // After layout is done and all DOM locks are released, initialize JS runtimes
        // for any iframe subframes that were just created. We do this OUTSIDE of all
        // DOM locks to avoid nested lock hangs (init_js_for_url runs init_stdlib which
        // uses tokio and may block).
        self.init_subframe_runtimes();
    }
    pub fn handle_resource_response(
        &mut self,
        res: crate::network::resources::ResourceResponse,
    ) -> bool {
        let mut needs_layout = false;

        if res.url == self.current_url {
            if let Ok(html) = String::from_utf8(res.data.clone()) {
                println!("[AceEngine] Main document downloaded. Calling process_html...");
                self.load_html(&html);
                println!("[AceEngine] process_html returned successfully.");
                needs_layout = true;
            }
        } else {
            let (is_css, css_data) = {
                let mut pending = self.pending_resources.lock().unwrap();
                if pending.contains(&res.url) {
                    pending.remove(&res.url);
                    (true, Some(res.data.clone()))
                } else {
                    (false, None)
                }
            };

            if is_css {
                if let Some(data) = css_data {
                    if let Ok(css) = String::from_utf8(data) {
                        println!("[AceEngine] External CSS downloaded: {}", res.url);
                        {
                            let mut external = self.external_css.lock().unwrap();
                            external.insert(res.url.clone(), css);
                        }
                        // Trigger style recomputation after dropping lock
                        self.update_stylesheet();
                        needs_layout = true;
                    }
                }
            }
        }

        // Push response down to subframes to check if it's theirs
        if let Some(ref dom_arc) = self.dom {
            let dom = dom_arc.lock().unwrap();
            if let Some(ref subframes_arc) = dom.subframes {
                let mut subframes = subframes_arc.lock().unwrap();
                for (_, sub_engine_arc) in subframes.iter_mut() {
                    let mut sub_engine = sub_engine_arc.lock().unwrap();
                    if sub_engine.handle_resource_response(res.clone()) {
                        needs_layout = true;
                    }
                }
            }
        }

        needs_layout
    }
    pub fn update_stylesheet(&mut self) {
        let mut css_source = String::new();
        if let Some(ref dom_arc) = self.dom {
            let dom = dom_arc.lock().unwrap();
            for node in &dom.nodes {
                if let crate::ace::engine::dom::AceNodeType::Element(el) = &node.node_type {
                    if el.tag == "style" {
                        for &child_idx in &node.children {
                            if let Some(child_node) = dom.get_node(child_idx) {
                                if let crate::ace::engine::dom::AceNodeType::Text(text) =
                                    &child_node.node_type
                                {
                                    css_source.push_str(text.as_ref());
                                    css_source.push_str("\n");
                                }
                            }
                        }
                    } else if el.tag == "link"
                        && el.attributes.get("rel").map(|s| s.as_str()) == Some("stylesheet")
                    {
                        if let Some(href) = el.attributes.get("href") {
                            // Resolve relative URL
                            if let Ok(base_url) = crate::ace::url::parse(&self.current_url, None) {
                                if let Ok(abs_url) = base_url.join(href) {
                                    let url_str = abs_url.to_string();

                                    // Check if we already have it
                                    let external = self.external_css.lock().unwrap();
                                    if let Some(content) = external.get(&url_str) {
                                        css_source.push_str(content);
                                        css_source.push_str("\n");
                                    } else {
                                        // Trigger download if not pending
                                        let mut pending = self.pending_resources.lock().unwrap();
                                        if !pending.contains(&url_str) {
                                            println!("[AceEngine] Triggering download for external CSS: {}", url_str);
                                            if let Some(ref rm) = self.resource_manager {
                                                rm.fetch(
                                                    url_str.clone(),
                                                    crate::network::resources::ResourceType::Css,
                                                    None,
                                                );
                                            }
                                            pending.insert(url_str);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let new_stylesheet = crate::ace::engine::style::parse(&css_source);
        println!(
            "[AceEngine] Parsed Author CSS, {} bytes injected. Rules fetched: {}, UA: {}",
            css_source.len(),
            new_stylesheet.rules.len(),
            new_stylesheet.user_agent_rules.len()
        );
        let mut current_style = self.stylesheet.lock().unwrap();
        // Copiar TODOS os campos da nova stylesheet (rules, rule_maps, media, supports, container, fonts, keyframes)
        current_style.rules = new_stylesheet.rules;
        current_style.author_rule_map = new_stylesheet.author_rule_map;
        current_style.media_rules = new_stylesheet.media_rules;
        current_style.supports_rules = new_stylesheet.supports_rules;
        current_style.container_rules = new_stylesheet.container_rules;
        current_style.font_faces = new_stylesheet.font_faces;
        current_style.keyframes = new_stylesheet.keyframes;
        println!("[DEBUG] update_stylesheet() setting styles_dirty!");
        self.styles_dirty
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn process_resource_responses(&mut self) -> bool {
        // Stub: processa respostas de recursos
        false
    }
    /// Initialize JS runtimes for all subframe iframes that don't have one yet.
    /// Must be called when NO DOM locks are held.
    pub(crate) fn init_subframe_runtimes(&mut self) {
        // Collect (node_idx, url, Arc<Mutex<AceEngine>>) for subframes that need a runtime
        let subframes_to_init: Vec<(usize, String, std::sync::Arc<std::sync::Mutex<AceEngine>>)> = {
            if let Some(ref dom_arc) = self.dom {
                let dom = dom_arc.lock().unwrap();
                if let Some(ref subframes_arc) = dom.subframes {
                    let subframes = subframes_arc.lock().unwrap();
                    subframes
                        .iter()
                        .filter(|(_, eng_arc)| {
                            let eng = eng_arc.lock().unwrap();
                            eng.js_runtime.is_none() && !eng.current_url.is_empty()
                        })
                        .map(|(idx, eng_arc)| {
                            let url = eng_arc.lock().unwrap().current_url.clone();
                            (*idx, url, eng_arc.clone())
                        })
                        .collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        };
        // No DOM/subframes locks held from here on
        for (_, url, sub_engine_arc) in subframes_to_init {
            // Create a temporary snapshot of the engine for init_js_for_url (needs resource_manager etc.)
            let snap = sub_engine_arc.lock().unwrap().clone();
            // init_js_for_url does NOT require any external locks - it creates a fresh JsRuntime
            if let Some(rt) = crate::ace::runtime::core::init::init_js_for_url(&url, &snap) {
                sub_engine_arc.lock().unwrap().js_runtime = Some(rt);
            }
        }
    }
    pub(crate) fn is_technical_tag(&self, tag: &str) -> bool {
        matches!(
            tag,
            "style" | "script" | "head" | "meta" | "link" | "title" | "template"
        )
    }
    pub(crate) fn has_technical_ancestor(&self, dom: &crate::ace::engine::dom::AceDOM, node_idx: usize) -> bool {
        let mut curr = dom.nodes.get(node_idx).and_then(|n| n.parent);
        while let Some(idx) = curr {
            if let Some(node) = dom.get_node(idx) {
                if let crate::ace::engine::dom::AceNodeType::Element(el) = &node.node_type {
                    if self.is_technical_tag(&el.tag) {
                        return true;
                    }
                }
                curr = node.parent;
            } else {
                break;
            }
        }
        false
    }
}
