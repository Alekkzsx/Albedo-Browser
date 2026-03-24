use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let JsonValue::Bool(b) = self {
            Some(*b)
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

    pub fn as_string(&self) -> Option<&str> {
        if let JsonValue::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        if let JsonValue::Array(a) = self {
            Some(a)
        } else {
            None
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        if let JsonValue::Object(o) = self {
            Some(o)
        } else {
            None
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        if let JsonValue::Object(o) = self {
            o.get(key)
        } else {
            None
        }
    }
}
