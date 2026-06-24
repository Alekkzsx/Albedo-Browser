use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{prelude::Opt, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};

// ─── Entrada no registry global de MQL (para disparo de change events) ──────

pub struct MqlEntry {
    pub query: String,
    pub last_matches: bool,
    // Persistent permite guardar Functions JS além do lifetime do Ctx
    pub listeners: Arc<Mutex<Vec<rquickjs::Persistent<Function<'static>>>>>,
}

// ─── MediaQueryList ───────────────────────────────────────────────────────────

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class(rename = "MediaQueryList")]
pub struct MediaQueryList {
    pub media_query: String,
    #[qjs(skip_trace)]
    pub viewport: Arc<Mutex<(i32, i32)>>,
    #[qjs(skip_trace)]
    pub listeners: Arc<Mutex<Vec<rquickjs::Persistent<Function<'static>>>>>,
}

#[rquickjs::methods]
impl MediaQueryList {
    // ── Propriedades ──────────────────────────────────────────────────────────

    #[qjs(get)]
    pub fn matches(&self) -> bool {
        let (vw, vh) = *self.viewport.lock().unwrap_or_else(|e| e.into_inner());
        crate::ace::engine::style::matches_media_query(
            &self.media_query,
            vw as f32,
            vh as f32,
            "light", // TODO: expor color_scheme real do JsRuntime quando implementado
        )
    }

    #[qjs(get)]
    pub fn media(&self) -> String {
        self.media_query.clone()
    }

    // onchange é um setter/getter de conveniência (não usa o listeners vec)
    // Implementado como propriedade JS simples — deixamos o QuickJS gerenciar
    #[qjs(get)]
    pub fn onchange<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Ok(Value::new_null(ctx))
    }
    #[qjs(set, rename = "onchange")]
    pub fn set_onchange<'js>(&self, _ctx: Ctx<'js>, _f: Opt<Function<'js>>) {
        // TODO: armazenar como listener especial se necessário
    }

    // ── addEventListener / removeEventListener ────────────────────────────────

    #[qjs(rename = "addEventListener")]
    pub fn add_event_listener<'js>(
        &self,
        ctx: Ctx<'js>,
        event_type: String,
        callback: Function<'js>,
        _options: Opt<Value<'js>>,
    ) -> Result<()> {
        if event_type != "change" {
            return Ok(());
        }
        let persistent = rquickjs::Persistent::save(&ctx, callback);
        self.listeners.lock().unwrap_or_else(|e| e.into_inner()).push(persistent);
        Ok(())
    }

    #[qjs(rename = "removeEventListener")]
    pub fn remove_event_listener<'js>(
        &self,
        event_type: String,
        _callback: Function<'js>,
        _options: Opt<Value<'js>>,
    ) -> Result<()> {
        if event_type != "change" {
            return Ok(());
        }
        // Remover por identidade — QuickJS não expõe ptr direto,
        // limpamos todos por ora (simplificação segura para v1)
        // TODO: implementar remoção precisa por referência quando necessário
        self.listeners.lock().unwrap_or_else(|e| e.into_inner()).clear();
        Ok(())
    }

    // ── addListener / removeListener (deprecated, mas necessário) ────────────

    #[qjs(rename = "addListener")]
    pub fn add_listener<'js>(&self, ctx: Ctx<'js>, callback: Function<'js>) -> Result<()> {
        let persistent = rquickjs::Persistent::save(&ctx, callback);
        self.listeners.lock().unwrap_or_else(|e| e.into_inner()).push(persistent);
        Ok(())
    }

    #[qjs(rename = "removeListener")]
    pub fn remove_listener<'js>(&self, _callback: Function<'js>) -> Result<()> {
        self.listeners.lock().unwrap_or_else(|e| e.into_inner()).clear();
        Ok(())
    }

    // ── dispatchEvent (stub de compatibilidade) ───────────────────────────────
    #[qjs(rename = "dispatchEvent")]
    pub fn dispatch_event(&self, _event: Value<'_>) -> bool {
        true
    }
}

// ─── Função principal: window.matchMedia(query) ───────────────────────────────

/// TODO: add docs
pub fn register(rt: &JsRuntime) -> Result<()> {
    let viewport = rt.screen_size.clone();
    let mql_registry = rt.mql_registry.clone();

    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            Class::<MediaQueryList>::define(&ctx.globals())?;

            let viewport_cap = viewport.clone();
            let registry_cap = mql_registry.clone();

            let match_media_fn =
                Function::new(ctx.clone(), move |_ctx: Ctx<'_>, query: String| {
                    let listeners: Arc<Mutex<Vec<rquickjs::Persistent<Function<'static>>>>> =
                        Arc::new(Mutex::new(Vec::new()));

                    // Calcular matches inicial
                    let (vw, vh) = *viewport_cap.lock().unwrap_or_else(|e| e.into_inner());
                    let initial_matches = crate::ace::engine::style::matches_media_query(
                        &query, vw as f32, vh as f32, "light",
                    );

                    // Registrar no MQL registry para receber change events
                    let entry = MqlEntry {
                        query: query.clone(),
                        last_matches: initial_matches,
                        listeners: listeners.clone(),
                    };
                    registry_cap.lock().unwrap_or_else(|e| e.into_inner()).push(entry);

                    // Retornar a view structure e deixar rquickjs embrulhar na classe JS
                    MediaQueryList {
                        media_query: query,
                        viewport: viewport_cap.clone(),
                        listeners,
                    }
                })?;

            ctx.globals().set("matchMedia", match_media_fn)?;
            Ok(())
        })
    })
}
