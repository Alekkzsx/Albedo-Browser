use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use rquickjs::{prelude::Rest, Class, Ctx, Function, Result, Value};
use std::sync::{Arc, Mutex};



#[rquickjs::methods]
impl Element {

    /// dialog.open (getter) — retorna true se o atributo "open" está presente
    #[qjs(get, rename = "open")]
    pub fn dialog_open_get(&self) -> bool {
        self.has_attribute("open".into())
    }

    /// dialog.returnValue (getter)
    #[qjs(get, rename = "returnValue")]
    pub fn dialog_return_value_get(&self) -> String {
        self.get_attribute("data-return-value".into())
            .unwrap_or_default()
    }

    /// dialog.returnValue (setter)
    #[qjs(set, rename = "returnValue")]
    pub fn dialog_return_value_set(&self, val: String) {
        self.set_attribute("data-return-value".into(), val);
    }
}
