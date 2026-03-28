use std::collections::HashMap;

pub mod lexer;
pub mod entities;
pub mod tokenizer;
pub mod tree_builder;

pub use lexer::{
    DoctypeToken as RawDoctypeToken, HtmlLexer, HtmlToken as RawHtmlToken,
    LexerError, LexerErrorKind, StartTagToken as RawStartTagToken,
};
pub use tokenizer::{
    CharacterToken, CommentToken, DoctypeToken, EndTagToken, HtmlToken, HtmlTokenizer,
    StartTagToken, TokenizerError, TokenizerErrorKind, TokenizerErrorSource,
};
pub use tree_builder::{
    build_document, build_document_with_errors, build_fragment, build_fragment_with_errors,
    InsertionMode, TreeBuildOutput, TreeBuilderError, TreeBuilderErrorKind,
    TreeBuilderErrorSource,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlDocument {
    pub doctype: Option<DoctypeToken>,
    pub children: Vec<HtmlNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HtmlNode {
    Element(HtmlElement),
    Text(String),
    Comment(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlElement {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<HtmlNode>,
}

impl HtmlElement {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            attributes: HashMap::new(),
            children: Vec::new(),
        }
    }
}

pub fn parse_document(input: &str) -> HtmlDocument {
    build_document(input)
}

pub fn parse_fragment(input: &str) -> Vec<HtmlNode> {
    build_fragment(input)
}

#[cfg(test)]
mod tests {
    use super::{parse_document, parse_fragment, HtmlNode};

    #[test]
    fn builds_document_shell() {
        let document = parse_document("<title>Hi</title><div>Hello</div>");
        let HtmlNode::Element(html) = &document.children[0] else {
            panic!("expected html element");
        };

        assert_eq!(html.tag, "html");
        assert_eq!(html.children.len(), 2);
    }

    #[test]
    fn parses_nested_fragment() {
        let fragment = parse_fragment("<div><span>Hello</span><br/></div>");
        let HtmlNode::Element(div) = &fragment[0] else {
            panic!("expected div");
        };

        assert_eq!(div.tag, "div");
        assert_eq!(div.children.len(), 2);
    }

    #[test]
    fn keeps_raw_text_content() {
        let fragment = parse_fragment("<script>if (a < b) { ok(); }</script>");
        let HtmlNode::Element(script) = &fragment[0] else {
            panic!("expected script");
        };

        assert_eq!(script.tag, "script");
        assert_eq!(
            script.children,
            vec![HtmlNode::Text("if (a < b) { ok(); }".to_string())]
        );
    }
}
