
//! Preload Scanner Avançado para ACE-HTML
//! 
//! Detecta recursos críticos em documentos HTML para pré-carregamento otimizado.
//! Suporta detecção de scripts, stylesheets, imagens, vídeos, fonts, e mais.

use std::collections::HashSet;

/// Tipos de recursos que podem ser pré-carregados
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreloadResourceType {
    Script,
    ModuleScript,
    Stylesheet,
    Image,
    Video,
    Audio,
    Source,
    Font,
    Fetch,
    Worker,
    Manifest,
    Icon,
    Prefetch,
    DnsPrefetch,
    Preconnect,
}

impl PreloadResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Script => "script",
            Self::ModuleScript => "module-script",
            Self::Stylesheet => "stylesheet",
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Source => "source",
            Self::Font => "font",
            Self::Fetch => "fetch",
            Self::Worker => "worker",
            Self::Manifest => "manifest",
            Self::Icon => "icon",
            Self::Prefetch => "prefetch",
            Self::DnsPrefetch => "dns-prefetch",
            Self::Preconnect => "preconnect",
        }
    }

    pub fn from_as_attr(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "script" => Some(Self::Script),
            "module-script" | "module" => Some(Self::ModuleScript),
            "style" | "stylesheet" => Some(Self::Stylesheet),
            "image" | "img" => Some(Self::Image),
            "video" => Some(Self::Video),
            "audio" => Some(Self::Audio),
            "source" => Some(Self::Source),
            "font" => Some(Self::Font),
            "fetch" => Some(Self::Fetch),
            "worker" => Some(Self::Worker),
            "manifest" => Some(Self::Manifest),
            "icon" => Some(Self::Icon),
            "prefetch" => Some(Self::Prefetch),
            "dns-prefetch" => Some(Self::DnsPrefetch),
            "preconnect" => Some(Self::Preconnect),
            _ => None,
        }
    }
}

/// Prioridade de carregamento de recursos
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourcePriority {
    Lowest,
    Low,
    Normal,
    High,
    Highest,
    Critical,
}

impl ResourcePriority {
    pub fn from_fetchpriority(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "high" => Self::High,
            "low" => Self::Low,
            "auto" | _ => Self::Normal,
        }
    }

    pub fn default_for_type(resource_type: PreloadResourceType) -> Self {
        match resource_type {
            PreloadResourceType::Stylesheet => Self::Highest,
            PreloadResourceType::Script | PreloadResourceType::ModuleScript => Self::Highest,
            PreloadResourceType::Font => Self::High,
            PreloadResourceType::Image => Self::Normal,
            PreloadResourceType::Video | PreloadResourceType::Audio => Self::Normal,
            PreloadResourceType::Prefetch | PreloadResourceType::DnsPrefetch => Self::Low,
            _ => Self::Normal,
        }
    }
}

/// Atributo crossorigin para recursos
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrossOrigin {
    Anonymous,
    UseCredentials,
}

impl CrossOrigin {
    pub fn from_attr(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "anonymous" => Some(Self::Anonymous),
            "use-credentials" => Some(Self::UseCredentials),
            "" => Some(Self::Anonymous),
            _ => None,
        }
    }
}

/// Requisição de pré-carregamento
#[derive(Debug, Clone)]
pub struct PreloadRequest {
    pub url: String,
    pub resource_type: PreloadResourceType,
    pub priority: ResourcePriority,
    pub crossorigin: Option<CrossOrigin>,
    pub integrity: Option<String>,
    pub media: Option<String>,
    pub fetchpriority: Option<String>,
    pub rel: Option<String>,
    pub as_attribute: Option<String>,
    pub is_module: bool,
    pub is_async: bool,
    pub is_defer: bool,
    pub loading: Option<String>, // "lazy", "eager"
}

impl PreloadRequest {
    pub fn new(url: String, resource_type: PreloadResourceType) -> Self {
        let priority = ResourcePriority::default_for_type(resource_type);
        Self {
            url,
            resource_type,
            priority,
            crossorigin: None,
            integrity: None,
            media: None,
            fetchpriority: None,
            rel: None,
            as_attribute: None,
            is_module: false,
            is_async: false,
            is_defer: false,
            loading: None,
        }
    }

    pub fn with_priority(mut self, priority: ResourcePriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_crossorigin(mut self, crossorigin: CrossOrigin) -> Self {
        self.crossorigin = Some(crossorigin);
        self
    }

    pub fn with_integrity(mut self, integrity: String) -> Self {
        self.integrity = Some(integrity);
        self
    }

    pub fn with_media(mut self, media: String) -> Self {
        self.media = Some(media);
        self
    }

    pub fn with_loading(mut self, loading: String) -> Self {
        self.loading = Some(loading);
        self
    }

    /// Se este recurso deve bloquear o render
    pub fn blocks_render(&self) -> bool {
        match self.resource_type {
            PreloadResourceType::Stylesheet => true,
            PreloadResourceType::Script if !self.is_async && !self.is_defer => true,
            _ => false,
        }
    }

    /// Se este recurso pode ser adiado
    pub fn can_defer(&self) -> bool {
        match self.resource_type {
            PreloadResourceType::Script => self.is_async || self.is_defer,
            PreloadResourceType::Image => self.loading.as_deref() == Some("lazy"),
            _ => false,
        }
    }
}

/// Scanner avançado de pré-carregamento
pub struct PreloadScanner {
    state: PreloadScannerState,
    current_tag: String,
    current_attr_name: String,
    current_attr_value: String,
    current_rel: String,
    current_as: String,
    requests: Vec<PreloadRequest>,
    seen_urls: HashSet<String>,
    base_url: String,
}

impl PreloadScanner {
    pub fn new() -> Self {
        Self {
            state: PreloadScannerState::Data,
            current_tag: String::new(),
            current_attr_name: String::new(),
            current_attr_value: String::new(),
            current_rel: String::new(),
            current_as: String::new(),
            requests: Vec::new(),
            seen_urls: HashSet::new(),
            base_url: String::new(),
        }
    }

    pub fn with_base_url(base_url: String) -> Self {
        let mut scanner = Self::new();
        scanner.base_url = base_url;
        scanner
    }

    /// Escaneia HTML e retorna requisições de preload
    pub fn scan(&mut self, input: &str) -> Vec<PreloadRequest> {
        self.requests.clear();
        self.seen_urls.clear();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            match self.state {
                PreloadScannerState::Data => {
                    if ch == '<' {
                        self.state = PreloadScannerState::TagOpen;
                    }
                }
                PreloadScannerState::TagOpen => {
                    if ch == '!' {
                        // Verifica se é comentário ou doctype
                        if chars.peek() == Some(&'-') {
                            chars.next();
                            if chars.peek() == Some(&'-') {
                                chars.next();
                                self.state = PreloadScannerState::Comment;
                            } else {
                                self.state = PreloadScannerState::BogusComment;
                            }
                        } else if chars.peek().map(|c| c.is_alphabetic()).unwrap_or(false) {
                            // DOCTYPE
                            self.state = PreloadScannerState::TagName;
                        } else {
                            self.state = PreloadScannerState::BogusComment;
                        }
                    } else if ch == '?' {
                        self.state = PreloadScannerState::BogusComment;
                    } else if ch == '/' {
                        // End tag - ignora para preload
                        self.state = PreloadScannerState::TagName;
                    } else if ch.is_alphabetic() {
                        self.current_tag.clear();
                        self.current_tag.push(ch.to_ascii_lowercase());
                        self.state = PreloadScannerState::TagName;
                    } else {
                        self.state = PreloadScannerState::Data;
                    }
                }
                PreloadScannerState::Comment => {
                    // Busca por -->
                    if ch == '-' {
                        if chars.peek() == Some(&'-') {
                            chars.next();
                            if chars.peek() == Some(&'>') {
                                chars.next();
                                self.state = PreloadScannerState::Data;
                            }
                        }
                    }
                }
                PreloadScannerState::BogusComment => {
                    if ch == '>' {
                        self.state = PreloadScannerState::Data;
                    }
                }
                PreloadScannerState::TagName => {
                    if ch.is_whitespace() {
                        self.state = PreloadScannerState::BeforeAttributeName;
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else if ch == '/' {
                        // Self-closing tag - continua
                    } else {
                        self.current_tag.push(ch.to_ascii_lowercase());
                    }
                }
                PreloadScannerState::BeforeAttributeName => {
                    if ch.is_whitespace() {
                        // Skip
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else if ch == '/' {
                        // Self-closing - continua
                    } else {
                        self.current_attr_name.clear();
                        self.current_attr_name.push(ch.to_ascii_lowercase());
                        self.state = PreloadScannerState::AttributeName;
                    }
                }
                PreloadScannerState::AttributeName => {
                    if ch == '=' {
                        self.state = PreloadScannerState::BeforeAttributeValue;
                    } else if ch.is_whitespace() {
                        self.state = PreloadScannerState::AfterAttributeName;
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else if ch == '/' {
                        self.state = PreloadScannerState::BeforeAttributeName;
                    } else {
                        self.current_attr_name.push(ch.to_ascii_lowercase());
                    }
                }
                PreloadScannerState::AfterAttributeName => {
                    if ch.is_whitespace() {
                        // Skip
                    } else if ch == '=' {
                        self.state = PreloadScannerState::BeforeAttributeValue;
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else if ch == '/' {
                        // Self-closing
                    } else {
                        self.current_attr_name.clear();
                        self.current_attr_name.push(ch.to_ascii_lowercase());
                        self.state = PreloadScannerState::AttributeName;
                    }
                }
                PreloadScannerState::BeforeAttributeValue => {
                    if ch.is_whitespace() {
                        // Skip
                    } else if ch == '"' {
                        self.current_attr_value.clear();
                        self.state = PreloadScannerState::AttributeValueDoubleQuoted;
                    } else if ch == '\'' {
                        self.current_attr_value.clear();
                        self.state = PreloadScannerState::AttributeValueSingleQuoted;
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else {
                        self.current_attr_value.clear();
                        self.current_attr_value.push(ch);
                        self.state = PreloadScannerState::AttributeValueUnquoted;
                    }
                }
                PreloadScannerState::AttributeValueDoubleQuoted => {
                    if ch == '"' {
                        self.process_attribute();
                        self.state = PreloadScannerState::AfterAttributeValueQuoted;
                    } else {
                        self.current_attr_value.push(ch);
                    }
                }
                PreloadScannerState::AttributeValueSingleQuoted => {
                    if ch == '\'' {
                        self.process_attribute();
                        self.state = PreloadScannerState::AfterAttributeValueQuoted;
                    } else {
                        self.current_attr_value.push(ch);
                    }
                }
                PreloadScannerState::AttributeValueUnquoted => {
                    if ch.is_whitespace() {
                        self.process_attribute();
                        self.state = PreloadScannerState::BeforeAttributeName;
                    } else if ch == '>' {
                        self.process_attribute();
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else {
                        self.current_attr_value.push(ch);
                    }
                }
                PreloadScannerState::AfterAttributeValueQuoted => {
                    if ch.is_whitespace() {
                        self.state = PreloadScannerState::BeforeAttributeName;
                    } else if ch == '/' {
                        // Self-closing
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else {
                        self.state = PreloadScannerState::BeforeAttributeName;
                    }
                }
            }
        }

        self.requests.clone()
    }
            match self.state {
                PreloadScannerState::Data => {
                    if ch == '<' {
                        self.state = PreloadScannerState::TagOpen;
                    }
                }
                PreloadScannerState::TagOpen => {
                    if ch == '!' {
                        self.state = PreloadScannerState::Comment;
                    } else if ch == '?' {
                        self.state = PreloadScannerState::BogusComment;
                    } else if ch.is_alphabetic() {
                        self.current_tag.clear();
                        self.current_tag.push(ch.to_ascii_lowercase());
                        self.state = PreloadScannerState::TagName;
                    } else {
                        self.state = PreloadScannerState::Data;
                    }
                }
                PreloadScannerState::Comment => {
                    // Simplified: just look for -->
                    if ch == '-' {
                        if chars.peek() == Some(&'-') {
                            chars.next();
                            if chars.peek() == Some(&'>') {
                                chars.next();
                                self.state = PreloadScannerState::Data;
                            }
                        }
                    }
                }
                PreloadScannerState::BogusComment => {
                    if ch == '>' {
                        self.state = PreloadScannerState::Data;
                    }
                }
                PreloadScannerState::TagName => {
                    if ch.is_whitespace() {
                        self.state = PreloadScannerState::BeforeAttributeName;
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else {
                        self.current_tag.push(ch.to_ascii_lowercase());
                    }
                }
                PreloadScannerState::BeforeAttributeName => {
                    if ch.is_whitespace() {
                        // Skip
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else if ch == '/' {
                        // Skip
                    } else {
                        self.current_attr_name.clear();
                        self.current_attr_name.push(ch.to_ascii_lowercase());
                        self.state = PreloadScannerState::AttributeName;
                    }
                }
                PreloadScannerState::AttributeName => {
                    if ch == '=' {
                        self.state = PreloadScannerState::BeforeAttributeValue;
                    } else if ch.is_whitespace() {
                        self.state = PreloadScannerState::AfterAttributeName;
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else {
                        self.current_attr_name.push(ch.to_ascii_lowercase());
                    }
                }
                PreloadScannerState::AfterAttributeName => {
                    if ch.is_whitespace() {
                        // Skip
                    } else if ch == '=' {
                        self.state = PreloadScannerState::BeforeAttributeValue;
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else if ch == '/' {
                        // Skip
                    } else {
                        self.current_attr_name.clear();
                        self.current_attr_name.push(ch.to_ascii_lowercase());
                        self.state = PreloadScannerState::AttributeName;
                    }
                }
                PreloadScannerState::BeforeAttributeValue => {
                    if ch.is_whitespace() {
                        // Skip
                    } else if ch == '"' {
                        self.current_attr_value.clear();
                        self.state = PreloadScannerState::AttributeValueDoubleQuoted;
                    } else if ch == '\'' {
                        self.current_attr_value.clear();
                        self.state = PreloadScannerState::AttributeValueSingleQuoted;
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else {
                        self.current_attr_value.clear();
                        self.current_attr_value.push(ch);
                        self.state = PreloadScannerState::AttributeValueUnquoted;
                    }
                }
                PreloadScannerState::AttributeValueDoubleQuoted => {
                    if ch == '"' {
                        self.process_attribute();
                        self.state = PreloadScannerState::AfterAttributeValueQuoted;
                    } else {
                        self.current_attr_value.push(ch);
                    }
                }
                PreloadScannerState::AttributeValueSingleQuoted => {
                    if ch == '\'' {
                        self.process_attribute();
                        self.state = PreloadScannerState::AfterAttributeValueQuoted;
                    } else {
                        self.current_attr_value.push(ch);
                    }
                }
                PreloadScannerState::AttributeValueUnquoted => {
                    if ch.is_whitespace() {
                        self.process_attribute();
                        self.state = PreloadScannerState::BeforeAttributeName;
                    } else if ch == '>' {
                        self.process_attribute();
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else {
                        self.current_attr_value.push(ch);
                    }
                }
                PreloadScannerState::AfterAttributeValueQuoted => {
                    if ch.is_whitespace() {
                        self.state = PreloadScannerState::BeforeAttributeName;
                    } else if ch == '/' {
                        // Skip
                    } else if ch == '>' {
                        self.emit_tag();
                        self.state = PreloadScannerState::Data;
                    } else {
                        self.state = PreloadScannerState::BeforeAttributeName;
                        // Reprocess character in BeforeAttributeName? No, we skip for simplicity in fast scan
                    }
                }
            }
        }

        self.requests.clone()
    }

    fn process_attribute(&mut self) {
        if self.current_attr_value.is_empty() { return; }

        let resource_type = match self.current_tag.as_str() {
            "script" if self.current_attr_name == "src" => Some(PreloadResourceType::Script),
            "link" if self.current_attr_name == "href" => Some(PreloadResourceType::Stylesheet), // Simpler: assumed CSS for now
            "img" if self.current_attr_name == "src" => Some(PreloadResourceType::Image),
            "video" if self.current_attr_name == "poster" => Some(PreloadResourceType::Video),
            "audio" if self.current_attr_name == "src" => Some(PreloadResourceType::Audio),
            "source" if self.current_attr_name == "src" => Some(PreloadResourceType::Source),
            _ => None,
        };

        if let Some(rt) = resource_type {
            self.requests.push(PreloadRequest {
                url: self.current_attr_value.clone(),
                resource_type: rt,
            });
        }
    }

    fn emit_tag(&mut self) {
        self.current_tag.clear();
        self.current_attr_name.clear();
        self.current_attr_value.clear();
    }
}
