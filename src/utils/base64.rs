use base64::{prelude::BASE64_STANDARD, Engine};

/// TODO: add docs
pub fn encode(data: &[u8]) -> String {
    BASE64_STANDARD.encode(data)
}

/// TODO: add docs
pub fn decode(s: &str) -> Result<Vec<u8>, String> {
    // Trims whitespace and newlines if any, which is common in base64 strings in HTML/DataURLs
    let cleaned = s.trim().replace(|c| c == '\r' || c == '\n', "");
    BASE64_STANDARD.decode(&cleaned).map_err(|e| e.to_string())
}
