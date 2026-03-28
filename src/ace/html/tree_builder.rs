use std::collections::HashMap;
use crate::ace::html::{
    DoctypeToken, EndTagToken, HtmlDocument, HtmlElement, HtmlNode, HtmlToken,
    HtmlTokenizer, StartTagToken, TokenizerErrorSource,
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
}

impl TreeBuilderError {
    pub fn new(kind: TreeBuilderErrorKind, mode: InsertionMode, message: impl Into<String>) -> Self {
        Self {
            kind,
            insertion_mode: mode,
            message: message.into(),
            source: TreeBuilderErrorSource::TreeBuilder,
        }
    }
}

pub struct TreeBuildOutput {
    pub document: HtmlDocument,
    pub errors: Vec<TreeBuilderError>,
}

#[derive(Clone, Debug)]
pub struct InternalNode {
    pub id: usize,
    pub data: InternalNodeData,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

#[derive(Clone, Debug)]
pub enum InternalNodeData {
    Document,
    Element {
        tag: String,
        attributes: HashMap<String, String>,
    },
    Text(String),
    Comment(String),
}

#[derive(Clone, Debug)]
enum ActiveFormattingEntry {
    Marker,
    Element(usize), // Node ID in arena
}

pub struct HtmlTreeBuilder<'a> {
    tokenizer: HtmlTokenizer<'a>,
    insertion_mode: InsertionMode,
    original_insertion_mode: InsertionMode,

    arena: Vec<InternalNode>,
    root_id: usize,
    open_elements: Vec<usize>,
    active_formatting_elements: Vec<ActiveFormattingEntry>,

    doctype: Option<DoctypeToken>,
    errors: Vec<TreeBuilderError>,
    head_element_id: Option<usize>,
    form_element_id: Option<usize>,
    
    scripting_enabled: bool,
    frameset_ok: bool,
    foster_parenting: bool,
}

impl<'a> HtmlTreeBuilder<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut arena = Vec::new();
        let root_id = 0;
        arena.push(InternalNode {
            id: root_id,
            data: InternalNodeData::Document,
            parent: None,
            children: Vec::new(),
        });

        Self {
            tokenizer: HtmlTokenizer::new(input),
            insertion_mode: InsertionMode::Initial,
            original_insertion_mode: InsertionMode::Initial,

            arena,
            root_id,
            open_elements: Vec::new(),
            active_formatting_elements: Vec::new(),

            doctype: None,
            errors: Vec::new(),
            head_element_id: None,
            form_element_id: None,
            
            scripting_enabled: true,
            frameset_ok: true,
            foster_parenting: false,
        }
    }

    pub fn run(mut self) -> TreeBuildOutput {
        let mut reprocess: Option<HtmlToken> = None;
        loop {
            let token = reprocess
                .take()
                .unwrap_or_else(|| self.tokenizer.next_token());
            
            self.collect_tokenizer_errors();

            let is_eof = matches!(token, HtmlToken::Eof);
            reprocess = self.process_token(token);
            
            if is_eof && reprocess.is_none() {
                break;
            }
        }

        // Finalize the tree
        let doc_node_children = self.arena[self.root_id].children.clone();
        let mut children = Vec::new();
        for &id in &doc_node_children {
            children.push(self.convert_to_html_node(id));
        }

        TreeBuildOutput {
            document: HtmlDocument {
                doctype: self.doctype,
                children,
            },
            errors: self.errors,
        }
    }

    fn convert_to_html_node(&self, id: usize) -> HtmlNode {
        let node = &self.arena[id];
        match &node.data {
            InternalNodeData::Element { tag, attributes } => {
                let mut children = Vec::new();
                for &child_id in &node.children {
                    children.push(self.convert_to_html_node(child_id));
                }
                HtmlNode::Element(HtmlElement {
                    tag: tag.clone(),
                    attributes: attributes.clone(),
                    children,
                })
            }
            InternalNodeData::Text(s) => HtmlNode::Text(s.clone()),
            InternalNodeData::Comment(s) => HtmlNode::Comment(s.clone()),
            InternalNodeData::Document => unreachable!("nested document node"),
        }
    }

    fn collect_tokenizer_errors(&mut self) {
        let tok_errors = self.tokenizer.take_errors();
        for err in tok_errors {
            let mut be = TreeBuilderError::new(TreeBuilderErrorKind::TokenizerError, self.insertion_mode, err.message);
            be.source = match err.source {
                TokenizerErrorSource::Lexer => TreeBuilderErrorSource::Lexer,
                TokenizerErrorSource::Tokenizer => TreeBuilderErrorSource::Tokenizer,
            };
            self.errors.push(be);
        }
    }

    fn process_token(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match self.insertion_mode {
            InsertionMode::Initial => self.handle_initial(token),
            InsertionMode::BeforeHtml => self.handle_before_html(token),
            InsertionMode::BeforeHead => self.handle_before_head(token),
            InsertionMode::InHead => self.handle_in_head(token),
            InsertionMode::InHeadNoscript => self.handle_in_head_noscript(token),
            InsertionMode::AfterHead => self.handle_after_head(token),
            InsertionMode::InBody => self.handle_in_body(token),
            InsertionMode::Text => self.handle_text(token),
            InsertionMode::InTable => self.handle_in_table(token),
            InsertionMode::InTableText => self.handle_in_table_text(token),
            InsertionMode::InCaption => self.handle_in_caption(token),
            InsertionMode::InColumnGroup => self.handle_in_column_group(token),
            InsertionMode::InTableBody => self.handle_in_table_body(token),
            InsertionMode::InRow => self.handle_in_row(token),
            InsertionMode::InCell => self.handle_in_cell(token),
            InsertionMode::InSelect => self.handle_in_select(token),
            InsertionMode::InSelectInTable => self.handle_in_select_in_table(token),
            InsertionMode::InTemplate => self.handle_in_template(token),
            InsertionMode::AfterBody => self.handle_after_body(token),
            InsertionMode::InFrameset => self.handle_in_frameset(token),
            InsertionMode::AfterFrameset => self.handle_after_frameset(token),
            InsertionMode::AfterAfterBody => self.handle_after_after_body(token),
        }
    }

    // --- Node Helpers ---

    fn create_node(&mut self, data: InternalNodeData) -> usize {
        let id = self.arena.len();
        self.arena.push(InternalNode {
            id,
            data,
            parent: None,
            children: Vec::new(),
        });
        id
    }

    fn append_node(&mut self, parent_id: usize, child_id: usize) {
        // Disconnect from old parent
        if let Some(old_parent) = self.arena[child_id].parent {
            self.arena[old_parent].children.retain(|&id| id != child_id);
        }
        
        self.arena[child_id].parent = Some(parent_id);
        self.arena[parent_id].children.push(child_id);
    }

    fn insert_at_appropriate_place(&mut self, node_id: usize, override_target: Option<usize>) {
        let target = override_target.unwrap_or_else(|| self.current_node());
        
        if self.foster_parenting && matches!(self.arena[target].data, InternalNodeData::Element { ref tag, .. } if matches!(tag.as_str(), "table" | "tbody" | "tfoot" | "thead" | "tr")) {
            // Find foster parent
            let mut last_table = None;
            for &id in self.open_elements.iter().rev() {
                if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                    if tag == "table" {
                        last_table = Some(id);
                        break;
                    }
                }
            }

            if let Some(table_id) = last_table {
                if let Some(parent_id) = self.arena[table_id].parent {
                    // Insert before table_id in parent's children
                    let pos = self.arena[parent_id].children.iter().position(|&x| x == table_id).unwrap();
                    self.arena[node_id].parent = Some(parent_id);
                    self.arena[parent_id].children.insert(pos, node_id);
                    return;
                } else {
                    // Parent is root or document
                    self.append_node(self.root_id, node_id);
                    return;
                }
            }
        }

        self.append_node(target, node_id);
    }

    fn insert_html_element(&mut self, tag: StartTagToken) -> usize {
        let id = self.create_node(InternalNodeData::Element {
            tag: tag.name.clone(),
            attributes: tag.attributes.clone(),
        });
        self.insert_at_appropriate_place(id, None);
        self.open_elements.push(id);
        id
    }

    fn insert_element_at_root(&mut self, tag: String, attributes: HashMap<String, String>) -> usize {
        let id = self.create_node(InternalNodeData::Element { tag, attributes });
        self.append_node(self.root_id, id);
        self.open_elements.push(id);
        id
    }

    fn insert_element_at_current(
        &mut self,
        tag: String,
        attributes: HashMap<String, String>,
    ) -> usize {
        let id = self.create_node(InternalNodeData::Element { tag, attributes });
        let current = self.current_node();
        self.append_node(current, id);
        self.open_elements.push(id);
        id
    }

    fn insert_text(&mut self, text: String) {
        let target = self.current_node();
        // Check if last child of target is text to merge
        if let Some(&last_child_id) = self.arena[target].children.last() {
            if let InternalNodeData::Text(ref mut existing) = self.arena[last_child_id].data {
                existing.push_str(&text);
                return;
            }
        }
        
        let id = self.create_node(InternalNodeData::Text(text));
        self.insert_at_appropriate_place(id, None);
    }

    fn current_node(&self) -> usize {
        *self.open_elements.last().unwrap_or(&self.root_id)
    }

    fn current_tag(&self) -> Option<&str> {
        self.open_elements.last().and_then(|&id| {
            match &self.arena[id].data {
                InternalNodeData::Element { tag, .. } => Some(tag.as_str()),
                _ => None,
            }
        })
    }

    fn has_open_element(&self, tag_name: &str) -> bool {
        self.open_elements.iter().any(|&id| {
            match &self.arena[id].data {
                InternalNodeData::Element { tag, .. } => tag == tag_name,
                _ => false,
            }
        })
    }

    fn pop_until(&mut self, tag_name: &str) {
        while let Some(&id) = self.open_elements.last() {
            let is_match = match &self.arena[id].data {
                InternalNodeData::Element { tag, .. } => tag == tag_name,
                _ => false,
            };
            self.open_elements.pop();
            if is_match { break; }
        }
    }

    fn generate_implied_end_tags(&mut self, exclude: Option<&str>) {
        while let Some(&id) = self.open_elements.last() {
            let tag = match &self.arena[id].data {
                InternalNodeData::Element { tag, .. } => tag,
                _ => break,
            };
            if let Some(ex) = exclude {
                if tag == ex { break; }
            }
            if matches!(tag.as_str(), "dd" | "dt" | "li" | "optgroup" | "option" | "p" | "rb" | "rp" | "rt" | "rtc") {
                self.open_elements.pop();
            } else {
                break;
            }
        }
    }

    // --- Insertion Mode Handlers ---

    fn handle_initial(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Doctype(dt) => {
                self.doctype = Some(dt);
                self.insertion_mode = InsertionMode::BeforeHtml;
                None
            }
            HtmlToken::Character(text) if text.data.trim().is_empty() => None,
            HtmlToken::Comment(comment) => {
                let id = self.create_node(InternalNodeData::Comment(comment.data));
                self.append_node(self.root_id, id);
                None
            }
            other => {
                self.insertion_mode = InsertionMode::BeforeHtml;
                Some(other)
            }
        }
    }

    fn handle_before_html(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Doctype(_) => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedDoctype, "unexpected DOCTYPE before <html>");
                None
            }
            HtmlToken::Comment(comment) => {
                let id = self.create_node(InternalNodeData::Comment(comment.data));
                self.append_node(self.root_id, id);
                None
            }
            HtmlToken::Character(text) if text.data.trim().is_empty() => None,
            HtmlToken::StartTag(tag) if tag.name == "html" => {
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::BeforeHead;
                None
            }
            HtmlToken::EndTag(tag) if !matches!(tag.name.as_str(), "html" | "body" | "br" | "head") => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "unexpected end tag before <html>");
                None
            }
            other => {
                let id = self.create_node(InternalNodeData::Element { tag: "html".to_string(), attributes: HashMap::new() });
                self.append_node(self.root_id, id);
                self.open_elements.push(id);
                self.insertion_mode = InsertionMode::BeforeHead;
                Some(other)
            }
        }
    }

    fn handle_before_head(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) if text.data.trim().is_empty() => {
                self.insert_text(text.data);
                None
            }
            HtmlToken::Comment(comment) => {
                let id = self.create_node(InternalNodeData::Comment(comment.data));
                let current = self.current_node();
                self.append_node(current, id);
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "html" => {
                self.handle_in_body(HtmlToken::StartTag(tag))
            }
            HtmlToken::StartTag(tag) if tag.name == "head" => {
                let id = self.insert_html_element(tag);
                self.head_element_id = Some(id);
                self.insertion_mode = InsertionMode::InHead;
                None
            }
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "html" | "body" | "br" | "head") => {
                let id = self.create_node(InternalNodeData::Element { tag: "head".to_string(), attributes: HashMap::new() });
                let current = self.current_node();
                self.append_node(current, id);
                self.open_elements.push(id);
                self.head_element_id = Some(id);
                self.insertion_mode = InsertionMode::InHead;
                Some(HtmlToken::EndTag(tag))
            }
            other => {
                let id = self.create_node(InternalNodeData::Element { tag: "head".to_string(), attributes: HashMap::new() });
                let current = self.current_node();
                self.append_node(current, id);
                self.open_elements.push(id);
                self.head_element_id = Some(id);
                self.insertion_mode = InsertionMode::InHead;
                Some(other)
            }
        }
    }

    fn handle_in_head(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) if text.data.trim().is_empty() => {
                self.insert_text(text.data);
                None
            }
            HtmlToken::Comment(comment) => {
                let id = self.create_node(InternalNodeData::Comment(comment.data));
                let current = self.current_node();
                self.append_node(current, id);
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "html" => {
                self.handle_in_body(HtmlToken::StartTag(tag))
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "base" | "basefont" | "bgsound" | "link" | "meta") => {
                self.insert_html_element(tag);
                self.open_elements.pop();
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "title" => {
                let tag_name = tag.name.clone();
                self.insert_html_element(tag);
                self.tokenizer.set_raw_text_tag(Some(tag_name));
                self.original_insertion_mode = self.insertion_mode;
                self.insertion_mode = InsertionMode::Text;
                None
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "noscript" | "noframes" | "style") => {
                let tag_name = tag.name.clone();
                self.insert_html_element(tag);
                self.tokenizer.set_raw_text_tag(Some(tag_name));
                self.original_insertion_mode = self.insertion_mode;
                self.insertion_mode = InsertionMode::Text;
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "script" => {
                let tag_name = tag.name.clone();
                self.insert_html_element(tag);
                self.tokenizer.set_raw_text_tag(Some(tag_name));
                self.original_insertion_mode = self.insertion_mode;
                self.insertion_mode = InsertionMode::Text;
                None
            }
            HtmlToken::EndTag(tag) if tag.name == "head" => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::AfterHead;
                None
            }
            other => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::AfterHead;
                Some(other)
            }
        }
    }

    fn handle_in_head_noscript(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::EndTag(tag) if tag.name == "noscript" => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InHead;
                None
            }
            other => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "token in head-noscript reprocessed in head");
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InHead;
                Some(other)
            }
        }
    }

    fn handle_after_head(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) if text.data.trim().is_empty() => {
                self.insert_text(text.data);
                None
            }
            HtmlToken::Comment(comment) => {
                let id = self.create_node(InternalNodeData::Comment(comment.data));
                let current = self.current_node();
                self.append_node(current, id);
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "body" => {
                self.insert_html_element(tag);
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InBody;
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "frameset" => {
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InFrameset;
                None
            }
            other => {
                let id = self.create_node(InternalNodeData::Element { tag: "body".to_string(), attributes: HashMap::new() });
                let current = self.current_node();
                self.append_node(current, id);
                self.open_elements.push(id);
                self.insertion_mode = InsertionMode::InBody;
                Some(other)
            }
        }
    }

    fn handle_in_body(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) => {
                self.reconstruct_active_formatting_elements();
                let is_empty = text.data.trim().is_empty();
                self.insert_text(text.data);
                if !is_empty {
                    self.frameset_ok = false;
                }
                None
            }
            HtmlToken::Comment(comment) => {
                let id = self.create_node(InternalNodeData::Comment(comment.data));
                let current = self.current_node();
                self.append_node(current, id);
                None
            }
            HtmlToken::StartTag(tag) => self.handle_in_body_start_tag(tag),
            HtmlToken::EndTag(tag) => self.handle_in_body_end_tag(tag),
            HtmlToken::Eof => None,
            other => {
                println!("[TreeBuilder] Warning: unhandled token in InBody: {:?}", other);
                None
            }
        }
    }

    fn handle_in_body_start_tag(&mut self, tag: StartTagToken) -> Option<HtmlToken> {
        match tag.name.as_str() {
            "html" => {
                // Merge attributes into <html>
                if let Some(&html_id) = self.open_elements.first() {
                    if let InternalNodeData::Element { ref mut attributes, .. } = self.arena[html_id].data {
                        for (k, v) in tag.attributes {
                            attributes.entry(k).or_insert(v);
                        }
                    }
                }
                None
            }
            "body" => {
                if self.open_elements.len() < 2 || self.current_tag() != Some("body") {
                    // Ignore
                } else {
                    // Merge attributes into <body>
                    if let InternalNodeData::Element { ref mut attributes, .. } = self.arena[self.open_elements[1]].data {
                        for (k, v) in tag.attributes {
                            attributes.entry(k).or_insert(v);
                        }
                    }
                }
                None
            }
            "div" | "p" | "article" | "aside" | "main" | "nav" | "section" | "address" | "blockquote" | "center" | "details" | "dialog" | "dir" | "dl" | "fieldset" | "figcaption" | "figure" | "footer" | "header" | "hgroup" | "menu" | "ol" | "ul" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if self.has_open_element("p") {
                    self.close_p_element();
                }
                self.insert_html_element(tag);
                None
            }
            "li" => {
                self.frameset_ok = false;
                self.generate_implied_end_tags(Some("li"));
                self.insert_html_element(tag);
                None
            }
            "a" => {
                if let Some(pos) = self.active_formatting_elements.iter().rposition(|e| matches!(e, ActiveFormattingEntry::Element(id) if matches!(self.arena[*id].data, InternalNodeData::Element { ref tag, .. } if tag == "a"))) {
                    self.adoption_agency_algorithm("a");
                    // Step 3 says remove it if still there? Usually AAA handles it.
                }
                self.reconstruct_active_formatting_elements();
                let id = self.insert_html_element(tag);
                self.active_formatting_elements.push(ActiveFormattingEntry::Element(id));
                None
            }
            "b" | "big" | "code" | "em" | "font" | "i" | "s" | "small" | "strike" | "strong" | "tt" | "u" => {
                self.reconstruct_active_formatting_elements();
                let id = self.insert_html_element(tag);
                self.active_formatting_elements.push(ActiveFormattingEntry::Element(id));
                None
            }
            "table" => {
                if self.has_open_element("p") {
                    self.close_p_element();
                }
                self.insert_html_element(tag);
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InTable;
                None
            }
            "area" | "br" | "embed" | "img" | "input" | "keygen" | "wbr" => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(tag);
                self.open_elements.pop();
                self.frameset_ok = false;
                None
            }
            "input" => {
                // Specific handling for type=hidden doesn't clear frameset_ok
                self.reconstruct_active_formatting_elements();
                let type_attr = tag.attributes.get("type").map(|s| s.to_lowercase());
                self.insert_html_element(tag);
                self.open_elements.pop();
                if type_attr.as_deref() != Some("hidden") {
                    self.frameset_ok = false;
                }
                None
            }
            "select" => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(tag);
                self.frameset_ok = false;
                match self.insertion_mode {
                    InsertionMode::InTable | InsertionMode::InTableBody | InsertionMode::InRow | InsertionMode::InCell => {
                        self.insertion_mode = InsertionMode::InSelectInTable;
                    }
                    _ => self.insertion_mode = InsertionMode::InSelect,
                }
                None
            }
            _ => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(tag);
                None
            }
        }
    }

    fn handle_in_body_end_tag(&mut self, tag: EndTagToken) -> Option<HtmlToken> {
        match tag.name.as_str() {
            "body" => {
                self.insertion_mode = InsertionMode::AfterBody;
                None
            }
            "html" => {
                self.insertion_mode = InsertionMode::AfterBody;
                Some(HtmlToken::EndTag(tag))
            }
            "div" | "p" | "article" | "aside" | "main" | "nav" | "section" | "address" | "blockquote" | "center" | "details" | "dialog" | "dir" | "dl" | "fieldset" | "figcaption" | "figure" | "footer" | "header" | "hgroup" | "menu" | "ol" | "ul" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if !self.has_open_element(&tag.name) {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("end tag </{}> ignored", tag.name));
                    return None;
                }
                self.generate_implied_end_tags(Some(&tag.name));
                self.pop_until(&tag.name);
                None
            }
            "li" => {
                self.generate_implied_end_tags(Some("li"));
                self.pop_until("li");
                None
            }
            "a" | "b" | "big" | "code" | "em" | "font" | "i" | "s" | "small" | "strike" | "strong" | "tt" | "u" => {
                self.adoption_agency_algorithm(&tag.name);
                None
            }
            _ => {
                self.pop_until(&tag.name);
                None
            }
        }
    }

    fn close_p_element(&mut self) {
        self.generate_implied_end_tags(Some("p"));
        self.pop_until("p");
    }

    // --- Adoption Agency Algorithm (Full) ---

    fn adoption_agency_algorithm(&mut self, subject: &str) {
        // Step 1: Loop
        for _ in 0..8 {
            // Step 2
            let formatting_element_pos = self.active_formatting_elements.iter().rposition(|e| {
                match e {
                    ActiveFormattingEntry::Element(id) => {
                        match &self.arena[*id].data {
                            InternalNodeData::Element { tag, .. } => tag == subject,
                            _ => false,
                        }
                    }
                    _ => false,
                }
            });

            let Some(f_pos) = formatting_element_pos else {
                self.handle_in_body_end_tag_standard(subject);
                return;
            };
            
            let formatting_element_id = match self.active_formatting_elements[f_pos] {
                ActiveFormattingEntry::Element(id) => id,
                _ => unreachable!(),
            };

            // Step 3
            let open_pos = self.open_elements.iter().position(|&id| id == formatting_element_id);
            if open_pos.is_none() {
                self.parse_error(TreeBuilderErrorKind::AdoptionAgency, "formatting element not in open stack");
                self.active_formatting_elements.remove(f_pos);
                return;
            }
            let open_pos = open_pos.unwrap();

            // Step 4
            // (Is it in scope? Simplified check: if it's in open stack it's usually in scope)

            // Step 5: Find furthest block
            let mut furthest_block_pos = None;
            for i in open_pos + 1..self.open_elements.len() {
                let id = self.open_elements[i];
                if self.is_special_element(id) {
                    furthest_block_pos = Some(i);
                    break;
                }
            }

            // Step 6: If no furthest block
            let Some(fb_pos) = furthest_block_pos else {
                self.open_elements.truncate(open_pos);
                self.active_formatting_elements.remove(f_pos);
                return;
            };

            // Step 7: Common ancestor
            let common_ancestor_id = self.open_elements[open_pos - 1];

            // Step 8: Bookmark
            let bookmark = f_pos;

            // Step 9: Inner loop
            let mut node_pos = fb_pos;
            let mut last_node_id = self.open_elements[fb_pos];
            
            for _ in 0..3 {
                node_pos -= 1;
                let node_id = self.open_elements[node_pos];

                // Check if node is in formatting elements
                let f_entry_pos = self.active_formatting_elements.iter().position(|e| matches!(e, ActiveFormattingEntry::Element(id) if *id == node_id));
                
                if f_entry_pos.is_none() {
                    self.open_elements.remove(node_pos);
                    continue;
                }

                // If node is formatting element
                if node_id == formatting_element_id {
                    break;
                }

                // Clone node
                let cloned_id = self.clone_element(node_id);
                // Replace in stacks
                let f_e_pos = f_entry_pos.unwrap();
                self.active_formatting_elements[f_e_pos] = ActiveFormattingEntry::Element(cloned_id);
                self.open_elements[node_pos] = cloned_id;
                
                // Reparent
                let current_node_id = cloned_id;
                // Move last_node_id to be a child of current_node_id
                self.append_node(current_node_id, last_node_id);
                last_node_id = current_node_id;
            }

            // Step 10: Reparent last node to common ancestor
            self.insert_at_appropriate_place(last_node_id, Some(common_ancestor_id));

            // Step 11: Clone formatting element
            let new_formatting_id = self.clone_element(formatting_element_id);
            
            // Step 12: Move children of fb into new formatting element
            let fb_id = self.open_elements[fb_pos];
            let fb_children = self.arena[fb_id].children.clone();
            for child_id in fb_children {
                self.append_node(new_formatting_id, child_id);
            }

            // Step 13: Append new formatting element to furthest block
            self.append_node(fb_id, new_formatting_id);

            // Step 14: Remove old formatting element from active formatting elements
            self.active_formatting_elements.remove(f_pos);
            // Insert bookmark
            self.active_formatting_elements.insert(bookmark, ActiveFormattingEntry::Element(new_formatting_id));

            // Step 15: Remove formatting element from stack of open elements
            self.open_elements.remove(open_pos);
            // Insert after fb
            let new_fb_pos = self.open_elements.iter().position(|&id| id == fb_id).unwrap();
            self.open_elements.insert(new_fb_pos + 1, new_formatting_id);
        }
    }

    fn is_special_element(&self, id: usize) -> bool {
        match &self.arena[id].data {
            InternalNodeData::Element { tag, .. } => {
                matches!(tag.as_str(), "address" | "applet" | "area" | "article" | "aside" | "base" | "basefont" | "bgsound" | "blockquote" | "body" | "br" | "button" | "caption" | "center" | "col" | "colgroup" | "dd" | "details" | "dir" | "div" | "dl" | "dt" | "embed" | "fieldset" | "figcaption" | "figure" | "footer" | "form" | "frame" | "frameset" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "head" | "header" | "hgroup" | "hr" | "html" | "iframe" | "img" | "input" | "keygen" | "li" | "link" | "listing" | "main" | "marquee" | "menu" | "meta" | "nav" | "noembed" | "noframes" | "noscript" | "object" | "ol" | "p" | "param" | "plaintext" | "pre" | "script" | "section" | "select" | "source" | "style" | "summary" | "table" | "tbody" | "td" | "template" | "textarea" | "tfoot" | "th" | "thead" | "title" | "tr" | "track" | "ul" | "wbr" | "xmp")
            }
            _ => false,
        }
    }

    fn clone_element(&mut self, id: usize) -> usize {
        match &self.arena[id].data {
            InternalNodeData::Element { tag, attributes } => {
                self.create_node(InternalNodeData::Element { tag: tag.clone(), attributes: attributes.clone() })
            }
            _ => unreachable!(),
        }
    }

    fn handle_in_body_end_tag_standard(&mut self, tag: &str) {
        self.pop_until(tag);
    }

    // --- Other Handlers ---

    fn handle_text(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) => {
                self.insert_text(text.data);
                None
            }
            HtmlToken::EndTag(tag) => {
                self.open_elements.pop();
                self.insertion_mode = self.original_insertion_mode;
                None
            }
            HtmlToken::Eof => {
                self.open_elements.pop();
                self.insertion_mode = self.original_insertion_mode;
                Some(HtmlToken::Eof)
            }
            _ => None,
        }
    }

    fn handle_in_table(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::StartTag(tag) if tag.name == "caption" => {
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InCaption;
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "colgroup" => {
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InColumnGroup;
                None
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InTableBody;
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "tr" => {
                self.insert_element_at_current("tbody".to_string(), HashMap::new());
                self.insertion_mode = InsertionMode::InTableBody;
                Some(HtmlToken::StartTag(tag))
            }
            HtmlToken::EndTag(tag) if tag.name == "table" => {
                self.pop_until("table");
                self.reset_insertion_mode_appropriately();
                None
            }
            other => {
                self.foster_parenting = true;
                let ret = self.handle_in_body(other);
                self.foster_parenting = false;
                ret
            }
        }
    }

    fn handle_in_table_text(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        self.handle_in_table(token) // Dummy for now
    }

    fn handle_in_caption(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::EndTag(tag) if tag.name == "caption" => {
                self.pop_until("caption");
                self.insertion_mode = InsertionMode::InTable;
                None
            }
            other => self.handle_in_body(other),
        }
    }

    fn handle_in_column_group(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::StartTag(tag) if tag.name == "col" => {
                self.insert_html_element(tag);
                self.open_elements.pop();
                None
            }
            HtmlToken::EndTag(tag) if tag.name == "colgroup" => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTable;
                None
            }
            other => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTable;
                Some(other)
            }
        }
    }

    fn handle_in_table_body(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::StartTag(tag) if tag.name == "tr" => {
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InRow;
                None
            }
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTable;
                None
            }
            other => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTable;
                Some(other)
            }
        }
    }

    fn handle_in_row(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InCell;
                None
            }
            HtmlToken::EndTag(tag) if tag.name == "tr" => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTableBody;
                None
            }
            other => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTableBody;
                Some(other)
            }
        }
    }

    fn handle_in_cell(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
                self.pop_until(tag.name.as_str());
                self.insertion_mode = InsertionMode::InRow;
                None
            }
            other => self.handle_in_body(other),
        }
    }

    fn handle_in_select(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::StartTag(tag) if tag.name == "option" => {
                if self.current_tag() == Some("option") {
                    self.open_elements.pop();
                }
                self.insert_html_element(tag);
                None
            }
            HtmlToken::EndTag(tag) if tag.name == "select" => {
                self.pop_until("select");
                self.reset_insertion_mode_appropriately();
                None
            }
            other => {
                // Ignore or handle basic text
                None
            }
        }
    }

    fn handle_in_select_in_table(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        self.handle_in_select(token)
    }

    fn handle_in_template(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        self.handle_in_body(token)
    }

    fn handle_after_body(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::EndTag(tag) if tag.name == "html" => {
                self.insertion_mode = InsertionMode::AfterAfterBody;
                None
            }
            HtmlToken::Eof => None,
            other => {
                self.insertion_mode = InsertionMode::InBody;
                Some(other)
            }
        }
    }

    fn handle_after_after_body(&mut self, _token: HtmlToken) -> Option<HtmlToken> {
        None
    }

    fn handle_in_frameset(&mut self, _token: HtmlToken) -> Option<HtmlToken> {
        None
    }

    fn handle_after_frameset(&mut self, _token: HtmlToken) -> Option<HtmlToken> {
        None
    }

    fn reset_insertion_mode_appropriately(&mut self) {
        // Simplified reset
        self.insertion_mode = InsertionMode::InBody;
    }

    fn reconstruct_active_formatting_elements(&mut self) {
        if self.active_formatting_elements.is_empty() { return; }
        
        // Find last element or marker
        let mut last_marker = self.active_formatting_elements.len();
        for i in (0..self.active_formatting_elements.len()).rev() {
            if matches!(self.active_formatting_elements[i], ActiveFormattingEntry::Marker) {
                last_marker = i;
                break;
            }
            if let ActiveFormattingEntry::Element(id) = self.active_formatting_elements[i] {
                // Check if in open elements
                if self.open_elements.iter().any(|&oid| oid == id) {
                    last_marker = i;
                    break;
                }
            }
        }
        
        if last_marker == self.active_formatting_elements.len() {
            last_marker = 0; // Reconstruct all from start if no marker and none in open stack
        } else {
            last_marker += 1; // Reconstruct from NEXT
        }

        for i in last_marker..self.active_formatting_elements.len() {
            if let ActiveFormattingEntry::Element(id) = self.active_formatting_elements[i] {
                let InternalNodeData::Element { ref tag, ref attributes } = self.arena[id].data else { continue; };
                let nid = self.create_node(InternalNodeData::Element { tag: tag.clone(), attributes: attributes.clone() });
                self.append_node(self.current_node(), nid);
                self.open_elements.push(nid);
                self.active_formatting_elements[i] = ActiveFormattingEntry::Element(nid);
            }
        }
    }

    fn clear_formatting_to_last_marker(&mut self) {
        while let Some(entry) = self.active_formatting_elements.pop() {
            if matches!(entry, ActiveFormattingEntry::Marker) {
                break;
            }
        }
    }

    fn parse_error(&mut self, kind: TreeBuilderErrorKind, message: impl Into<String>) {
        self.errors.push(TreeBuilderError::new(kind, self.insertion_mode, message));
    }
}

pub fn build_document(input: &str) -> HtmlDocument {
    let builder = HtmlTreeBuilder::new(input);
    builder.run().document
}

pub fn build_document_with_errors(input: &str) -> TreeBuildOutput {
    let builder = HtmlTreeBuilder::new(input);
    builder.run()
}

pub fn build_fragment(input: &str) -> Vec<HtmlNode> {
    // Simplified fragment for now
    let builder = HtmlTreeBuilder::new(input);
    builder.run().document.children
}

pub fn build_fragment_with_errors(input: &str) -> TreeBuildOutput {
    let builder = HtmlTreeBuilder::new(input);
    builder.run()
}

#[cfg(test)]
mod tests {
    use super::build_document;
    use crate::ace::html::HtmlNode;
    use std::collections::HashMap;

    #[test]
    fn test_aaa_p_b_i() {
        let html = "<p><b><i>x</b>y</i></p>";
        let doc = build_document(html);
        // HTML Spec AAA expectation: <p><b><i>x</i></b><i>y</i></p>
        
        let p = &doc.children[0]; // In our arena builder, html nodes are flushed to children.
        if let HtmlNode::Element(el_p) = p {
            assert_eq!(el_p.tag, "p");
            // Element 0: <b><i>x</i></b>
            let b = &el_p.children[0];
            if let HtmlNode::Element(el_b) = b {
                assert_eq!(el_b.tag, "b");
                let i = &el_b.children[0];
                if let HtmlNode::Element(el_i) = i {
                    assert_eq!(el_i.tag, "i");
                    assert_eq!(el_i.children[0], HtmlNode::Text("x".to_string()));
                } else { panic!("Expected <i>"); }
            } else { panic!("Expected <b>"); }

            // Element 1: <i>y</i>
            let i2 = &el_p.children[1];
            if let HtmlNode::Element(el_i2) = i2 {
                assert_eq!(el_i2.tag, "i");
                assert_eq!(el_i2.children[0], HtmlNode::Text("y".to_string()));
            } else { panic!("Expected second <i>"); }
        } else {
            panic!("Expected <p> at root children, found {:?}", p);
        }
    }

    #[test]
    fn test_foster_parenting() {
        let html = "<table>hello<tr><td>cell</td></tr></table>";
        let doc = build_document(html);
        // Expectation: "hello" is foster-parented before the table
        assert_eq!(doc.children.len(), 2);
        assert_eq!(doc.children[0], HtmlNode::Text("hello".to_string()));
        if let HtmlNode::Element(el_table) = &doc.children[1] {
            assert_eq!(el_table.tag, "table");
        } else { panic!("Expected table"); }
    }
}
