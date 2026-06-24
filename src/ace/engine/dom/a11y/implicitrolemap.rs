use super::*;
//! Accessibility Tree (ARIA 1.2) Implementation - W3C WAI-ARIA Spec
//! 
//! Este módulo implementa:
//! - Mapeamento implícito de roles HTML → ARIA
//! - Accessible Name Computation (AccName 1.2)
//! - States & Properties ARIA
//! - Relations (aria-controls, aria-owns, etc.)
//! - Tree traversal para screen readers

use std::collections::HashMap;
use crate::ace::engine::dom::{AceDOM, AceNodeType};

/// Role ARIA de um elemento


impl ImplicitRoleMap {
    /// Retorna o role ARIA implícito para um elemento HTML
    pub fn get_implicit_role(tag: &str, attributes: &HashMap<String, String>) -> AriaRole {
        // Verifica se há role explícito via attribute
        if let Some(explicit_role) = attributes.get("role") {
            if let Some(role) = AriaRole::from_str(explicit_role) {
                return role;
            }
        }
        
        // Mapeamento implícito baseado no elemento HTML
        match tag.to_lowercase().as_str() {
            "a" => {
                if attributes.contains_key("href") {
                    AriaRole::Link
                } else {
                    AriaRole::Generic
                }
            },
            "article" => AriaRole::Article,
            "aside" => AriaRole::Complementary,
            "button" => AriaRole::Button,
            "datalist" => AriaRole::Listbox,
            "dd" => AriaRole::Definition,
            "details" => AriaRole::Group,
            "dfn" => AriaRole::Term,
            "dialog" => AriaRole::Dialog,
            "dt" => AriaRole::Term,
            "fieldset" => AriaRole::Group,
            "figure" => AriaRole::Figure,
            "footer" => {
                // footer dentro de body → contentinfo, senão → generic
                AriaRole::Contentinfo
            },
            "form" => AriaRole::Form,
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => AriaRole::Heading,
            "header" => {
                // header dentro de body → banner, senão → generic
                AriaRole::Banner
            },
            "hr" => AriaRole::Separator,
            "img" => {
                if attributes.get("alt").map(|s| s.is_empty()).unwrap_or(false) {
                    AriaRole::None // Imagem decorativa
                } else {
                    AriaRole::Img
                }
            },
            "input" => {
                let input_type = attributes.get("type")
                    .map(|s| s.as_str())
                    .unwrap_or("text");
                
                match input_type {
                    "button" | "submit" | "reset" | "image" => AriaRole::Button,
                    "checkbox" => AriaRole::Checkbox,
                    "color" => AriaRole::Textbox,
                    "date" | "datetime-local" | "month" | "time" | "week" => AriaRole::Combobox,
                    "email" | "tel" | "url" => AriaRole::Textbox,
                    "number" => AriaRole::Spinbutton,
                    "password" => AriaRole::Textbox,
                    "radio" => AriaRole::Radio,
                    "range" => AriaRole::Slider,
                    "search" => AriaRole::Searchbox,
                    "text" | "" => AriaRole::Textbox,
                    "hidden" => AriaRole::None,
                    _ => AriaRole::Generic,
                }
            },
            "li" => AriaRole::ListItem,
            "main" => AriaRole::Main,
            "math" => AriaRole::Math,
            "menu" => AriaRole::List,
            "nav" => AriaRole::Navigation,
            "ol" => AriaRole::List,
            "optgroup" => AriaRole::Group,
            "option" => AriaRole::Option,
            "output" => AriaRole::Status,
            "progress" => AriaRole::Progressbar,
            "section" => AriaRole::Region,
            "select" => {
                if attributes.contains_key("multiple") {
                    AriaRole::Listbox
                } else {
                    AriaRole::Combobox
                }
            },
            "summary" => AriaRole::Button,
            "table" => AriaRole::Table,
            "tbody" | "tfoot" | "thead" => AriaRole::Rowgroup,
            "td" => AriaRole::Cell,
            "textarea" => AriaRole::Textbox,
            "th" => {
                let scope = attributes.get("scope")
                    .map(|s| s.as_str())
                    .unwrap_or("");
                match scope {
                    "col" | "colgroup" => AriaRole::Columnheader,
                    "row" | "rowgroup" => AriaRole::Rowheader,
                    _ => AriaRole::Rowheader,
                }
            },
            "tr" => AriaRole::Row,
            "ul" => AriaRole::List,
            
            // Elements with no implicit role
            "b" | "i" | "span" | "div" | "pre" | "blockquote" | "code" | "em" | "strong" | "small" => {
                AriaRole::Generic
            },
            
            _ => AriaRole::Generic,
        }
    }
}
