/// HTML iframe element bindings and cross-frame communication support
use rquickjs::{Ctx, Value, Result as JsResult, Object, String as JsString};
use std::sync::{Arc, Mutex};

/// Wrapper for iframe-specific properties
#[derive(Clone)]
pub struct IFrameBinding {
    /// Reference to parent window (for window.parent in subframes)
    pub parent_window: Option<Value<'static>>,
    /// Reference to iframe element in parent DOM (for window.frameElement)
    pub frame_element_idx: Option<usize>,
    /// The origin URL of the iframe content (for same-origin policy)
    pub content_origin: String,
    /// Parent document origin (for SOP enforcement)
    pub parent_origin: String,
}

impl IFrameBinding {
    /// TODO: add docs
    pub fn new(content_origin: String, parent_origin: String) -> Self {
        Self {
            parent_window: None,
            frame_element_idx: None,
            content_origin,
            parent_origin,
        }
    }

    /// Check if two origins are the same for cross-frame access
    /// Simple check: protocol://host:port
    pub fn is_same_origin(&self) -> bool {
        // Extract scheme + host + port from both URLs
        self.normalize_origin(&self.content_origin) == self.normalize_origin(&self.parent_origin)
    }

    /// TODO: add docs
    fn normalize_origin(&self, url: &str) -> String {
        // Simple normalization: extract scheme://host:port
        // For now, just do a basic prefix match since url crate may not be available
        if url.starts_with("http://") || url.starts_with("https://") {
            // Extract the authority part (scheme://host:port)
            let after_scheme = if let Some(pos) = url.find("://") {
                &url[pos + 3..]
            } else {
                url
            };

            // Split on first slash to get host:port
            let authority = if let Some(pos) = after_scheme.find('/') {
                &after_scheme[..pos]
            } else {
                after_scheme
            };

            let scheme = if url.starts_with("https") { "https" } else { "http" };
            format!("{}://{}", scheme, authority)
        } else {
            url.to_string()
        }
    }
}

/// Register iframe-specific properties on the window object
/// This is called when creating a subframe runtime to set up parent references
pub fn setup_iframe_window<'js>(
    ctx: &Ctx<'js>,
    global: &Object<'js>,
    parent_window: Option<Value<'js>>,
    frame_element: Option<Value<'js>>,
    top_window: Value<'js>,
) -> JsResult<()> {
    // Set window.parent to the parent window (or self if this is top level)
    if let Some(parent) = parent_window {
        global.set("parent", parent)?;
    } else {
        global.set("parent", global.clone())?;
    }

    // Set window.frameElement to the iframe element in parent DOM
    if let Some(frame_el) = frame_element {
        global.set("frameElement", frame_el)?;
    } else {
        global.set("frameElement", Value::new_null(ctx.clone()))?;
    }

    // Set window.top to the topmost window (for cross-frame navigation)
    global.set("top", top_window)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_origin_policy() {
        let iframe = IFrameBinding::new(
            "https://example.com/page.html".to_string(),
            "https://example.com/index.html".to_string(),
        );
        assert!(iframe.is_same_origin());

        let iframe_diff_port = IFrameBinding::new(
            "https://example.com:8080/page.html".to_string(),
            "https://example.com:9090/index.html".to_string(),
        );
        assert!(!iframe_diff_port.is_same_origin());

        let iframe_diff_scheme = IFrameBinding::new(
            "https://example.com/page.html".to_string(),
            "http://example.com/index.html".to_string(),
        );
        assert!(!iframe_diff_scheme.is_same_origin());
    }
}
