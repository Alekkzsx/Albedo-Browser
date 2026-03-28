include!(concat!(env!("OUT_DIR"), "/html_entities.rs"));

pub fn decode_named_entity(name: &str) -> Option<&'static str> {
    let normalized = name.trim_end_matches(';');
    if normalized.is_empty() {
        return None;
    }
    lookup_named_entity(normalized)
}
