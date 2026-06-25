use super::*;
// AceDOM String Interning System
// Otimização crítica de memória para strings repetidas (tag names, attributes)
// Reduz uso de memória em 60-80% para strings frequentes

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Hasher rápido e seguro para string interning
use fxhash::FxBuildHasher;



/// Helper para tags HTML comuns (pré-internadas)
pub mod html_tags {
    use super::*;
    
    lazy_static::lazy_static! {
        pub static ref DIV: Arc<str> = global::intern("div");
        pub static ref SPAN: Arc<str> = global::intern("span");
        pub static ref P: Arc<str> = global::intern("p");
        pub static ref A: Arc<str> = global::intern("a");
        pub static ref IMG: Arc<str> = global::intern("img");
        pub static ref BUTTON: Arc<str> = global::intern("button");
        pub static ref INPUT: Arc<str> = global::intern("input");
        pub static ref FORM: Arc<str> = global::intern("form");
        pub static ref UL: Arc<str> = global::intern("ul");
        pub static ref OL: Arc<str> = global::intern("ol");
        pub static ref LI: Arc<str> = global::intern("li");
        pub static ref H1: Arc<str> = global::intern("h1");
        pub static ref H2: Arc<str> = global::intern("h2");
        pub static ref H3: Arc<str> = global::intern("h3");
        pub static ref HEADER: Arc<str> = global::intern("header");
        pub static ref FOOTER: Arc<str> = global::intern("footer");
        pub static ref NAV: Arc<str> = global::intern("nav");
        pub static ref SECTION: Arc<str> = global::intern("section");
        pub static ref ARTICLE: Arc<str> = global::intern("article");
        pub static ref ASIDE: Arc<str> = global::intern("aside");
        pub static ref MAIN: Arc<str> = global::intern("main");
    }
}

/// Helper para atributos HTML comuns
pub mod html_attrs {
    use super::*;
    
    lazy_static::lazy_static! {
        pub static ref CLASS: Arc<str> = global::intern("class");
        pub static ref ID: Arc<str> = global::intern("id");
        pub static ref STYLE: Arc<str> = global::intern("style");
        pub static ref HREF: Arc<str> = global::intern("href");
        pub static ref SRC: Arc<str> = global::intern("src");
        pub static ref ALT: Arc<str> = global::intern("alt");
        pub static ref TITLE: Arc<str> = global::intern("title");
        pub static ref NAME: Arc<str> = global::intern("name");
        pub static ref TYPE: Arc<str> = global::intern("type");
        pub static ref VALUE: Arc<str> = global::intern("value");
        pub static ref PLACEHOLDER: Arc<str> = global::intern("placeholder");
        pub static ref DISABLED: Arc<str> = global::intern("disabled");
        pub static ref CHECKED: Arc<str> = global::intern("checked");
        pub static ref SELECTED: Arc<str> = global::intern("selected");
        pub static ref ROLE: Arc<str> = global::intern("role");
        pub static ref ARIA_LABEL: Arc<str> = global::intern("aria-label");
        pub static ref ARIA_HIDDEN: Arc<str> = global::intern("aria-hidden");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
pub(crate) fn test_intern_same_string() {
        let mut interner = StringInterner::default();
        
        let s1 = interner.intern("hello");
        let s2 = interner.intern("hello");
        
        // Devem apontar para o mesmo Arc
        assert!(Arc::ptr_eq(&s1, &s2));
        assert_eq!(interner.len(), 1);
    }

    #[test]
pub(crate) fn test_intern_different_strings() {
        let mut interner = StringInterner::default();
        
        let s1 = interner.intern("hello");
        let s2 = interner.intern("world");
        
        assert_ne!(s1, s2);
        assert_eq!(interner.len(), 2);
    }

    #[test]
pub(crate) fn test_stats_tracking() {
        let mut interner = StringInterner::default();
        
        interner.intern("test");
        interner.intern("test"); // Hit
        interner.intern("test"); // Hit
        
        let stats = interner.get_stats();
        assert_eq!(stats.total_interns, 3);
        assert_eq!(stats.cache_hits, 2);
        assert_eq!(stats.cache_misses, 1);
        assert!(stats.memory_saved_bytes > 0);
    }

    #[test]
pub(crate) fn test_global_intern() {
        // Limpar antes do teste
        global::clear();
        
        let s1 = global::intern("global_test");
        let s2 = global::intern("global_test");
        
        assert!(Arc::ptr_eq(&s1, &s2));
    }

    #[test]
pub(crate) fn test_html_tags_helper() {
        // Verificar se as tags pré-internadas funcionam
        assert_eq!(html_tags::DIV.as_ref(), "div");
        assert_eq!(html_tags::BUTTON.as_ref(), "button");
        assert_eq!(html_attrs::CLASS.as_ref(), "class");
        assert_eq!(html_attrs::ID.as_ref(), "id");
    }

    #[test]
pub(crate) fn test_memory_efficiency() {
        let mut interner = StringInterner::default();
        
        // Internar a mesma string 1000 vezes
        for _ in 0..1000 {
            interner.intern("repeated_string_for_memory_test");
        }
        
        // Deve haver apenas 1 cópia na memória
        assert_eq!(interner.len(), 1);
        
        let stats = interner.get_stats();
        // Economia significativa de memória
        assert!(stats.memory_saved_bytes > 30 * 999); // 30 chars * 999 repetições
    }
}
