include!(concat!(env!("OUT_DIR"), "/html_entities.rs"));

pub fn decode_named_entity(name: &str) -> Option<&'static str> {
    let trimmed = name.trim();
    let normalized = trimmed.trim_end_matches(';');
    if normalized.is_empty() {
        return None;
    }
    let key = if normalized.starts_with('&') {
        normalized.to_string()
    } else {
        format!("&{normalized}")
    };
    lookup_named_entity(&key)
}
