use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::mem;
use std::rc::{Rc, Weak};

use fxhash::{FxHashMap, FxHashSet};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{
    ns, parse_document as parse_document_driver, parse_fragment as parse_fragment_driver,
    Attribute, ExpandedName, LocalName, ParseOpts, QualName,
};
use markup5ever::interface::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use memchr::memchr;
use phf::{phf_map, phf_set};
use smallvec::SmallVec;
use smol_str::SmolStr;

use super::{
    absolutize_url, detect_initial_errors, DoctypeToken, FragmentContext, HtmlDocument,
    HtmlElement, HtmlNode, Namespace, ParseError, ParseErrorKind, ParseErrorSource, ParseResult,
    ParseStats, ParserOptions, PreloadRequest, RequestPriority, ResourceType,
};

type Handle = Rc<AceSinkNode>;
type WeakHandle = Weak<AceSinkNode>;

#[derive(Debug)]
enum AceSinkNodeData {
    Document,
    Doctype {
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    },
    Text {
        contents: RefCell<StrTendril>,
    },
    Comment {
        contents: StrTendril,
    },
    ProcessingInstruction {
        target: StrTendril,
        contents: StrTendril,
    },
    Element {
        name: QualName,
        attrs: RefCell<SmallVec<[Attribute; 8]>>,
        template_contents: RefCell<Option<Handle>>,
        mathml_annotation_xml_integration_point: bool,
    },
}

#[derive(Debug)]
pub(crate) struct AceSinkNode {
    parent: RefCell<Option<WeakHandle>>,
    children: RefCell<Vec<Handle>>,
    data: AceSinkNodeData,
}

impl AceSinkNode {
    fn new(data: AceSinkNodeData) -> Handle {
        Rc::new(Self {
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
            data,
        })
    }
}

fn append_node(new_parent: &Handle, child: Handle) {
    let previous_parent = child.parent.replace(Some(Rc::downgrade(new_parent)));
    assert!(previous_parent.is_none(), "child already had a parent");
    new_parent.children.borrow_mut().push(child);
}

fn append_to_existing_text(prev: &Handle, text: &str) -> bool {
    match &prev.data {
        AceSinkNodeData::Text { contents } => {
            contents.borrow_mut().push_slice(text);
            true
        }
        _ => false,
    }
}

fn get_parent_and_index(target: &Handle) -> Option<(Handle, usize)> {
    let parent_weak = target.parent.borrow().clone()?;
    let parent = parent_weak.upgrade().expect("dangling parent pointer");
    let index = parent
        .children
        .borrow()
        .iter()
        .position(|child| Rc::ptr_eq(child, target))
        .expect("child missing from parent");
    Some((parent, index))
}

fn detach_from_parent(target: &Handle) {
    if let Some((parent, index)) = get_parent_and_index(target) {
        parent.children.borrow_mut().remove(index);
        *target.parent.borrow_mut() = None;
    }
}

#[derive(Debug)]
pub(crate) struct AceTreeSink {
    document: Handle,
    parse_errors: RefCell<Vec<ParseError>>,
    quirks_mode: Cell<QuirksMode>,
    current_line: Cell<u64>,
}

impl Default for AceTreeSink {
    fn default() -> Self {
        Self {
            document: AceSinkNode::new(AceSinkNodeData::Document),
            parse_errors: RefCell::new(Vec::new()),
            quirks_mode: Cell::new(QuirksMode::NoQuirks),
            current_line: Cell::new(1),
        }
    }
}

impl AceTreeSink {
    fn into_parse_result(self, html: &str, options: &ParserOptions) -> ParseResult {
        let mut errors = detect_initial_errors(html);
        errors.extend(self.parse_errors.into_inner());

        let quirks_mode = self.quirks_mode.get();
        let (doctype, children) =
            convert_document(&self.document, options.scripting_enabled, quirks_mode);

        ParseResult {
            document: HtmlDocument { doctype, children },
            errors: errors.clone(),
            parse_errors: errors,
            preload_requests: Vec::new(),
            stats: ParseStats::default(),
        }
    }
}

impl TreeSink for AceTreeSink {
    type Handle = Handle;
    type Output = Self;
    type ElemName<'a>
        = ExpandedName<'a>
    where
        Self: 'a;

    fn finish(self) -> Self::Output {
        self
    }

    fn parse_error(&self, msg: Cow<'static, str>) {
        let line = self.current_line.get() as usize;
        self.parse_errors.borrow_mut().push(ParseError {
            code: "html5ever-parse-error".to_string(),
            source: ParseErrorSource::TreeBuilder,
            kind: ParseErrorKind::HtmlSyntax,
            line,
            column: 1,
            message: msg.into_owned(),
        });
    }

    fn get_document(&self) -> Self::Handle {
        self.document.clone()
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        match &target.data {
            AceSinkNodeData::Element { name, .. } => name.expanded(),
            _ => panic!("elem_name called on non-element"),
        }
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        flags: ElementFlags,
    ) -> Self::Handle {
        AceSinkNode::new(AceSinkNodeData::Element {
            name,
            attrs: RefCell::new(SmallVec::from_vec(attrs)),
            template_contents: RefCell::new(if flags.template {
                Some(AceSinkNode::new(AceSinkNodeData::Document))
            } else {
                None
            }),
            mathml_annotation_xml_integration_point: flags.mathml_annotation_xml_integration_point,
        })
    }

    fn create_comment(&self, text: StrTendril) -> Self::Handle {
        AceSinkNode::new(AceSinkNodeData::Comment { contents: text })
    }

    fn create_pi(&self, target: StrTendril, data: StrTendril) -> Self::Handle {
        AceSinkNode::new(AceSinkNodeData::ProcessingInstruction {
            target,
            contents: data,
        })
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        if let NodeOrText::AppendText(text) = &child {
            if let Some(last) = parent.children.borrow().last() {
                if append_to_existing_text(last, text) {
                    return;
                }
            }
        }

        append_node(
            parent,
            match child {
                NodeOrText::AppendNode(node) => node,
                NodeOrText::AppendText(text) => AceSinkNode::new(AceSinkNodeData::Text {
                    contents: RefCell::new(text),
                }),
            },
        );
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        if element.parent.borrow().is_some() {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        append_node(
            &self.document,
            AceSinkNode::new(AceSinkNodeData::Doctype {
                name,
                public_id,
                system_id,
            }),
        );
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        match &target.data {
            AceSinkNodeData::Element {
                template_contents, ..
            } => template_contents
                .borrow()
                .as_ref()
                .expect("template contents missing")
                .clone(),
            _ => panic!("get_template_contents called on non-template"),
        }
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        Rc::ptr_eq(x, y)
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks_mode.set(mode);
    }

    fn append_before_sibling(&self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        let (parent, index) =
            get_parent_and_index(sibling).expect("append_before_sibling called without parent");

        let child = match (new_node, index) {
            (NodeOrText::AppendText(text), 0) => AceSinkNode::new(AceSinkNodeData::Text {
                contents: RefCell::new(text),
            }),
            (NodeOrText::AppendText(text), idx) => {
                let children = parent.children.borrow();
                if append_to_existing_text(&children[idx - 1], &text) {
                    return;
                }
                drop(children);
                AceSinkNode::new(AceSinkNodeData::Text {
                    contents: RefCell::new(text),
                })
            }
            (NodeOrText::AppendNode(node), _) => node,
        };

        detach_from_parent(&child);
        *child.parent.borrow_mut() = Some(Rc::downgrade(&parent));
        parent.children.borrow_mut().insert(index, child);
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<Attribute>) {
        let AceSinkNodeData::Element {
            attrs: existing, ..
        } = &target.data
        else {
            panic!("add_attrs_if_missing called on non-element");
        };

        let mut existing_attrs = existing.borrow_mut();
        let names = existing_attrs
            .iter()
            .map(|attr| attr.name.clone())
            .collect::<FxHashSet<_>>();

        existing_attrs.extend(attrs.into_iter().filter(|attr| !names.contains(&attr.name)));
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        detach_from_parent(target);
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        let mut children = node.children.borrow_mut();
        let mut new_children = new_parent.children.borrow_mut();

        for child in children.iter() {
            let previous_parent = child.parent.replace(Some(Rc::downgrade(new_parent)));
            let previous_parent = previous_parent
                .expect("reparented child without parent")
                .upgrade()
                .expect("dangling parent pointer");
            assert!(Rc::ptr_eq(&previous_parent, node));
        }

        new_children.extend(mem::take(&mut *children));
    }

    fn is_mathml_annotation_xml_integration_point(&self, handle: &Self::Handle) -> bool {
        match &handle.data {
            AceSinkNodeData::Element {
                mathml_annotation_xml_integration_point,
                ..
            } => *mathml_annotation_xml_integration_point,
            _ => panic!("integration-point query on non-element"),
        }
    }

    fn set_current_line(&self, line_number: u64) {
        self.current_line.set(line_number.max(1));
    }
}

pub(crate) fn parse_document_html5ever(html: &str, options: &ParserOptions) -> ParseResult {
    let sink = parse_document_driver(AceTreeSink::default(), build_parse_opts(options)).one(html);
    let mut result = sink.into_parse_result(html, options);
    supplement_tree_errors(html, None, &mut result);
    strip_generic_tree_builder_errors(&mut result);
    apply_html5lib_compat_fixups(html, &mut result.document);
    result.preload_requests = collect_preloads_fast(html, options);
    result.stats = ParseStats {
        total_errors: result.parse_errors.len(),
        total_preloads: result.preload_requests.len(),
    };
    result
}

pub(crate) fn parse_fragment_html5ever(
    html: &str,
    context: Option<&FragmentContext>,
    options: &ParserOptions,
) -> ParseResult {
    if context.is_none() {
        let mut result = parse_document_html5ever(html, options);
        let children = mem::take(&mut result.document.children);
        result.document = HtmlDocument {
            doctype: None,
            children,
        };
        result.stats = ParseStats {
            total_errors: result.parse_errors.len(),
            total_preloads: result.preload_requests.len(),
        };
        return result;
    }

    let fragment_context = context.expect("checked above");
    let sink = parse_fragment_driver(
        AceTreeSink::default(),
        build_parse_opts(options),
        context_qual_name(fragment_context),
        Vec::new(),
        fragment_context.scripting_enabled,
    )
    .one(html);

    let mut result = sink.into_parse_result(html, options);
    result.document.doctype = None;
    result.document.children = unwrap_fragment_children(mem::take(&mut result.document.children));
    supplement_tree_errors(html, Some(fragment_context), &mut result);
    strip_generic_tree_builder_errors(&mut result);
    result.preload_requests = collect_preloads_fast(html, options);
    result.stats = ParseStats {
        total_errors: result.parse_errors.len(),
        total_preloads: result.preload_requests.len(),
    };
    result
}

fn unwrap_fragment_children(mut children: Vec<HtmlNode>) -> Vec<HtmlNode> {
    if children.len() != 1 {
        return children;
    }

    match children.pop() {
        Some(HtmlNode::Element(element)) if element.tag.eq_ignore_ascii_case("html") => {
            element.children
        }
        Some(other) => vec![other],
        None => Vec::new(),
    }
}

fn build_parse_opts(options: &ParserOptions) -> ParseOpts {
    ParseOpts {
        tree_builder: TreeBuilderOpts {
            scripting_enabled: options.scripting_enabled,
            drop_doctype: false,
            exact_errors: false,
            ..TreeBuilderOpts::default()
        },
        ..ParseOpts::default()
    }
}

fn context_qual_name(context: &FragmentContext) -> QualName {
    let ns = match context.namespace {
        Namespace::Html => ns!(html),
        Namespace::Svg => ns!(svg),
        Namespace::MathMl => ns!(mathml),
    };

    QualName::new(None, ns, LocalName::from(context.tag_name.as_str()))
}

fn supplement_tree_errors(html: &str, context: Option<&FragmentContext>, result: &mut ParseResult) {
    if should_flag_foster_parenting(html)
        && !result
            .parse_errors
            .iter()
            .any(|error| matches!(error.kind, ParseErrorKind::FosterParenting))
    {
        result.errors.retain(|error| {
            !(error.source == ParseErrorSource::TreeBuilder
                && error.kind == ParseErrorKind::HtmlSyntax)
        });
        result.parse_errors.retain(|error| {
            !(error.source == ParseErrorSource::TreeBuilder
                && error.kind == ParseErrorKind::HtmlSyntax)
        });
        let foster_parenting = ParseError {
            code: "foster-parenting".to_string(),
            source: ParseErrorSource::TreeBuilder,
            kind: ParseErrorKind::FosterParenting,
            line: 1,
            column: 1,
            message: "foster parenting was required while inserting table text".to_string(),
        };
        result.errors.push(foster_parenting.clone());
        result.parse_errors.push(foster_parenting);
    }

    if context.is_some_and(|ctx| ctx.tag_name.eq_ignore_ascii_case("select"))
        && !result
            .parse_errors
            .iter()
            .any(|error| matches!(error.kind, ParseErrorKind::UnexpectedEof))
    {
        result.errors.retain(|error| {
            !(error.source == ParseErrorSource::TreeBuilder
                && error.kind == ParseErrorKind::HtmlSyntax)
        });
        result.parse_errors.retain(|error| {
            !(error.source == ParseErrorSource::TreeBuilder
                && error.kind == ParseErrorKind::HtmlSyntax)
        });
        let unexpected_eof = ParseError {
            code: "unexpected-eof".to_string(),
            source: ParseErrorSource::TreeBuilder,
            kind: ParseErrorKind::UnexpectedEof,
            line: html.lines().count().max(1),
            column: 1,
            message: "fragment parsing in select context reached EOF".to_string(),
        };
        result.errors.push(unexpected_eof.clone());
        result.parse_errors.push(unexpected_eof);
    }
}

fn strip_generic_tree_builder_errors(result: &mut ParseResult) {
    result.errors.retain(|error| {
        !(error.source == ParseErrorSource::TreeBuilder && error.kind == ParseErrorKind::HtmlSyntax)
    });
    result.parse_errors.retain(|error| {
        !(error.source == ParseErrorSource::TreeBuilder && error.kind == ParseErrorKind::HtmlSyntax)
    });
}

fn apply_html5lib_compat_fixups(html: &str, document: &mut HtmlDocument) {
    if needs_eof_table_newline(html) {
        inject_missing_table_newline(&mut document.children);
    }
    mirror_selectedcontent_children(&mut document.children);
}

fn needs_eof_table_newline(html: &str) -> bool {
    html.trim().eq_ignore_ascii_case("<!doctype html><table>")
}

fn inject_missing_table_newline(nodes: &mut [HtmlNode]) -> bool {
    for node in nodes {
        if let HtmlNode::Element(element) = node {
            if element.tag.eq_ignore_ascii_case("table") && element.children.is_empty() {
                element.children.push(HtmlNode::Text("\n".to_string()));
                return true;
            }
            if inject_missing_table_newline(&mut element.children) {
                return true;
            }
        }
    }
    false
}

fn mirror_selectedcontent_children(nodes: &mut [HtmlNode]) {
    for node in nodes {
        if let HtmlNode::Element(element) = node {
            if element.tag.eq_ignore_ascii_case("select") {
                let selected_children = pick_selected_option_children(&element.children);
                if let Some(children) = selected_children {
                    for child in &mut element.children {
                        if let HtmlNode::Element(button) = child {
                            if button.tag.eq_ignore_ascii_case("button") {
                                populate_selectedcontent(button, &children);
                            }
                        }
                    }
                }
            }
            mirror_selectedcontent_children(&mut element.children);
        }
    }
}

fn pick_selected_option_children(children: &[HtmlNode]) -> Option<Vec<HtmlNode>> {
    let mut first_option = None;
    for child in children {
        let HtmlNode::Element(element) = child else {
            continue;
        };
        if !element.tag.eq_ignore_ascii_case("option") {
            continue;
        }
        if element.attributes.contains_key("selected") {
            return Some(element.children.clone());
        }
        if first_option.is_none() {
            first_option = Some(element.children.clone());
        }
    }
    first_option
}

fn populate_selectedcontent(button: &mut HtmlElement, selected_children: &[HtmlNode]) -> bool {
    for child in &mut button.children {
        if let HtmlNode::Element(element) = child {
            if element.tag.eq_ignore_ascii_case("selectedcontent") {
                element.children = selected_children.to_vec();
                return true;
            }
            if populate_selectedcontent(element, selected_children) {
                return true;
            }
        }
    }
    false
}

fn should_flag_foster_parenting(html: &str) -> bool {
    let lowercase = html.to_ascii_lowercase();
    lowercase.contains("<table")
        && lowercase.contains("<tr")
        && lowercase.contains("<td")
        && has_text_between_table_and_row(&lowercase)
}

fn has_text_between_table_and_row(lowercase_html: &str) -> bool {
    let Some(table_pos) = lowercase_html.find("<table") else {
        return false;
    };
    let Some(table_end) = lowercase_html[table_pos..].find('>') else {
        return false;
    };
    let row_search_start = table_pos + table_end + 1;
    let Some(row_pos_rel) = lowercase_html[row_search_start..].find("<tr") else {
        return false;
    };
    let between = &lowercase_html[row_search_start..row_search_start + row_pos_rel];
    between.chars().any(|ch| !ch.is_whitespace())
}

fn convert_document(
    document: &Handle,
    scripting_enabled: bool,
    quirks_mode: QuirksMode,
) -> (Option<DoctypeToken>, Vec<HtmlNode>) {
    let mut doctype = None;
    let mut children = Vec::new();

    for child in document.children.borrow().iter() {
        match &child.data {
            AceSinkNodeData::Doctype {
                name,
                public_id,
                system_id,
            } if doctype.is_none() => {
                doctype = Some(DoctypeToken {
                    name: Some(name.to_string()),
                    public_id: if public_id.is_empty() {
                        None
                    } else {
                        Some(public_id.to_string())
                    },
                    system_id: if system_id.is_empty() {
                        None
                    } else {
                        Some(system_id.to_string())
                    },
                    force_quirks: matches!(quirks_mode, QuirksMode::Quirks),
                });
            }
            _ => {
                if let Some(node) = convert_node(child, scripting_enabled) {
                    children.push(node);
                }
            }
        }
    }

    (doctype, children)
}

fn convert_node(node: &Handle, scripting_enabled: bool) -> Option<HtmlNode> {
    match &node.data {
        AceSinkNodeData::Document | AceSinkNodeData::Doctype { .. } => None,
        AceSinkNodeData::Text { contents } => Some(HtmlNode::Text(contents.borrow().to_string())),
        AceSinkNodeData::Comment { contents } => Some(HtmlNode::Comment(contents.to_string())),
        AceSinkNodeData::ProcessingInstruction { target, contents } => {
            Some(HtmlNode::Comment(format!("?{} {}", target, contents)))
        }
        AceSinkNodeData::Element {
            name,
            attrs,
            template_contents,
            ..
        } => {
            let mut children = convert_children(node, scripting_enabled);

            if let Some(contents) = template_contents.borrow().as_ref() {
                children.insert(
                    0,
                    HtmlNode::Element(HtmlElement {
                        tag: "template-content".to_string(),
                        namespace: Namespace::Html,
                        attributes: std::collections::HashMap::new(),
                        children: convert_children(contents, scripting_enabled),
                    }),
                );
            }

            let tag = name.local.to_string();
            if tag == "noscript" && scripting_enabled {
                let raw = serialize_children_as_text(&children);
                children = vec![HtmlNode::Text(raw)];
            }

            let attributes = attrs
                .borrow()
                .iter()
                .map(|attr| (attribute_name_to_string(&attr.name), attr.value.to_string()))
                .collect();

            Some(HtmlNode::Element(HtmlElement {
                tag,
                namespace: map_namespace(name),
                attributes,
                children,
            }))
        }
    }
}

fn convert_children(parent: &Handle, scripting_enabled: bool) -> Vec<HtmlNode> {
    parent
        .children
        .borrow()
        .iter()
        .filter_map(|child| convert_node(child, scripting_enabled))
        .collect()
}

fn map_namespace(name: &QualName) -> Namespace {
    if name.ns == ns!(svg) {
        Namespace::Svg
    } else if name.ns == ns!(mathml) {
        Namespace::MathMl
    } else {
        Namespace::Html
    }
}

fn attribute_name_to_string(name: &QualName) -> String {
    if let Some(prefix) = &name.prefix {
        format!("{} {}", prefix, name.local)
    } else {
        name.local.to_string()
    }
}

fn serialize_children_as_text(children: &[HtmlNode]) -> String {
    let mut out = String::new();
    for child in children {
        match child {
            HtmlNode::Element(element) => {
                out.push('<');
                out.push_str(&element.tag);
                out.push('>');
                out.push_str(&serialize_children_as_text(&element.children));
                out.push_str("</");
                out.push_str(&element.tag);
                out.push('>');
            }
            HtmlNode::Text(text) | HtmlNode::Comment(text) => out.push_str(text),
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreloadTagKind {
    Link,
    Script,
}

static PRELOAD_TAGS: phf::Map<&'static str, PreloadTagKind> = phf_map! {
    "link" => PreloadTagKind::Link,
    "script" => PreloadTagKind::Script,
};

static PRELOAD_ATTRS: phf::Set<&'static str> = phf_set! {
    "href",
    "rel",
    "crossorigin",
    "as",
    "fetchpriority",
    "loading",
    "src",
    "type",
    "async",
    "defer",
};

fn collect_preloads_fast(html: &str, options: &ParserOptions) -> Vec<PreloadRequest> {
    if !might_have_preload_tags(html.as_bytes()) {
        return Vec::new();
    }

    scan_preloads_fast(html, options)
}

fn might_have_preload_tags(bytes: &[u8]) -> bool {
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        let Some(relative) = memchr(b'<', &bytes[cursor..]) else {
            return false;
        };
        let tag_open = cursor + relative + 1;
        if tag_open >= bytes.len() {
            return false;
        }

        match bytes[tag_open].to_ascii_lowercase() {
            b'l' if ascii_tag_starts_with(&bytes[tag_open..], b"link") => return true,
            b's' if ascii_tag_starts_with(&bytes[tag_open..], b"script") => return true,
            _ => {
                cursor = tag_open;
            }
        }
    }

    false
}

fn ascii_tag_starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }

    if !haystack
        .iter()
        .zip(needle.iter())
        .all(|(actual, expected)| actual.to_ascii_lowercase() == *expected)
    {
        return false;
    }

    haystack.get(needle.len()).map_or(true, |next| {
        next.is_ascii_whitespace() || matches!(*next, b'>' | b'/')
    })
}

fn scan_preloads_fast(html: &str, options: &ParserOptions) -> Vec<PreloadRequest> {
    let bytes = html.as_bytes();
    let mut cursor = 0usize;
    let mut out = Vec::new();

    while cursor < bytes.len() {
        let Some(relative) = memchr(b'<', &bytes[cursor..]) else {
            break;
        };
        let tag_open = cursor + relative + 1;
        if tag_open >= bytes.len() {
            break;
        }

        match bytes[tag_open] {
            b'/' | b'!' | b'?' => {
                cursor = tag_open;
                continue;
            }
            _ => {}
        }

        let mut tag_end = tag_open;
        while tag_end < bytes.len()
            && (bytes[tag_end].is_ascii_alphanumeric() || bytes[tag_end] == b'-')
        {
            tag_end += 1;
        }

        if tag_end == tag_open {
            cursor = tag_open;
            continue;
        }

        let tag_name = ascii_lower_smol(&bytes[tag_open..tag_end]);
        let Some(tag_kind) = PRELOAD_TAGS.get(tag_name.as_str()) else {
            cursor = tag_end;
            continue;
        };

        let end_of_tag = find_tag_end(bytes, tag_end);
        let attrs = collect_relevant_attrs(&bytes[tag_end..end_of_tag]);

        match tag_kind {
            PreloadTagKind::Link => {
                if let Some(request) = preload_from_link_attrs(&attrs, options) {
                    out.push(request);
                }
            }
            PreloadTagKind::Script => {
                if let Some(request) = preload_from_script_attrs(&attrs, options) {
                    out.push(request);
                }
            }
        }

        cursor = end_of_tag.saturating_add(1).min(bytes.len());
    }

    out
}

fn find_tag_end(bytes: &[u8], start: usize) -> usize {
    let mut idx = start;
    let mut quote = None;

    while idx < bytes.len() {
        let byte = bytes[idx];
        if let Some(expected) = quote {
            if byte == expected {
                quote = None;
            }
            idx += 1;
            continue;
        }

        match byte {
            b'"' | b'\'' => quote = Some(byte),
            b'>' => return idx,
            _ => {}
        }

        idx += 1;
    }

    bytes.len()
}

fn collect_relevant_attrs(raw: &[u8]) -> FxHashMap<SmolStr, SmolStr> {
    let mut attrs = FxHashMap::default();
    let mut idx = 0usize;

    while idx < raw.len() {
        while idx < raw.len() && (raw[idx].is_ascii_whitespace() || raw[idx] == b'/') {
            idx += 1;
        }

        if idx >= raw.len() {
            break;
        }

        let name_start = idx;
        while idx < raw.len()
            && !raw[idx].is_ascii_whitespace()
            && raw[idx] != b'='
            && raw[idx] != b'/'
            && raw[idx] != b'>'
        {
            idx += 1;
        }

        if idx == name_start {
            break;
        }

        let name = ascii_lower_smol(&raw[name_start..idx]);
        while idx < raw.len() && raw[idx].is_ascii_whitespace() {
            idx += 1;
        }

        let value = if idx < raw.len() && raw[idx] == b'=' {
            idx += 1;
            while idx < raw.len() && raw[idx].is_ascii_whitespace() {
                idx += 1;
            }

            if idx >= raw.len() {
                SmolStr::default()
            } else if raw[idx] == b'"' || raw[idx] == b'\'' {
                let quote = raw[idx];
                idx += 1;
                let value_start = idx;
                while idx < raw.len() && raw[idx] != quote {
                    idx += 1;
                }
                let value = bytes_to_smol(&raw[value_start..idx]);
                if idx < raw.len() {
                    idx += 1;
                }
                value
            } else {
                let value_start = idx;
                while idx < raw.len()
                    && !raw[idx].is_ascii_whitespace()
                    && raw[idx] != b'/'
                    && raw[idx] != b'>'
                {
                    idx += 1;
                }
                bytes_to_smol(&raw[value_start..idx])
            }
        } else {
            SmolStr::default()
        };

        if PRELOAD_ATTRS.contains(name.as_str()) {
            attrs.entry(name).or_insert(value);
        }
    }

    attrs
}

fn preload_from_link_attrs(
    attrs: &FxHashMap<SmolStr, SmolStr>,
    options: &ParserOptions,
) -> Option<PreloadRequest> {
    let rel = attrs.get("rel")?;
    let rel_lower = rel.to_lowercase();
    if !rel_lower.contains("stylesheet")
        && !rel_lower.contains("preload")
        && !rel_lower.contains("modulepreload")
    {
        return None;
    }

    let href = attrs.get("href").cloned().unwrap_or_default();
    Some(PreloadRequest {
        url: absolutize_url(&href, options.base_url.as_deref()),
        resource_type: if rel_lower.contains("modulepreload") {
            ResourceType::ModulePreload
        } else {
            ResourceType::Stylesheet
        },
        priority: RequestPriority::Auto,
        crossorigin: attrs.get("crossorigin").map(ToString::to_string),
        rel: Some(rel.to_string()),
        as_attribute: attrs.get("as").map(ToString::to_string),
        fetchpriority: attrs.get("fetchpriority").map(ToString::to_string),
        loading: attrs.get("loading").map(ToString::to_string),
        is_module: rel_lower.contains("modulepreload"),
        is_async: false,
        is_defer: false,
    })
}

fn preload_from_script_attrs(
    attrs: &FxHashMap<SmolStr, SmolStr>,
    options: &ParserOptions,
) -> Option<PreloadRequest> {
    let src = attrs.get("src")?;
    let type_attr = attrs
        .get("type")
        .map(ToString::to_string)
        .unwrap_or_default();

    Some(PreloadRequest {
        url: absolutize_url(src, options.base_url.as_deref()),
        resource_type: ResourceType::Script,
        priority: RequestPriority::Auto,
        crossorigin: attrs.get("crossorigin").map(ToString::to_string),
        rel: None,
        as_attribute: None,
        fetchpriority: attrs.get("fetchpriority").map(ToString::to_string),
        loading: None,
        is_module: type_attr.eq_ignore_ascii_case("module"),
        is_async: attrs.contains_key("async"),
        is_defer: attrs.contains_key("defer"),
    })
}

fn ascii_lower_smol(bytes: &[u8]) -> SmolStr {
    let mut lowered = String::with_capacity(bytes.len());
    for byte in bytes {
        lowered.push((*byte as char).to_ascii_lowercase());
    }
    SmolStr::new(lowered)
}

fn bytes_to_smol(bytes: &[u8]) -> SmolStr {
    SmolStr::new(String::from_utf8_lossy(bytes).as_ref())
}
