use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::mem;
use std::rc::{Rc, Weak};

use fxhash::FxHashSet;
use html5ever::tendril::StrTendril;
use html5ever::{Attribute, QualName};
use markup5ever::interface::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use smallvec::SmallVec;

use super::types::{
    DoctypeToken, FragmentContext, HtmlDocument, HtmlElement, HtmlNode, Namespace, ParseError,
    ParseErrorKind, ParseErrorSource, ParseResult, ParseStats, ParserOptions,
};

pub type Handle = Rc<AceSinkNode>;
pub type WeakHandle = Weak<AceSinkNode>;

#[derive(Debug)]
pub enum AceSinkNodeData {
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
pub struct AceSinkNode {
    pub parent: RefCell<Option<WeakHandle>>,
    pub children: RefCell<Vec<Handle>>,
    pub data: AceSinkNodeData,
}

impl AceSinkNode {
    pub fn new(data: AceSinkNodeData) -> Handle {
        Rc::new(Self {
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
            data,
        })
    }
}

pub fn append_node(new_parent: &Handle, child: Handle) {
    let previous_parent = child.parent.replace(Some(Rc::downgrade(new_parent)));
    assert!(previous_parent.is_none(), "child already had a parent");
    new_parent.children.borrow_mut().push(child);
}

pub fn append_to_existing_text(prev: &Handle, text: &str) -> bool {
    match &prev.data {
        AceSinkNodeData::Text { contents } => {
            contents.borrow_mut().push_slice(text);
            true
        }
        _ => false,
    }
}

pub fn get_parent_and_index(target: &Handle) -> Option<(Handle, usize)> {
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

pub fn detach_from_parent(target: &Handle) {
    if let Some((parent, index)) = get_parent_and_index(target) {
        parent.children.borrow_mut().remove(index);
        *target.parent.borrow_mut() = None;
    }
}

#[derive(Debug)]
pub struct AceTreeSink {
    pub document: Handle,
    pub parse_errors: RefCell<Vec<ParseError>>,
    pub quirks_mode: Cell<QuirksMode>,
    pub current_line: Cell<u64>,
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
    pub fn into_parse_result(self, html: &str, options: &ParserOptions) -> ParseResult {
        let mut errors = super::detect_initial_errors(html);
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

pub fn convert_document(
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

pub fn convert_node(node: &Handle, scripting_enabled: bool) -> Option<HtmlNode> {
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

pub fn convert_children(parent: &Handle, scripting_enabled: bool) -> Vec<HtmlNode> {
    parent
        .children
        .borrow()
        .iter()
        .filter_map(|child| convert_node(child, scripting_enabled))
        .collect()
}

pub fn map_namespace(name: &QualName) -> Namespace {
    use html5ever::ns;
    if name.ns == ns!(svg) {
        Namespace::Svg
    } else if name.ns == ns!(mathml) {
        Namespace::MathMl
    } else {
        Namespace::Html
    }
}

pub fn attribute_name_to_string(name: &QualName) -> String {
    if let Some(prefix) = &name.prefix {
        format!("{} {}", prefix, name.local)
    } else {
        name.local.to_string()
    }
}

pub fn serialize_children_as_text(children: &[HtmlNode]) -> String {
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

impl TreeSink for AceTreeSink {
    type Handle = Handle;
    type Output = Self;
    type ElemName<'a> = html5ever::ExpandedName<'a> where Self: 'a;

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
