//! # Folha de Estilo Padrão do User-Agent (WHATWG HTML §15)
//!
//! Folha de estilo normativa inicial do navegador para todos os elementos HTML.

use crate::cascade::origin::StyleSheetOrigin;
use crate::model::stylesheet::StyleSheet;

pub const DEFAULT_USER_AGENT_CSS: &str = r#"
html {
    display: block;
}

body {
    display: block;
    margin: 8px;
}

div, article, aside, footer, header, main, nav, section, address, blockquote, figure, figcaption, form, fieldset {
    display: block;
}

p {
    display: block;
    margin-top: 1em;
    margin-bottom: 1em;
}

h1 {
    display: block;
    font-size: 2em;
    font-weight: bold;
    margin-top: 0.67em;
    margin-bottom: 0.67em;
}

h2 {
    display: block;
    font-size: 1.5em;
    font-weight: bold;
    margin-top: 0.83em;
    margin-bottom: 0.83em;
}

h3 {
    display: block;
    font-size: 1.17em;
    font-weight: bold;
    margin-top: 1em;
    margin-bottom: 1em;
}

h4 {
    display: block;
    font-size: 1em;
    font-weight: bold;
    margin-top: 1.33em;
    margin-bottom: 1.33em;
}

h5 {
    display: block;
    font-size: 0.83em;
    font-weight: bold;
    margin-top: 1.67em;
    margin-bottom: 1.67em;
}

h6 {
    display: block;
    font-size: 0.67em;
    font-weight: bold;
    margin-top: 2.33em;
    margin-bottom: 2.33em;
}

ul, ol {
    display: block;
    margin-top: 1em;
    margin-bottom: 1em;
    padding-left: 40px;
}

li {
    display: list-item;
}

pre {
    display: block;
    white-space: pre;
    margin-top: 1em;
    margin-bottom: 1em;
}

hr {
    display: block;
    margin-top: 0.5em;
    margin-bottom: 0.5em;
    border-style: solid;
    border-width: 1px;
}

b, strong {
    font-weight: bold;
}

i, em {
    font-style: italic;
}

table {
    display: table;
    box-sizing: border-box;
}

tr {
    display: table-row;
}

td, th {
    display: table-cell;
    padding: 1px;
}

th {
    font-weight: bold;
    text-align: center;
}

head, script, style, template, link, meta, title {
    display: none;
}
"#;

/// Constrói a folha de estilo oficial do User-Agent.
pub fn default_user_agent_stylesheet() -> StyleSheet {
    StyleSheet::parse(DEFAULT_USER_AGENT_CSS, StyleSheetOrigin::UserAgent)
}
