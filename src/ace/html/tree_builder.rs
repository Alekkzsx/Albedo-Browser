use crate::ace::html::{
    lexer::LexerState, DoctypeToken, EndTagToken, FragmentContext, HtmlDocument, HtmlElement,
    HtmlNode, HtmlTokenKind, HtmlTokenizer, Namespace, ParserOptions, PreloadRequest,
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
            line: 0, // Placeholder, usually updated with with_pos or from context
            column: 0,
        }
    }

    pub fn with_pos(mut self, line: usize, col: usize) -> Self {
        self.line = line;
        self.column = col;
        self
    }
}

#[derive(Debug)]
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

/// Active Formatting Elements list entry (spec §13.2.4.3).
#[derive(Clone, Debug)]
// TECH_DEBT: Marker is constructed by push_afe_marker() which is called in
// insertion mode handlers being implemented in subtask 1.8.
#[allow(dead_code)]
enum ActiveFormattingElement {
    /// Scope marker — inserted at boundaries (body, object, marquee, td, th, caption).
    Marker,
    /// A formatting element currently in the list, identified by its node id.
    Element(usize),
}

#[derive(Clone, Debug)]
struct BuilderNode {
    data: BuilderNodeData,
    parent: Option<usize>,
    children: Vec<usize>,
}

pub struct HtmlTreeBuilder {
    tokenizer: HtmlTokenizer,
    options: ParserOptions,
    insertion_mode: InsertionMode,
    /// Stack of original insertion modes used by template elements (spec §13.2.4.1).
    template_insertion_modes: Vec<InsertionMode>,
    preload_scanner: PreloadScanner,
    preload_requests: Vec<PreloadRequest>,
    doctype: Option<DoctypeToken>,
    errors: Vec<TreeBuilderError>,
    nodes: Vec<BuilderNode>,
    root_children: Vec<usize>,
    open_elements: Vec<usize>,
    /// Active formatting elements list (spec §13.2.4.3).
    active_formatting_elements: Vec<ActiveFormattingElement>,
    html_element_id: Option<usize>,
    head_element_id: Option<usize>,
    body_element_id: Option<usize>,
    frameset_element_id: Option<usize>,
    /// frameset-ok flag (spec §13.2.4.1).
    // TECH_DEBT: read in frameset handling (subtask 1.8).
    #[allow(dead_code)]
    frameset_ok: bool,
    fragment_context: Option<FragmentContext>,
    fragment_root_id: Option<usize>,
    ignored_noscript_depth: usize,
    current_token_line: usize,
    current_token_column: usize,
}

impl HtmlTreeBuilder {
    pub fn new(input: &str) -> Self {
        Self::with_options(input, ParserOptions::default())
    }

    pub fn empty() -> Self {
        Self::with_options_empty(ParserOptions::default())
    }

    pub fn with_options(input: &str, options: ParserOptions) -> Self {
        let preload_scanner = if let Some(base_url) = options
            .base_url
            .clone()
            .or_else(|| options.source_url.clone())
        {
            PreloadScanner::with_base_url(base_url)
        } else {
            PreloadScanner::new()
        };

        let mut builder = Self {
            tokenizer: HtmlTokenizer::new(input),
            options,
            insertion_mode: InsertionMode::Initial,
            template_insertion_modes: Vec::new(),
            preload_scanner,
            preload_requests: Vec::new(),
            doctype: None,
            errors: Vec::new(),
            nodes: Vec::new(),
            root_children: Vec::new(),
            open_elements: Vec::new(),
            active_formatting_elements: Vec::new(),
            html_element_id: None,
            head_element_id: None,
            body_element_id: None,
            frameset_element_id: None,
            frameset_ok: true,
            fragment_context: None,
            fragment_root_id: None,
            ignored_noscript_depth: 0,
            current_token_line: 1,
            current_token_column: 1,
        };
        builder.process_tokens();
        builder
    }

    pub fn with_options_empty(options: ParserOptions) -> Self {
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
            tokenizer: HtmlTokenizer::empty(),
            options,
            insertion_mode: InsertionMode::Initial,
            template_insertion_modes: Vec::new(),
            preload_scanner,
            preload_requests: Vec::new(),
            doctype: None,
            errors: Vec::new(),
            nodes: Vec::new(),
            root_children: Vec::new(),
            open_elements: Vec::new(),
            active_formatting_elements: Vec::new(),
            html_element_id: None,
            head_element_id: None,
            body_element_id: None,
            frameset_element_id: None,
            frameset_ok: true,
            fragment_context: None,
            fragment_root_id: None,
            ignored_noscript_depth: 0,
            current_token_line: 1,
            current_token_column: 1,
        }
    }

    pub fn feed(&mut self, input: &str) {
        self.tokenizer.feed(input);
        self.process_tokens();
    }

    pub fn end(&mut self) {
        self.tokenizer.end();
        self.process_tokens();
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
        
        // If context is a template element, push "in template" onto the stack (spec §13.4)
        if normalized_tag_name == "template" {
            self.template_insertion_modes.push(InsertionMode::InTemplate);
        }
        
        self.insertion_mode = match normalized_tag_name.as_str() {
            "table" => InsertionMode::InTable,
            "tbody" | "thead" | "tfoot" => InsertionMode::InTableBody,
            "tr" => InsertionMode::InRow,
            "td" | "th" => InsertionMode::InCell,
            "select" => InsertionMode::InSelect,
            "template" => InsertionMode::InTemplate,
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
        self.process_tokens();
        self.finish()
    }

    pub fn process_tokens(&mut self) {
        self.speculate();

        while let Some(token) = self.tokenizer.next_token() {
            self.tokenizer
                .set_cdata_allowed(self.should_allow_cdata_section());
            self.collect_tokenizer_errors();

            self.current_token_line = token.line;
            self.current_token_column = token.column;

            let is_eof = matches!(token.kind, HtmlTokenKind::Eof);

            match token.kind {
                HtmlTokenKind::Eof => {}
                HtmlTokenKind::Doctype(dt) => self.handle_doctype(dt),
                HtmlTokenKind::StartTag(tag) => self.handle_start_tag(tag),
                HtmlTokenKind::EndTag(tag) => self.handle_end_tag(tag),
                HtmlTokenKind::Character(text) => self.handle_text(text.data),
                HtmlTokenKind::Comment(comment) => self.handle_comment(comment.data),
            }

            if is_eof {
                break;
            }
        }
    }

    pub fn finish(mut self) -> TreeBuildOutput {
        self.process_tokens();

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
                doctype: self.doctype.clone(),
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
        if self.fragment_context.is_some()
            || self.doctype.is_some()
            || self.html_element_id.is_some()
        {
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

        // Handle AfterAfterBody and AfterAfterFrameset modes
        if self.fragment_context.is_none() {
            match self.insertion_mode {
                InsertionMode::AfterAfterBody => {
                    if tag.name != "html" {
                        self.errors.push(
                            TreeBuilderError::new(
                                TreeBuilderErrorKind::UnexpectedToken,
                                self.insertion_mode,
                                format!("unexpected start tag <{}> in AfterAfterBody mode", tag.name),
                            )
                            .with_pos(self.current_token_line, self.current_token_column),
                        );
                        self.insertion_mode = InsertionMode::InBody;
                        // Reprocess token
                        return self.handle_start_tag(tag);
                    }
                }
                InsertionMode::AfterAfterFrameset => {
                    if !matches!(tag.name.as_str(), "html" | "noframes") {
                        self.errors.push(
                            TreeBuilderError::new(
                                TreeBuilderErrorKind::UnexpectedToken,
                                self.insertion_mode,
                                format!("unexpected start tag <{}> in AfterAfterFrameset mode", tag.name),
                            )
                            .with_pos(self.current_token_line, self.current_token_column),
                        );
                        return;
                    }
                }
                InsertionMode::AfterBody => {
                    if !matches!(tag.name.as_str(), "html" | "body" | "frameset") {
                        self.ensure_body_element();
                    }
                }
                _ => {}
            }
        }

        // Handle InFrameset mode (spec §13.2.6.4.18)
        if matches!(self.insertion_mode, InsertionMode::InFrameset) {
            if self.process_in_frameset_mode(&tag) {
                return;
            }
            // If process_in_frameset_mode returns false, continue with normal processing
        }

        // Handle AfterFrameset mode (spec §13.2.6.4.19)
        if matches!(self.insertion_mode, InsertionMode::AfterFrameset) {
            if self.process_after_frameset_mode(&tag) {
                return;
            }
            // If process_after_frameset_mode returns false, continue with normal processing
        }

        self.maybe_exit_foreign_content_for_start_tag(&tag.name);

        if matches!(self.insertion_mode, InsertionMode::InSelectInTable)
            && matches!(
                tag.name.as_str(),
                "caption" | "table" | "tbody" | "tfoot" | "thead" | "tr" | "td" | "th"
            )
        {
            self.close_until("select");
            self.reset_insertion_mode();
            self.maybe_exit_foreign_content_for_start_tag(&tag.name);
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
            && !(tag.name == "noframes" && matches!(self.insertion_mode, InsertionMode::InFrameset))
        {
            self.ensure_body_element();
        }

        if matches!(self.insertion_mode, InsertionMode::InBody | InsertionMode::InCell)
            && !matches!(
                tag.name.as_str(),
                "html" | "head" | "body" | "frameset" | "template"
            )
        {
            self.reconstruct_active_formatting_elements();
        }

        if self.fragment_context.is_none()
            && tag.name == "frame"
            && !matches!(self.insertion_mode, InsertionMode::InFrameset)
        {
            self.errors.push(
                TreeBuilderError::new(
                    TreeBuilderErrorKind::UnexpectedToken,
                    self.insertion_mode,
                    "ignored <frame> outside frameset",
                )
                .with_pos(self.current_token_line, self.current_token_column),
            );
            return;
        }

        self.close_optional_element_for_start(&tag.name);
        self.prepare_for_table_insertion(&tag.name);

        if matches!(self.insertion_mode, InsertionMode::InTable)
            && should_foster_parent_start_tag_in_table_mode(&tag.name)
        {
            let tag_name = tag.name.clone();
            let element_id = self.insert_element_before_open_table(self.make_element(&mut tag));
            let should_push = match self.node_namespace(element_id) {
                Namespace::Html => !is_void_element(&tag_name),
                _ => !tag.self_closing,
            };
            if should_push {
                self.open_elements.push(element_id);
            }
            self.update_tokenizer_state_for_tag(&tag_name);
            self.insertion_mode = InsertionMode::InBody;
            return;
        }

        if tag.name == "select"
            && matches!(
                self.insertion_mode,
                InsertionMode::InTable
                    | InsertionMode::InCaption
                    | InsertionMode::InTableBody
                    | InsertionMode::InRow
                    | InsertionMode::InCell
            )
        {
            let select_id = self.insert_element_before_open_table(self.make_element(&mut tag));
            self.open_elements.push(select_id);
            self.insertion_mode = InsertionMode::InSelectInTable;
            return;
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
                if matches!(
                    self.insertion_mode,
                    InsertionMode::AfterBody | InsertionMode::AfterAfterBody
                ) {
                    self.errors.push(
                        TreeBuilderError::new(
                            TreeBuilderErrorKind::UnexpectedToken,
                            self.insertion_mode,
                            "ignored <frameset> after body",
                        )
                        .with_pos(self.current_token_line, self.current_token_column),
                    );
                    return;
                }
                let id = if self.frameset_element_id.is_none() {
                    let id = self.insert_element_under_html(element);
                    self.frameset_element_id = Some(id);
                    id
                } else {
                    self.insert_element_at_current(element)
                };
                if self.frameset_element_id == Some(id) {
                    self.body_element_id = None;
                    self.open_elements.retain(|&open| Some(open) == self.html_element_id);
                }
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
                self.template_insertion_modes.push(self.insertion_mode);
                self.push_afe_marker();
                self.open_elements.push(template_id);
                self.open_elements.push(content_id);
                self.insertion_mode = InsertionMode::InTemplate;
            }
            "noframes" if matches!(self.insertion_mode, InsertionMode::InFrameset | InsertionMode::AfterFrameset | InsertionMode::AfterAfterFrameset) => {
                // In frameset contexts, noframes is handled as raw text
                let id = self.insert_element_at_current(element);
                self.open_elements.push(id);
                self.tokenizer.set_raw_text_tag(Some("noframes".to_string()));
                self.tokenizer.set_state(LexerState::RawText);
            }
            _ => {
                if tag_name == "option" && matches!(self.current_tag(), Some("option")) {
                    self.open_elements.pop();
                }
                let id = self.insert_element_at_current(element);
                let should_push = match self.node_namespace(id) {
                    Namespace::Html => !is_void_element(&tag_name),
                    _ => !tag.self_closing,
                };

                if should_push {
                    self.open_elements.push(id);
                    // If this is a formatting element, push it onto the AFE list
                    // so the Adoption Agency Algorithm can find it during end-tag processing.
                    // (spec §13.2.4.3 — "push onto the list of active formatting elements")
                    if is_formatting_element(&tag_name) {
                        self.push_active_formatting_element(id);
                    }
                    if matches!(tag_name.as_str(), "caption" | "td" | "th" | "object" | "marquee") {
                        self.push_afe_marker();
                    }
                } else if tag.self_closing && matches!(self.node_namespace(id), Namespace::Html) {
                    println!("TBD: emit parse error for self-closing non-void HTML element");
                }
                self.update_tokenizer_state_for_tag(&tag_name);
                self.insertion_mode = match tag_name.as_str() {
                    "table" => InsertionMode::InTable,
                    "colgroup" => InsertionMode::InColumnGroup,
                    "tbody" | "thead" | "tfoot" => InsertionMode::InTableBody,
                    "tr" => InsertionMode::InRow,
                    "caption" => InsertionMode::InCaption,
                    "td" | "th" => InsertionMode::InCell,
                    "col" if matches!(self.insertion_mode, InsertionMode::InColumnGroup) => {
                        InsertionMode::InColumnGroup
                    }
                    "select" if matches!(
                        self.insertion_mode,
                        InsertionMode::InTable
                            | InsertionMode::InCaption
                            | InsertionMode::InTableBody
                            | InsertionMode::InRow
                            | InsertionMode::InCell
                    ) => InsertionMode::InSelectInTable,
                    "select" => InsertionMode::InSelect,
                    _
                        if matches!(
                            self.insertion_mode,
                            InsertionMode::InSelect | InsertionMode::InSelectInTable
                        ) =>
                    {
                        self.insertion_mode
                    }
                    "frameset" => InsertionMode::InFrameset,
                    _ if matches!(self.insertion_mode, InsertionMode::InTemplate) => {
                        // In template mode, handle specific tags that change mode (spec §13.2.6.4.16)
                        if let Some(new_mode) = self.process_in_template_start_tag(&tag_name) {
                            new_mode
                        } else {
                            self.insertion_mode
                        }
                    }
                    _ => InsertionMode::InBody,
                };
            }
        }
    }

    fn handle_end_tag(&mut self, tag: EndTagToken) {
        let ln = self.current_token_line;
        let col = self.current_token_column;

        if tag.name == "noscript" && self.ignored_noscript_depth > 0 {
            self.ignored_noscript_depth -= 1;
            self.reset_insertion_mode();
            return;
        }

        // ── InFrameset mode handling (spec §13.2.6.4.18) ──────────────────────
        if matches!(self.insertion_mode, InsertionMode::InFrameset) {
            if self.process_in_frameset_end_tag(&tag.name) {
                return;
            }
        }

        // ── AfterFrameset mode handling (spec §13.2.6.4.19) ───────────────────
        if matches!(self.insertion_mode, InsertionMode::AfterFrameset) {
            if self.process_after_frameset_end_tag(&tag.name) {
                return;
            }
        }

        // ── AfterAfterFrameset mode handling (spec §13.2.6.4.21) ──────────────
        if matches!(self.insertion_mode, InsertionMode::AfterAfterFrameset) {
            // Any end tag → parse error, ignore
            self.errors.push(
                TreeBuilderError::new(
                    TreeBuilderErrorKind::UnexpectedEndTag,
                    self.insertion_mode,
                    format!("unexpected end tag </{tag_name}> in after after frameset mode", tag_name = tag.name),
                )
                .with_pos(ln, col),
            );
            return;
        }

        // ── InTemplate mode handling (spec §13.2.6.4.16) ─────────────────────
        if matches!(self.insertion_mode, InsertionMode::InTemplate) {
            // Only </template> is valid in InTemplate mode
            if tag.name != "template" {
                // Parse error: ignore any other end tag
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        format!("unexpected end tag </{tag_name}> in template mode", tag_name = tag.name),
                    )
                    .with_pos(ln, col),
                );
                return;
            }
            // </template> will be handled below
        }

        // ── </head> ──────────────────────────────────────────────────────────
        if self.fragment_context.is_none() && tag.name == "head" {
            if matches!(self.current_tag(), Some("head")) {
                self.open_elements.pop();
            }
            self.insertion_mode = InsertionMode::AfterHead;
            return;
        }

        // ── </p> — spec §13.2.6.4.7 ─────────────────────────────────────────
        if tag.name == "p" {
            if !self.has_p_in_button_scope() {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "end tag </p> but no <p> in button scope",
                    )
                    .with_pos(ln, col),
                );
                // Insert and immediately pop a synthetic <p> element.
                let p_id = self.insert_element_at_current(HtmlElement::new("p"));
                // (do not push to open_elements — it ends immediately)
                let _ = p_id;
            } else {
                self.generate_implied_end_tags(Some("p"));
                if !matches!(self.current_tag(), Some("p")) {
                    self.errors.push(
                        TreeBuilderError::new(
                            TreeBuilderErrorKind::UnexpectedEndTag,
                            self.insertion_mode,
                            "end tag </p> did not match current node",
                        )
                        .with_pos(ln, col),
                    );
                }
                self.close_until("p");
            }
            self.reset_insertion_mode();
            return;
        }

        // ── </li> — spec §13.2.6.4.7 ────────────────────────────────────────
        if tag.name == "li" {
            if !self.has_element_in_list_item_scope("li") {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "end tag </li> but no <li> in list-item scope",
                    )
                    .with_pos(ln, col),
                );
            } else {
                self.generate_implied_end_tags(Some("li"));
                if !matches!(self.current_tag(), Some("li")) {
                    self.errors.push(
                        TreeBuilderError::new(
                            TreeBuilderErrorKind::UnexpectedEndTag,
                            self.insertion_mode,
                            "end tag </li> did not match current node",
                        )
                        .with_pos(ln, col),
                    );
                }
                self.close_until("li");
            }
            self.reset_insertion_mode();
            return;
        }

        // ── </dd> / </dt> — spec §13.2.6.4.7 ────────────────────────────────
        if matches!(tag.name.as_str(), "dd" | "dt") {
            if !self.has_element_in_scope(&tag.name) {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        format!("end tag </{tag_name}> but not in scope", tag_name = tag.name),
                    )
                    .with_pos(ln, col),
                );
            } else {
                self.generate_implied_end_tags(Some(&tag.name));
                if self.current_tag().map_or(true, |t| !t.eq_ignore_ascii_case(&tag.name)) {
                    self.errors.push(
                        TreeBuilderError::new(
                            TreeBuilderErrorKind::UnexpectedEndTag,
                            self.insertion_mode,
                            format!("end tag </{tag_name}> did not match current node", tag_name = tag.name),
                        )
                        .with_pos(ln, col),
                    );
                }
                self.close_until(&tag.name);
            }
            self.reset_insertion_mode();
            return;
        }

        // ── </h1> … </h6> — spec §13.2.6.4.7 ───────────────────────────────
                // In-body block/container end tags.
        if matches!(
            tag.name.as_str(),
            "address"
                | "article"
                | "aside"
                | "blockquote"
                | "button"
                | "center"
                | "details"
                | "dialog"
                | "dir"
                | "div"
                | "dl"
                | "fieldset"
                | "figcaption"
                | "figure"
                | "footer"
                | "header"
                | "hgroup"
                | "main"
                | "menu"
                | "nav"
                | "ol"
                | "section"
                | "summary"
                | "ul"
        ) {
            if !self.has_element_in_scope(&tag.name) {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        format!("end tag </{tag_name}> but not in scope", tag_name = tag.name),
                    )
                    .with_pos(ln, col),
                );
            } else {
                self.generate_implied_end_tags(None);
                if self.current_tag().map_or(true, |current| !current.eq_ignore_ascii_case(&tag.name))
                {
                    self.errors.push(
                        TreeBuilderError::new(
                            TreeBuilderErrorKind::UnexpectedEndTag,
                            self.insertion_mode,
                            format!(
                                "end tag </{tag_name}> did not match current node",
                                tag_name = tag.name
                            ),
                        )
                        .with_pos(ln, col),
                    );
                }
                self.close_until(&tag.name);
            }
            self.reset_insertion_mode();
            return;
        }
        if matches!(tag.name.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
            // Check if any heading is in scope.
            let any_heading_in_scope = ["h1", "h2", "h3", "h4", "h5", "h6"]
                .iter()
                .any(|h| self.has_element_in_scope(h));
            if !any_heading_in_scope {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        format!("end tag </{tag_name}> but no heading in scope", tag_name = tag.name),
                    )
                    .with_pos(ln, col),
                );
            } else {
                self.generate_implied_end_tags(None);
                let cur = self.current_tag().map(str::to_string);
                if cur.as_deref().map_or(true, |t| !t.eq_ignore_ascii_case(&tag.name)) {
                    self.errors.push(
                        TreeBuilderError::new(
                            TreeBuilderErrorKind::UnexpectedEndTag,
                            self.insertion_mode,
                            format!("end tag </{tag_name}> did not match current node", tag_name = tag.name),
                        )
                        .with_pos(ln, col),
                    );
                }
                // Pop until we find any heading element.
                loop {
                    let top = self.current_tag().map(str::to_string);
                    match top.as_deref() {
                        Some("h1" | "h2" | "h3" | "h4" | "h5" | "h6") => {
                            self.open_elements.pop();
                            break;
                        }
                        None => break,
                        _ => { self.open_elements.pop(); }
                    }
                }
            }
            self.reset_insertion_mode();
            return;
        }

        // ── </body> — spec §13.2.6.4.7 ───────────────────────────────────────
        if self.fragment_context.is_none() && tag.name == "body" {
            if self.find_open_element("table").is_some() {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "ignored </body> while table is still open",
                    )
                    .with_pos(ln, col),
                );
                return;
            }
            if !self.has_element_in_scope("body") {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "end tag </body> but no <body> in scope",
                    )
                    .with_pos(ln, col),
                );
                self.ensure_body_element();
            }
            // Unclosed elements that are not in "allowed to be unclosed" set produce parse errors.
            let unclosed_tags: Vec<String> = self.open_elements.iter().rev()
                .filter_map(|&id| self.node_tag(id).map(str::to_string))
                .filter(|t| !matches!(
                    t.as_str(),
                    "dd" | "dt" | "li" | "optgroup" | "option" | "p" | "rb" | "rp"
                        | "rt" | "rtc" | "tbody" | "td" | "tfoot" | "th" | "thead"
                        | "tr" | "body" | "html"
                ))
                .collect();
            for tag_name in &unclosed_tags {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEof,
                        self.insertion_mode,
                        format!("unclosed element <{tag_name}> at </body>"),
                    )
                    .with_pos(ln, col),
                );
            }
            self.close_until("body");
            self.insertion_mode = InsertionMode::AfterBody;
            return;
        }

        // ── </html> ───────────────────────────────────────────────────────────
        if self.fragment_context.is_none() && tag.name == "html" {
            if self.find_open_element("table").is_some() {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "ignored </html> while table is still open",
                    )
                    .with_pos(ln, col),
                );
                return;
            }
            self.close_until("body");
            self.close_until("html");
            self.insertion_mode = InsertionMode::AfterAfterBody;
            return;
        }

        // ── </frameset> ───────────────────────────────────────────────────────
        if self.fragment_context.is_none() && tag.name == "frameset" {
            if !self.close_until("frameset") {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "end tag </frameset> but no <frameset> in scope",
                    )
                    .with_pos(ln, col),
                );
                return;
            }
            self.insertion_mode = if self.find_open_element("frameset").is_some() {
                InsertionMode::InFrameset
            } else {
                InsertionMode::AfterFrameset
            };
            return;
        }

        // ── </noframes> in frameset contexts ──────────────────────────────────
        if tag.name == "noframes" && matches!(
            self.insertion_mode,
            InsertionMode::InFrameset | InsertionMode::AfterFrameset | InsertionMode::AfterAfterFrameset
        ) {
            if !self.close_until("noframes") {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "end tag </noframes> but no <noframes> in scope",
                    )
                    .with_pos(ln, col),
                );
                return;
            }
            self.reset_insertion_mode();
            return;
        }

        // ── </table> ──────────────────────────────────────────────────────────
        if tag.name == "table" {
            if !self.has_element_in_table_scope("table") {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "end tag </table> but no <table> in table scope",
                    )
                    .with_pos(ln, col),
                );
                return;
            }

            while let Some(id) = self.open_elements.pop() {
                if self.node_tag(id).is_some_and(|node_tag| node_tag.eq_ignore_ascii_case("table")) {
                    break;
                }
            }
            self.reset_insertion_mode();
            return;
        }

        // ── </template> ───────────────────────────────────────────────────────
        if tag.name == "template" {
            if !self.has_element_in_scope("template") {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "end tag </template> but no <template> in scope",
                    )
                    .with_pos(ln, col),
                );
                return;
            }

            while let Some(id) = self.open_elements.pop() {
                if self.node_tag(id).is_some_and(|node_tag| node_tag.eq_ignore_ascii_case("template")) {
                    break;
                }
            }
            self.clear_afe_to_last_marker();
            let _ = self.template_insertion_modes.pop();
            self.reset_insertion_mode();
            return;
        }

        // ── </caption> ────────────────────────────────────────────────────────
        if tag.name == "caption" {
            if !self.has_element_in_table_scope("caption") {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "end tag </caption> but no <caption> in table scope",
                    )
                    .with_pos(ln, col),
                );
                return;
            }

            self.generate_implied_end_tags(None);
            if !matches!(self.current_tag(), Some("caption")) {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "end tag </caption> did not match current node",
                    )
                    .with_pos(ln, col),
                );
            }
            self.close_until("caption");
            self.clear_afe_to_last_marker();
            self.insertion_mode = InsertionMode::InTable;
            return;
        }

        // ── </td> / </th> ─────────────────────────────────────────────────────
        if matches!(tag.name.as_str(), "td" | "th") {
            if !self.has_element_in_table_scope(&tag.name) {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        format!("end tag </{tag_name}> but not in table scope", tag_name = tag.name),
                    )
                    .with_pos(ln, col),
                );
                return;
            }

            self.generate_implied_end_tags(None);
            if self.current_tag().map_or(true, |current| !current.eq_ignore_ascii_case(&tag.name)) {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        format!("end tag </{tag_name}> did not match current node", tag_name = tag.name),
                    )
                    .with_pos(ln, col),
                );
            }
            self.close_until(&tag.name);
            self.clear_afe_to_last_marker();
            self.insertion_mode = InsertionMode::InRow;
            return;
        }

        // ── </select> ─────────────────────────────────────────────────────────
        if tag.name == "select" {
            if !self.has_element_in_scope("select") {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "end tag </select> but no <select> in scope",
                    )
                    .with_pos(ln, col),
                );
                return;
            }
            self.close_until("select");
            self.reset_insertion_mode();
            return;
        }

        // ── Formatting elements → Adoption Agency Algorithm ───────────────────
        if is_formatting_element(&tag.name) {
            // Run the full Adoption Agency Algorithm (spec §13.2.6.4.7).
            if self.run_adoption_agency_algorithm(&tag.name) {
                self.reset_insertion_mode();
                return;
            }
        }

        // ── Default: in-body "any other end tag" search ───────────────────────
        if !self.handle_generic_end_tag(&tag.name) {
            self.errors.push(
                TreeBuilderError::new(
                    TreeBuilderErrorKind::UnexpectedEndTag,
                    self.insertion_mode,
                    format!("unexpected end tag </{tag_name}>", tag_name = tag.name),
                )
                .with_pos(ln, col),
            );
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

        // Handle AfterAfterBody and AfterAfterFrameset modes
        if self.fragment_context.is_none() {
            match self.insertion_mode {
                InsertionMode::AfterAfterBody => {
                    if !text.trim().is_empty() {
                        // Non-whitespace → parse error, switch to "in body" and reprocess
                        self.errors.push(TreeBuilderError::new(
                            TreeBuilderErrorKind::UnexpectedCharacter,
                            self.insertion_mode,
                            "unexpected non-whitespace character in AfterAfterBody mode",
                        ));
                        self.insertion_mode = InsertionMode::InBody;
                        return self.handle_text(text);
                    }
                    // Whitespace → process using "in body" rules (insert into body)
                    if let Some(body_id) = self.body_element_id {
                        self.append_text_to_parent(body_id, text);
                    }
                    return;
                }
                InsertionMode::AfterAfterFrameset => {
                    if !text.trim().is_empty() {
                        self.errors.push(TreeBuilderError::new(
                            TreeBuilderErrorKind::UnexpectedCharacter,
                            self.insertion_mode,
                            "unexpected non-whitespace character in AfterAfterFrameset mode",
                        ));
                        return;
                    }
                    // Whitespace is allowed
                }
                InsertionMode::InFrameset | InsertionMode::AfterFrameset => {
                    // Only whitespace is allowed in frameset modes
                    if !text.trim().is_empty() {
                        self.errors.push(TreeBuilderError::new(
                            TreeBuilderErrorKind::UnexpectedCharacter,
                            self.insertion_mode,
                            "unexpected non-whitespace character in frameset mode",
                        ));
                        return;
                    }
                }
                _ => {}
            }
        }

        if self.fragment_context.is_none() && self.current_parent_id().is_none() {
            self.ensure_body_element();
        }

        // Spec §13.2.6.4.1: before inserting characters in body mode,
        // reconstruct the active formatting elements (reopens elements like <i>
        // that were popped from the open stack by the AAA but remain in the AFE list).
        if matches!(
            self.insertion_mode,
            InsertionMode::InBody | InsertionMode::InCell
        ) {
            self.reconstruct_active_formatting_elements();
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

        // In AfterAfterBody and AfterAfterFrameset, comments go to the document root
        if matches!(
            self.insertion_mode,
            InsertionMode::AfterAfterBody | InsertionMode::AfterAfterFrameset
        ) {
            let comment_id = self.create_comment_node(comment, None);
            self.root_children.push(comment_id);
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
        match namespace {
            Namespace::Svg => {
                tag.name = adjust_svg_tag_name(&tag.name);
                normalize_svg_attributes(&mut tag.attributes);
            }
            Namespace::MathMl => {
                normalize_mathml_attributes(&mut tag.attributes);
            }
            _ => {}
        }
        adjust_foreign_attributes(&mut tag.attributes);

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

    fn insert_element_before_open_table(&mut self, element: HtmlElement) -> usize {
        // Foster parenting algorithm (spec §13.2.6.1)
        
        // Step 1: Find last template and last table in stack
        let last_template_pos = self.open_elements.iter().rposition(|&id| {
            self.node_tag(id).map_or(false, |tag| tag.eq_ignore_ascii_case("template"))
        });
        let last_table_pos = self.open_elements.iter().rposition(|&id| {
            self.node_tag(id).map_or(false, |tag| tag.eq_ignore_ascii_case("table"))
        });
        
        // Step 2: If there's a template and (no table OR template is below table)
        if let Some(template_pos) = last_template_pos {
            if last_table_pos.is_none() || template_pos > last_table_pos.unwrap() {
                // Insert inside template's template content
                let template_id = self.open_elements[template_pos];
                // Find the template-content child
                if let Some(&content_id) = self.nodes[template_id].children.iter().find(|&&child_id| {
                    matches!(&self.nodes[child_id].data, BuilderNodeData::Element(el) if el.tag == "template-content")
                }) {
                    return self.create_and_attach(element, content_id);
                }
                // Fallback: insert directly in template
                return self.create_and_attach(element, template_id);
            }
        }
        
        // Step 3: If no table, insert inside first element (html) after its last child
        let Some(table_pos) = last_table_pos else {
            if let Some(&html_id) = self.open_elements.first() {
                return self.create_and_attach(element, html_id);
            }
            return self.insert_root_element(element);
        };
        
        let table_id = self.open_elements[table_pos];
        let parent_id = self.nodes[table_id].parent;
        let inserted_id = self.create_element(element, parent_id);

        // Step 4: If table has parent, insert before table
        if let Some(parent_id) = parent_id {
            let siblings = &mut self.nodes[parent_id].children;
            if let Some(pos) = siblings.iter().position(|&id| id == table_id) {
                siblings.insert(pos, inserted_id);
            } else {
                siblings.push(inserted_id);
            }
        } else if let Some(pos) = self.root_children.iter().position(|&id| id == table_id) {
            self.root_children.insert(pos, inserted_id);
        } else {
            // Step 5: Otherwise, insert in element above table (previous element)
            if table_pos > 0 {
                let previous_element_id = self.open_elements[table_pos - 1];
                self.nodes[inserted_id].parent = Some(previous_element_id);
                self.nodes[previous_element_id].children.push(inserted_id);
            } else {
                self.root_children.push(inserted_id);
            }
        }

        inserted_id
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

    fn handle_generic_end_tag(&mut self, tag_name: &str) -> bool {
        for idx in (0..self.open_elements.len()).rev() {
            let id = self.open_elements[idx];
            let Some(node_tag) = self.node_tag(id) else {
                continue;
            };

            if node_tag.eq_ignore_ascii_case("template-content") {
                continue;
            }

            if node_tag.eq_ignore_ascii_case(tag_name) {
                self.open_elements.truncate(idx);
                return true;
            }

            if is_special_element(node_tag) {
                return false;
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
        if matches!(tag_name, "tr") && self.has_open_cell_in_table_scope() {
            self.close_cell_for_reprocessing();
            self.close_row_for_reprocessing();
        } else if matches!(tag_name, "td" | "th") && self.has_open_cell_in_table_scope() {
            self.close_cell_for_reprocessing();
        }

        match (self.current_tag(), tag_name) {
            (Some("table"), "table") => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InBody;
            }
            (Some("table"), "col") => {
                let colgroup_id = self.insert_element_at_current(HtmlElement::new("colgroup"));
                self.open_elements.push(colgroup_id);
                self.insertion_mode = InsertionMode::InColumnGroup;
            }
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
            (Some("tbody" | "thead" | "tfoot" | "tr" | "td" | "th"), "caption") => {
                while let Some(current) = self.current_tag().map(str::to_string) {
                    if current.eq_ignore_ascii_case("table") {
                        break;
                    }
                    self.open_elements.pop();
                }
                self.insertion_mode = InsertionMode::InTable;
            }
            _ => {}
        }
    }

    fn close_cell_for_reprocessing(&mut self) {
        while let Some(id) = self.open_elements.pop() {
            if self
                .node_tag(id)
                .is_some_and(|tag| matches!(tag, "td" | "th"))
            {
                break;
            }
        }
        self.clear_afe_to_last_marker();
        self.insertion_mode = InsertionMode::InRow;
    }

    fn close_row_for_reprocessing(&mut self) {
        while let Some(id) = self.open_elements.pop() {
            if self.node_tag(id).is_some_and(|tag| tag.eq_ignore_ascii_case("tr")) {
                break;
            }
        }
        self.insertion_mode = InsertionMode::InTableBody;
    }

    fn has_open_cell_in_table_scope(&self) -> bool {
        self.has_element_in_table_scope("td") || self.has_element_in_table_scope("th")
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

    /// Full Adoption Agency Algorithm — spec §13.2.6.4.7.
    ///
    /// Called for end tags that are formatting elements (a, b, big, code, em, font, i, s, small,
    /// span, strike, strong, tt, u). Returns true if the algorithm ran (even if it produced errors).
    fn run_adoption_agency_algorithm(&mut self, tag_name: &str) -> bool {
        // Step 1: If the current node is a matching formatting element not in scope →
        // pop it and return.
        if let Some(&cur_id) = self.open_elements.last() {
            if self.node_tag(cur_id).map_or(false, |t| t.eq_ignore_ascii_case(tag_name)) {
                if !self.has_element_in_scope(tag_name) {
                    self.open_elements.pop();
                    return true;
                }
            }
        }

        // Outer loop — up to 8 iterations.
        for _ in 0..8 {
            // Step 4a: Find the most recent entry for this tag in the AFE list.
            let afe_pos = self.active_formatting_elements.iter().rposition(|e| {
                if let ActiveFormattingElement::Element(id) = e {
                    self.node_tag(*id).map_or(false, |t| t.eq_ignore_ascii_case(tag_name))
                } else {
                    false
                }
            });
            let formatting_element_id = match afe_pos {
                None => {
                    // Step 4b: Not in active list — other end-tag processing.
                    return false;
                }
                Some(pos) => match self.active_formatting_elements[pos] {
                    ActiveFormattingElement::Element(id) => id,
                    ActiveFormattingElement::Marker => return false,
                },
            };

            // Step 4c: formatting element not in open elements → parse error, remove from AFE.
            let fe_open_pos = self.open_elements.iter().rposition(|&id| id == formatting_element_id);
            if fe_open_pos.is_none() {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::AdoptionAgency,
                        self.insertion_mode,
                        "adoption agency: formatting element not in open elements",
                    )
                    .with_pos(self.current_token_line, self.current_token_column),
                );
                if let Some(pos) = afe_pos {
                    self.active_formatting_elements.remove(pos);
                }
                return true;
            }
            let fe_open_pos = fe_open_pos.unwrap();

            // Step 4d: If not in scope → parse error, return.
            if !self.has_element_in_scope(tag_name) {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::AdoptionAgency,
                        self.insertion_mode,
                        "adoption agency: formatting element not in scope",
                    )
                    .with_pos(self.current_token_line, self.current_token_column),
                );
                return true;
            }

            // Step 4e: If not current node — the spec annotates this as a "parse error"
            // but conformant parsers (verified by html5lib test corpus) do NOT emit this
            // to the error list; it is purely an internal algorithm annotation.
            // We intentionally omit the error push here.

            // Step 4f: Find furthest block — the topmost special element below the
            // formatting element in the open elements stack.
            let furthest_block_pos = self.open_elements[fe_open_pos + 1..]
                .iter()
                .rposition(|&id| {
                    self.node_tag(id).map_or(false, |t| is_special_element(t))
                })
                .map(|rel| fe_open_pos + 1 + rel);

            if furthest_block_pos.is_none() {
                // Step 4g: No furthest block — pop everything up to and including
                // the formatting element, and remove from AFE.
                while let Some(id) = self.open_elements.pop() {
                    if id == formatting_element_id {
                        break;
                    }
                }
                if let Some(pos) = afe_pos {
                    self.active_formatting_elements.remove(pos);
                }
                return true;
            }
            let furthest_block_pos = furthest_block_pos.unwrap();
            let furthest_block_id = self.open_elements[furthest_block_pos];

            // Step 4h: Common ancestor — the element just below the formatting element.
            let common_ancestor_id = if fe_open_pos > 0 {
                self.open_elements[fe_open_pos - 1]
            } else {
                // Formatting element is at root — no common ancestor; foster-parent.
                furthest_block_id
            };

            // Step 4i: bookmark — position after formatting_element in AFE.
            let afe_len = self.active_formatting_elements.len();
            let mut bookmark = afe_pos.unwrap_or(afe_len);

            // Steps 4j–4n: Inner loop.
            let mut last_node_id = furthest_block_id;
            let mut node_pos = furthest_block_pos;
            let mut inner_loop_counter = 0usize;
            
            loop {
                // Step 4k1: Increment innerLoopCounter
                inner_loop_counter += 1;
                
                // Step 4k2: Move node to the one before it in the stack.
                if node_pos == 0 {
                    break;
                }
                node_pos -= 1;
                let node_id = self.open_elements[node_pos];

                // Step 4k3: If node is formattingElement, break.
                if node_id == formatting_element_id {
                    break;
                }

                // Step 4k4: If innerLoopCounter > 3 and node is in AFE, remove from AFE.
                let node_afe_pos = self.active_formatting_elements.iter().rposition(|e| {
                    matches!(e, ActiveFormattingElement::Element(id) if *id == node_id)
                });
                
                if let Some(afe_pos) = node_afe_pos {
                    if inner_loop_counter > 3 {
                        self.active_formatting_elements.remove(afe_pos);
                        // After removal, node is no longer in AFE, so continue to next step
                    }
                }
                
                // Step 4k5: If node not in AFE, remove from open elements, continue.
                let node_afe_pos = self.active_formatting_elements.iter().rposition(|e| {
                    matches!(e, ActiveFormattingElement::Element(id) if *id == node_id)
                });
                if node_afe_pos.is_none() {
                    self.open_elements.remove(node_pos);
                    // Adjust node_pos since we removed an element
                    if node_pos > 0 {
                        node_pos += 1; // Will be decremented in next iteration
                    }
                    continue;
                }
                let node_afe_pos = node_afe_pos.unwrap();

                // Step 4k6: Create a clone of node; replace in AFE and open elements.
                let clone_data = if let BuilderNodeData::Element(el) = &self.nodes[node_id].data {
                    el.clone()
                } else {
                    break;
                };
                let clone_id = self.create_element(clone_data, None);
                self.active_formatting_elements[node_afe_pos] = ActiveFormattingElement::Element(clone_id);
                self.open_elements[node_pos] = clone_id;

                // Step 4k7: If last_node is the furthest block, update bookmark.
                if last_node_id == furthest_block_id {
                    bookmark = node_afe_pos + 1;
                }

                // Step 4k8: Append last_node to node (the clone).
                // First, remove last_node from its current parent.
                let last_parent = self.nodes[last_node_id].parent;
                if let Some(p) = last_parent {
                    self.nodes[p].children.retain(|&c| c != last_node_id);
                }
                // Then append to clone.
                self.nodes[last_node_id].parent = Some(clone_id);
                self.nodes[clone_id].children.push(last_node_id);
                
                // Step 4k9: Set last_node to node (the clone).
                last_node_id = clone_id;
            }

            // Step 4n: Insert last_node into appropriate place for common_ancestor.
            // (Using foster-parenting logic if needed.)
            let is_table_ctx = self.node_tag(common_ancestor_id)
                .map_or(false, |t| matches!(t, "table" | "tbody" | "tfoot" | "thead" | "tr"));
            if is_table_ctx {
                // Foster parent last_node before the table element.
                if let Some(table_id) = self.find_open_element("table") {
                    let table_parent = self.nodes[table_id].parent;
                    self.nodes[last_node_id].parent = table_parent;
                    if let Some(p) = table_parent {
                        let pos = self.nodes[p].children.iter().position(|&c| c == table_id).unwrap_or(self.nodes[p].children.len());
                        self.nodes[p].children.insert(pos, last_node_id);
                    } else {
                        let pos = self.root_children.iter().position(|&c| c == table_id).unwrap_or(self.root_children.len());
                        self.root_children.insert(pos, last_node_id);
                    }
                } else {
                    self.nodes[last_node_id].parent = Some(common_ancestor_id);
                    self.nodes[common_ancestor_id].children.push(last_node_id);
                }
            } else {
                // Remove last_node from old parent first.
                if let Some(old_parent) = self.nodes[last_node_id].parent {
                    self.nodes[old_parent].children.retain(|&c| c != last_node_id);
                }
                self.nodes[last_node_id].parent = Some(common_ancestor_id);
                self.nodes[common_ancestor_id].children.push(last_node_id);
            }

            // Step 4o: Create clone of formatting_element; insert children of furthest block
            // into clone; append clone to furthest_block; remove formatting_element from AFE
            // and open elements; insert clone at bookmark in AFE; insert clone after furthest
            // block in open elements.
            let fe_clone_data = if let BuilderNodeData::Element(el) = &self.nodes[formatting_element_id].data {
                el.clone()
            } else {
                return true;
            };
            let fe_clone_id = self.create_element(fe_clone_data, Some(furthest_block_id));

            // Move children of furthest_block into fe_clone.
            let fb_children: Vec<usize> = self.nodes[furthest_block_id].children.drain(..).collect();
            for &child in &fb_children {
                self.nodes[child].parent = Some(fe_clone_id);
                self.nodes[fe_clone_id].children.push(child);
            }
            self.nodes[furthest_block_id].children.push(fe_clone_id);

            // Remove formatting_element from AFE; insert clone at bookmark.
            if let Some(pos) = afe_pos {
                self.active_formatting_elements.remove(pos);
                let insert_pos = if bookmark > pos { bookmark - 1 } else { bookmark };
                let insert_pos = insert_pos.min(self.active_formatting_elements.len());
                self.active_formatting_elements.insert(insert_pos, ActiveFormattingElement::Element(fe_clone_id));
            }

            // Remove formatting_element from open elements; insert clone after furthest_block.
            self.open_elements.retain(|&id| id != formatting_element_id);
            if let Some(fb_pos) = self.open_elements.iter().position(|&id| id == furthest_block_id) {
                self.open_elements.insert(fb_pos + 1, fe_clone_id);
            }

        }
        true
    }

    // ─── Active Formatting Elements ────────────────────────────────────────────

    /// Push an element onto the AFE list with Noah's Ark duplicate pruning (spec §13.2.4.3).
    fn push_active_formatting_element(&mut self, id: usize) {
        // Count how many entries for this same element (same tag name + same attrs) already exist
        // since the last Marker. If 3 or more, remove the oldest one (Noah's Ark clause).
        let tag = match &self.nodes[id].data {
            BuilderNodeData::Element(el) => el.tag.clone(),
            _ => return,
        };
        let attrs_snapshot: std::collections::HashMap<String, String> =
            if let BuilderNodeData::Element(el) = &self.nodes[id].data {
                el.attributes.clone()
            } else {
                std::collections::HashMap::new()
            };

        let mut count = 0usize;
        let mut oldest_pos = None;
        for (pos, entry) in self.active_formatting_elements.iter().enumerate().rev() {
            match entry {
                ActiveFormattingElement::Marker => break,
                ActiveFormattingElement::Element(eid) => {
                    if let BuilderNodeData::Element(el) = &self.nodes[*eid].data {
                        if el.tag == tag && el.attributes == attrs_snapshot {
                            count += 1;
                            oldest_pos = Some(pos);
                        }
                    }
                }
            }
        }
        if count >= 3 {
            if let Some(pos) = oldest_pos {
                self.active_formatting_elements.remove(pos);
            }
        }
        self.active_formatting_elements.push(ActiveFormattingElement::Element(id));
    }

    /// Reconstruct the AFE list (spec §13.2.4.3 "reconstruct the active formatting elements").
    fn reconstruct_active_formatting_elements(&mut self) {
        if self.active_formatting_elements.is_empty() {
            return;
        }
        // If the last entry is a Marker or already open, do nothing.
        match self.active_formatting_elements.last() {
            Some(ActiveFormattingElement::Marker) => return,
            Some(ActiveFormattingElement::Element(id)) => {
                if self.open_elements.contains(id) {
                    return;
                }
            }
            None => return,
        }

        // Walk backwards to find the last Marker or open element.
        let mut entry_idx = self.active_formatting_elements.len() - 1;
        loop {
            if entry_idx == 0 {
                break;
            }
            entry_idx -= 1;
            match &self.active_formatting_elements[entry_idx] {
                ActiveFormattingElement::Marker => {
                    entry_idx += 1;
                    break;
                }
                ActiveFormattingElement::Element(id) => {
                    if self.open_elements.contains(id) {
                        entry_idx += 1;
                        break;
                    }
                }
            }
        }

        // Reopen entries from entry_idx forward.
        let len = self.active_formatting_elements.len();
        while entry_idx < len {
            let existing_id = match &self.active_formatting_elements[entry_idx] {
                ActiveFormattingElement::Element(id) => *id,
                ActiveFormattingElement::Marker => { entry_idx += 1; continue; }
            };
            let clone_data = if let BuilderNodeData::Element(el) = &self.nodes[existing_id].data {
                el.clone()
            } else { entry_idx += 1; continue; };

            let new_id = self.insert_element_at_current(clone_data);
            self.open_elements.push(new_id);
            self.active_formatting_elements[entry_idx] = ActiveFormattingElement::Element(new_id);
            entry_idx += 1;
        }
    }

    /// Insert a Marker into the AFE list.
    // TECH_DEBT: called from body/table/select boundary handlers in subtask 1.8.
    #[allow(dead_code)]
    fn push_afe_marker(&mut self) {
        self.active_formatting_elements.push(ActiveFormattingElement::Marker);
    }

    /// Clear the AFE list back to the last Marker (spec §13.2.4.3).
    // TECH_DEBT: called from body/select close handlers in subtask 1.8.
    #[allow(dead_code)]
    fn clear_afe_to_last_marker(&mut self) {
        while let Some(entry) = self.active_formatting_elements.pop() {
            if matches!(entry, ActiveFormattingElement::Marker) {
                break;
            }
        }
    }

    // ─── Scope Checks (spec §13.2.4.2) ────────────────────────────────────────

    /// Check if the given tag has an element in scope (general scope delimiters).
    fn has_element_in_scope(&self, tag_name: &str) -> bool {
        self.has_element_in_scope_with_delimiters(tag_name, &SCOPE_DELIMITERS)
    }

    /// Check if `p` is in button scope.
    fn has_p_in_button_scope(&self) -> bool {
        self.has_element_in_scope_with_delimiters("p", &BUTTON_SCOPE_DELIMITERS)
    }

    /// Check for element in list-item scope.
    fn has_element_in_list_item_scope(&self, tag_name: &str) -> bool {
        self.has_element_in_scope_with_delimiters(tag_name, &LIST_ITEM_SCOPE_DELIMITERS)
    }

    /// Check for element in table scope.
    // TECH_DEBT: called from table cell (</td>, </th>) end-tag handlers — pending integration.
    #[allow(dead_code)]
    fn has_element_in_table_scope(&self, tag_name: &str) -> bool {
        self.has_element_in_scope_with_delimiters(tag_name, &TABLE_SCOPE_DELIMITERS)
    }

    /// Walk the open elements stack downward; return true if tag_name is found before any delimiter.
    fn has_element_in_scope_with_delimiters(&self, tag_name: &str, delimiters: &[&str]) -> bool {
        for &id in self.open_elements.iter().rev() {
            if let BuilderNodeData::Element(el) = &self.nodes[id].data {
                if el.tag.eq_ignore_ascii_case(tag_name) {
                    return true;
                }
                if delimiters.iter().any(|&d| el.tag.eq_ignore_ascii_case(d)) {
                    return false;
                }
            }
        }
        false
    }

    // ─── Implied End Tags (spec §13.2.6.3) ────────────────────────────────────

    /// Generate implied end tags, optionally excluding `exception`.
    fn generate_implied_end_tags(&mut self, exception: Option<&str>) {
        loop {
            let cur = self.current_tag().map(str::to_string);
            match cur.as_deref() {
                Some(t @ ("dd" | "dt" | "li" | "optgroup" | "option" | "p" | "rb" | "rp" | "rt" | "rtc")) => {
                    if exception.map_or(false, |e| e.eq_ignore_ascii_case(t)) {
                        break;
                    }
                    self.open_elements.pop();
                }
                _ => break,
            }
        }
    }

    fn insert_foster_parented_text(&mut self, text: String) -> bool {
        // Foster parenting algorithm for text (spec §13.2.6.1)
        
        // Step 1: Find last template and last table in stack
        let last_template_pos = self.open_elements.iter().rposition(|&id| {
            self.node_tag(id).map_or(false, |tag| tag.eq_ignore_ascii_case("template"))
        });
        let last_table_pos = self.open_elements.iter().rposition(|&id| {
            self.node_tag(id).map_or(false, |tag| tag.eq_ignore_ascii_case("table"))
        });
        
        // Step 2: If there's a template and (no table OR template is below table)
        if let Some(template_pos) = last_template_pos {
            if last_table_pos.is_none() || template_pos > last_table_pos.unwrap() {
                // Insert inside template's template content
                let template_id = self.open_elements[template_pos];
                // Find the template-content child
                if let Some(&content_id) = self.nodes[template_id].children.iter().find(|&&child_id| {
                    matches!(&self.nodes[child_id].data, BuilderNodeData::Element(el) if el.tag == "template-content")
                }) {
                    let text_id = self.create_text_node(text, Some(content_id));
                    self.nodes[content_id].children.push(text_id);
                    return true;
                }
                // Fallback: insert directly in template
                let text_id = self.create_text_node(text, Some(template_id));
                self.nodes[template_id].children.push(text_id);
                return true;
            }
        }
        
        // Step 3: If no table, insert in body or first element
        let Some(table_pos) = last_table_pos else {
            if let Some(body_id) = self.body_element_id {
                let text_id = self.create_text_node(text, Some(body_id));
                self.nodes[body_id].children.push(text_id);
                return true;
            }
            if let Some(&html_id) = self.open_elements.first() {
                let text_id = self.create_text_node(text, Some(html_id));
                self.nodes[html_id].children.push(text_id);
                return true;
            }
            return false;
        };

        let table_id = self.open_elements[table_pos];
        let parent_id = self.nodes[table_id].parent;
        let text_id = self.create_text_node(text, parent_id);

        // Step 4: If table has parent, insert before table
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
            // Step 5: Otherwise, insert in element above table
            if table_pos > 0 {
                let previous_element_id = self.open_elements[table_pos - 1];
                self.nodes[text_id].parent = Some(previous_element_id);
                self.nodes[previous_element_id].children.push(text_id);
            } else {
                self.root_children.push(text_id);
            }
        }

        true
    }

    fn maybe_exit_foreign_content_for_start_tag(&mut self, tag_name: &str) {
        if !is_html_breakout_start_tag(tag_name) {
            return;
        }

        loop {
            let Some(current_id) = self.current_parent_id() else {
                break;
            };
            let Some(current_tag) = self.node_tag(current_id).map(str::to_string) else {
                break;
            };
            let BuilderNodeData::Element(current_element) = &self.nodes[current_id].data else {
                break;
            };

            let stop_here = match current_element.namespace {
                Namespace::Html => true,
                Namespace::Svg => {
                    is_svg_html_integration_point(&current_tag)
                        && !should_escape_integration_point_for_table_breakout(
                            &current_tag,
                            tag_name,
                            self.find_open_element("table").is_some(),
                        )
                }
                Namespace::MathMl => is_mathml_html_integration_point(&current_tag),
            };

            if stop_here {
                break;
            }

            self.open_elements.pop();
        }

        self.reset_insertion_mode();
    }

    fn find_open_element(&self, tag_name: &str) -> Option<usize> {
        self.open_elements.iter().rev().copied().find(|&id| {
            matches!(
                &self.nodes[id].data,
                BuilderNodeData::Element(element) if element.tag.eq_ignore_ascii_case(tag_name)
            )
        })
    }

    fn node_tag(&self, id: usize) -> Option<&str> {
        match &self.nodes[id].data {
            BuilderNodeData::Element(el) => Some(el.tag.as_str()),
            _ => None,
        }
    }

    fn node_namespace(&self, id: usize) -> Namespace {
        match &self.nodes[id].data {
            BuilderNodeData::Element(el) => el.namespace,
            _ => Namespace::Html,
        }
    }

    /// Spec §13.2.3.1 — reset the insertion mode appropriately.
    fn reset_insertion_mode(&mut self) {
        // Walk the open elements from last to first.
        let len = self.open_elements.len();
        for i in (0..len).rev() {
            let id = self.open_elements[i];
            let is_last = i == len - 1;
            let (tag, namespace) = match &self.nodes[id].data {
                BuilderNodeData::Element(el) => (el.tag.clone(), el.namespace),
                _ => continue,
            };
            let tag = tag.as_str();

            // In fragment mode: if we reach the context element, use its context.
            let tag = if is_last {
                if let Some(ref ctx) = self.fragment_context {
                    ctx.tag_name.as_str()
                } else {
                    tag
                }
            } else {
                tag
            };

            if !matches!(namespace, Namespace::Html) {
                continue;
            }

            self.insertion_mode = match tag {
                "select" => {
                    // Check if there's a table ancestor.
                    let mut mode = InsertionMode::InSelect;
                    for j in (0..i).rev() {
                        if let BuilderNodeData::Element(el) = &self.nodes[self.open_elements[j]].data {
                            if el.tag.eq_ignore_ascii_case("template") {
                                break;
                            }
                            if el.tag.eq_ignore_ascii_case("table") {
                                mode = InsertionMode::InSelectInTable;
                                break;
                            }
                        }
                    }
                    self.insertion_mode = mode;
                    return;
                }
                "option" | "optgroup" => continue,
                "td" | "th" if !is_last => InsertionMode::InCell,
                "tr" => InsertionMode::InRow,
                "tbody" | "thead" | "tfoot" => InsertionMode::InTableBody,
                "caption" => InsertionMode::InCaption,
                "colgroup" => InsertionMode::InColumnGroup,
                "table" => InsertionMode::InTable,
                "template" => {
                    self.insertion_mode = self.template_insertion_modes.last().copied().unwrap_or(InsertionMode::InBody);
                    return;
                }
                "head" if !is_last => InsertionMode::InHead,
                "body" => InsertionMode::InBody,
                "frameset" => InsertionMode::InFrameset,
                "html" => {
                    if self.head_element_id.is_none() {
                        InsertionMode::BeforeHead
                    } else {
                        InsertionMode::AfterHead
                    }
                }
                _ if is_last => InsertionMode::InBody,
                _ => continue,
            };
            return;
        }
        self.insertion_mode = InsertionMode::InBody;
    }

    fn should_foster_parent_text(&self) -> bool {
        matches!(
            self.current_tag(),
            Some("table" | "tbody" | "tfoot" | "thead" | "tr" | "colgroup")
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

    /// Process a start tag token in "in template" insertion mode (spec §13.2.6.4.16).
    fn process_in_template_start_tag(&mut self, tag: &str) -> Option<InsertionMode> {
        match tag {
            // Characters, comments, DOCTYPE → process using "in body" rules
            // (handled by caller)
            
            // base, basefont, bgsound, link, meta, noframes, script, style, template, title
            // → process using "in head" rules (handled by caller)
            
            // caption, colgroup, tbody, tfoot, thead
            "caption" | "colgroup" | "tbody" | "tfoot" | "thead" => {
                self.template_insertion_modes.pop();
                self.template_insertion_modes.push(InsertionMode::InTable);
                Some(InsertionMode::InTable)
            }
            
            // col
            "col" => {
                self.template_insertion_modes.pop();
                self.template_insertion_modes.push(InsertionMode::InColumnGroup);
                Some(InsertionMode::InColumnGroup)
            }
            
            // tr
            "tr" => {
                self.template_insertion_modes.pop();
                self.template_insertion_modes.push(InsertionMode::InTableBody);
                Some(InsertionMode::InTableBody)
            }
            
            // td, th
            "td" | "th" => {
                self.template_insertion_modes.pop();
                self.template_insertion_modes.push(InsertionMode::InRow);
                Some(InsertionMode::InRow)
            }
            
            // Anything else
            _ => {
                self.template_insertion_modes.pop();
                self.template_insertion_modes.push(InsertionMode::InBody);
                Some(InsertionMode::InBody)
            }
        }
    }

    /// Handle end tag </template> in "in template" insertion mode (spec §13.2.6.4.16).
    #[allow(dead_code)]
    fn process_in_template_end_tag(&mut self, tag_name: &str) -> bool {
        if tag_name == "template" {
            // Process using "in head" rules
            // This will be handled by the existing template end tag logic
            return true;
        }
        // Any other end tag → parse error, ignore
        false
    }

    /// Process tokens in "in frameset" insertion mode (spec §13.2.6.4.18).
    fn process_in_frameset_mode(&mut self, tag: &StartTagToken) -> bool {
        match tag.name.as_str() {
            // Whitespace characters → insert
            // (handled by handle_text)
            
            // Comments → insert
            // (handled by handle_comment)
            
            // DOCTYPE → parse error, ignore
            // (handled by handle_doctype)
            
            // <html> → process using "in body" rules
            "html" => false, // Let caller handle it
            
            // <frameset> → insert element
            "frameset" => {
                let element = self.make_element(&mut tag.clone());
                let id = self.insert_element_at_current(element);
                self.open_elements.push(id);
                self.insertion_mode = InsertionMode::InFrameset;
                true
            }
            
            // <frame> → insert element, immediately pop, acknowledge self-closing
            "frame" => {
                let element = self.make_element(&mut tag.clone());
                let id = self.insert_element_at_current(element);
                // Immediately pop (frame is void)
                // Don't push to open_elements
                let _ = id;
                true
            }
            
            // <noframes> → process using "in head" rules
            "noframes" => false, // Already handled in handle_start_tag
            
            // Anything else → parse error, ignore
            _ => {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedToken,
                        self.insertion_mode,
                        format!("unexpected start tag <{}> in frameset mode", tag.name),
                    )
                    .with_pos(self.current_token_line, self.current_token_column),
                );
                true // Token handled (ignored)
            }
        }
    }

    /// Process end tags in "in frameset" insertion mode (spec §13.2.6.4.18).
    fn process_in_frameset_end_tag(&mut self, tag_name: &str) -> bool {
        if tag_name == "frameset" {
            // If current node is root html element → parse error, ignore
            if self.open_elements.len() == 1 {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedEndTag,
                        self.insertion_mode,
                        "unexpected </frameset> when current node is root html element",
                    )
                    .with_pos(self.current_token_line, self.current_token_column),
                );
                return true;
            }
            
            // Otherwise, pop current node
            self.open_elements.pop();
            
            // If not fragment case and current node is no longer frameset → switch to after frameset
            if self.fragment_context.is_none() {
                if let Some(&current_id) = self.open_elements.last() {
                    if !matches!(self.node_tag(current_id), Some("frameset")) {
                        self.insertion_mode = InsertionMode::AfterFrameset;
                    }
                }
            }
            
            return true;
        }
        
        // Any other end tag → parse error, ignore
        self.errors.push(
            TreeBuilderError::new(
                TreeBuilderErrorKind::UnexpectedEndTag,
                self.insertion_mode,
                format!("unexpected end tag </{tag_name}> in frameset mode"),
            )
            .with_pos(self.current_token_line, self.current_token_column),
        );
        true
    }

    /// Process tokens in "after frameset" insertion mode (spec §13.2.6.4.19).
    fn process_after_frameset_mode(&mut self, tag: &StartTagToken) -> bool {
        match tag.name.as_str() {
            // Whitespace → insert (handled by handle_text)
            // Comments → insert (handled by handle_comment)
            // DOCTYPE → parse error, ignore (handled by handle_doctype)
            
            // <html> → process using "in body" rules
            "html" => false, // Let caller handle it
            
            // <noframes> → process using "in head" rules
            "noframes" => false, // Already handled in handle_start_tag
            
            // Anything else → parse error, ignore
            _ => {
                self.errors.push(
                    TreeBuilderError::new(
                        TreeBuilderErrorKind::UnexpectedToken,
                        self.insertion_mode,
                        format!("unexpected start tag <{}> in after frameset mode", tag.name),
                    )
                    .with_pos(self.current_token_line, self.current_token_column),
                );
                true // Token handled (ignored)
            }
        }
    }

    /// Process end tags in "after frameset" insertion mode (spec §13.2.6.4.19).
    fn process_after_frameset_end_tag(&mut self, tag_name: &str) -> bool {
        if tag_name == "html" {
            // Switch to "after after frameset"
            self.insertion_mode = InsertionMode::AfterAfterFrameset;
            return true;
        }
        
        // Any other end tag → parse error, ignore
        self.errors.push(
            TreeBuilderError::new(
                TreeBuilderErrorKind::UnexpectedEndTag,
                self.insertion_mode,
                format!("unexpected end tag </{tag_name}> in after frameset mode"),
            )
            .with_pos(self.current_token_line, self.current_token_column),
        );
        true
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

// ─── Scope delimiter sets (spec §13.2.4.2) ─────────────────────────────────

/// General scope delimiters (default scope).
const SCOPE_DELIMITERS: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object",
    "template",
    // MathML integration points
    "mi", "mo", "mn", "ms", "mtext", "annotation-xml",
    // SVG integration points
    "foreignObject", "desc", "title",
];

/// Button scope adds `button` to the general scope delimiters.
const BUTTON_SCOPE_DELIMITERS: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object",
    "template", "button",
    "mi", "mo", "mn", "ms", "mtext", "annotation-xml",
    "foreignObject", "desc", "title",
];

/// List-item scope adds `ol` and `ul`.
const LIST_ITEM_SCOPE_DELIMITERS: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object",
    "template", "ol", "ul",
    "mi", "mo", "mn", "ms", "mtext", "annotation-xml",
    "foreignObject", "desc", "title",
];

/// Table scope: only `html`, `table`, `template`.
// TECH_DEBT: used by has_element_in_table_scope() — pending integration.
#[allow(dead_code)]
const TABLE_SCOPE_DELIMITERS: &[&str] = &["html", "table", "template"];

/// "Special" elements for the Adoption Agency Algorithm (spec §13.2.6.4.7).
///
/// An element is special if it's in the list of elements treated as block-level
/// elements by the parsing algorithm, preventing formatting elements from crossing them.
fn is_special_element(tag: &str) -> bool {
    matches!(
        tag,
        "address" | "applet" | "area" | "article" | "aside" | "base" | "basefont"
            | "bgsound" | "blockquote" | "body" | "br" | "button" | "caption" | "center"
            | "col" | "colgroup" | "dd" | "details" | "dir" | "div" | "dl" | "dt"
            | "embed" | "fieldset" | "figcaption" | "figure" | "footer" | "form"
            | "frame" | "frameset" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            | "head" | "header" | "hgroup" | "hr" | "html" | "iframe" | "img"
            | "input" | "keygen" | "li" | "link" | "listing" | "main" | "marquee"
            | "menu" | "meta" | "nav" | "noembed" | "noframes" | "noscript"
            | "object" | "ol" | "p" | "param" | "plaintext" | "pre" | "script"
            | "search" | "section" | "select" | "source" | "style" | "summary"
            | "table" | "tbody" | "td" | "template" | "textarea" | "tfoot" | "th"
            | "thead" | "title" | "tr" | "track" | "ul" | "wbr" | "xmp"
            // MathML
            | "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml"
            // SVG
            | "foreignObject" | "desc"
    )
}

fn is_svg_html_integration_point(tag: &str) -> bool {
    tag.eq_ignore_ascii_case("foreignObject")
}

fn is_mathml_html_integration_point(tag: &str) -> bool {
    tag.eq_ignore_ascii_case("annotation-xml")
}

fn is_html_breakout_start_tag(tag: &str) -> bool {
    matches!(
        tag,
        "b" | "big" | "blockquote" | "body" | "br" | "caption" | "center" | "code"
            | "col" | "colgroup" | "dd" | "div" | "dl" | "dt" | "em" | "embed"
            | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "head" | "hr" | "i"
            | "img" | "li" | "listing" | "menu" | "meta" | "nobr" | "ol" | "p"
            | "pre" | "ruby" | "s" | "small" | "span" | "strong" | "strike"
            | "sub" | "sup" | "table" | "tbody" | "td" | "tfoot" | "th"
            | "thead" | "tr" | "tt" | "u" | "ul" | "var"
    )
}

fn should_foster_parent_start_tag_in_table_mode(tag: &str) -> bool {
    !matches!(
        tag,
        "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr" | "td" | "th"
            | "table" | "style" | "script" | "template"
    )
}

fn should_escape_integration_point_for_table_breakout(
    current_tag: &str,
    incoming_tag: &str,
    has_open_table: bool,
) -> bool {
    has_open_table
        && current_tag.eq_ignore_ascii_case("foreignObject")
        && matches!(
            incoming_tag,
            "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr" | "td" | "th"
                | "table"
        )
}

fn adjust_tag_name_for_namespace(tag_name: &str, namespace: Namespace) -> String {
    match namespace {
        Namespace::Svg => adjust_svg_tag_name(tag_name),
        _ => tag_name.to_string(),
    }
}

fn adjust_svg_tag_name(tag_name: &str) -> String {
    match tag_name.to_ascii_lowercase().as_str() {
        "altglyph" => "altGlyph".to_string(),
        "altglyphdef" => "altGlyphDef".to_string(),
        "altglyphitem" => "altGlyphItem".to_string(),
        "animatecolor" => "animateColor".to_string(),
        "animatemotion" => "animateMotion".to_string(),
        "animatetransform" => "animateTransform".to_string(),
        "clippath" => "clipPath".to_string(),
        "feblend" => "feBlend".to_string(),
        "fecolormatrix" => "feColorMatrix".to_string(),
        "fecomponenttransfer" => "feComponentTransfer".to_string(),
        "fecomposite" => "feComposite".to_string(),
        "feconvolvematrix" => "feConvolveMatrix".to_string(),
        "fediffuselighting" => "feDiffuseLighting".to_string(),
        "fedisplacementmap" => "feDisplacementMap".to_string(),
        "fedistantlight" => "feDistantLight".to_string(),
        "feflood" => "feFlood".to_string(),
        "fefunca" => "feFuncA".to_string(),
        "fefuncb" => "feFuncB".to_string(),
        "fefuncg" => "feFuncG".to_string(),
        "fefuncr" => "feFuncR".to_string(),
        "fegaussianblur" => "feGaussianBlur".to_string(),
        "feimage" => "feImage".to_string(),
        "femerge" => "feMerge".to_string(),
        "femergenode" => "feMergeNode".to_string(),
        "femorphology" => "feMorphology".to_string(),
        "feoffset" => "feOffset".to_string(),
        "fepointlight" => "fePointLight".to_string(),
        "fespecularlighting" => "feSpecularLighting".to_string(),
        "fespotlight" => "feSpotLight".to_string(),
        "fetile" => "feTile".to_string(),
        "feturbulence" => "feTurbulence".to_string(),
        "foreignobject" => "foreignObject".to_string(),
        "glyphref" => "glyphRef".to_string(),
        "lineargradient" => "linearGradient".to_string(),
        "radialgradient" => "radialGradient".to_string(),
        "textpath" => "textPath".to_string(),
        _ => tag_name.to_string(),
    }
}

fn normalize_svg_attributes(attrs: &mut std::collections::HashMap<String, String>) {
    let mut normalized = std::collections::HashMap::with_capacity(attrs.len());
    for (name, value) in std::mem::take(attrs) {
        normalized.insert(adjust_svg_attribute_name(&name), value);
    }
    *attrs = normalized;
}

fn adjust_svg_attribute_name(name: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "attributename" => "attributeName".to_string(),
        "attributetype" => "attributeType".to_string(),
        "basefrequency" => "baseFrequency".to_string(),
        "baseprofile" => "baseProfile".to_string(),
        "calcmode" => "calcMode".to_string(),
        "clippathunits" => "clipPathUnits".to_string(),
        "diffuseconstant" => "diffuseConstant".to_string(),
        "edgemode" => "edgeMode".to_string(),
        "filterunits" => "filterUnits".to_string(),
        "glyphref" => "glyphRef".to_string(),
        "gradienttransform" => "gradientTransform".to_string(),
        "gradientunits" => "gradientUnits".to_string(),
        "kernelmatrix" => "kernelMatrix".to_string(),
        "kernelunitlength" => "kernelUnitLength".to_string(),
        "keypoints" => "keyPoints".to_string(),
        "keysplines" => "keySplines".to_string(),
        "keytimes" => "keyTimes".to_string(),
        "lengthadjust" => "lengthAdjust".to_string(),
        "limitingconeangle" => "limitingConeAngle".to_string(),
        "markerheight" => "markerHeight".to_string(),
        "markerunits" => "markerUnits".to_string(),
        "markerwidth" => "markerWidth".to_string(),
        "maskcontentunits" => "maskContentUnits".to_string(),
        "maskunits" => "maskUnits".to_string(),
        "numoctaves" => "numOctaves".to_string(),
        "pathlength" => "pathLength".to_string(),
        "patterncontentunits" => "patternContentUnits".to_string(),
        "patterntransform" => "patternTransform".to_string(),
        "patternunits" => "patternUnits".to_string(),
        "pointsatx" => "pointsAtX".to_string(),
        "pointsaty" => "pointsAtY".to_string(),
        "pointsatz" => "pointsAtZ".to_string(),
        "preservealpha" => "preserveAlpha".to_string(),
        "preserveaspectratio" => "preserveAspectRatio".to_string(),
        "primitiveunits" => "primitiveUnits".to_string(),
        "refx" => "refX".to_string(),
        "refy" => "refY".to_string(),
        "repeatcount" => "repeatCount".to_string(),
        "repeatdur" => "repeatDur".to_string(),
        "requiredextensions" => "requiredExtensions".to_string(),
        "requiredfeatures" => "requiredFeatures".to_string(),
        "specularconstant" => "specularConstant".to_string(),
        "specularexponent" => "specularExponent".to_string(),
        "spreadmethod" => "spreadMethod".to_string(),
        "startoffset" => "startOffset".to_string(),
        "stddeviation" => "stdDeviation".to_string(),
        "stitchtiles" => "stitchTiles".to_string(),
        "surfacescale" => "surfaceScale".to_string(),
        "systemlanguage" => "systemLanguage".to_string(),
        "tablevalues" => "tableValues".to_string(),
        "targetx" => "targetX".to_string(),
        "targety" => "targetY".to_string(),
        "textlength" => "textLength".to_string(),
        "viewbox" => "viewBox".to_string(),
        "viewtarget" => "viewTarget".to_string(),
        "xchannelselector" => "xChannelSelector".to_string(),
        "ychannelselector" => "yChannelSelector".to_string(),
        "zoomandpan" => "zoomAndPan".to_string(),
        _ => name.to_string(),
    }
}

fn normalize_mathml_attributes(attrs: &mut std::collections::HashMap<String, String>) {
    let mut normalized = std::collections::HashMap::with_capacity(attrs.len());
    for (name, value) in std::mem::take(attrs) {
        normalized.insert(adjust_mathml_attribute_name(&name), value);
    }
    *attrs = normalized;
}

fn adjust_mathml_attribute_name(name: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "definitionurl" => "definitionURL".to_string(),
        _ => name.to_string(),
    }
}

fn adjust_foreign_attributes(attrs: &mut std::collections::HashMap<String, String>) {
    let mut normalized = std::collections::HashMap::with_capacity(attrs.len());
    for (name, value) in std::mem::take(attrs) {
        normalized.insert(adjust_foreign_attribute_name(&name), value);
    }
    *attrs = normalized;
}

fn adjust_foreign_attribute_name(name: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "xlink:actuate" => "xlink:actuate".to_string(),
        "xlink:arcrole" => "xlink:arcrole".to_string(),
        "xlink:href" => "xlink:href".to_string(),
        "xlink:role" => "xlink:role".to_string(),
        "xlink:show" => "xlink:show".to_string(),
        "xlink:title" => "xlink:title".to_string(),
        "xlink:type" => "xlink:type".to_string(),
        "xml:base" => "xml:base".to_string(),
        "xml:lang" => "xml:lang".to_string(),
        "xml:space" => "xml:space".to_string(),
        "xmlns" => "xmlns".to_string(),
        "xmlns:xlink" => "xmlns:xlink".to_string(),
        _ => name.to_string(),
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
    // Fragment parsing must configure context BEFORE token consumption.
    // `with_options` eagerly processes tokens, so use the empty constructor here.
    let mut builder = HtmlTreeBuilder::with_options_empty(options.clone());
    if let Some(context) = context {
        builder.setup_fragment_context(context);
    }
    builder.feed(input);
    builder.end();
    builder.finish()
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

    #[test]
    fn canonicalizes_svg_tag_and_attribute_names() {
        let output = build_document_with_errors(
            "<svg viewbox='' preserveaspectratio=''><lineargradient></lineargradient></svg>",
        );
        let tree = serialize_nodes(&output.document.children);
        assert!(tree.contains("|     <svg svg>\n|       preserveAspectRatio=\"\"\n|       viewBox=\"\""));
        assert!(tree.contains("|       <svg linearGradient>"));
    }

    #[test]
    fn canonicalizes_mathml_attribute_names() {
        let output = build_document_with_errors(
            "<math definitionurl='test'></math>",
        );
        let tree = serialize_nodes(&output.document.children);
        assert!(tree.contains("|     <math math>\n|       definitionURL=\"test\""));
    }

    #[test]
    fn adjusts_foreign_namespace_attributes() {
        let output = build_document_with_errors(
            "<svg xlink:href='test' xml:lang='en'></svg>",
        );
        let tree = serialize_nodes(&output.document.children);
        assert!(tree.contains("|     <svg svg>\n|       xlink:href=\"test\"\n|       xml:lang=\"en\""));
    }

    #[test]
    fn honors_self_closing_only_in_foreign_content() {
        // En HTML, <div /> no es auto-cerrado.
        let output_html = build_document_with_errors("<div />text</div>");
        let tree_html = serialize_nodes(&output_html.document.children);
        assert!(tree_html.contains("|   <div>\n|     \"text\""));

        // En SVG, <rect /> ES auto-cerrado.
        let output_svg = build_document_with_errors("<svg><rect /></svg>text");
        let tree_svg = serialize_nodes(&output_svg.document.children);
        assert!(tree_svg.contains("|     <svg svg>\n|       <svg rect>\n|   \"text\""));
    }

    #[test]
    fn inserts_after_body_tokens_into_body() {
        let output = build_document_with_errors("<!doctype html></body><meta>");
        let tree = serialize_nodes(&output.document.children);
        assert!(tree.contains("|   <head>\n|   <body>\n|     <meta>"));
    }

    #[test]
    fn keeps_noframes_inside_outer_frameset() {
        let output = build_document_with_errors(
            "<frame></frame></frame><frameset><frame><frameset><frame></frameset><noframes></frameset><noframes>",
        );
        let tree = serialize_nodes(&output.document.children);
        assert!(tree.contains("|   <frameset>\n|     <frame>\n|     <frameset>\n|       <frame>\n|     <noframes>"));
        assert!(tree.contains("|       \"</frameset><noframes>\""));
        assert!(!tree.contains("|   <body>\n|     <noframes>"));
    }
}

