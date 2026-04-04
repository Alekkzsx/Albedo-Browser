use crate::ace::html::{
    lexer::LexerState, DoctypeToken, EndTagToken, FragmentContext, HtmlDocument, HtmlElement,
    HtmlNode, HtmlToken, HtmlTokenizer, Namespace, ParserOptions, PreloadRequest,
    PreloadScanner, ShadowRootMode, StartTagToken, TokenizerErrorSource,
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
    options: ParserOptions,
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
    frameset_element_id: Option<usize>,
    fragment_context: Option<FragmentContext>,
    fragment_root_id: Option<usize>,
    ignored_noscript_depth: usize,
}

impl<'a> HtmlTreeBuilder<'a> {
    pub fn new(input: &'a str) -> Self {
        Self::with_options(input, ParserOptions::default())
    }

    pub fn with_options(input: &'a str, options: ParserOptions) -> Self {
        let preload_scanner = if let Some(base_url) = options
            .base_url
            .clone()
            .or_else(|| options.source_url.clone())
        {
            PreloadScanner::with_base_url(base_url)
        } else {
            PreloadScanner::new()
        };

        Self {
            tokenizer: HtmlTokenizer::new(input),
            options,
            insertion_mode: InsertionMode::Initial,
            preload_scanner,
            preload_requests: Vec::new(),
            doctype: None,
            errors: Vec::new(),
            nodes: Vec::new(),
            root_children: Vec::new(),
            open_elements: Vec::new(),
            html_element_id: None,
            head_element_id: None,
            body_element_id: None,
            frameset_element_id: None,
            fragment_context: None,
            fragment_root_id: None,
            ignored_noscript_depth: 0,
        }
    }

    pub fn setup_fragment_mode(&mut self, context: &str) {
        self.setup_fragment_context(&FragmentContext::new(context));
    }

    pub fn setup_fragment_context(&mut self, context: &FragmentContext) {
        let normalized_tag_name = context.tag_name.to_ascii_lowercase();
        self.options.scripting_enabled = context.scripting_enabled;
        self.fragment_context = Some(FragmentContext {
            tag_name: normalized_tag_name.clone(),
            namespace: context.namespace,
            scripting_enabled: context.scripting_enabled,
        });

        let context_id = self.create_element(
            HtmlElement::with_namespace(
                adjust_tag_name_for_namespace(&normalized_tag_name, context.namespace),
                context.namespace,
            ),
            None,
        );
        self.fragment_root_id = Some(context_id);
        self.open_elements.push(context_id);
        self.insertion_mode = match normalized_tag_name.as_str() {
            "table" => InsertionMode::InTable,
            "tbody" | "thead" | "tfoot" => InsertionMode::InTableBody,
            "tr" => InsertionMode::InRow,
            "td" | "th" => InsertionMode::InCell,
            "select" => InsertionMode::InSelect,
            _ => InsertionMode::InBody,
        };

        match normalized_tag_name.as_str() {
            "title" | "textarea" => {
                self.tokenizer.set_raw_text_tag(Some(normalized_tag_name.clone()));
                self.tokenizer.set_state(LexerState::RcData);
            }
            "style" | "xmp" | "iframe" | "noembed" | "noframes" => {
                self.tokenizer.set_raw_text_tag(Some(normalized_tag_name.clone()));
                self.tokenizer.set_state(LexerState::RawText);
            }
            "script" => {
                self.tokenizer.set_raw_text_tag(Some(normalized_tag_name));
                self.tokenizer.set_state(LexerState::ScriptData);
            }
            "plaintext" => self.tokenizer.set_state(LexerState::PlainText),
            _ => {}
        }
    }

    pub fn run(mut self) -> TreeBuildOutput {
        self.speculate();

        loop {
            self.tokenizer
                .set_cdata_allowed(self.should_allow_cdata_section());
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
            if self.body_element_id.is_none() && self.frameset_element_id.is_none() {
                self.ensure_body_element();
            }
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

        if self.fragment_context_tag() == Some("select")
            && matches!(self.current_tag(), Some("select"))
        {
            self.errors.push(TreeBuilderError::new(
                TreeBuilderErrorKind::UnexpectedEof,
                self.insertion_mode,
                "unexpected EOF while select fragment remained open",
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
        if !self.options.collect_preloads {
            return;
        }

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
            if self.fragment_context_tag() == Some("script")
                && err.source == TokenizerErrorSource::Lexer
                && err.message.contains("EOF in script data")
            {
                continue;
            }

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

        if matches!(self.current_tag(), Some("head")) && !is_head_content_tag(&tag.name) && tag.name != "head" {
            self.open_elements.pop();
            self.insertion_mode = InsertionMode::AfterHead;
        }

        let insert_into_head = self.should_insert_into_head(&tag.name);
        if insert_into_head {
            self.focus_existing_head();
        }

        if self.body_element_id.is_none()
            && self.fragment_context.is_none()
            && !insert_into_head
            && self.find_open_element("template").is_none()
            && tag.name != "html"
            && tag.name != "head"
            && tag.name != "body"
            && tag.name != "frameset"
            && tag.name != "frame"
        {
            self.ensure_body_element();
        }

        self.close_optional_element_for_start(&tag.name);
        self.prepare_for_table_insertion(&tag.name);

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
                } else if self.body_element_id.is_none() && !matches!(self.current_tag(), Some("head")) {
                    self.focus_existing_head();
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
            "frameset" if self.fragment_context.is_none() => {
                let id = if let Some(existing) = self.frameset_element_id {
                    existing
                } else {
                    let id = self.insert_element_under_html(element);
                    self.frameset_element_id = Some(id);
                    id
                };
                self.body_element_id = None;
                self.open_elements.retain(|&open| Some(open) == self.html_element_id);
                self.open_elements.push(id);
                self.insertion_mode = InsertionMode::InFrameset;
            }
            "noscript" if self.options.scripting_enabled && self.fragment_context.is_none() => {
                self.ignored_noscript_depth += 1;
                self.tokenizer
                    .set_raw_text_tag(Some("noscript".to_string()));
                self.tokenizer.set_state(LexerState::RawText);
                self.insertion_mode = InsertionMode::InBody;
            }
            "template" => {
                let template_id = self.insert_element_at_current(element);
                let content_id =
                    self.create_and_attach(HtmlElement::new("template-content"), template_id);
                self.open_elements.push(template_id);
                self.open_elements.push(content_id);
                self.insertion_mode = InsertionMode::InTemplate;
            }
            _ => {
                if tag_name == "option" && matches!(self.current_tag(), Some("option")) {
                    self.open_elements.pop();
                }
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
                    "frameset" => InsertionMode::InFrameset,
                    _ => InsertionMode::InBody,
                };
            }
        }
    }

    fn handle_end_tag(&mut self, tag: EndTagToken) {
        if tag.name == "noscript" && self.ignored_noscript_depth > 0 {
            self.ignored_noscript_depth -= 1;
            self.reset_insertion_mode();
            return;
        }

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

        if self.fragment_context.is_none() && tag.name == "frameset" {
            self.close_until("frameset");
            self.insertion_mode = InsertionMode::AfterFrameset;
            return;
        }

        if is_formatting_element(&tag.name) && self.close_formatting_element_with_reopen(&tag.name) {
            self.reset_insertion_mode();
            return;
        }

        if !self.close_until(&tag.name) {
            self.errors.push(TreeBuilderError::new(
                TreeBuilderErrorKind::UnexpectedEndTag,
                self.insertion_mode,
                format!("unexpected end tag </{}>", tag.name),
            ));
        }

        self.reset_insertion_mode();
    }

    fn handle_text(&mut self, text: String) {
        if text.is_empty() {
            return;
        }

        if self.ignored_noscript_depth > 0 {
            return;
        }

        if self.fragment_context.is_none() && self.current_parent_id().is_none() {
            self.ensure_body_element();
        }

        if self.should_foster_parent_text() && !text.trim().is_empty() {
            self.errors.push(TreeBuilderError::new(
                TreeBuilderErrorKind::FosterParenting,
                self.insertion_mode,
                "text foster-parented outside table",
            ));
            if self.insert_foster_parented_text(text.clone()) {
                return;
            }
        }

        if let Some(parent_id) = self.current_parent_id() {
            self.append_text_to_parent(parent_id, text);
        } else {
            self.append_text_to_root(text);
        }
    }

    fn handle_comment(&mut self, comment: String) {
        if self.ignored_noscript_depth > 0 {
            return;
        }

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
        let mut element = HtmlElement::with_namespace(
            adjust_tag_name_for_namespace(&tag.name, namespace),
            namespace,
        );
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

    fn determine_namespace(&self, tag_name: &str) -> Namespace {
        if tag_name == "svg" {
            return Namespace::Svg;
        }
        if tag_name == "math" {
            return Namespace::MathMl;
        }

        let Some(parent_id) = self.current_parent_id() else {
            return Namespace::Html;
        };
        match &self.nodes[parent_id].data {
            BuilderNodeData::Element(parent) => match parent.namespace {
                Namespace::Svg if !is_svg_html_integration_point(&parent.tag) => Namespace::Svg,
                Namespace::MathMl => Namespace::MathMl,
                _ => Namespace::Html,
            },
            BuilderNodeData::Text(_) | BuilderNodeData::Comment(_) => Namespace::Html,
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

    fn create_element(&mut self, element: HtmlElement, _parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(BuilderNode {
            data: BuilderNodeData::Element(element),
            parent: _parent,
            children: Vec::new(),
        });
        id
    }

    fn create_text_node(&mut self, text: String, _parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(BuilderNode {
            data: BuilderNodeData::Text(text),
            parent: _parent,
            children: Vec::new(),
        });
        id
    }

    fn create_comment_node(&mut self, comment: String, _parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(BuilderNode {
            data: BuilderNodeData::Comment(comment),
            parent: _parent,
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

    fn fragment_context_tag(&self) -> Option<&str> {
        self.fragment_context
            .as_ref()
            .map(|context| context.tag_name.as_str())
    }

    fn should_allow_cdata_section(&self) -> bool {
        let Some(parent_id) = self.current_parent_id() else {
            return matches!(
                self.fragment_context.as_ref().map(|context| context.namespace),
                Some(Namespace::Svg | Namespace::MathMl)
            );
        };

        matches!(
            &self.nodes[parent_id].data,
            BuilderNodeData::Element(element)
                if matches!(element.namespace, Namespace::Svg | Namespace::MathMl)
        )
    }

    fn should_insert_into_head(&self, tag_name: &str) -> bool {
        self.fragment_context.is_none()
            && self.body_element_id.is_none()
            && self.frameset_element_id.is_none()
            && self.head_element_id.is_some()
            && self.find_open_element("template").is_none()
            && is_head_content_tag(tag_name)
    }

    fn focus_existing_head(&mut self) {
        let Some(head_id) = self.head_element_id else {
            return;
        };

        self.open_elements
            .retain(|&id| Some(id) == self.html_element_id || id == head_id);
        if self.open_elements.last().copied() != Some(head_id) {
            self.open_elements.push(head_id);
        }
        self.insertion_mode = InsertionMode::InHead;
    }

    fn append_text_to_parent(&mut self, parent_id: usize, text: String) {
        let last_child = self.nodes[parent_id].children.last().copied();
        if let Some(last_child_id) = last_child {
            if let BuilderNodeData::Text(existing) = &mut self.nodes[last_child_id].data {
                existing.push_str(&text);
                return;
            }
        }

        let text_id = self.create_text_node(text, Some(parent_id));
        self.nodes[parent_id].children.push(text_id);
    }

    fn append_text_to_root(&mut self, text: String) {
        let last_root = self.root_children.last().copied();
        if let Some(last_root_id) = last_root {
            if let BuilderNodeData::Text(existing) = &mut self.nodes[last_root_id].data {
                existing.push_str(&text);
                return;
            }
        }

        let text_id = self.create_text_node(text, None);
        self.root_children.push(text_id);
    }

    fn close_until(&mut self, tag_name: &str) -> bool {
        while let Some(id) = self.open_elements.pop() {
            if let BuilderNodeData::Element(element) = &self.nodes[id].data {
                if element.tag.eq_ignore_ascii_case(tag_name) {
                    return true;
                }
            }
        }
        false
    }

    fn close_optional_element_for_start(&mut self, tag_name: &str) {
        match tag_name {
            "li" => {
                self.pop_open_element("li");
            }
            "dd" | "dt" => {
                if !self.pop_open_element("dd") {
                    self.pop_open_element("dt");
                }
            }
            "p" => {
                self.pop_open_element("p");
            }
            "option" => {
                self.pop_open_element("option");
            }
            _ => {}
        }
    }

    fn prepare_for_table_insertion(&mut self, tag_name: &str) {
        match (self.current_tag(), tag_name) {
            (Some("table"), "tr" | "td" | "th") => {
                let tbody_id = self.insert_element_at_current(HtmlElement::new("tbody"));
                self.open_elements.push(tbody_id);
                self.insertion_mode = InsertionMode::InTableBody;
                if matches!(tag_name, "td" | "th") {
                    let tr_id = self.insert_element_at_current(HtmlElement::new("tr"));
                    self.open_elements.push(tr_id);
                    self.insertion_mode = InsertionMode::InRow;
                }
            }
            (Some("tbody" | "thead" | "tfoot"), "td" | "th") => {
                let tr_id = self.insert_element_at_current(HtmlElement::new("tr"));
                self.open_elements.push(tr_id);
                self.insertion_mode = InsertionMode::InRow;
            }
            _ => {}
        }
    }

    fn pop_open_element(&mut self, tag_name: &str) -> bool {
        let Some(match_pos) = self.open_elements.iter().rposition(|&id| {
            matches!(
                &self.nodes[id].data,
                BuilderNodeData::Element(element) if element.tag.eq_ignore_ascii_case(tag_name)
            )
        }) else {
            return false;
        };

        self.open_elements.truncate(match_pos);
        true
    }

    fn close_formatting_element_with_reopen(&mut self, tag_name: &str) -> bool {
        let Some(match_pos) = self.open_elements.iter().rposition(|&id| {
            matches!(
                &self.nodes[id].data,
                BuilderNodeData::Element(element) if element.tag.eq_ignore_ascii_case(tag_name)
            )
        }) else {
            return false;
        };

        let target_id = self.open_elements[match_pos];
        let reopen = self.open_elements[match_pos + 1..]
            .iter()
            .filter_map(|&id| match &self.nodes[id].data {
                BuilderNodeData::Element(element) if is_formatting_element(&element.tag) => {
                    Some(element.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        while let Some(id) = self.open_elements.pop() {
            if id == target_id {
                break;
            }
        }

        for element in reopen {
            let reopened_id = self.insert_element_at_current(element);
            self.open_elements.push(reopened_id);
        }

        true
    }

    fn insert_foster_parented_text(&mut self, text: String) -> bool {
        let Some(table_id) = self.find_open_element("table") else {
            if let Some(body_id) = self.body_element_id {
                let text_id = self.create_text_node(text, Some(body_id));
                self.nodes[body_id].children.push(text_id);
                return true;
            }
            return false;
        };

        let parent_id = self.nodes[table_id].parent;
        let text_id = self.create_text_node(text, parent_id);

        if let Some(parent_id) = parent_id {
            let siblings = &mut self.nodes[parent_id].children;
            if let Some(pos) = siblings.iter().position(|&id| id == table_id) {
                siblings.insert(pos, text_id);
            } else {
                siblings.push(text_id);
            }
        } else if let Some(pos) = self.root_children.iter().position(|&id| id == table_id) {
            self.root_children.insert(pos, text_id);
        } else {
            self.root_children.push(text_id);
        }

        true
    }

    fn find_open_element(&self, tag_name: &str) -> Option<usize> {
        self.open_elements.iter().rev().copied().find(|&id| {
            matches!(
                &self.nodes[id].data,
                BuilderNodeData::Element(element) if element.tag.eq_ignore_ascii_case(tag_name)
            )
        })
    }

    fn reset_insertion_mode(&mut self) {
        self.insertion_mode = match self.current_tag() {
            Some("head") => InsertionMode::InHead,
            Some("frameset") => InsertionMode::InFrameset,
            Some("select") => InsertionMode::InSelect,
            Some("td" | "th") => InsertionMode::InCell,
            Some("tr") => InsertionMode::InRow,
            Some("tbody" | "thead" | "tfoot") => InsertionMode::InTableBody,
            Some("table") => InsertionMode::InTable,
            Some("body") => InsertionMode::InBody,
            Some("html") if self.frameset_element_id.is_some() => InsertionMode::AfterFrameset,
            Some("html") if self.body_element_id.is_some() => InsertionMode::AfterBody,
            Some("html") => InsertionMode::AfterHead,
            _ => InsertionMode::InBody,
        };
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
            "noscript" if !self.options.scripting_enabled => {
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

fn is_formatting_element(tag: &str) -> bool {
    matches!(
        tag,
        "a" | "b" | "big" | "code" | "em" | "font" | "i" | "s" | "small" | "span"
            | "strike" | "strong" | "tt" | "u"
    )
}

fn is_svg_html_integration_point(tag: &str) -> bool {
    tag.eq_ignore_ascii_case("foreignObject")
}

fn adjust_tag_name_for_namespace(tag_name: &str, namespace: Namespace) -> String {
    match namespace {
        Namespace::Svg if tag_name.eq_ignore_ascii_case("foreignobject") => {
            "foreignObject".to_string()
        }
        _ => tag_name.to_string(),
    }
}

pub fn build_document(input: &str) -> HtmlDocument {
    build_document_with_errors(input).document
}

pub fn build_document_with_errors(input: &str) -> TreeBuildOutput {
    build_document_with_errors_and_options(input, &ParserOptions::default())
}

pub fn build_document_with_errors_and_options(
    input: &str,
    options: &ParserOptions,
) -> TreeBuildOutput {
    HtmlTreeBuilder::with_options(input, options.clone()).run()
}

pub fn build_fragment(input: &str, context_element: Option<&str>) -> Vec<HtmlNode> {
    build_fragment_with_errors(input, context_element).document.children
}

pub fn build_fragment_with_errors(input: &str, context_element: Option<&str>) -> TreeBuildOutput {
    build_fragment_with_errors_and_options(input, context_element, &ParserOptions::default())
}

pub fn build_fragment_with_errors_and_options(
    input: &str,
    context_element: Option<&str>,
    options: &ParserOptions,
) -> TreeBuildOutput {
    let context = context_element.map(FragmentContext::new);
    build_fragment_with_context_and_options(input, context.as_ref(), options)
}

pub fn build_fragment_with_context_and_options(
    input: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> TreeBuildOutput {
    let mut builder = HtmlTreeBuilder::with_options(input, options.clone());
    if let Some(context) = context {
        builder.setup_fragment_context(context);
    }
    builder.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serialize_nodes(nodes: &[HtmlNode]) -> String {
        fn walk(node: &HtmlNode, indent: usize, out: &mut String) {
            let prefix = format!("| {}", "  ".repeat(indent));
            match node {
                HtmlNode::Element(el) => {
                    let ns_prefix = match el.namespace {
                        crate::ace::html::Namespace::Html => "",
                        crate::ace::html::Namespace::Svg => "svg ",
                        crate::ace::html::Namespace::MathMl => "math ",
                    };
                    out.push_str(&format!("{}<{}{}>\n", prefix, ns_prefix, el.tag));
                    let mut attrs: Vec<_> = el.attributes.iter().collect();
                    attrs.sort_by(|a, b| a.0.cmp(b.0));
                    for (name, value) in attrs {
                        out.push_str(&format!("{}  {}=\"{}\"\n", prefix, name, value));
                    }
                    for child in &el.children {
                        walk(child, indent + 1, out);
                    }
                }
                HtmlNode::Text(text) => out.push_str(&format!("{}\"{}\"\n", prefix, text)),
                HtmlNode::Comment(text) => out.push_str(&format!("{}<!-- {} -->\n", prefix, text)),
            }
        }

        let mut out = String::new();
        for node in nodes {
            walk(node, 0, &mut out);
        }
        out
    }

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

    #[test]
    fn foster_parents_text_before_table() {
        let output = build_document_with_errors("<table>hello<tr><td>cell</td></tr></table>");
        let tree = serialize_nodes(&output.document.children);
        assert!(tree.contains("|     \"hello\"\n|     <table>"));
    }

    #[test]
    fn reopens_formatting_after_misnested_end_tag() {
        let output = build_document_with_errors("<p><b><i>x</b>y</i></p>");
        let tree = serialize_nodes(&output.document.children);
        assert!(tree.contains("|       <b>\n|         <i>\n|           \"x\""));
        assert!(tree.contains("|       <i>\n|         \"y\""));
    }

    #[test]
    fn keeps_frameset_outside_body() {
        let output = build_document_with_errors("<html><frameset><frame src=\"a\"></frameset></html>");
        let tree = serialize_nodes(&output.document.children);
        assert!(tree.contains("|   <frameset>\n|     <frame>\n|       src=\"a\""));
        assert!(!tree.contains("|   <body>\n|     <frameset>"));
    }

    #[test]
    fn select_fragment_reports_unexpected_eof() {
        let output = build_fragment_with_errors("<option>yes</option>", Some("select"));
        assert!(output
            .errors
            .iter()
            .any(|error| error.kind == TreeBuilderErrorKind::UnexpectedEof));
    }

    #[test]
    fn reuses_implicit_head_for_explicit_head_tag() {
        let output = build_document_with_errors("<html><head></head><body></body></html>");
        assert!(output.errors.is_empty());
    }

    #[test]
    fn keeps_template_contents_under_head() {
        let output = build_document_with_errors("<template><div>content</div></template>");
        let tree = serialize_nodes(&output.document.children);
        assert!(tree.contains("|   <head>\n|     <template>\n|       <template-content>\n|         <div>\n|           \"content\""));
        assert!(tree.contains("|   <body>\n"));
    }

    #[test]
    fn merges_adjacent_script_fragment_text_nodes() {
        let output = build_fragment_with_errors("var x = '<div>';", Some("script"));
        let tree = serialize_nodes(&output.document.children);
        assert_eq!(tree, "| \"var x = '<div>';\"\n");
        assert!(output.errors.is_empty());
    }

    #[test]
    fn ignores_noscript_contents_when_scripting_is_enabled() {
        let output = build_document_with_errors_and_options(
            "<noscript>JS disabled</noscript>",
            &ParserOptions::default(),
        );
        let tree = serialize_nodes(&output.document.children);
        assert_eq!(tree, "| <html>\n|   <head>\n|   <body>\n");
    }

    #[test]
    fn allows_cdata_in_svg_fragment_context() {
        let context = FragmentContext::new("svg").with_namespace(Namespace::Svg);
        let output = build_fragment_with_context_and_options(
            "<![CDATA[test data]]>",
            Some(&context),
            &ParserOptions::default(),
        );
        let tree = serialize_nodes(&output.document.children);
        assert_eq!(tree, "| \"test data\"\n");
        assert!(output.errors.is_empty());
    }
}
