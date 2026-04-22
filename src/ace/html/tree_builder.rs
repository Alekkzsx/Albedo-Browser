use crate::ace::html::tokenizer_v2::{AceTokenizer, AceTokenKind};
use crate::ace::util::allocator::AceAllocator;
use fxhash::FxHashMap;
use smol_str::SmolStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub struct AceNodeV2<'a> {
    pub kind: AceNodeKind<'a>,
    pub children: Vec<usize>, 
    pub parent: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum AceNodeKind<'a> {
    Document,
    Element {
        name: SmolStr,
        attributes: FxHashMap<SmolStr, SmolStr>,
    },
    Text(&'a str),
    Comment(&'a str),
}

pub struct HtmlTreeBuilder<'a> {
    tokenizer: AceTokenizer<'a>,
    allocator: &'a AceAllocator,
    insertion_mode: InsertionMode,
    
    // Stack of open elements (WHATWG §13.2.4.2)
    open_elements: Vec<usize>,
    
    // Armazenamento temporário dos nós durante o parsing deste chunk
    nodes: Vec<AceNodeV2<'a>>,
    
    // Estado adicional da spec
    head_element: Option<usize>,
    form_element: Option<usize>,
    frameset_ok: bool,
}

impl<'a> HtmlTreeBuilder<'a> {
    pub fn new(input: &'a str, allocator: &'a AceAllocator) -> Self {
        let mut builder = Self {
            tokenizer: AceTokenizer::new(input, allocator),
            allocator,
            insertion_mode: InsertionMode::Initial,
            open_elements: Vec::new(),
            nodes: Vec::with_capacity(512),
            head_element: None,
            form_element: None,
            frameset_ok: true,
        };
        
        // Criar o nó Document raiz (índice 0)
        builder.nodes.push(AceNodeV2 {
            kind: AceNodeKind::Document,
            children: Vec::new(),
            parent: None,
        });
        builder.open_elements.push(0);
        
        builder
    }

    pub fn run(&mut self) -> Vec<AceNodeV2<'a>> {
        loop {
            let token = self.tokenizer.next_token();
            match token {
                Some(AceTokenKind::Eof) | None => break,
                Some(t) => self.handle_token(t),
            }
        }
        std::mem::take(&mut self.nodes)
    }

    fn current_node_idx(&self) -> usize {
        *self.open_elements.last().expect("Stack of open elements was empty")
    }

    fn insert_element(&mut self, name: &str, attrs: Vec<(&'a str, &'a str)>) -> usize {
        let mut attributes = FxHashMap::default();
        for (k, v) in attrs {
            attributes.insert(SmolStr::new(k), SmolStr::new(v));
        }

        let node_idx = self.nodes.len();
        let parent_idx = self.current_node_idx();

        self.nodes.push(AceNodeV2 {
            kind: AceNodeKind::Element {
                name: SmolStr::new(name),
                attributes,
            },
            children: Vec::new(),
            parent: Some(parent_idx),
        });

        self.nodes[parent_idx].children.push(node_idx);
        self.open_elements.push(node_idx);
        node_idx
    }

    fn insert_text(&mut self, data: &'a str) {
        let parent_idx = self.current_node_idx();
        let node_idx = self.nodes.len();
        self.nodes.push(AceNodeV2 {
            kind: AceNodeKind::Text(data),
            children: Vec::new(),
            parent: Some(parent_idx),
        });
        self.nodes[parent_idx].children.push(node_idx);
    }

    fn handle_token(&mut self, token: AceTokenKind<'a>) {
        match self.insertion_mode {
            InsertionMode::Initial => self.handle_initial(token),
            InsertionMode::BeforeHtml => self.handle_before_html(token),
            InsertionMode::BeforeHead => self.handle_before_head(token),
            InsertionMode::InHead => self.handle_in_head(token),
            InsertionMode::AfterHead => self.handle_after_head(token),
            InsertionMode::InBody => self.handle_in_body(token),
            _ => self.handle_in_body(token),
        }
    }

    fn handle_initial(&mut self, token: AceTokenKind<'a>) {
        match token {
            AceTokenKind::Doctype { .. } => self.insertion_mode = InsertionMode::BeforeHtml,
            _ => {
                self.insertion_mode = InsertionMode::BeforeHtml;
                self.handle_token(token);
            }
        }
    }

    fn handle_before_html(&mut self, token: AceTokenKind<'a>) {
        match token {
            AceTokenKind::StartTag { name, attributes, .. } if name == "html" => {
                self.insert_element("html", attributes);
                self.insertion_mode = InsertionMode::BeforeHead;
            }
            _ => {
                self.insert_element("html", Vec::new());
                self.insertion_mode = InsertionMode::BeforeHead;
                self.handle_token(token);
            }
        }
    }

    fn handle_before_head(&mut self, token: AceTokenKind<'a>) {
        match token {
            AceTokenKind::StartTag { name, attributes, .. } if name == "head" => {
                let idx = self.insert_element("head", attributes);
                self.head_element = Some(idx);
                self.insertion_mode = InsertionMode::InHead;
            }
            _ => {
                let idx = self.insert_element("head", Vec::new());
                self.head_element = Some(idx);
                self.insertion_mode = InsertionMode::InHead;
                self.handle_token(token);
            }
        }
    }

    fn handle_in_head(&mut self, token: AceTokenKind<'a>) {
        match token {
            AceTokenKind::StartTag { name, attributes, .. } if matches!(name, "meta" | "link" | "title" | "style") => {
                self.insert_element(name, attributes);
                self.open_elements.pop();
            }
            AceTokenKind::EndTag { name } if name == "head" => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::AfterHead;
            }
            _ => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::AfterHead;
                self.handle_token(token);
            }
        }
    }

    fn handle_after_head(&mut self, token: AceTokenKind<'a>) {
        match token {
            AceTokenKind::StartTag { name, attributes, .. } if name == "body" => {
                self.insert_element("body", attributes);
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InBody;
            }
            _ => {
                self.insert_element("body", Vec::new());
                self.insertion_mode = InsertionMode::InBody;
                self.handle_token(token);
            }
        }
    }

    fn handle_in_body(&mut self, token: AceTokenKind<'a>) {
        match token {
            AceTokenKind::Text { data } => self.insert_text(data),
            AceTokenKind::StartTag { name, attributes, .. } => {
                self.insert_element(name, attributes);
                if matches!(name, "img" | "br" | "hr" | "input" | "meta" | "link") {
                    self.open_elements.pop();
                }
            }
            AceTokenKind::EndTag { name } => {
                if let Some(pos) = self.open_elements.iter().rposition(|&idx| {
                    if let AceNodeKind::Element { name: n, .. } = &self.nodes[idx].kind {
                        n == name
                    } else {
                        false
                    }
                }) {
                    self.open_elements.truncate(pos);
                }
            }
            _ => {}
        }
    }
}
