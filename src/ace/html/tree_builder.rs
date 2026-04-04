use crate::ace::html::{
    lexer::LexerState, DoctypeToken, EndTagToken, HtmlDocument, HtmlElement, HtmlNode, HtmlToken,
    HtmlTokenizer, PreloadRequest, PreloadScanner, ShadowRootMode, StartTagToken,
    TokenizerErrorSource,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InSelect,
    InSelectInTable,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeBuilderErrorKind {
    UnexpectedToken,
    UnexpectedEndTag,
    UnexpectedDoctype,
    UnexpectedCharacter,
    NestedHead,
    FosterParenting,
    AdoptionAgency,
    TokenizerError,
    UnexpectedEof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeBuilderErrorSource {
    Lexer,
    Tokenizer,
    TreeBuilder,
}

#[derive(Debug, Clone)]
pub struct TreeBuilderError {
    pub kind: TreeBuilderErrorKind,
    pub insertion_mode: InsertionMode,
    pub message: String,
    pub source: TreeBuilderErrorSource,
    pub line: usize,
    pub column: usize,
}

impl TreeBuilderError {
    pub fn new(kind: TreeBuilderErrorKind, mode: InsertionMode, message: impl Into<String>) -> Self {
        Self {
            kind,
            insertion_mode: mode,
            message: message.into(),
            source: TreeBuilderErrorSource::TreeBuilder,
            line: 1,
            column: 1,
        }
    }

    pub fn with_pos(mut self, line: usize, col: usize) -> Self {
        self.line = line;
        self.column = col;
        self
    }
}

pub struct TreeBuildOutput {
    pub document: HtmlDocument,
    pub errors: Vec<TreeBuilderError>,
    pub preload_requests: Vec<PreloadRequest>,
}

#[derive(Clone, Debug)]
enum BuilderNodeData {
    Element(HtmlElement),
    Text(String),
    Comment(String),
}

#[derive(Clone, Debug)]
struct BuilderNode {
    data: BuilderNodeData,
    parent: Option<usize>,
    children: Vec<usize>,
}

pub struct HtmlTreeBuilder<'a> {
    tokenizer: HtmlTokenizer<'a>,
    insertion_mode: InsertionMode,
    preload_scanner: PreloadScanner,
    preload_requests: Vec<PreloadRequest>,
    doctype: Option<DoctypeToken>,
    errors: Vec<TreeBuilderError>,
    nodes: Vec<BuilderNode>,
    root_children: Vec<usize>,
    open_elements: Vec<usize>,
    html_element_id: Option<usize>,
    head_element_id: Option<usize>,
    body_element_id: Option<usize>,
    fragment_context: Option<String>,
    fragment_root_id: Option<usize>,
}

impl<'a> HtmlTreeBuilder<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            tokenizer: HtmlTokenizer::new(input),
            insertion_mode: InsertionMode::Initial,
            preload_scanner: PreloadScanner::new(),
            preload_requests: Vec::new(),
            doctype: None,
            errors: Vec::new(),
            nodes: Vec::new(),
            root_children: Vec::new(),
            open_elements: Vec::new(),
            html_element_id: None,
            head_element_id: None,
            body_element_id: None,
            fragment_context: None,
            fragment_root_id: None,
        }
    }

    pub fn setup_fragment_mode(&mut self, context: &str) {
        self.fragment_context = Some(context.to_ascii_lowercase());
        let context_id = self.create_element(HtmlElement::new(context.to_ascii_lowercase()), None);
        self.fragment_root_id = Some(context_id);
        self.open_elements.push(context_id);
        self.insertion_mode = match context {
            "table" => InsertionMode::InTable,
            "tbody" | "thead" | "tfoot" => InsertionMode::InTableBody,
            "tr" => InsertionMode::InRow,
            "td" | "th" => InsertionMode::InCell,
            "select" => InsertionMode::InSelect,
            _ => InsertionMode::InBody,
        };

        match context {
            "title" | "textarea" => {
                self.tokenizer.set_raw_text_tag(Some(context.to_string()));
                self.tokenizer.set_state(LexerState::RcData);
            }
            "style" | "xmp" | "iframe" | "noembed" | "noframes" => {
                self.tokenizer.set_raw_text_tag(Some(context.to_string()));
                self.tokenizer.set_state(LexerState::RawText);
            }
            "script" => {
                self.tokenizer.set_raw_text_tag(Some(context.to_string()));
                self.tokenizer.set_state(LexerState::ScriptData);
            }
            "plaintext" => self.tokenizer.set_state(LexerState::PlainText),
            _ => {}
        }
    }

    pub fn run(mut self) -> TreeBuildOutput {
        self.speculate();

        loop {
            let token = self.tokenizer.next_token();
            self.collect_tokenizer_errors();

            match token {
                HtmlToken::Eof => break,
                HtmlToken::Doctype(dt) => self.handle_doctype(dt),
                HtmlToken::StartTag(tag) => self.handle_start_tag(tag),
                HtmlToken::EndTag(tag) => self.handle_end_tag(tag),
                HtmlToken::Character(text) => self.handle_text(text.data),
                HtmlToken::Comment(comment) => self.handle_comment(comment.data),
            }
        }

        if self.fragment_context.is_none() {
            self.ensure_document_structure();
        }

        let document_children = if let Some(context_id) = self.fragment_root_id {
            self.nodes[context_id]
                .children
                .iter()
                .map(|&id| self.to_html_node(id))
                .collect()
        } else {
            self.root_children
                .iter()
                .map(|&id| self.to_html_node(id))
                .collect()
        };

        if !self.open_elements.is_empty() && self.fragment_context.is_none() {
            self.errors.push(TreeBuilderError::new(
                TreeBuilderErrorKind::UnexpectedEof,
                self.insertion_mode,
                "unexpected EOF while elements remained open",
            ));
        }

        TreeBuildOutput {
            document: HtmlDocument {
                doctype: self.doctype,
                children: document_children,
            },
            errors: self.errors,
            preload_requests: self.preload_requests,
        }
    }

    fn speculate(&mut self) {
        let remaining = self.tokenizer.lexer.remaining_input();
        if remaining.is_empty() {
            return;
        }

        for req in self.preload_scanner.scan(&remaining) {
            if !self.preload_requests.iter().any(|existing| existing.url == req.url) {
                self.preload_requests.push(req);
            }
        }
    }

    fn collect_tokenizer_errors(&mut self) {
        for err in self.tokenizer.take_errors() {
            let mut converted = TreeBuilderError::new(
                TreeBuilderErrorKind::TokenizerError,
                self.insertion_mode,
                err.message,
            )
            .with_pos(err.line, err.column);
            converted.source = match err.source {
                TokenizerErrorSource::Lexer => TreeBuilderErrorSource::Lexer,
                TokenizerErrorSource::Tokenizer => TreeBuilderErrorSource::Tokenizer,
            };
            self.errors.push(converted);
        }
    }

    fn handle_doctype(&mut self, doctype: DoctypeToken) {
        if self.fragment_context.is_some() || self.doctype.is_some() {
            self.errors.push(TreeBuilderError::new(
                TreeBuilderErrorKind::UnexpectedDoctype,
                self.insertion_mode,
                "unexpected doctype token",
            ));
            return;
        }

        self.doctype = Some(doctype);
        self.insertion_mode = InsertionMode::BeforeHtml;
    }

    fn handle_start_tag(&mut self, mut tag: StartTagToken) {
        if self.fragment_context.is_none() {
            self.ensure_document_structure();
        }

        if self.fragment_context.as_deref() == Some("table") && tag.name == "tr" {
            let tbody_id = self.insert_element_at_current(HtmlElement::new("tbody"));
            self.open_elements.push(tbody_id);
            self.insertion_mode = InsertionMode::InTableBody;
        }

        if matches!(self.current_tag(), Some("head")) && !is_head_content_tag(&tag.name) && tag.name != "head" {
            self.open_elements.pop();
            self.insertion_mode = InsertionMode::AfterHead;
        }

        if self.body_element_id.is_none() && self.fragment_context.is_none() && tag.name != "html" && tag.name != "head" && tag.name != "body" {
            self.ensure_body_element();
        }

        let tag_name = tag.name.clone();
        let element = self.make_element(&mut tag);

        match tag_name.as_str() {
            "html" if self.fragment_context.is_none() => {
                if self.html_element_id.is_none() {
                    let id = self.insert_root_element(element);
                    self.html_element_id = Some(id);
                    self.open_elements.push(id);
                }
                self.insertion_mode = InsertionMode::BeforeHead;
            }
            "head" if self.fragment_context.is_none() => {
                if self.head_element_id.is_none() {
                    let id = self.insert_element_under_html(element);
                    self.head_element_id = Some(id);
                    self.open_elements.push(id);
                    self.insertion_mode = InsertionMode::InHead;
                } else {
                    self.errors.push(TreeBuilderError::new(
                        TreeBuilderErrorKind::NestedHead,
                        self.insertion_mode,
                        "nested head element",
                    ));
                }
            }
            "body" if self.fragment_context.is_none() => {
                let id = if let Some(existing) = self.body_element_id {
                    existing
                } else {
                    let id = self.insert_element_under_html(element);
                    self.body_element_id = Some(id);
                    id
                };
                self.open_elements.retain(|&open| Some(open) == self.html_element_id);
                self.open_elements.push(id);
                self.insertion_mode = InsertionMode::InBody;
            }
            _ => {
                let id = self.insert_element_at_current(element);
                if !is_void_element(&tag_name) && !tag.self_closing {
                    self.open_elements.push(id);
                }
                self.update_tokenizer_state_for_tag(&tag_name);
                self.insertion_mode = match tag_name.as_str() {
                    "table" => InsertionMode::InTable,
                    "tbody" | "thead" | "tfoot" => InsertionMode::InTableBody,
                    "tr" => InsertionMode::InRow,
                    "td" | "th" => InsertionMode::InCell,
                    "select" => InsertionMode::InSelect,
                    _ => InsertionMode::InBody,
                };
            }
        }
    }

    fn handle_end_tag(&mut self, tag: EndTagToken) {
        if self.fragment_context.is_none() && tag.name == "head" {
            if matches!(self.current_tag(), Some("head")) {
                self.open_elements.pop();
            }
            self.insertion_mode = InsertionMode::AfterHead;
            return;
        }

        if self.fragment_context.is_none() && tag.name == "body" {
            self.close_until("body");
            self.insertion_mode = InsertionMode::AfterBody;
            return;
        }

        if self.fragment_context.is_none() && tag.name == "html" {
            self.close_until("body");
            self.close_until("html");
            self.insertion_mode = InsertionMode::AfterAfterBody;
            return;
        }

        if !self.close_until(&tag.name) {
            self.errors.push(TreeBuilderError::new(
                TreeBuilderErrorKind::UnexpectedEndTag,
                self.insertion_mode,
                format!("unexpected end tag </{}>", tag.name),
            ));
        }

        self.insertion_mode = InsertionMode::InBody;
    }

    fn handle_text(&mut self, text: String) {
        if text.is_empty() {
            return;
        }

        if self.fragment_context.is_none() {
            self.ensure_body_element();
        }

        if self.should_foster_parent_text() && !text.trim().is_empty() {
            self.errors.push(TreeBuilderError::new(
                TreeBuilderErrorKind::FosterParenting,
                self.insertion_mode,
                "text foster-parented outside table",
            ));
            if let Some(body_id) = self.body_element_id {
                let text_id = self.create_text_node(text, Some(body_id));
                self.nodes[body_id].children.push(text_id);
                return;
            }
        }

        if let Some(parent_id) = self.current_parent_id() {
            let text_id = self.create_text_node(text, Some(parent_id));
            self.nodes[parent_id].children.push(text_id);
        } else {
            let text_id = self.create_text_node(text, None);
            self.root_children.push(text_id);
        }
    }

    fn handle_comment(&mut self, comment: String) {
        if let Some(parent_id) = self.current_parent_id() {
            let comment_id = self.create_comment_node(comment, Some(parent_id));
            self.nodes[parent_id].children.push(comment_id);
        } else {
            let comment_id = self.create_comment_node(comment, None);
            self.root_children.push(comment_id);
        }
    }

    fn ensure_document_structure(&mut self) {
        if self.html_element_id.is_none() {
            let html_id = self.insert_root_element(HtmlElement::new("html"));
            self.html_element_id = Some(html_id);
            self.open_elements.push(html_id);
            self.insertion_mode = InsertionMode::BeforeHead;
        }

        if self.head_element_id.is_none() {
            let head_id = self.insert_element_under_html(HtmlElement::new("head"));
            self.head_element_id = Some(head_id);
            self.insertion_mode = InsertionMode::AfterHead;
        }
    }

    fn ensure_body_element(&mut self) {
        self.ensure_document_structure();
        if self.body_element_id.is_none() {
            let body_id = self.insert_element_under_html(HtmlElement::new("body"));
            self.body_element_id = Some(body_id);
        }

        if let Some(body_id) = self.body_element_id {
            self.open_elements.retain(|&id| Some(id) == self.html_element_id || id == body_id);
            if self.open_elements.last().copied() != Some(body_id) {
                self.open_elements.push(body_id);
            }
            self.insertion_mode = InsertionMode::InBody;
        }
    }

    fn make_element(&self, tag: &mut StartTagToken) -> HtmlElement {
        let namespace = self.determine_namespace(&tag.name);
        let mut element = HtmlElement::with_namespace(tag.name.clone(), namespace);
        let mut attrs = std::mem::take(&mut tag.attributes);

        element.slot_name = attrs.remove("slot");
        element.is_value = attrs.remove("is");
        element.shadow_root_mode = attrs.remove("shadowrootmode").and_then(|mode| match mode.as_str() {
            "open" => Some(ShadowRootMode::Open),
            "closed" => Some(ShadowRootMode::Closed),
            _ => None,
        });
        element.attributes = attrs;
        element
    }

    fn determine_namespace(&self, tag_name: &str) -> crate::ace::html::Namespace {
        if tag_name == "svg" {
            return crate::ace::html::Namespace::Svg;
        }
        if tag_name == "math" {
            return crate::ace::html::Namespace::MathMl;
        }

        let Some(parent_id) = self.current_parent_id() else {
            return crate::ace::html::Namespace::Html;
        };
        match &self.nodes[parent_id].data {
            BuilderNodeData::Element(parent) => match parent.namespace {
                crate::ace::html::Namespace::Svg if parent.tag != "foreignObject" => {
                    crate::ace::html::Namespace::Svg
                }
                crate::ace::html::Namespace::MathMl => crate::ace::html::Namespace::MathMl,
                _ => crate::ace::html::Namespace::Html,
            },
            BuilderNodeData::Text(_) | BuilderNodeData::Comment(_) => crate::ace::html::Namespace::Html,
        }
    }

    fn insert_root_element(&mut self, element: HtmlElement) -> usize {
        let id = self.create_element(element, None);
        self.root_children.push(id);
        id
    }

    fn insert_element_under_html(&mut self, element: HtmlElement) -> usize {
        if let Some(html_id) = self.html_element_id {
            self.create_and_attach(element, html_id)
        } else {
            self.insert_root_element(element)
        }
    }

    fn insert_element_at_current(&mut self, element: HtmlElement) -> usize {
        if let Some(parent_id) = self.current_parent_id() {
            self.create_and_attach(element, parent_id)
        } else {
            self.insert_root_element(element)
        }
    }

    fn create_and_attach(&mut self, element: HtmlElement, parent_id: usize) -> usize {
        let child_id = self.create_element(element, Some(parent_id));
        self.nodes[parent_id].children.push(child_id);
        child_id
    }

    fn create_element(&mut self, element: HtmlElement, parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(BuilderNode {
            data: BuilderNodeData::Element(element),
            parent,
            children: Vec::new(),
        });
        id
    }

    fn create_text_node(&mut self, text: String, parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(BuilderNode {
            data: BuilderNodeData::Text(text),
            parent,
            children: Vec::new(),
        });
        id
    }

    fn create_comment_node(&mut self, comment: String, parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(BuilderNode {
            data: BuilderNodeData::Comment(comment),
            parent,
            children: Vec::new(),
        });
        id
    }

    fn current_parent_id(&self) -> Option<usize> {
        self.open_elements.last().copied()
    }

    fn current_tag(&self) -> Option<&str> {
        let id = self.current_parent_id()?;
        match &self.nodes[id].data {
            BuilderNodeData::Element(element) => Some(element.tag.as_str()),
            BuilderNodeData::Text(_) | BuilderNodeData::Comment(_) => None,
        }
    }

    fn close_until(&mut self, tag_name: &str) -> bool {
        while let Some(id) = self.open_elements.pop() {
            if let BuilderNodeData::Element(element) = &self.nodes[id].data {
                if element.tag == tag_name {
                    return true;
                }
            }
        }
        false
    }

    fn should_foster_parent_text(&self) -> bool {
        matches!(
            self.current_tag(),
            Some("table" | "tbody" | "tfoot" | "thead" | "tr")
        )
    }

    fn update_tokenizer_state_for_tag(&mut self, tag_name: &str) {
        match tag_name {
            "title" | "textarea" => {
                self.tokenizer.set_raw_text_tag(Some(tag_name.to_string()));
                self.tokenizer.set_state(LexerState::RcData);
            }
            "style" | "xmp" | "iframe" | "noembed" | "noframes" => {
                self.tokenizer.set_raw_text_tag(Some(tag_name.to_string()));
                self.tokenizer.set_state(LexerState::RawText);
            }
            "script" => {
                self.tokenizer.set_raw_text_tag(Some(tag_name.to_string()));
                self.tokenizer.set_state(LexerState::ScriptData);
            }
            "plaintext" => self.tokenizer.set_state(LexerState::PlainText),
            _ => {}
        }
    }

    fn to_html_node(&self, id: usize) -> HtmlNode {
        match &self.nodes[id].data {
            BuilderNodeData::Element(element) => {
                let mut built = element.clone();
                built.children = self.nodes[id]
                    .children
                    .iter()
                    .map(|&child_id| self.to_html_node(child_id))
                    .collect();
                HtmlNode::Element(built)
            }
            BuilderNodeData::Text(text) => HtmlNode::Text(text.clone()),
            BuilderNodeData::Comment(comment) => HtmlNode::Comment(comment.clone()),
        }
    }
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
            | "frame"
    )
}

fn is_head_content_tag(tag: &str) -> bool {
    matches!(
        tag,
        "base"
            | "basefont"
            | "bgsound"
            | "link"
            | "meta"
            | "noframes"
            | "script"
            | "style"
            | "template"
            | "title"
    )
}

pub fn build_document(input: &str) -> HtmlDocument {
    build_document_with_errors(input).document
}

pub fn build_document_with_errors(input: &str) -> TreeBuildOutput {
    HtmlTreeBuilder::new(input).run()
}

pub fn build_fragment(input: &str, context_element: Option<&str>) -> Vec<HtmlNode> {
    build_fragment_with_errors(input, context_element).document.children
}

pub fn build_fragment_with_errors(input: &str, context_element: Option<&str>) -> TreeBuildOutput {
    let mut builder = HtmlTreeBuilder::new(input);
    if let Some(context) = context_element {
        builder.setup_fragment_mode(context);
    }
    builder.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_implicit_document_structure() {
        let output = build_document_with_errors("<div>Hello</div>");
        let html = match &output.document.children[0] {
            HtmlNode::Element(el) => el,
            _ => panic!("expected html element"),
        };
        assert_eq!(html.tag, "html");
    }

    #[test]
    fn parses_basic_fragment() {
        let nodes = build_fragment("<span>hello</span>", Some("div"));
        let span = match &nodes[0] {
            HtmlNode::Element(el) => el,
            _ => panic!("expected span"),
        };
        assert_eq!(span.tag, "span");
    }
}
