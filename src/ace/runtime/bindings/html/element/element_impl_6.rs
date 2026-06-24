use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



#[rquickjs::methods]
impl Element {

    #[qjs(get, rename = "origin")]
    pub fn origin(&self) -> String {
        let href = self::props::href(self);
        if let Ok(u) = crate::ace::url::parse(&href, None) {
            return u.origin();
        }
        "".to_string()
    }

    #[qjs(get, rename = "pathname")]
    pub fn pathname(&self) -> String {
        let href = self::props::href(self);
        if let Ok(u) = crate::ace::url::parse(&href, None) {
            return u.path();
        }
        "".to_string()
    }

    #[qjs(get, rename = "width")]
    pub fn width(&self) -> i32 {
        self::props::width(self)
    }

    #[qjs(set, rename = "width")]
    pub fn set_width(&self, val: i32) {
        self::props::set_width(self, val)
    }

    #[qjs(get, rename = "height")]
    pub fn height(&self) -> i32 {
        self::props::height(self)
    }

    #[qjs(set, rename = "height")]
    pub fn set_height(&self, val: i32) {
        self::props::set_height(self, val)
    }

    #[qjs(rename = "getContext")]
    pub fn get_context<'js>(&self, ctx: Ctx<'js>, type_: String) -> Result<Value<'js>> {
        self::canvas::get_context(self, ctx, type_)
    }

    // ─── HTMLDialogElement API ───────────────────────────────────────

    /// dialog.show() — abre o dialog como non-modal
    #[qjs(rename = "show")]
    pub fn dialog_show(&self) {
        let is_dialog = {
            let dom = self.dom.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(node) = dom.get_node(self.index) {
                matches!(&node.node_type, AceNodeType::Element(el) if el.tag == "dialog")
            } else {
                false
            }
        };
        if !is_dialog {
            return;
        }

        // Se já está aberto, no-op
        if self.has_attribute("open".into()) {
            return;
        }

        self.set_attribute("open".into(), String::new());
        self.mark_mutation();
    }

    /// dialog.showModal() — abre como modal com backdrop
    #[qjs(rename = "showModal")]
    pub fn dialog_show_modal(&self) {
        let is_dialog = {
            let dom = self.dom.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(node) = dom.get_node(self.index) {
                matches!(&node.node_type, AceNodeType::Element(el) if el.tag == "dialog")
            } else {
                false
            }
        };
        if !is_dialog {
            return;
        }

        // Se já está aberto, no-op (spec diz InvalidStateError, mas sem throw por agora)
        if self.has_attribute("open".into()) {
            return;
        }

        self.set_attribute("open".into(), String::new());
        self.set_attribute("data-ace-modal".into(), String::new());
        self.mark_mutation();
    }

    /// dialog.close(returnValue?) — fecha o dialog, dispara evento "close"
    #[qjs(rename = "close")]
    pub fn dialog_close(&self, return_value: Option<String>) {
        let is_dialog = {
            let dom = self.dom.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(node) = dom.get_node(self.index) {
                matches!(&node.node_type, AceNodeType::Element(el) if el.tag == "dialog")
            } else {
                false
            }
        };
        if !is_dialog {
            return;
        }

        // Se não está aberto, no-op
        if !self.has_attribute("open".into()) {
            return;
        }

        // Armazenar returnValue se fornecido
        if let Some(rv) = return_value {
            self.set_attribute("data-return-value".into(), rv);
        }

        self.remove_attribute("open".into());
        self.remove_attribute("data-ace-modal".into());
        self.mark_mutation();

        // Disparar evento "close" no nó (via DOM mutation — o pulse tratará)
        // Nota: Para disparar imediatamente, seria necessário acesso ao JS runtime aqui.
        // A marca de mutation garante que o layout recompute.
    }
}
