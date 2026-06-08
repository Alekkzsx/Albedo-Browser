//! Utilitários comuns para o motor ACE
//! Este módulo contém funções utilitárias compartilhadas entre diferentes partes do motor

/// Escapa caracteres especiais para uso em conteúdo HTML
pub fn escape_html(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(c),
        }
    }
    result
}

/// Escapa caracteres especiais para uso em valores de atributos HTML
pub fn escape_attr(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(c),
        }
    }
    result
}

/// Verifica se um elemento HTML é um elemento vazio (void element)
pub fn is_void_element(tag_name: &str) -> bool {
    matches!(
        tag_name.to_ascii_lowercase().as_str(),
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" 
        | "link" | "meta" | "param" | "source" | "track" | "wbr"
    )
}

/// Obtém o timestamp atual em milissegundos
pub fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<div>"), "&lt;div&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
    }

    #[test]
    fn test_escape_attr() {
        assert_eq!(escape_attr("value\"test"), "value&quot;test");
    }

    #[test]
    fn test_is_void_element() {
        assert!(is_void_element("br"));
        assert!(is_void_element("BR"));
        assert!(is_void_element("img"));
        assert!(!is_void_element("div"));
        assert!(!is_void_element("span"));
    }

    #[test]
    fn test_current_timestamp_ms() {
        let t1 = current_timestamp_ms();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = current_timestamp_ms();
        assert!(t2 >= t1);
    }
}

