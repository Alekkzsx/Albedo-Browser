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

impl From<serde_json::Value> for JsonValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => JsonValue::Null,
            serde_json::Value::Bool(b) => JsonValue::Bool(b),
            serde_json::Value::Number(n) => JsonValue::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => JsonValue::String(s),
            serde_json::Value::Array(arr) => {
                JsonValue::Array(arr.into_iter().map(JsonValue::from).collect())
            }
            serde_json::Value::Object(obj) => {
                let map = obj.into_iter().map(|(k, v)| (k, JsonValue::from(v))).collect();
                JsonValue::Object(map)
            }
        }
    }
}

impl From<JsonValue> for serde_json::Value {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => serde_json::Value::Null,
            JsonValue::Bool(b) => serde_json::Value::Bool(b),
            JsonValue::Number(n) => {
                if let Some(num) = serde_json::Number::from_f64(n) {
                    serde_json::Value::Number(num)
                } else {
                    serde_json::Value::Null
                }
            }
            JsonValue::String(s) => serde_json::Value::String(s),
            JsonValue::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(serde_json::Value::from).collect())
            }
            JsonValue::Object(obj) => {
                let map = obj.into_iter().map(|(k, v)| (k, serde_json::Value::from(v))).collect();
                serde_json::Value::Object(map)
            }
        }
    }
}

pub fn parse(s: &str) -> Result<JsonValue, String> {
    let parsed: serde_json::Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
    Ok(JsonValue::from(parsed))
}

pub fn stringify(v: &JsonValue) -> String {
    let val = serde_json::Value::from(v.clone());
    serde_json::to_string(&val).unwrap_or_default()
}
