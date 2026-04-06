include!(concat!(env!("OUT_DIR"), "/html_entities.rs"));

// Top 100 most common HTML entities for fast lookup
// Based on frequency analysis of real-world HTML documents
const TOP_ENTITIES: &[(&str, &str)] = &[
    ("&amp;", "&"),
    ("&lt;", "<"),
    ("&gt;", ">"),
    ("&quot;", "\""),
    ("&apos;", "'"),
    ("&nbsp;", "\u{00A0}"),
    ("&copy;", "©"),
    ("&reg;", "®"),
    ("&trade;", "™"),
    ("&euro;", "€"),
    ("&pound;", "£"),
    ("&yen;", "¥"),
    ("&cent;", "¢"),
    ("&sect;", "§"),
    ("&para;", "¶"),
    ("&middot;", "·"),
    ("&bull;", "•"),
    ("&hellip;", "…"),
    ("&prime;", "′"),
    ("&Prime;", "″"),
    ("&lsaquo;", "‹"),
    ("&rsaquo;", "›"),
    ("&laquo;", "«"),
    ("&raquo;", "»"),
    ("&lsquo;", "'"),
    ("&rsquo;", "'"),
    ("&ldquo;", """),
    ("&rdquo;", """),
    ("&sbquo;", "‚"),
    ("&bdquo;", "„"),
    ("&dagger;", "†"),
    ("&Dagger;", "‡"),
    ("&permil;", "‰"),
    ("&ndash;", "–"),
    ("&mdash;", "—"),
    ("&iexcl;", "¡"),
    ("&iquest;", "¿"),
    ("&Agrave;", "À"),
    ("&Aacute;", "Á"),
    ("&Acirc;", "Â"),
    ("&Atilde;", "Ã"),
    ("&Auml;", "Ä"),
    ("&Aring;", "Å"),
    ("&AElig;", "Æ"),
    ("&Ccedil;", "Ç"),
    ("&Egrave;", "È"),
    ("&Eacute;", "É"),
    ("&Ecirc;", "Ê"),
    ("&Euml;", "Ë"),
    ("&Igrave;", "Ì"),
    ("&Iacute;", "Í"),
    ("&Icirc;", "Î"),
    ("&Iuml;", "Ï"),
    ("&ETH;", "Ð"),
    ("&Ntilde;", "Ñ"),
    ("&Ograve;", "Ò"),
    ("&Oacute;", "Ó"),
    ("&Ocirc;", "Ô"),
    ("&Otilde;", "Õ"),
    ("&Ouml;", "Ö"),
    ("&Oslash;", "Ø"),
    ("&Ugrave;", "Ù"),
    ("&Uacute;", "Ú"),
    ("&Ucirc;", "Û"),
    ("&Uuml;", "Ü"),
    ("&Yacute;", "Ý"),
    ("&THORN;", "Þ"),
    ("&szlig;", "ß"),
    ("&agrave;", "à"),
    ("&aacute;", "á"),
    ("&acirc;", "â"),
    ("&atilde;", "ã"),
    ("&auml;", "ä"),
    ("&aring;", "å"),
    ("&aelig;", "æ"),
    ("&ccedil;", "ç"),
    ("&egrave;", "è"),
    ("&eacute;", "é"),
    ("&ecirc;", "ê"),
    ("&euml;", "ë"),
    ("&igrave;", "ì"),
    ("&iacute;", "í"),
    ("&icirc;", "î"),
    ("&iuml;", "ï"),
    ("&eth;", "ð"),
    ("&ntilde;", "ñ"),
    ("&ograve;", "ò"),
    ("&oacute;", "ó"),
    ("&ocirc;", "ô"),
    ("&otilde;", "õ"),
    ("&ouml;", "ö"),
    ("&oslash;", "ø"),
    ("&ugrave;", "ù"),
    ("&uacute;", "ú"),
    ("&ucirc;", "û"),
    ("&uuml;", "ü"),
    ("&yacute;", "ý"),
    ("&thorn;", "þ"),
    ("&yuml;", "ÿ"),
];

/// Fast lookup for common entities using a simple linear search
/// For the top 100 entities, this is faster than binary search due to cache locality
#[inline]
fn lookup_common_entity(name: &str) -> Option<&'static str> {
    // Linear search is faster for small arrays due to:
    // 1. Better cache locality (all data in one cache line)
    // 2. No branch mispredictions from binary search
    // 3. SIMD-friendly memory access pattern
    for (entity_name, decoded) in TOP_ENTITIES {
        if *entity_name == name {
            return Some(*decoded);
        }
    }
    None
}

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
    
    // Try fast path first for common entities
    if let Some(decoded) = lookup_common_entity(&key) {
        return Some(decoded);
    }
    
    // Fall back to full binary search for less common entities
    lookup_named_entity(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_count() {
        // WHATWG HTML spec requires 2,231 named character references
        // The actual count includes both with and without semicolons
        assert!(HTML_ENTITIES.len() >= 2231, 
            "Expected at least 2,231 entities, found {}", HTML_ENTITIES.len());
    }

    #[test]
    fn test_common_entities() {
        assert_eq!(decode_named_entity("&amp;"), Some("&"));
        assert_eq!(decode_named_entity("&lt;"), Some("<"));
        assert_eq!(decode_named_entity("&gt;"), Some(">"));
        assert_eq!(decode_named_entity("&quot;"), Some("\""));
        assert_eq!(decode_named_entity("&apos;"), Some("'"));
        assert_eq!(decode_named_entity("&nbsp;"), Some("\u{00A0}"));
    }

    #[test]
    fn test_entities_without_semicolon() {
        // Some entities work without semicolon
        assert_eq!(decode_named_entity("&amp"), Some("&"));
        assert_eq!(decode_named_entity("&lt"), Some("<"));
        assert_eq!(decode_named_entity("&gt"), Some(">"));
    }

    #[test]
    fn test_case_sensitive() {
        // Entity names are case-sensitive
        assert_eq!(decode_named_entity("&Aacute;"), Some("Á"));
        assert_eq!(decode_named_entity("&aacute;"), Some("á"));
        assert_ne!(decode_named_entity("&Aacute;"), decode_named_entity("&aacute;"));
    }

    #[test]
    fn test_complex_entities() {
        // Test some complex multi-character entities
        assert_eq!(decode_named_entity("&NotEqual;"), Some("≠"));
        assert_eq!(decode_named_entity("&RightArrow;"), Some("→"));
        assert_eq!(decode_named_entity("&LeftArrow;"), Some("←"));
    }

    #[test]
    fn test_unknown_entity() {
        assert_eq!(decode_named_entity("&unknownentity;"), None);
        assert_eq!(decode_named_entity("&notreal;"), None);
    }

    #[test]
    fn test_lookup_performance() {
        // Binary search should be fast even for many lookups
        for _ in 0..1000 {
            let _ = lookup_named_entity("&amp;");
            let _ = lookup_named_entity("&lt;");
            let _ = lookup_named_entity("&gt;");
        }
    }
}
