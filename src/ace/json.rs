
pub use albedo_jit::contracts::core::JsonValue;

/// TODO: add docs
fn from_serde(value: serde_json::Value) -> JsonValue {
    match value {
        serde_json::Value::Null => JsonValue::Null,
        serde_json::Value::Bool(b) => JsonValue::Bool(b),
        serde_json::Value::Number(n) => JsonValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => JsonValue::String(s),
        serde_json::Value::Array(arr) => {
            JsonValue::Array(arr.into_iter().map(from_serde).collect())
        }
        serde_json::Value::Object(obj) => {
            let map = obj.into_iter().map(|(k, v)| (k, from_serde(v))).collect();
            JsonValue::Object(map)
        }
    }
}

/// TODO: add docs
fn to_serde(value: &JsonValue) -> serde_json::Value {
    match value {
        JsonValue::Null => serde_json::Value::Null,
        JsonValue::Bool(b) => serde_json::Value::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n) {
                serde_json::Value::Number(num)
            } else {
                serde_json::Value::Null
            }
        }
        JsonValue::String(s) => serde_json::Value::String(s.clone()),
        JsonValue::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(to_serde).collect())
        }
        JsonValue::Object(obj) => {
            let map = obj.iter().map(|(k, v)| (k.clone(), to_serde(v))).collect();
            serde_json::Value::Object(map)
        }
    }
}

/// TODO: add docs
pub fn parse(s: &str) -> Result<JsonValue, String> {
    let parsed: serde_json::Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
    Ok(from_serde(parsed))
}

/// TODO: add docs
pub fn stringify(v: &JsonValue) -> String {
    let serde_value = to_serde(v);
    serde_json::to_string(&value).unwrap_or_default()
}
