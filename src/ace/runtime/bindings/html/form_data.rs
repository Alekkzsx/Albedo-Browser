use rquickjs::{Ctx, Result, Value};
use std::collections::HashMap;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct FormData {
    #[qjs(skip_trace)]
    pub data: HashMap<String, Vec<String>>,
}

#[rquickjs::methods]
impl FormData {
    #[qjs(constructor)]
    pub fn new(_ctx: Ctx<'_>, form: Option<Value<'_>>) -> Result<Self> {
        let data = HashMap::new();

        if let Some(form_val) = form {
            if form_val.is_object() {
                // In a real browser, this would extract values from the form's input elements.
                // For now, we'll try to get it if it's an Element of type 'FORM'.
                // This requires access to the DOM, which we don't have easily here without more bindings.
                // A better way would be to have a helper in JS or Element to populate this.
                println!("FormData constructor called with form element (stub: data extraction not yet fully implemented from Element)");
            }
        }

        Ok(Self { data })
    }

    pub fn append(&mut self, name: String, value: String) {
        self.data.entry(name).or_insert_with(Vec::new).push(value);
    }

    pub fn delete(&mut self, name: String) {
        self.data.remove(&name);
    }

    pub fn get(&self, name: String) -> Option<String> {
        self.data.get(&name).and_then(|v| v.first().cloned())
    }

    #[qjs(rename = "getAll")]
    pub fn get_all(&self, name: String) -> Vec<String> {
        self.data.get(&name).cloned().unwrap_or_default()
    }

    pub fn has(&self, name: String) -> bool {
        self.data.contains_key(&name)
    }

    pub fn set(&mut self, name: String, value: String) {
        self.data.insert(name, vec![value]);
    }

    // Iterators and other methods could be added here
}
