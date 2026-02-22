use rquickjs::{Ctx, Class, Result, Value};
use crate::runtime::bindings::html::document::Document;
use kuchiki::traits::TendrilSink;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct DOMParser {}

#[rquickjs::methods]
impl DOMParser {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[qjs(rename = "parseFromString")]
    pub fn parse_from_string<'js>(&self, ctx: Ctx<'js>, source: String, _mime_type: String) -> Result<Value<'js>> {
        // MDN: DOMParser.parseFromString(str, mimeType) returns a Document.
        // We can reuse the document creation logic if we have access to it or just create a new one.
        
        // Since we already have a 'document' in the global, maybe we can clone its settings 
        // but with a new DOM parsed from string.
        
        // This is a bit complex because we need the current document's stylesheet, mutations, etc.
        // For simplicity, let's try to get them from the global document.
        let global_doc: Class<Document> = ctx.globals().get("document")?;
        let doc_borrow = global_doc.borrow();
        
        let kuchiki_root = kuchiki::parse_html().from_utf8().one(source.as_bytes());
        let new_dom = std::sync::Arc::new(std::sync::Mutex::new(crate::engine::dom::AceDOM::from_kuchiki(kuchiki_root)));
        
        let new_doc = Document {
            dom: new_dom,
            stylesheet: doc_borrow.stylesheet.clone(),
            mutations: doc_borrow.mutations.clone(),
            stylesheet_dirty: doc_borrow.stylesheet_dirty.clone(),
            cookie_storage: doc_borrow.cookie_storage.clone(),
            resource_manager: doc_borrow.resource_manager.clone(),
            primitives: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            canvas_contexts: doc_borrow.canvas_contexts.clone(),
            pending_scroll: doc_borrow.pending_scroll.clone(),
            element_geometry: doc_borrow.element_geometry.clone(),
            element_scroll: doc_borrow.element_scroll.clone(),
            url: "about:blank".into(),
            referrer: "".into(),
        };
        
        let instance = Class::instance(ctx, new_doc)?;
        Ok(instance.into_value())
    }
}
