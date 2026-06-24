use super::*;
use super::runtime::JsRuntime;
use crate::ace::engine::AceEngine;
use std::sync::{Arc, Mutex};



/// TODO: add docs
pub fn init_js_for_url(url: &str, engine: &AceEngine) -> Option<JsRuntime> {
    if let Ok(mut rt) = JsRuntime::new() {
        // ... (existing code omitted for brevity in thought, but I will provide the full block)
        // Actually I should provide the full block as per rules.
        // Wait, I'm using replace_file_content, so I need to match the block.
        // I'll use a smaller range.
        tracing::info!(url = %url, "Initializing JS runtime");
        rt.context.lock().unwrap_or_else(|e| e.into_inner()).with(|ctx| {
            let _ = ctx.globals().set("__albedo_rt__", rt.clone());
        });
        tracing::debug!("__albedo_rt__ set");
        *rt.resource_manager.lock().unwrap_or_else(|e| e.into_inner()) = engine.resource_manager.clone();
        rt.element_geometry = engine.element_geometry.clone();
        rt.element_scroll = engine.element_scroll.clone();
        *rt.origin.lock().unwrap_or_else(|e| e.into_inner()) = crate::network::security::Origin::from_url(url);
        let origin_str = get_origin(url);

        // ES Modules: Configurar ModuleRegistry e registrar Loader/Resolver
        {
            let mut registry = rt.module_registry.lock().unwrap_or_else(|e| e.into_inner());
            *registry = crate::ace::runtime::core::module_loader::ModuleRegistry::new(url);
        }

        // Registrar o resolver e loader no Runtime do QuickJS
        {
            let registry_clone = rt.module_registry.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let resolver = crate::ace::runtime::core::module_loader::AlbedoModuleResolver {
                registry: registry_clone.clone(),
            };
            let loader = crate::ace::runtime::core::module_loader::AlbedoModuleLoader {
                registry: registry_clone,
            };
            let runtime = rt.runtime.lock().unwrap_or_else(|e| e.into_inner());
            runtime.set_loader(resolver, loader);
        }

        if let Err(e) = init_storage(&rt, origin_str) {
            tracing::error!(?e, origin = origin_str, "Failed to initialize storage");
        }
        crate::ace::runtime::core::registry::register_runtime(rt.id, Arc::new(Mutex::new(rt.clone())));
        if let Some(dom) = &engine.dom {
            if let Err(e) = crate::ace::runtime::bindings::html::document::register(
                &rt,
                dom.clone(),
                engine.stylesheet.clone(),
                engine.primitives.clone(),
                engine.canvas_contexts.clone(),
                url.to_string(),
                "".to_string(),
                engine.resource_manager.clone(),
            ) {
                tracing::error!(?e, "Failed to register document API");
            }
        }
        if let Err(e) = crate::ace::runtime::bindings::utils::console::Console::register(&rt) {
            tracing::error!(?e, "Failed to register console");
        }
        if let Err(e) = register_events(&rt) {
            tracing::error!(?e, "Failed to register events");
        }
        if let Err(e) = init_stdlib(&rt, &url) {
            tracing::error!(?e, "Failed to init stdlib");
        }

        // DOM is already fully parsed and available on the engine.
        // This mirrors the real browser: JS runtime is ready + DOM is interactive.
        rt.set_ready_state("interactive"); // fires DOMContentLoaded

        return Some(rt);
    }
    None
}
