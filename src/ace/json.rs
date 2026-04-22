use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn as_string(&self) -> Option<&str> {
        if let JsonValue::String(s) = self {
            Some(s)
        } else {
            None
        }
    }
    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        if let JsonValue::Object(m) = self {
            Some(m)
        } else {
            None
        }
    }
    pub fn as_number(&self) -> Option<f64> {
        if let JsonValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        if let JsonValue::Object(m) = self {
            m.get(key)
        } else {
            None
        }
    }
}

pub fn parse(_s: &str) -> Result<JsonValue, String> {
    Ok(JsonValue::Null)
}

pub fn stringify(_v: &JsonValue) -> String {
    String::new()
}
