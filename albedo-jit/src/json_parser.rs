use crate::contracts::core::JsonValue;

pub fn parse_json(text: &str) -> Result<JsonValue, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    Ok(from_serde(v))
}

fn from_serde(v: serde_json::Value) -> JsonValue {
    match v {
        serde_json::Value::Null => JsonValue::Null,
        serde_json::Value::Bool(b) => JsonValue::Bool(b),
        serde_json::Value::Number(n) => JsonValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => JsonValue::String(s),
        serde_json::Value::Array(arr) => {
            JsonValue::Array(arr.into_iter().map(from_serde).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = std::collections::HashMap::new();
            for (k, v) in obj {
                map.insert(k, from_serde(v));
            }
            JsonValue::Object(map)
        }
    }
}
