use crate::ace::html::{
    DoctypeToken, EndTagToken, HtmlDocument, HtmlElement, HtmlNode, HtmlToken,
    StartTagToken, TokenizerErrorSource,
    HtmlTokenizer,
    PreloadScanner, PreloadRequest,
    lexer::LexerState,
    arena::{NodeArena, NodeId},
    interner::{StringInterner, StringId},
    small_attr_map::SmallAttributeMap,
};
use std::collections::HashMap;

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
            line: 1, // Default or placeholder
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
        tag: StringId,
        namespace: crate::ace::html::Namespace,
        attributes: SmallAttributeMap,
        slot_name: Option<StringId>,
        is_value: Option<StringId>,
        shadow_root_mode: Option<crate::ace::html::ShadowRootMode>,
        shadow_root: Option<Box<crate::ace::html::HtmlDocument>>,
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

    // Arena allocator para nodes - substitui Vec<InternalNode>
    arena: NodeArena,
    // Mapeamento de NodeId (arena) para índice interno (para open_elements, etc.)
    node_id_to_index: HashMap<NodeId, usize>,
    index_to_node_id: HashMap<usize, NodeId>,
    next_internal_index: usize,
    
    root_id: NodeId,
    open_elements: Vec<NodeId>,
    active_formatting_elements: Vec<ActiveFormattingEntry>,
    template_insertion_modes: Vec<InsertionMode>,
    preload_scanner: PreloadScanner,
    preload_requests: Vec<PreloadRequest>,

    doctype: Option<DoctypeToken>,
    errors: Vec<TreeBuilderError>,
    head_element_id: Option<NodeId>,
    form_element_id: Option<NodeId>,
    
    #[allow(dead_code)]
    scripting_enabled: bool,
    frameset_ok: bool,
    foster_parenting: bool,
    quirks_mode: bool,
    pending_table_characters: Vec<char>,
    
    // String interner para tag names e attribute names
    interner: StringInterner,
}

impl<'a> HtmlTreeBuilder<'a> {
    pub fn is_whitespace(ch: char) -> bool {
        matches!(ch, ' ' | '\t' | '\r' | '\n' | '\x0C')
    }

    pub fn new(input: &'a str) -> Self {
        let arena = NodeArena::new();
        let interner = StringInterner::new();
        
        // Cria node documento na arena
        let root_data = InternalNodeData::Document;
        let root_id = arena.alloc(root_data);
        
        Self {
            tokenizer: HtmlTokenizer::new(input),
            insertion_mode: InsertionMode::Initial,
            original_insertion_mode: InsertionMode::Initial,

            arena,
            node_id_to_index: HashMap::new(),
            index_to_node_id: HashMap::new(),
            next_internal_index: 0,
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
            quirks_mode: false,
            pending_table_characters: Vec::new(),
            preload_scanner: PreloadScanner::new(),
            preload_requests: Vec::new(),
            template_insertion_modes: Vec::new(),
            interner,
        }
    }

    pub fn speculate(&mut self) {
        let remaining = self.tokenizer.lexer.remaining_input();
        if !remaining.is_empty() {
            let requests = self.preload_scanner.scan(&remaining);
            for req in requests {
                // simple deduplication or just push? for now just push
                if !self.preload_requests.iter().any(|r| r.url == req.url) {
                    self.preload_requests.push(req);
                }
            }
        }
    }

    pub fn run(mut self) -> TreeBuildOutput {
        let mut reprocess: Option<HtmlToken> = None;
        loop {
            // Trigger speculation periodically or when blocking
            self.speculate();

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
            preload_requests: self.preload_requests,
        }
    }

    fn convert_to_html_node(&self, id: usize) -> HtmlNode {
        let node = &self.arena[id];
        match &node.data {
            InternalNodeData::Element { tag, namespace, attributes, slot_name, is_value, shadow_root_mode, shadow_root } => {
                let mut children = Vec::new();
                for &child_id in &node.children {
                    children.push(self.convert_to_html_node(child_id));
                }
                HtmlNode::Element(HtmlElement {
                    tag: self.interner.resolve(*tag).unwrap_or("").to_string(),
                    namespace: *namespace,
                    attributes: self.resolve_attributes(attributes),
                    children,
                    slot_name: slot_name.and_then(|id| self.interner.resolve(id).map(str::to_string)),
                    is_value: is_value.and_then(|id| self.interner.resolve(id).map(str::to_string)),
                    shadow_root_mode: *shadow_root_mode,
                    shadow_root: shadow_root.clone(),
                })
            }
            InternalNodeData::Text(s) => HtmlNode::Text(s.clone()),
            InternalNodeData::Comment(s) => HtmlNode::Comment(s.clone()),
            InternalNodeData::Document => unreachable!("nested document node"),
        }
    }

    fn resolve_attributes(&self, attributes: &SmallAttributeMap) -> HashMap<String, String> {
        attributes
            .iter()
            .filter_map(|(key, value)| {
                Some((
                    self.interner.resolve(key)?.to_string(),
                    self.interner.resolve(value)?.to_string(),
                ))
            })
            .collect()
    }

    fn collect_tokenizer_errors(&mut self) {
        let tok_errors = self.tokenizer.take_errors();
        for err in tok_errors {
            let mut be = TreeBuilderError::new(TreeBuilderErrorKind::TokenizerError, self.insertion_mode, err.message)
                .with_pos(err.line, err.column);
            be.source = match err.source {
                TokenizerErrorSource::Lexer => TreeBuilderErrorSource::Lexer,
                TokenizerErrorSource::Tokenizer => TreeBuilderErrorSource::Tokenizer,
            };
            self.errors.push(be);
        }
    }

    fn process_token(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        if let Some(id) = self.open_elements.last() {
            if let InternalNodeData::Element { namespace, tag, .. } = &self.arena[*id].data {
                if *namespace != crate::ace::html::Namespace::Html {
                    if !self.is_mathml_text_integration_point(*id) && !self.is_html_integration_point(*id) {
                        return self.handle_foreign_content(token);
                    }
                    if let crate::ace::html::Namespace::MathMl = namespace {
                        if tag == "annotation-xml" {
                            if let InternalNodeData::Element { ref attributes, .. } = self.arena[*id].data {
                                if let Some(attr) = attributes.get("encoding") {
                                    if attr == "text/html" || attr == "application/xhtml+xml" {
                                        // Treat as HTML integration point
                                    } else {
                                        return self.handle_foreign_content(token);
                                    }
                                } else {
                                    return self.handle_foreign_content(token);
                                }
                            }
                        }
                    }
                }
            }
        }

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
            InsertionMode::AfterAfterFrameset => self.handle_after_after_frameset(token),
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

    fn insert_at_appropriate_place(&mut self, nid: usize, override_target: Option<usize>) {
        let (target, before) = if let Some(target_id) = override_target {
            (target_id, None)
        } else {
            let t = self.current_node();
            if self.foster_parenting && matches!(self.arena[t].data, InternalNodeData::Element { ref tag, .. } if matches!(tag.as_str(), "table" | "tbody" | "tfoot" | "thead" | "tr")) {
                // WHATWG 13.2.6.4.1: Foster parenting
                let mut last_template = None;
                let mut last_table = None;
                for (i, &id) in self.open_elements.iter().enumerate().rev() {
                    if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                        if tag == "template" && last_template.is_none() {
                            last_template = Some(i);
                        }
                        if tag == "table" && last_table.is_none() {
                            last_table = Some(i);
                        }
                    }
                }

                if let (Some(temp_pos), Some(table_pos)) = (last_template, last_table) {
                    if temp_pos > table_pos {
                        (self.open_elements[temp_pos], None)
                    } else {
                        self.get_foster_parent(table_pos)
                    }
                } else if let Some(temp_pos) = last_template {
                    (self.open_elements[temp_pos], None)
                } else if let Some(table_pos) = last_table {
                    self.get_foster_parent(table_pos)
                } else {
                    (self.open_elements[0], None) // html element
                }
            } else {
                (t, None)
            }
        };

        if let Some(before_id) = before {
            self.insert_before(target, nid, before_id);
        } else {
            self.append_node(target, nid);
        }
    }

    fn get_foster_parent(&self, table_pos: usize) -> (usize, Option<usize>) {
        let table_id = self.open_elements[table_pos];
        if let Some(parent_id) = self.arena[table_id].parent {
            (parent_id, Some(table_id))
        } else {
            (self.open_elements[table_pos - 1], None)
        }
    }

    fn insert_before(&mut self, parent_id: usize, child_id: usize, before_id: usize) {
        if let Some(old_parent) = self.arena[child_id].parent {
            self.arena[old_parent].children.retain(|&id| id != child_id);
        }
        
        self.arena[child_id].parent = Some(parent_id);
        let pos = self.arena[parent_id].children.iter().position(|&id| id == before_id).unwrap_or(self.arena[parent_id].children.len());
        self.arena[parent_id].children.insert(pos, child_id);
    }

    fn reconstruct_active_formatting_elements(&mut self) {
        // WHATWG 13.2.6.4.3
        if self.active_formatting_elements.is_empty() { return; }
        
        let last_entry_pos = self.active_formatting_elements.len() - 1;
        if matches!(self.active_formatting_elements[last_entry_pos], ActiveFormattingEntry::Marker) { return; }
        if let ActiveFormattingEntry::Element(id) = self.active_formatting_elements[last_entry_pos] {
            if self.open_elements.contains(&id) { return; }
        }

        let mut entry_pos = last_entry_pos;
        loop {
            if entry_pos == 0 { break; }
            entry_pos -= 1;
            let entry = &self.active_formatting_elements[entry_pos];
            if matches!(entry, ActiveFormattingEntry::Marker) {
                entry_pos += 1;
                break;
            }
            if let ActiveFormattingEntry::Element(id) = entry {
                if self.open_elements.contains(id) {
                    entry_pos += 1;
                    break;
                }
            }
        }

        while entry_pos < self.active_formatting_elements.len() {
            let id = if let ActiveFormattingEntry::Element(id) = self.active_formatting_elements[entry_pos] { id } else { unreachable!() };
            let new_nid = self.clone_element(id);
            self.insert_at_appropriate_place(new_nid, None);
            self.open_elements.push(new_nid);
            self.active_formatting_elements[entry_pos] = ActiveFormattingEntry::Element(new_nid);
            entry_pos += 1;
        }
    }

    fn adjust_svg_tag_name(&self, tag: &mut String) {
        match tag.as_str() {
            "altglyph" => *tag = "altGlyph".to_string(),
            "altglyphdef" => *tag = "altGlyphDef".to_string(),
            "altglyphitem" => *tag = "altGlyphItem".to_string(),
            "animatecolor" => *tag = "animateColor".to_string(),
            "animatemotion" => *tag = "animateMotion".to_string(),
            "animatetransform" => *tag = "animateTransform".to_string(),
            "clippath" => *tag = "clipPath".to_string(),
            "feblend" => *tag = "feBlend".to_string(),
            "fecolormatrix" => *tag = "feColorMatrix".to_string(),
            "fecomponenttransfer" => *tag = "feComponentTransfer".to_string(),
            "fecomposite" => *tag = "feComposite".to_string(),
            "feconvolvematrix" => *tag = "feConvolveMatrix".to_string(),
            "fediffuselighting" => *tag = "feDiffuseLighting".to_string(),
            "fedisplacementmap" => *tag = "feDisplacementMap".to_string(),
            "fedistantlight" => *tag = "feDistantLight".to_string(),
            "fedrop_shadow" => *tag = "feDropShadow".to_string(),
            "feflood" => *tag = "feFlood".to_string(),
            "fefunca" => *tag = "feFuncA".to_string(),
            "fefuncb" => *tag = "feFuncB".to_string(),
            "fefuncg" => *tag = "feFuncG".to_string(),
            "fefuncr" => *tag = "feFuncR".to_string(),
            "fegaussianblur" => *tag = "feGaussianBlur".to_string(),
            "feimage" => *tag = "feImage".to_string(),
            "femerge" => *tag = "feMerge".to_string(),
            "femergenode" => *tag = "feMergeNode".to_string(),
            "femorphology" => *tag = "feMorphology".to_string(),
            "feoffset" => *tag = "feOffset".to_string(),
            "fepointlight" => *tag = "fePointLight".to_string(),
            "fespecularlighting" => *tag = "feSpecularLighting".to_string(),
            "fespotlight" => *tag = "feSpotLight".to_string(),
            "fetile" => *tag = "feTile".to_string(),
            "feturbulence" => *tag = "feTurbulence".to_string(),
            "foreignobject" => *tag = "foreignObject".to_string(),
            "glyphref" => *tag = "glyphRef".to_string(),
            "lineargradient" => *tag = "linearGradient".to_string(),
            "radialgradient" => *tag = "radialGradient".to_string(),
            "textpath" => *tag = "textPath".to_string(),
            _ => {}
        }
    }

    fn adjust_mathml_attributes(&self, attributes: &mut HashMap<String, String>) {
        let mappings = [
            ("definitionurl", "definitionURL"),
        ];
        for (old, new) in mappings {
            if let Some(val) = attributes.remove(old) {
                attributes.insert(new.to_string(), val);
            }
        }
    }

    fn adjust_svg_attributes(&self, attributes: &mut HashMap<String, String>) {
        let mappings = [
            ("attributename", "attributeName"),
            ("attributetype", "attributeType"),
            ("basefrequency", "baseFrequency"),
            ("baseprofile", "baseProfile"),
            ("calcmode", "calcMode"),
            ("clippathunits", "clipPathUnits"),
            ("diffuseconstant", "diffuseConstant"),
            ("edgemode", "edgeMode"),
            ("filterunits", "filterUnits"),
            ("glyphref", "glyphRef"),
            ("gradienttransform", "gradientTransform"),
            ("gradientunits", "gradientUnits"),
            ("kernelmatrix", "kernelMatrix"),
            ("kernelunitlength", "kernelUnitLength"),
            ("keypoints", "keyPoints"),
            ("keysplines", "keySplines"),
            ("keytimes", "keyTimes"),
            ("lengthadjust", "lengthAdjust"),
            ("limitingconeangle", "limitingConeAngle"),
            ("markerheight", "markerHeight"),
            ("markerunits", "markerUnits"),
            ("markerwidth", "markerWidth"),
            ("maskcontentunits", "maskContentUnits"),
            ("maskunits", "maskUnits"),
            ("numoctaves", "numOctaves"),
            ("pathlength", "pathLength"),
            ("patterncontentunits", "patternContentUnits"),
            ("patterntransform", "patternTransform"),
            ("patternunits", "patternUnits"),
            ("pointsatx", "pointsAtX"),
            ("pointsaty", "pointsAtY"),
            ("pointsatz", "pointsAtZ"),
            ("preservealpha", "preserveAlpha"),
            ("preserveaspectratio", "preserveAspectRatio"),
            ("primitiveunits", "primitiveUnits"),
            ("refx", "refX"),
            ("refy", "refY"),
            ("repeatcount", "repeatCount"),
            ("repeatdur", "repeatDur"),
            ("requiredextensions", "requiredExtensions"),
            ("requiredfeatures", "requiredFeatures"),
            ("specularconstant", "specularConstant"),
            ("specularexponent", "specularExponent"),
            ("spreadmethod", "spreadMethod"),
            ("startoffset", "startOffset"),
            ("stddeviation", "stdDeviation"),
            ("stitchtiles", "stitchTiles"),
            ("surfacescale", "surfaceScale"),
            ("systemlanguage", "systemLanguage"),
            ("tablevalues", "tableValues"),
            ("targetx", "targetX"),
            ("targety", "targetY"),
            ("viewbox", "viewBox"),
            ("viewtarget", "viewTarget"),
            ("xchannelselector", "xChannelSelector"),
            ("ychannelselector", "yChannelSelector"),
            ("zoomandpan", "zoomAndPan"),
        ];

        for (old, new) in mappings {
            if let Some(val) = attributes.remove(old) {
                attributes.insert(new.to_string(), val);
            }
        }
    }

    fn adjust_foreign_attributes(&self, attributes: &mut HashMap<String, String>) {
        let mappings = [
            ("xlink:actuate", "xlink:actuate"),
            ("xlink:arcrole", "xlink:arcrole"),
            ("xlink:href", "xlink:href"),
            ("xlink:role", "xlink:role"),
            ("xlink:show", "xlink:show"),
            ("xlink:title", "xlink:title"),
            ("xlink:type", "xlink:type"),
            ("xml:base", "xml:base"),
            ("xml:lang", "xml:lang"),
            ("xml:space", "xml:space"),
            ("xmlns", "xmlns"),
            ("xmlns:xlink", "xmlns:xlink"),
        ];

        for (old, new_key) in mappings {
            if let Some(val) = attributes.remove(old) {
                attributes.insert(new_key.to_string(), val);
            }
        }
    }

    fn insert_html_element(&mut self, tag: StartTagToken) -> usize {
        self.insert_element(tag, crate::ace::html::Namespace::Html)
    }

    fn insert_element(&mut self, mut tag: StartTagToken, ns: crate::ace::html::Namespace) -> usize {
        let is_self_closing = tag.self_closing && ns != crate::ace::html::Namespace::Html;
        
        self.adjust_foreign_attributes(&mut tag.attributes);
        if ns == crate::ace::html::Namespace::MathMl {
             self.adjust_mathml_attributes(&mut tag.attributes);
        }
        if ns == crate::ace::html::Namespace::Svg {
             self.adjust_svg_tag_name(&mut tag.name);
             self.adjust_svg_attributes(&mut tag.attributes);
        }

        // Extract special attributes for Shadow DOM and Custom Elements
        let slot_name = tag.attributes.remove("slot").map(|s| StringId::intern(&s));
        let is_value = tag.attributes.remove("is").map(|s| StringId::intern(&s));
        let shadow_root_mode = tag.attributes.remove("shadowrootmode").and_then(|mode| {
            match mode.as_str() {
                "open" => Some(crate::ace::html::ShadowRootMode::Open),
                "closed" => Some(crate::ace::html::ShadowRootMode::Closed),
                _ => None,
            }
        });

        // Convert attributes to SmallAttributeMap with StringId
        let mut attributes = SmallAttributeMap::with_capacity(tag.attributes.len());
        for (key, value) in tag.attributes {
            let key_id = StringId::intern(&key);
            let value_id = StringId::intern(&value);
            attributes.insert(key_id, value_id);
        }

        let tag_id = StringId::intern(&tag.name);

        let nid = self.create_node(InternalNodeData::Element {
            tag: tag_id,
            namespace: ns,
            attributes,
        });
        
        // Store special properties in the node's extended data
        self.set_element_special_properties(nid, slot_name, is_value, shadow_root_mode);
        
        self.insert_at_appropriate_place(nid, None);
        
        if !is_self_closing {
            self.open_elements.push(nid);
        }
        nid
    }

    fn insert_element_at_current(&mut self, tag: &str, attributes: SmallAttributeMap) -> usize {
        let tag_id = StringId::intern(tag);
        let nid = self.create_node(InternalNodeData::Element {
            tag: tag_id,
            namespace: crate::ace::html::Namespace::Html,
            attributes,
        });
        self.insert_at_appropriate_place(nid, None);
        self.open_elements.push(nid);
        nid
    }

    // Fase 2: Recursos Avançados - Shadow DOM e Custom Elements
    
    fn set_element_special_properties(
        &mut self, 
        node_id: usize, 
        slot_name: Option<StringId>, 
        is_value: Option<StringId>, 
        shadow_root_mode: Option<crate::ace::html::ShadowRootMode>
    ) {
        if let InternalNodeData::Element { 
            slot_name: ref mut stored_slot,
            is_value: ref mut stored_is,
            shadow_root_mode: ref mut stored_shadow,
            .. 
        } = self.arena[node_id].data {
            *stored_slot = slot_name;
            *stored_is = is_value;
            *stored_shadow = shadow_root_mode;
        }
    }

    fn validate_custom_element_name(name: &str) -> bool {
        // Regras da especificação Web Components para custom element names
        // Deve conter pelo menos um hífen
        if !name.contains('-') { 
            return false; 
        }
        // Não pode começar com hífen
        if name.starts_with('-') { 
            return false; 
        }
        // Não pode terminar com hífen
        if name.ends_with('-') { 
            return false; 
        }
        // Primeiro caractere deve ser lowercase ASCII, underscore ou colon
        let first_char = name.chars().next().unwrap_or('\0');
        if !first_char.is_ascii_lowercase() && first_char != '_' && first_char != ':' {
            return false;
        }
        // Todos os caracteres devem ser válidos (ASCII alphanumeric, hífen, underscore, ponto, colon)
        for ch in name.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' && ch != '.' && ch != ':' {
                return false;
            }
        }
        true
    }

    fn process_declarative_shadow_root(&mut self, host_id: usize, mode: crate::ace::html::ShadowRootMode) {
        // Coleta os filhos atuais do elemento host
        let children_to_move: Vec<usize> = {
            let node = &self.arena[host_id];
            if let InternalNodeData::Element { ref children, .. } = node.data {
                children.clone()
            } else {
                Vec::new()
            }
        };

        if children_to_move.is_empty() {
            return;
        }

        // Cria um novo documento para o shadow root
        let mut shadow_doc = crate::ace::html::HtmlDocument {
            doctype: None,
            children: Vec::new(),
        };

        // Move cada filho para o shadow document
        for child_id in children_to_move {
            // Remove o filho do nó host
            if let Some(parent_id) = self.arena[child_id].parent {
                self.arena[parent_id].children.retain(|&id| id != child_id);
            }
            self.arena[child_id].parent = None;

            // Converte para HtmlNode e adiciona ao shadow doc
            let html_node = self.convert_to_html_node(child_id);
            shadow_doc.children.push(html_node);
        }

        // Armazena o shadow root no nó host
        if let InternalNodeData::Element { ref mut shadow_root, .. } = self.arena[host_id].data {
            *shadow_root = Some(Box::new(shadow_doc));
        }
    }

    fn handle_slot_element(&mut self, tag_name: &str, mut attributes: HashMap<String, String>) -> usize {
        // Elementos <slot> são elementos especiais no Shadow DOM
        let slot_name = attributes.remove("name").map(|s| StringId::intern(&s));
        
        // Convert attributes to SmallAttributeMap with StringId
        let mut attr_map = SmallAttributeMap::with_capacity(attributes.len());
        for (key, value) in attributes {
            let key_id = StringId::intern(&key);
            let value_id = StringId::intern(&value);
            attr_map.insert(key_id, value_id);
        }
        
        let tag_id = StringId::intern(tag_name);
        
        let nid = self.create_node(InternalNodeData::Element {
            tag: tag_id,
            namespace: crate::ace::html::Namespace::Html,
            attributes: attr_map,
        });
        
        // Define o nome do slot nas propriedades especiais
        self.set_element_special_properties(nid, slot_name, None, None);
        
        self.insert_at_appropriate_place(nid, None);
        self.open_elements.push(nid);
        nid
    }

    // Constantes com elementos SVG e MathML completos
    const SVG_ELEMENTS: &'static [&'static str] = &[
        "svg", "animate", "animateMotion", "animateTransform", "circle", "clipPath",
        "defs", "desc", "ellipse", "feBlend", "feColorMatrix", "feComponentTransfer",
        "feComposite", "feConvolveMatrix", "feDiffuseLighting", "feDisplacementMap",
        "feDistantLight", "feDropShadow", "feFlood", "feFuncA", "feFuncB", "feFuncG",
        "feFuncR", "feGaussianBlur", "feImage", "feMerge", "feMergeNode", "feMorphology",
        "feOffset", "fePointLight", "feSpecularLighting", "feSpotLight", "feTile",
        "feTurbulence", "filter", "foreignObject", "g", "image", "line", "linearGradient",
        "marker", "mask", "metadata", "mpath", "path", "pattern", "polygon", "polyline",
        "radialGradient", "rect", "stop", "switch", "symbol", "text", "textPath", "tspan",
        "use", "view"
    ];

    const MATHML_ELEMENTS: &'static [&'static str] = &[
        "math", "mi", "mn", "mo", "mrow", "msup", "msub", "msubsup", "mfrac", "msqrt",
        "mroot", "mtable", "mtr", "mtd", "mth", "mfenced", "menclose", "merror", "mpadded",
        "mphantom", "maction", "semantics", "annotation", "annotation-xml"
    ];

    fn is_svg_element(tag_name: &str) -> bool {
        Self::SVG_ELEMENTS.contains(&tag_name)
    }

    fn is_mathml_element(tag_name: &str) -> bool {
        Self::MATHML_ELEMENTS.contains(&tag_name)
    }

    // - [x] Fase 5: Templates e Verificação
    // - [x] Implementar suporte básico a `<template>`.
    // - [x] Executar bateria de testes de estresse (HTML5Lib compat).
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

    fn has_element_in_scope_with_id(&self, target_id: usize) -> bool {
        for &id in self.open_elements.iter().rev() {
            if id == target_id {
                return true;
            }
            if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                if matches!(tag.as_str(), "applet" | "caption" | "html" | "table" | "td" | "th" | "marquee" | "object" | "template") {
                    return false;
                }
            }
        }
        false
    }

    fn has_element_in_scope(&self, tag_name: &str) -> bool {
        for &id in self.open_elements.iter().rev() {
            if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                if tag == tag_name {
                    return true;
                }
                if matches!(tag.as_str(), "applet" | "caption" | "html" | "table" | "td" | "th" | "marquee" | "object" | "template") {
                    return false;
                }
            }
        }
        false
    }

    fn has_element_in_list_item_scope(&self, tag_name: &str) -> bool {
        for &id in self.open_elements.iter().rev() {
            if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                if tag == tag_name {
                    return true;
                }
                if matches!(tag.as_str(), "applet" | "caption" | "html" | "table" | "td" | "th" | "marquee" | "object" | "template" | "ol" | "ul") {
                    return false;
                }
            }
        }
        false
    }

    fn has_element_in_button_scope(&self, tag_name: &str) -> bool {
        for &id in self.open_elements.iter().rev() {
            if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                if tag == tag_name {
                    return true;
                }
                if matches!(tag.as_str(), "applet" | "caption" | "html" | "table" | "td" | "th" | "marquee" | "object" | "template" | "button") {
                    return false;
                }
            }
        }
        false
    }


    fn has_element_in_select_scope(&self, tag_name: &str) -> bool {
        for &id in self.open_elements.iter().rev() {
            if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                if tag == tag_name {
                    return true;
                }
                if !matches!(tag.as_str(), "optgroup" | "option") {
                    return false;
                }
            }
        }
        false
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
                let name = dt.name.as_deref().unwrap_or("");
                let public_id = dt.public_id.as_deref().unwrap_or("");
                let system_id = dt.system_id.as_deref().unwrap_or("");
                
                if dt.force_quirks || name != "html" || self.is_quirky_doctype(public_id, system_id) {
                    self.quirks_mode = true;
                }
                
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
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "unexpected token in initial mode");
                self.quirks_mode = true;
                self.insertion_mode = InsertionMode::BeforeHtml;
                Some(other)
            }
        }
    }

    fn is_quirky_doctype(&self, public_id: &str, system_id: &str) -> bool {
        // WHATWG 13.2.6.4.1
        let p = public_id.to_lowercase();
        let s = system_id.to_lowercase();
        
        if p == "+//silmaril//dtd html pro v0r11 19970101//en"
           || p == "-//as//dtd html 3.0//en//"
           || p == "-//advasoft//dtd html 3.0 aswedit + extensions//en"
           || p == "-//ietf//dtd html 2.0 level 1//en"
           || p == "-//ietf//dtd html 2.0 level 2//en"
           || p == "-//ietf//dtd html 2.0 strict level 1//en"
           || p == "-//ietf//dtd html 2.0 strict level 2//en"
           || p == "-//ietf//dtd html 2.0 strict//en"
           || p == "-//ietf//dtd html 2.0//en"
           || p == "-//ietf//dtd html 2.1e//en"
           || p == "-//ietf//dtd html 3.0//en"
           || p == "-//ietf//dtd html 3.0//en//"
           || p == "-//ietf//dtd html 3.2 final//en"
           || p == "-//ietf//dtd html 3.2//en"
           || p == "-//ietf//dtd html level 0//en"
           || p == "-//ietf//dtd html level 1//en"
           || p == "-//ietf//dtd html level 2//en"
           || p == "-//ietf//dtd html level 3//en"
           || p == "-//ietf//dtd html strict level 0//en"
           || p == "-//ietf//dtd html strict level 1//en"
           || p == "-//ietf//dtd html strict level 2//en"
           || p == "-//ietf//dtd html strict level 3//en"
           || p == "-//ietf//dtd html strict//en"
           || p == "-//ietf//dtd html//en"
           || p == "-//metrius//dtd html 2.0//en"
           || p == "-//microsoft//dtd internet explorer 2.0 html strict//en"
           || p == "-//microsoft//dtd internet explorer 2.0 html//en"
           || p == "-//microsoft//dtd internet explorer 2.0 tables//en"
           || p == "-//microsoft//dtd internet explorer 3.0 html strict//en"
           || p == "-//microsoft//dtd internet explorer 3.0 html//en"
           || p == "-//microsoft//dtd internet explorer 3.0 tables//en"
           || p == "-//netscape comm. corp.//dtd html//en"
           || p == "-//netscape comm. corp.//dtd strict html//en"
           || p == "-//o'reilly and associates//dtd html 2.0//en"
           || p == "-//o'reilly and associates//dtd html extended 1.0//en"
           || p == "-//spyglass//dtd html 2.0 extended//en"
           || p == "-//sq//dtd html 2.0 hotmetal + extensions//en"
           || p == "-//sun microsystems dtd html 2.0//en"
           || p == "-//ucla style//dtd html 2.0//en"
           || p == "-//w3c//dtd html 3 1995-03-24//en"
           || p == "-//w3c//dtd html 3.2 draft//en"
           || p == "-//w3c//dtd html 3.2 final//en"
           || p == "-//w3c//dtd html 3.2//en"
           || p == "-//w3c//dtd html 3.2s draft//en"
           || p == "-//w3c//dtd html 4.0 frameset//en"
           || p == "-//w3c//dtd html 4.0 transitional//en"
           || p == "-//w3c//dtd html experimental 19960712//en"
           || p == "-//w3c//dtd html experimental 970421//en"
           || p == "-//w3c//dtd w3 html//en"
           || p == "-//w3o//dtd w3 html 3.0//en"
           || p == "-//w3o//dtd w3 html 3.0//en//"
           || p == "-//webtechs//dtd mozilla html 2.0//en"
           || p == "-//webtechs//dtd mozilla html//en"
           || p == "html"
        { return true; }

        if p.contains("transitional") || p.contains("frameset") { return true; }

        if s.is_empty() && (p == "-//w3c//dtd html 4.01 transitional//en" || p == "-//w3c//dtd html 4.01 frameset//en") {
            return true;
        }

        false
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
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "unexpected token before <html>");
                let id = self.create_node(InternalNodeData::Element { 
                    tag: StringId::intern("html"), 
                    namespace: crate::ace::html::Namespace::Html, 
                    attributes: SmallAttributeMap::new() 
                });
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
                let id = self.create_node(InternalNodeData::Element { 
                    tag: StringId::intern("head"), 
                    namespace: crate::ace::html::Namespace::Html, 
                    attributes: SmallAttributeMap::new() 
                });
                let current = self.current_node();
                self.append_node(current, id);
                self.open_elements.push(id);
                self.head_element_id = Some(id);
                self.insertion_mode = InsertionMode::InHead;
                Some(HtmlToken::EndTag(tag))
            }
            other => {
                let id = self.create_node(InternalNodeData::Element { 
                    tag: StringId::intern("head"), 
                    namespace: crate::ace::html::Namespace::Html, 
                    attributes: SmallAttributeMap::new() 
                });
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
            HtmlToken::StartTag(tag) if tag.name == "html" => {
                self.handle_in_body(HtmlToken::StartTag(tag))
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
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style" | "template" | "title") => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "head element in after-head");
                let head_id = self.head_element_id.expect("head element should exist");
                self.open_elements.push(head_id);
                let ret = self.handle_in_head(HtmlToken::StartTag(tag));
                self.open_elements.retain(|&id| id != head_id);
                ret
            }
            HtmlToken::EndTag(tag) if tag.name == "template" => {
                self.handle_in_head(HtmlToken::EndTag(tag))
            }
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "body" | "html" | "br") => {
                let id = self.create_node(InternalNodeData::Element { 
                    tag: StringId::intern("body"), 
                    namespace: crate::ace::html::Namespace::Html, 
                    attributes: SmallAttributeMap::new() 
                });
                self.insert_at_appropriate_place(id, None);
                self.open_elements.push(id);
                self.insertion_mode = InsertionMode::InBody;
                Some(HtmlToken::EndTag(tag))
            }
            other => {
                let id = self.create_node(InternalNodeData::Element { 
                    tag: StringId::intern("body"), 
                    namespace: crate::ace::html::Namespace::Html, 
                    attributes: SmallAttributeMap::new() 
                });
                self.insert_at_appropriate_place(id, None);
                self.open_elements.push(id);
                self.insertion_mode = InsertionMode::InBody;
                Some(other)
            }
        }
    }

    fn handle_in_body(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        let mut token = token;
        if let Some(id) = self.open_elements.last() {
            if let InternalNodeData::Element { namespace, .. } = &self.arena[*id].data {
                if *namespace != crate::ace::html::Namespace::Html {
                    if let Some(res) = self.handle_foreign_content(token.clone()) {
                        token = res;
                    } else {
                        return None;
                    }
                }
            }
        }

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
                if self.open_elements.len() < 2 || !matches!(self.arena[self.open_elements[1]].data, InternalNodeData::Element { ref tag, .. } if tag == "body") {
                    // Ignore
                } else {
                    if let InternalNodeData::Element { ref mut attributes, .. } = self.arena[self.open_elements[1]].data {
                        for (k, v) in tag.attributes {
                            attributes.entry(k).or_insert(v);
                        }
                    }
                }
                None
            }
            "p" | "address" | "article" | "aside" | "blockquote" | "center" | "details" | "dialog" | "dir" | "div" | "dl" | "fieldset" | "figcaption" | "figure" | "footer" | "header" | "hgroup" | "main" | "menu" | "nav" | "ol" | "section" | "ul" => {
                if self.has_element_in_button_scope("p") {
                    self.close_p_element();
                }
                self.insert_html_element(tag);
                None
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if self.has_element_in_button_scope("p") {
                    self.close_p_element();
                }
                if matches!(self.current_tag(), Some("h1") | Some("h2") | Some("h3") | Some("h4") | Some("h5") | Some("h6")) {
                    self.open_elements.pop();
                }
                self.insert_html_element(tag);
                None
            }
            "pre" | "listing" => {
                if self.has_element_in_button_scope("p") {
                    self.close_p_element();
                }
                self.insert_html_element(tag);
                self.frameset_ok = false;
                None
            }
            "form" => {
                if self.form_element_id.is_none() {
                    if self.has_element_in_button_scope("p") {
                        self.close_p_element();
                    }
                    let id = self.insert_html_element(tag);
                    self.form_element_id = Some(id);
                }
                None
            }
            "li" => {
                self.frameset_ok = false;
                for &id in self.open_elements.iter().rev() {
                    if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                        if tag == "li" {
                            self.pop_until("li");
                            break;
                        }
                        if self.is_special_element(id) && !matches!(tag.as_str(), "address" | "div" | "p") {
                            break;
                        }
                    }
                }
                if self.has_element_in_button_scope("p") {
                    self.close_p_element();
                }
                self.insert_html_element(tag);
                None
            }
            "dd" | "dt" => {
                self.frameset_ok = false;
                let mut to_pop = None;
                for &id in self.open_elements.iter().rev() {
                    if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                        if tag == "dd" || tag == "dt" {
                            to_pop = Some(tag.clone());
                            break;
                        }
                        if self.is_special_element(id) && !matches!(tag.as_str(), "address" | "div" | "p") {
                            break;
                        }
                    }
                }
                if let Some(tag_name) = to_pop {
                    self.pop_until(&tag_name);
                }
                if self.has_element_in_button_scope("p") {
                    self.close_p_element();
                }
                self.insert_html_element(tag);
                None
            }
            "plaintext" => {
                if self.has_element_in_button_scope("p") {
                    self.close_p_element();
                }
                self.insert_html_element(tag);
                self.tokenizer.set_raw_text_tag(Some("plaintext".to_string()));
                None
            }
            "button" => {
                if self.has_element_in_scope("button") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "nested button");
                    self.handle_in_body_end_tag(EndTagToken { name: "button".to_string() });
                    return self.handle_in_body_start_tag(tag);
                }
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(tag);
                self.frameset_ok = false;
                None
            }
            "a" => {
                if let Some(pos) = self.active_formatting_elements.iter().rposition(|e| matches!(e, ActiveFormattingEntry::Element(id) if matches!(self.arena[*id].data, InternalNodeData::Element { ref tag, .. } if tag == "a"))) {
                    self.parse_error(TreeBuilderErrorKind::AdoptionAgency, "nested anchor");
                    self.adoption_agency_algorithm("a");
                    // Remove from open elements if it stayed there
                    if let Some(id) = self.active_formatting_elements.get(pos).and_then(|e| match e { ActiveFormattingEntry::Element(id) => Some(*id), _ => None }) {
                        self.open_elements.retain(|&oid| oid != id);
                        self.active_formatting_elements.remove(pos);
                    }
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
            "nobr" => {
                self.reconstruct_active_formatting_elements();
                if self.has_element_in_scope("nobr") {
                    self.parse_error(TreeBuilderErrorKind::AdoptionAgency, "nested nobr");
                    self.adoption_agency_algorithm("nobr");
                    self.reconstruct_active_formatting_elements();
                }
                let id = self.insert_html_element(tag);
                self.active_formatting_elements.push(ActiveFormattingEntry::Element(id));
                None
            }
            "applet" | "marquee" | "object" => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(tag);
                self.active_formatting_elements.push(ActiveFormattingEntry::Marker);
                self.frameset_ok = false;
                None
            }
            "table" => {
                if !self.quirks_mode && self.has_element_in_button_scope("p") {
                    self.close_p_element();
                }
                self.insert_html_element(tag);
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InTable;
                None
            }
            "area" | "br" | "embed" | "img" | "keygen" | "wbr" => {
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(tag);
                self.open_elements.pop();
                self.frameset_ok = false;
                None
            }
            "input" => {
                self.reconstruct_active_formatting_elements();
                let type_attr = tag.attributes.get("type").map(|s| s.to_lowercase());
                self.insert_html_element(tag);
                self.open_elements.pop();
                if type_attr.as_deref() != Some("hidden") {
                    self.frameset_ok = false;
                }
                None
            }
            "hr" => {
                if self.has_element_in_button_scope("p") {
                    self.close_p_element();
                }
                self.insert_html_element(tag);
                self.open_elements.pop();
                self.frameset_ok = false;
                None
            }
            "image" => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "saw <image> tag");
                let mut new_tag = tag;
                new_tag.name = "img".to_string();
                self.handle_in_body_start_tag(new_tag)
            }
            "textarea" => {
                self.insert_html_element(tag);
                self.tokenizer.set_raw_text_tag(Some("textarea".to_string()));
                self.original_insertion_mode = self.insertion_mode;
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::Text;
                None
            }
            "xmp" => {
                if self.has_element_in_button_scope("p") {
                    self.close_p_element();
                }
                self.reconstruct_active_formatting_elements();
                self.frameset_ok = false;
                self.insert_html_element(tag);
                self.tokenizer.set_raw_text_tag(Some("xmp".to_string()));
                self.original_insertion_mode = self.insertion_mode;
                self.insertion_mode = InsertionMode::Text;
                None
            }
            "iframe" | "noembed" | "noscript" => {
                self.frameset_ok = false;
                let tag_name = tag.name.clone();
                self.insert_html_element(tag);
                self.tokenizer.set_raw_text_tag(Some(tag_name));
                self.original_insertion_mode = self.insertion_mode;
                self.insertion_mode = InsertionMode::Text;
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
            "optgroup" | "option" => {
                if self.current_tag() == Some("option") {
                    self.open_elements.pop();
                }
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(tag);
                None
            }
            "rb" | "rt" | "rtc" | "rp" => {
                if self.has_element_in_scope("ruby") {
                    self.generate_implied_end_tags(None);
                }
                self.insert_html_element(tag);
                None
            }
            "math" => {
                self.reconstruct_active_formatting_elements();
                self.insert_element(tag, crate::ace::html::Namespace::MathMl);
                None
            }
            "svg" => {
                self.reconstruct_active_formatting_elements();
                self.insert_element(tag, crate::ace::html::Namespace::Svg);
                None
            }
            "caption" | "col" | "colgroup" | "frame" | "head" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr" => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, format!("unexpected {} in InBody", tag.name));
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
        match tag.name.as_ref() {
            "body" => {
                if !self.has_element_in_scope("body") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "end tag </body> not in scope");
                    return None;
                }
                // Check if there are other elements in scope than body/html
                for &id in self.open_elements.iter().skip(2) {
                    if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                        if !matches!(tag.as_str(), "body" | "html") {
                            self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("end tag </body> with open {} element", tag));
                            break;
                        }
                    }
                }
                self.insertion_mode = InsertionMode::AfterBody;
                None
            }
            "html" => {
                if !self.has_element_in_scope("body") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "end tag </html> but <body> not in scope");
                    return None;
                }
                for &id in self.open_elements.iter().skip(2) {
                    if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                        if !matches!(tag.as_str(), "body" | "html") {
                            self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("end tag </html> with open {} element", tag));
                            break;
                        }
                    }
                }
                self.insertion_mode = InsertionMode::AfterBody;
                Some(HtmlToken::EndTag(tag))
            }
            "div" | "article" | "aside" | "blockquote" | "center" | "details" | "dialog" | "dir" | "dl" | "fieldset" | "figcaption" | "figure" | "footer" | "header" | "hgroup" | "main" | "menu" | "nav" | "ol" | "section" | "ul" => {
                if !self.has_element_in_scope(tag.name.as_str()) {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("end tag </{}> not in scope", tag.name));
                    return None;
                }
                self.generate_implied_end_tags(Some(tag.name.as_str()));
                self.pop_until(tag.name.as_str());
                None
            }
            "p" => {
                if !self.has_element_in_button_scope("p") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "end tag </p> not in button scope");
                    self.handle_in_body_start_tag(StartTagToken { name: "p".to_string(), attributes: HashMap::new(), self_closing: false });
                    return self.handle_in_body_end_tag(tag);
                }
                self.close_p_element();
                None
            }
            "li" => {
                if !self.has_element_in_list_item_scope("li") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "end tag </li> not in scope");
                    return None;
                }
                self.generate_implied_end_tags(Some("li"));
                self.pop_until("li");
                None
            }
            "dd" | "dt" => {
                if !self.has_element_in_scope(tag.name.as_str()) {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("end tag </{}> not in scope", tag.name));
                    return None;
                }
                self.generate_implied_end_tags(Some(tag.name.as_str()));
                self.pop_until(tag.name.as_str());
                None
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if !self.has_element_in_scope("h1") && !self.has_element_in_scope("h2") && !self.has_element_in_scope("h3") && !self.has_element_in_scope("h4") && !self.has_element_in_scope("h5") && !self.has_element_in_scope("h6") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("end tag </{}> not in scope", tag.name));
                    return None;
                }
                self.generate_implied_end_tags(Some(tag.name.as_str()));
                self.pop_until(tag.name.as_str());
                None
            }
            "a" | "b" | "big" | "code" | "em" | "font" | "i" | "nobr" | "s" | "small" | "strike" | "strong" | "tt" | "u" => {
                self.adoption_agency_algorithm(tag.name.as_str());
                None
            }
            "applet" | "marquee" | "object" => {
                if !self.has_element_in_scope(tag.name.as_str()) {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("end tag </{}> not in scope", tag.name));
                    return None;
                }
                self.generate_implied_end_tags(Some(tag.name.as_str()));
                self.pop_until(tag.name.as_str());
                self.clear_formatting_to_last_marker();
                None
            }
            "br" => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "saw end tag </br>");
                self.handle_in_body_start_tag(StartTagToken { name: "br".to_string(), attributes: HashMap::new(), self_closing: false });
                None
            }
            _ => {
                for i in (0..self.open_elements.len()).rev() {
                    let id = self.open_elements[i];
                    if let InternalNodeData::Element { tag: ref element_tag, .. } = self.arena[id].data {
                        if element_tag == &tag.name {
                            let tag_to_generate = element_tag.clone();
                            self.generate_implied_end_tags(Some(&tag_to_generate));
                            self.open_elements.truncate(i);
                            break;
                        }
                        if self.is_special_element(id) {
                            self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("unexpected end tag </{}>", tag.name));
                            break;
                        }
                    }
                }
                None
            }
        }
    }

    fn close_p_element(&mut self) {
        self.generate_implied_end_tags(Some("p"));
        self.pop_until("p");
    }

    fn handle_text(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) => {
                self.insert_text(text.data);
                None
            }
            HtmlToken::EndTag(_) => {
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

    fn insert_comment(&mut self, data: String) {
        let id = self.create_node(InternalNodeData::Comment(data));
        self.insert_at_appropriate_place(id, None);
    }

    // --- Adoption Agency Algorithm (Full) ---

    fn adoption_agency_algorithm(&mut self, subject: &str) {
        // WHATWG 13.2.6.4.7
        
        // Step 2: Outer loop
        for _outer in 0..8 {
            // Step 3: Find formatting element
            let mut formatting_element_pos = None;
            for i in (0..self.active_formatting_elements.len()).rev() {
                let entry = &self.active_formatting_elements[i];
                if matches!(entry, ActiveFormattingEntry::Marker) {
                    break;
                }
                if let ActiveFormattingEntry::Element(id) = entry {
                    if let InternalNodeData::Element { ref tag, .. } = self.arena[*id].data {
                        if tag == subject {
                            formatting_element_pos = Some(i);
                            break;
                        }
                    }
                }
            }

            let Some(f_pos) = formatting_element_pos else {
                self.handle_in_body_end_tag_standard(subject);
                return;
            };
            
            let formatting_element_id = match self.active_formatting_elements[f_pos] {
                ActiveFormattingEntry::Element(id) => id,
                _ => unreachable!(),
            };

            // Step 4: Check if in open stack
            let open_pos = self.open_elements.iter().position(|&id| id == formatting_element_id);
            if open_pos.is_none() {
                self.parse_error(TreeBuilderErrorKind::AdoptionAgency, "formatting element not in open stack");
                self.active_formatting_elements.remove(f_pos);
                return;
            }
            let open_pos = open_pos.expect("formatting element must be in open stack by step 4");

            // Step 5: Check if in scope
            if !self.has_element_in_scope_with_id(formatting_element_id) {
                self.parse_error(TreeBuilderErrorKind::AdoptionAgency, "formatting element not in scope");
                return;
            }

            // Step 6: Check if node is formatting element
            if self.open_elements.last() != Some(&formatting_element_id) {
                self.parse_error(TreeBuilderErrorKind::AdoptionAgency, "node is not formatting element");
            }

            // Step 7: Find furthest block
            let mut furthest_block_pos = None;
            for i in open_pos + 1..self.open_elements.len() {
                let id = self.open_elements[i];
                if self.is_special_element(id) {
                    furthest_block_pos = Some(i);
                    break;
                }
            }

            // Step 8: If no furthest block, pop until formatting element and return
            let Some(fb_pos) = furthest_block_pos else {
                while let Some(id) = self.open_elements.pop() {
                    if id == formatting_element_id { break; }
                }
                self.active_formatting_elements.remove(f_pos);
                return;
            };

            // Step 9: Common ancestor
            let common_ancestor_id = self.open_elements[open_pos - 1];

            // Step 10: Bookmark
            let mut bookmark_pos = f_pos;

            // Step 11: Inner loop
            let mut last_node_id = self.open_elements[fb_pos];
            let mut node_pos = fb_pos;
            
            for _inner in 0..3 {
                node_pos -= 1;
                let node_id = self.open_elements[node_pos];

                // Check in active formatting elements
                let f_entry_pos = self.active_formatting_elements.iter().position(|e| matches!(e, ActiveFormattingEntry::Element(id) if *id == node_id));
                
                if f_entry_pos.is_none() {
                    self.open_elements.remove(node_pos);
                    continue;
                }

                if node_id == formatting_element_id {
                    break;
                }

                // If node is furthest block, update bookmark
                let cloned_id = self.clone_element(node_id);
                // Replace in stacks
                let f_e_pos = f_entry_pos.expect("node_id must be in active formatting elements per step 11.1");
                self.active_formatting_elements[f_e_pos] = ActiveFormattingEntry::Element(cloned_id);
                self.open_elements[node_pos] = cloned_id;
                
                if last_node_id == self.open_elements[fb_pos] {
                    bookmark_pos = f_e_pos + 1;
                }
                
                // Detach last_node from its parent and append to node
                if let Some(parent) = self.arena[last_node_id].parent {
                    self.arena[parent].children.retain(|&id| id != last_node_id);
                }
                self.append_node(cloned_id, last_node_id);
                last_node_id = cloned_id;
            }

            // Step 12: Reparent last node to common ancestor (handle foster parenting)
            if let Some(parent) = self.arena[last_node_id].parent {
                self.arena[parent].children.retain(|&id| id != last_node_id);
            }
            self.insert_at_appropriate_place(last_node_id, Some(common_ancestor_id));

            // Step 13: New formatting element
            let new_formatting_id = self.clone_element(formatting_element_id);
            
            // Step 14: Move children of fb into new formatting element
            let fb_id = self.open_elements[fb_pos];
            let fb_children = self.arena[fb_id].children.clone();
            self.arena[fb_id].children.clear();
            for child_id in fb_children {
                self.arena[child_id].parent = Some(new_formatting_id);
                self.arena[new_formatting_id].children.push(child_id);
            }

            // Step 15: Append new formatting element to furthest block
            self.append_node(fb_id, new_formatting_id);

            // Step 16: Adjust stacks
            self.active_formatting_elements.remove(f_pos);
            self.active_formatting_elements.insert(bookmark_pos.min(self.active_formatting_elements.len()), ActiveFormattingEntry::Element(new_formatting_id));

            self.open_elements.retain(|&id| id != formatting_element_id);
            let fb_stack_pos = self.open_elements.iter().position(|&id| id == fb_id).unwrap();
            self.open_elements.insert(fb_stack_pos + 1, new_formatting_id);
        }
    }

    fn is_special_element(&self, id: usize) -> bool {
        match &self.arena[id].data {
            InternalNodeData::Element { tag, .. } => {
                matches!(tag.as_str(), "address" | "applet" | "area" | "article" | "aside" | "base" | "basefont" | "bgsound" | "blockquote" | "body" | "br" | "button" | "caption" | "center" | "col" | "colgroup" | "dd" | "details" | "dir" | "div" | "dl" | "dt" | "embed" | "fieldset" | "figcaption" | "figure" | "footer" | "form" | "frame" | "frameset" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "head" | "header" | "hgroup" | "hr" | "html" | "iframe" | "img" | "input" | "keygen" | "li" | "link" | "listing" | "main" | "marquee" | "menu" | "meta" | "nav" | "noembed" | "noframes" | "noscript" | "object" | "ol" | "p" | "param" | "plaintext" | "pre" | "script" | "section" | "select" | "source" | "style" | "summary" | "table" | "tbody" | "td" | "template" | "textarea" | "tfoot" | "th" | "thead" | "title" | "tr" | "track" | "ul" | "wbr" | "xmp" | "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml" | "foreignObject" | "desc")
            }
            _ => false,
        }
    }

    fn clone_element(&mut self, id: usize) -> usize {
        match &self.arena[id].data {
            InternalNodeData::Element { tag, namespace, attributes } => {
                self.create_node(InternalNodeData::Element { 
                    tag: tag.clone(), 
                    namespace: *namespace,
                    attributes: attributes.clone() 
                })
            }
            _ => unreachable!(),
        }
    }

    pub fn set_state(&mut self, state: LexerState) {
        self.tokenizer.lexer.set_state(state);
    }

    fn handle_in_body_end_tag_standard(&mut self, tag: &str) {
        self.pop_until(tag);
    }

    fn handle_in_table(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Comment(comment) => {
                self.insert_comment(comment.data);
                None
            }
            HtmlToken::Doctype(_) => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedDoctype, "unexpected DOCTYPE in table");
                None
            }
            HtmlToken::Character(text) if matches!(self.current_tag(), Some("table") | Some("tbody") | Some("tfoot") | Some("thead") | Some("tr")) => {
                self.pending_table_characters.clear();
                self.original_insertion_mode = self.insertion_mode;
                self.insertion_mode = InsertionMode::InTableText;
                Some(HtmlToken::Character(text))
            }
            HtmlToken::StartTag(tag) if tag.name == "caption" => {
                self.clear_pending_table_characters();
                self.active_formatting_elements.push(ActiveFormattingEntry::Marker);
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InCaption;
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "colgroup" => {
                self.clear_pending_table_characters();
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InColumnGroup;
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "col" => {
                self.handle_in_table(HtmlToken::StartTag(StartTagToken { name: "colgroup".to_string(), attributes: HashMap::new(), self_closing: false }))?;
                self.handle_in_table(HtmlToken::StartTag(tag))
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
                self.clear_pending_table_characters();
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InTableBody;
                None
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "td" | "th" | "tr") => {
                self.clear_pending_table_characters();
                self.insert_element_at_current("tbody".to_string(), HashMap::new());
                self.insertion_mode = InsertionMode::InTableBody;
                Some(HtmlToken::StartTag(tag))
            }
            HtmlToken::StartTag(tag) if tag.name == "table" => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "unexpected <table> in table");
                if self.handle_in_table(HtmlToken::EndTag(EndTagToken { name: "table".to_string() })).is_none() {
                    return Some(HtmlToken::StartTag(tag));
                }
                None
            }
            HtmlToken::EndTag(tag) if tag.name == "table" => {
                if !self.has_element_in_scope("table") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "end tag </table> not in scope");
                    return None;
                }
                self.pop_until("table");
                self.reset_insertion_mode_appropriately();
                None
            }
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "body" | "caption" | "col" | "colgroup" | "html" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr") => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("unexpected end tag </{}> in table", tag.name));
                None
            }
            HtmlToken::Eof => {
                if self.current_tag() != Some("html") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEof, "EOF in table");
                }
                None
            }
            other => {
                self.parse_error(TreeBuilderErrorKind::FosterParenting, "foster parenting in table");
                self.foster_parenting = true;
                let ret = self.handle_in_body(other);
                self.foster_parenting = false;
                ret
            }
        }
    }

    fn clear_pending_table_characters(&mut self) {
        if !self.pending_table_characters.is_empty() {
             // Handle pending characters by foster parenting them if non-whitespace
             let all_ws = self.pending_table_characters.iter().all(|&c| HtmlTreeBuilder::is_whitespace(c));
             let text: String = self.pending_table_characters.drain(..).collect();
             if !all_ws {
                 self.foster_parenting = true;
                 self.insert_text(text);
                 self.foster_parenting = false;
             } else {
                 self.insert_text(text);
             }
        }
    }

    fn handle_in_table_text(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) => {
                for c in text.data.chars() {
                    if c == '\0' {
                        self.parse_error(TreeBuilderErrorKind::UnexpectedCharacter, "null character in table text");
                    } else {
                        self.pending_table_characters.push(c);
                    }
                }
                None
            }
            other => {
                let all_whitespace = self.pending_table_characters.iter().all(|&c| HtmlTreeBuilder::is_whitespace(c));
                let text: String = self.pending_table_characters.drain(..).collect();
                
                if !all_whitespace {
                    self.parse_error(TreeBuilderErrorKind::FosterParenting, "non-whitespace in table text");
                    self.foster_parenting = true;
                    self.insert_text(text);
                    self.foster_parenting = false;
                } else {
                    self.insert_text(text);
                }
                
                self.insertion_mode = self.original_insertion_mode;
                Some(other)
            }
        }
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
                self.clear_stack_back_to_table_body_context();
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InRow;
                None
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, format!("unexpected <{}> in table body", tag.name));
                self.handle_in_table_body(HtmlToken::StartTag(StartTagToken { name: "tr".to_string(), attributes: HashMap::new(), self_closing: false }))?;
                self.handle_in_table_body(HtmlToken::StartTag(tag))
            }
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
                if !self.has_element_in_scope(tag.name.as_str()) {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("end tag </{}> not in scope", tag.name));
                    return None;
                }
                self.clear_stack_back_to_table_body_context();
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTable;
                None
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead") => {
                if !self.has_element_in_scope("tbody") && !self.has_element_in_scope("thead") && !self.has_element_in_scope("tfoot") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "no table body in scope");
                    return None;
                }
                self.clear_stack_back_to_table_body_context();
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTable;
                Some(HtmlToken::StartTag(tag))
            }
            HtmlToken::EndTag(tag) if tag.name == "table" => {
                if !self.has_element_in_scope("tbody") && !self.has_element_in_scope("thead") && !self.has_element_in_scope("tfoot") {
                     self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "end tag </table> without table body");
                     return None;
                }
                self.clear_stack_back_to_table_body_context();
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTable;
                Some(HtmlToken::EndTag(tag))
            }
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th" | "tr") => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("unexpected end tag </{}> in table body", tag.name));
                None
            }
            other => self.handle_in_table(other),
        }
    }

    fn clear_stack_back_to_table_body_context(&mut self) {
        while let Some(&id) = self.open_elements.last() {
             if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                 if matches!(tag.as_str(), "tbody" | "tfoot" | "thead" | "template" | "html") {
                     break;
                 }
             }
             self.open_elements.pop();
        }
    }

    fn handle_in_row(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
                self.clear_stack_back_to_table_row_context();
                self.insert_html_element(tag);
                self.insertion_mode = InsertionMode::InCell;
                self.active_formatting_elements.push(ActiveFormattingEntry::Marker);
                None
            }
            HtmlToken::EndTag(tag) if tag.name == "tr" => {
                if !self.has_element_in_scope("tr") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "end tag </tr> not in scope");
                    return None;
                }
                self.clear_stack_back_to_table_row_context();
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTableBody;
                None
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr") => {
                if self.handle_in_row(HtmlToken::EndTag(EndTagToken { name: "tr".to_string() })).is_none() {
                    return Some(HtmlToken::StartTag(tag));
                }
                None
            }
            HtmlToken::EndTag(tag) if tag.name == "table" => {
                if self.handle_in_row(HtmlToken::EndTag(EndTagToken { name: "tr".to_string() })).is_none() {
                    return Some(HtmlToken::EndTag(tag));
                }
                None
            }
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
                if !self.has_element_in_scope(tag.name.as_str()) {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("unexpected end tag </{}> in row", tag.name));
                    return None;
                }
                if self.handle_in_row(HtmlToken::EndTag(EndTagToken { name: "tr".to_string() })).is_none() {
                    return Some(HtmlToken::EndTag(tag));
                }
                None
            }
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th") => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("unexpected end tag </{}> in row", tag.name));
                None
            }
            other => self.handle_in_table(other),
        }
    }

    fn clear_stack_back_to_table_row_context(&mut self) {
        while let Some(&id) = self.open_elements.last() {
             if let InternalNodeData::Element { ref tag, .. } = self.arena[id].data {
                 if matches!(tag.as_str(), "tr" | "template" | "html") {
                     break;
                 }
             }
             self.open_elements.pop();
        }
    }

    fn handle_in_cell(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
                if !self.has_element_in_scope(tag.name.as_str()) {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("end tag </{}> not in scope", tag.name));
                    return None;
                }
                self.generate_implied_end_tags(None);
                self.pop_until(tag.name.as_str());
                self.clear_formatting_to_last_marker();
                self.insertion_mode = InsertionMode::InRow;
                None
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "caption" | "col" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr") => {
                if self.handle_in_cell(HtmlToken::EndTag(EndTagToken { name: self.current_tag().unwrap_or("").to_string() })).is_none() {
                    return Some(HtmlToken::StartTag(tag));
                }
                None
            }
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "body" | "caption" | "col" | "colgroup" | "html") => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("unexpected end tag </{}> in cell", tag.name));
                None
            }
            HtmlToken::EndTag(tag) if matches!(tag.name.as_str(), "table" | "tbody" | "tfoot" | "thead" | "tr") => {
                if !self.has_element_in_scope(tag.name.as_str()) {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("unexpected end tag </{}> in cell", tag.name));
                    return None;
                }
                if self.handle_in_cell(HtmlToken::EndTag(EndTagToken { name: self.current_tag().unwrap_or("").to_string() })).is_none() {
                    return Some(HtmlToken::EndTag(tag));
                }
                None
            }
            other => self.handle_in_body(other),
        }
    }

    fn handle_in_select(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) => {
                let data = text.data.replace('\0', "");
                if !data.is_empty() {
                    self.insert_text(data);
                }
                None
            }
            HtmlToken::Comment(comment) => {
                self.insert_comment(comment.data);
                None
            }
            HtmlToken::Doctype(_) => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedDoctype, "unexpected DOCTYPE in select");
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "html" => {
                self.handle_in_body(HtmlToken::StartTag(tag))
            }
            HtmlToken::StartTag(tag) if tag.name == "option" => {
                if self.current_tag() == Some("option") {
                    self.open_elements.pop();
                }
                self.insert_html_element(tag);
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "optgroup" => {
                if self.current_tag() == Some("option") {
                    self.open_elements.pop();
                }
                if self.current_tag() == Some("optgroup") {
                    self.open_elements.pop();
                }
                self.insert_html_element(tag);
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "hr" => {
                if self.current_tag() == Some("option") {
                    self.open_elements.pop();
                }
                if self.current_tag() == Some("optgroup") {
                    self.open_elements.pop();
                }
                self.insert_html_element(tag);
                self.open_elements.pop();
                None
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "script" | "template") => {
                self.handle_in_head(HtmlToken::StartTag(tag))
            }
            HtmlToken::EndTag(tag) if tag.name == "optgroup" => {
                if self.current_tag() == Some("option") && self.open_elements.len() >= 2 && matches!(self.arena[self.open_elements[self.open_elements.len()-2]].data, InternalNodeData::Element { ref tag, .. } if tag == "optgroup") {
                    self.open_elements.pop();
                }
                if self.current_tag() == Some("optgroup") {
                    self.open_elements.pop();
                } else {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "end tag </optgroup> ignored");
                }
                None
            }
            HtmlToken::EndTag(tag) if tag.name == "option" => {
                if self.current_tag() == Some("option") {
                    self.open_elements.pop();
                } else {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "end tag </option> ignored");
                }
                None
            }
            HtmlToken::EndTag(tag) if tag.name == "select" => {
                if !self.has_element_in_select_scope("select") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "end tag </select> not in scope");
                    return None;
                }
                self.pop_until("select");
                self.reset_insertion_mode_appropriately();
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "select" => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "unexpected <select> in InSelect");
                if !self.has_element_in_select_scope("select") {
                    return None;
                }
                self.pop_until("select");
                self.reset_insertion_mode_appropriately();
                None
            }
            HtmlToken::EndTag(tag) if tag.name == "template" => {
                self.handle_in_head(HtmlToken::EndTag(tag))
            }
            HtmlToken::Eof => {
                if self.current_tag() != Some("html") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEof, "EOF in select");
                }
                None
            }
            _other => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "unexpected token in InSelect");
                None
            }
        }
    }

    fn handle_in_select_in_table(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        self.handle_in_select(token)
    }

    fn handle_in_template(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(_) | HtmlToken::Comment(_) | HtmlToken::Doctype(_) => {
                self.handle_in_body(token)
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style" | "template" | "title") => {
                self.handle_in_head(HtmlToken::StartTag(tag))
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "caption" | "colgroup" | "tbody" | "tfoot" | "thead") => {
                self.template_insertion_modes.pop();
                self.template_insertion_modes.push(InsertionMode::InTable);
                self.handle_in_table(HtmlToken::StartTag(tag))
            }
            HtmlToken::StartTag(tag) if tag.name == "col" => {
                self.template_insertion_modes.pop();
                self.template_insertion_modes.push(InsertionMode::InColumnGroup);
                self.handle_in_column_group(HtmlToken::StartTag(tag))
            }
            HtmlToken::StartTag(tag) if tag.name == "tr" => {
                self.template_insertion_modes.pop();
                self.template_insertion_modes.push(InsertionMode::InTableBody);
                self.handle_in_table_body(HtmlToken::StartTag(tag))
            }
            HtmlToken::StartTag(tag) if matches!(tag.name.as_str(), "td" | "th") => {
                self.template_insertion_modes.pop();
                self.template_insertion_modes.push(InsertionMode::InRow);
                self.handle_in_row(HtmlToken::StartTag(tag))
            }
            HtmlToken::StartTag(tag) => {
                self.template_insertion_modes.pop();
                self.template_insertion_modes.push(InsertionMode::InBody);
                self.handle_in_body(HtmlToken::StartTag(tag))
            }
            HtmlToken::EndTag(tag) if tag.name == "template" => {
                self.handle_in_head(HtmlToken::EndTag(tag))
            }
            HtmlToken::EndTag(_) => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "unexpected end tag in template");
                None
            }
            HtmlToken::Eof => {
                if !self.open_elements.iter().any(|&id| matches!(self.arena[id].data, InternalNodeData::Element { ref tag, .. } if tag == "template")) {
                    return None;
                }
                self.parse_error(TreeBuilderErrorKind::UnexpectedEof, "EOF in template");
                self.pop_until("template");
                self.clear_formatting_to_last_marker();
                self.template_insertion_modes.pop();
                self.reset_insertion_mode_appropriately();
                Some(HtmlToken::Eof)
            }
        }
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

    fn handle_after_after_body(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Comment(comment) => {
                let id = self.create_node(InternalNodeData::Comment(comment.data));
                self.append_node(self.root_id, id);
                None
            }
            HtmlToken::Doctype(_) | HtmlToken::Character(_) | HtmlToken::StartTag(_) => {
                self.insertion_mode = InsertionMode::InBody;
                Some(token)
            }
            _ => None,
        }
    }

    fn handle_in_frameset(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) if text.data.trim().is_empty() => {
                self.insert_text(text.data);
                None
            }
            HtmlToken::Comment(comment) => {
                self.insert_comment(comment.data);
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "html" => self.handle_in_body(HtmlToken::StartTag(tag)),
            HtmlToken::StartTag(tag) if tag.name == "frameset" => {
                self.insert_html_element(tag);
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "frame" => {
                self.insert_html_element(tag);
                self.open_elements.pop();
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "noframes" => self.handle_in_head(HtmlToken::StartTag(tag)),
            HtmlToken::EndTag(tag) if tag.name == "frameset" => {
                if self.current_tag() == Some("html") {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, "unexpected </frameset>");
                } else {
                    self.open_elements.pop();
                    if self.insertion_mode != InsertionMode::Initial && self.current_tag() != Some("frameset") {
                        self.insertion_mode = InsertionMode::AfterFrameset;
                    }
                }
                None
            }
            HtmlToken::Eof => {
               if self.current_tag() != Some("html") {
                   self.parse_error(TreeBuilderErrorKind::UnexpectedEof, "EOF in frameset");
               }
               None
            }
            _ => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "unexpected token in frameset");
                None
            }
        }
    }

    fn handle_after_frameset(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) if text.data.trim().is_empty() => {
                self.insert_text(text.data);
                None
            }
            HtmlToken::Comment(comment) => {
                self.insert_comment(comment.data);
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "html" => self.handle_in_body(HtmlToken::StartTag(tag)),
            HtmlToken::EndTag(tag) if tag.name == "html" => {
                self.insertion_mode = InsertionMode::AfterAfterFrameset;
                None
            }
            HtmlToken::StartTag(tag) if tag.name == "noframes" => self.handle_in_head(HtmlToken::StartTag(tag)),
            HtmlToken::Eof => None,
            _ => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedToken, "unexpected token after frameset");
                None
            }
        }
    }

    fn handle_after_after_frameset(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Comment(comment) => {
                let id = self.create_node(InternalNodeData::Comment(comment.data));
                self.append_node(self.root_id, id);
                None
            }
            HtmlToken::Doctype(_) | HtmlToken::Character(_) | HtmlToken::StartTag(_) => {
                self.insertion_mode = InsertionMode::InBody;
                Some(token)
            }
            _ => None,
        }
    }

    fn reset_insertion_mode_appropriately(&mut self) {
        // WHATWG 13.2.6.4.1: Reset the insertion mode appropriately
        let mut last = false;
        for i in (0..self.open_elements.len()).rev() {
            let id = self.open_elements[i];
            match &self.arena[id].data {
                InternalNodeData::Element { tag, namespace, .. } => {
                    if i == 0 {
                        last = true;
                    }
                    
                    if *namespace != crate::ace::html::Namespace::Html && !self.is_integration_point(id) {
                        self.insertion_mode = InsertionMode::InBody;
                        return;
                    }

                    match tag.as_str() {
                        "select" => {
                            self.insertion_mode = InsertionMode::InSelect;
                            return;
                        }
                        "td" | "th" if !last => {
                            self.insertion_mode = InsertionMode::InCell;
                            return;
                        }
                        "tr" => {
                            self.insertion_mode = InsertionMode::InRow;
                            return;
                        }
                        "tbody" | "thead" | "tfoot" => {
                            self.insertion_mode = InsertionMode::InTableBody;
                            return;
                        }
                        "caption" => {
                            self.insertion_mode = InsertionMode::InCaption;
                            return;
                        }
                        "colgroup" => {
                            self.insertion_mode = InsertionMode::InColumnGroup;
                            return;
                        }
                        "table" => {
                            self.insertion_mode = InsertionMode::InTable;
                            return;
                        }
                        "head" if !last => {
                            self.insertion_mode = InsertionMode::InHead;
                            return;
                        }
                        "body" => {
                            self.insertion_mode = InsertionMode::InBody;
                            return;
                        }
                        "frameset" => {
                            self.insertion_mode = InsertionMode::InFrameset;
                            return;
                        }
                        "html" => {
                            if self.head_element_id.is_none() {
                                self.insertion_mode = InsertionMode::BeforeHead;
                            } else {
                                self.insertion_mode = InsertionMode::AfterHead;
                            }
                            return;
                        }
                        _ => {
                            if last {
                                self.insertion_mode = InsertionMode::InBody;
                                return;
                            }
                        }
                    }
                }
                _ => break,
            }
        }
        self.insertion_mode = InsertionMode::InBody;
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

    fn handle_foreign_content(&mut self, token: HtmlToken) -> Option<HtmlToken> {
        match token {
            HtmlToken::Character(text) => {
                let data = text.data.replace('\0', "\u{FFFD}");
                self.insert_text(data.clone());
                if !data.trim().is_empty() {
                    self.frameset_ok = false;
                }
                None
            }
            HtmlToken::Comment(comment) => {
                self.insert_comment(comment.data);
                None
            }
            HtmlToken::Doctype(_) => {
                self.parse_error(TreeBuilderErrorKind::UnexpectedDoctype, "unexpected DOCTYPE in foreign content");
                None
            }
            HtmlToken::StartTag(tag) => {
                if matches!(tag.name.as_str(), "b" | "big" | "blockquote" | "body" | "br" | "center" | "code" | "dd" | "div" | "dl" | "dt" | "em" | "embed" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "head" | "hr" | "i" | "img" | "li" | "listing" | "menu" | "meta" | "nobr" | "ol" | "p" | "pre" | "ruby" | "s" | "small" | "span" | "strong" | "strike" | "sub" | "sup" | "table" | "tt" | "u" | "ul" | "var")
                   || (tag.name == "font" && (tag.attributes.contains_key("color") || tag.attributes.contains_key("face") || tag.attributes.contains_key("size"))) {
                    self.parse_error(TreeBuilderErrorKind::UnexpectedToken, format!("unexpected HTML tag {} in foreign content", &tag.name));
                    while let Some(&id) = self.open_elements.last() {
                         if let InternalNodeData::Element { namespace, .. } = &self.arena[id].data {
                             if *namespace == crate::ace::html::Namespace::Html || self.is_integration_point(id) {
                                 break;
                             }
                         }
                         self.open_elements.pop();
                    }
                    return Some(HtmlToken::StartTag(tag.clone()));
                }
                
                let ns = self.current_node_namespace();
                self.insert_element(tag, ns);
                None
            }
            HtmlToken::EndTag(tag) => {
                let mut node_index = None;
                for (i, &id) in self.open_elements.iter().enumerate().rev() {
                    if let InternalNodeData::Element { tag: ref node_tag, .. } = self.arena[id].data {
                        if node_tag.eq_ignore_ascii_case(&tag.name) {
                            node_index = Some(i);
                            break;
                        }
                        if let InternalNodeData::Element { namespace, .. } = &self.arena[id].data {
                            if *namespace == crate::ace::html::Namespace::Html {
                                return self.handle_in_body(HtmlToken::EndTag(tag));
                            }
                        }
                    }
                }

                if let Some(i) = node_index {
                    if let InternalNodeData::Element { tag: ref current_tag, .. } = self.arena[*self.open_elements.last().unwrap()].data {
                        if !current_tag.eq_ignore_ascii_case(&tag.name) {
                            self.parse_error(TreeBuilderErrorKind::UnexpectedEndTag, format!("unexpected end tag </{}> in foreign content", tag.name));
                        }
                    }
                    self.open_elements.truncate(i);
                }
                None
            }
            HtmlToken::Eof => {
                None
            }
        }
    }

    fn current_node_namespace(&self) -> crate::ace::html::Namespace {
        if let Some(&id) = self.open_elements.last() {
            if let InternalNodeData::Element { namespace, .. } = &self.arena[id].data {
                return *namespace;
            }
        }
        crate::ace::html::Namespace::Html
    }

    fn is_mathml_text_integration_point(&self, id: usize) -> bool {
        if let InternalNodeData::Element { tag, namespace, .. } = &self.arena[id].data {
            if *namespace == crate::ace::html::Namespace::MathMl {
                return matches!(tag.as_str(), "mi" | "mo" | "mn" | "ms" | "mtext");
            }
        }
        false
    }

    fn is_html_integration_point(&self, id: usize) -> bool {
        if let InternalNodeData::Element { tag, namespace, attributes } = &self.arena[id].data {
            match namespace {
                crate::ace::html::Namespace::MathMl => {
                    if tag == "annotation-xml" {
                        if let Some(encoding) = attributes.get("encoding") {
                            let enc = encoding.to_lowercase();
                            return enc == "text/html" || enc == "application/xhtml+xml";
                        }
                    }
                }
                crate::ace::html::Namespace::Svg => {
                    return matches!(tag.as_str(), "foreignObject" | "desc" | "title");
                }
                _ => {}
            }
        }
        false
    }

    fn is_integration_point(&self, id: usize) -> bool {
        self.is_mathml_text_integration_point(id) || self.is_html_integration_point(id)
    }

    fn setup_fragment_mode(&mut self, context: &str) {
        // WHATWG 13.2.11: Parsing HTML fragments
        use crate::ace::html::lexer::LexerState;

        let tag = context.to_string();
        let nid = self.create_node(InternalNodeData::Element { 
            tag: tag.clone(), 
            namespace: crate::ace::html::Namespace::Html, 
            attributes: HashMap::new() 
        });
        self.append_node(self.root_id, nid);
        self.open_elements.push(nid);
        
        match tag.as_str() {
            "title" | "textarea" => {
                self.tokenizer.set_state(LexerState::RcData);
            }
            "style" | "xmp" | "iframe" | "noembed" | "noframes" => {
                self.tokenizer.set_state(LexerState::RawText);
            }
            "script" => {
                self.tokenizer.set_state(LexerState::ScriptData);
            }
            "noscript" if self.scripting_enabled => {
                self.tokenizer.set_state(LexerState::RawText);
            }
            "plaintext" => {
                self.tokenizer.set_state(LexerState::PlainText);
            }
            _ => {}
        }
        
        self.reset_insertion_mode_appropriately();
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

pub fn build_fragment(input: &str, context_element: Option<&str>) -> Vec<HtmlNode> {
    let mut builder = HtmlTreeBuilder::new(input);
    
    if let Some(context) = context_element {
        builder.setup_fragment_mode(context);
    }
    
    let output = builder.run();
    extract_fragment_children(&output.document.children, context_element.is_some())
}

pub fn build_fragment_with_errors(input: &str, context_element: Option<&str>) -> TreeBuildOutput {
    let mut builder = HtmlTreeBuilder::new(input);
    
    if let Some(context) = context_element {
        builder.setup_fragment_mode(context);
    }
    
    let mut output = builder.run();
    output.document.children = extract_fragment_children(&output.document.children, context_element.is_some());
    output
}

fn extract_fragment_children(document_children: &[HtmlNode], is_fragment: bool) -> Vec<HtmlNode> {
    if !is_fragment {
        return document_children.to_vec();
    }

    // Para fragmentos, o primeiro elemento na raiz do documento simulado é o nosso contexto
    for node in document_children {
        if let HtmlNode::Element(el) = node {
            return el.children.clone();
        }
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::build_document;
    use crate::ace::html::HtmlNode;

    #[test]
    fn test_aaa_p_b_i() {
        let html = "<p><b><i>x</b>y</i></p>";
        let doc = build_document(html);
        // HTML Spec AAA expectation: <p><b><i>x</i></b><i>y</i></p>

        let HtmlNode::Element(html) = &doc.children[0] else {
            panic!("expected html element");
        };
        let HtmlNode::Element(body) = &html.children[1] else {
            panic!("expected body element");
        };
        let HtmlNode::Element(el_p) = &body.children[0] else {
            panic!("expected p element");
        };

        assert_eq!(el_p.tag, "p");
        let HtmlNode::Element(el_b) = &el_p.children[0] else {
            panic!("expected b");
        };
        assert_eq!(el_b.tag, "b");
        let HtmlNode::Element(el_i) = &el_b.children[0] else {
            panic!("expected i");
        };
        assert_eq!(el_i.tag, "i");
        assert_eq!(el_i.children[0], HtmlNode::Text("x".to_string()));

        let HtmlNode::Element(el_i2) = &el_p.children[1] else {
            panic!("expected second i");
        };
        assert_eq!(el_i2.tag, "i");
        assert_eq!(el_i2.children[0], HtmlNode::Text("y".to_string()));
    }

    #[test]
    fn test_foster_parenting() {
        let html = "<table>hello<tr><td>cell</td></tr></table>";
        let doc = build_document(html);
        // Expectation: "hello" is foster-parented before the table

        let HtmlNode::Element(html) = &doc.children[0] else {
            panic!("expected html element");
        };
        let HtmlNode::Element(body) = &html.children[1] else {
            panic!("expected body element");
        };

        assert_eq!(body.children.len(), 2);
        assert_eq!(body.children[0], HtmlNode::Text("hello".to_string()));
        let HtmlNode::Element(el_table) = &body.children[1] else {
            panic!("expected table element");
        };
        assert_eq!(el_table.tag, "table");
    }

    #[test]
    fn test_svg_foreign_content() {
        // Teste: div (HTML) -> svg (SVG) -> circle (SVG/Self-Closing) -> foreignObject (SVG) -> span (HTML/Integration)
        let html = "<div><svg><circle /><foreignObject><span>hi</span></foreignObject></svg></div>";
        let doc = build_document(html);

        let HtmlNode::Element(html_el) = &doc.children[0] else { panic!("expected html"); };
        let HtmlNode::Element(body) = &html_el.children[1] else { panic!("expected body"); };
        let HtmlNode::Element(div) = &body.children[0] else { panic!("expected div"); };
        let HtmlNode::Element(svg) = &div.children[0] else { panic!("expected svg"); };
        
        assert_eq!(svg.tag, "svg");
        assert_eq!(svg.namespace, crate::ace::html::Namespace::Svg);
        
        let HtmlNode::Element(circle) = &svg.children[0] else { panic!("expected circle"); };
        assert_eq!(circle.tag, "circle");
        assert_eq!(circle.namespace, crate::ace::html::Namespace::Svg);
        assert_eq!(circle.children.len(), 0); // Deve estar vazio por ser self-closing em SVG

        let HtmlNode::Element(fo) = &svg.children[1] else { panic!("expected foreignObject"); };
        assert_eq!(fo.tag, "foreignObject");
        assert_eq!(fo.namespace, crate::ace::html::Namespace::Svg);

        let HtmlNode::Element(span) = &fo.children[0] else { panic!("expected span"); };
        assert_eq!(span.tag, "span");
        assert_eq!(span.namespace, crate::ace::html::Namespace::Html); // Volta para HTML através do integration point
    }
}
