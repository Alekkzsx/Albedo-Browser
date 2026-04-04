//! Preload scanner fast-path para descoberta especulativa de recursos.

use std::collections::{HashMap, HashSet};

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
            _ => Self::Normal,
        }
    }

    pub fn default_for_type(resource_type: PreloadResourceType) -> Self {
        match resource_type {
            PreloadResourceType::Stylesheet => Self::Highest,
            PreloadResourceType::Script | PreloadResourceType::ModuleScript => Self::Highest,
            PreloadResourceType::Font => Self::High,
            PreloadResourceType::Prefetch | PreloadResourceType::DnsPrefetch => Self::Low,
            _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrossOrigin {
    Anonymous,
    UseCredentials,
}

impl CrossOrigin {
    pub fn from_attr(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "anonymous" | "" => Some(Self::Anonymous),
            "use-credentials" => Some(Self::UseCredentials),
            _ => None,
        }
    }
}

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
    pub loading: Option<String>,
}

impl PreloadRequest {
    pub fn new(url: String, resource_type: PreloadResourceType) -> Self {
        Self {
            url,
            resource_type,
            priority: ResourcePriority::default_for_type(resource_type),
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

    pub fn blocks_render(&self) -> bool {
        match self.resource_type {
            PreloadResourceType::Stylesheet => true,
            PreloadResourceType::Script if !self.is_async && !self.is_defer => true,
            _ => false,
        }
    }

    pub fn can_defer(&self) -> bool {
        match self.resource_type {
            PreloadResourceType::Script => self.is_async || self.is_defer,
            PreloadResourceType::Image => self.loading.as_deref() == Some("lazy"),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreloadScannerState {
    Data,
    TagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    Comment,
    BogusComment,
}

pub struct PreloadScanner {
    state: PreloadScannerState,
    current_tag: String,
    current_attr_name: String,
    current_attr_value: String,
    current_attrs: HashMap<String, String>,
    current_is_end_tag: bool,
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
            current_attrs: HashMap::new(),
            current_is_end_tag: false,
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

    pub fn scan(&mut self, input: &str) -> Vec<PreloadRequest> {
        self.requests.clear();
        self.seen_urls.clear();
        self.reset_tag_state();
        self.state = PreloadScannerState::Data;

        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            match self.state {
                PreloadScannerState::Data => {
                    if ch == '<' {
                        self.state = PreloadScannerState::TagOpen;
                        self.reset_tag_state();
                    }
                }
                PreloadScannerState::TagOpen => {
                    if ch == '!' {
                        if chars.peek() == Some(&'-') {
                            chars.next();
                            if chars.peek() == Some(&'-') {
                                chars.next();
                                self.state = PreloadScannerState::Comment;
                            } else {
                                self.state = PreloadScannerState::BogusComment;
                            }
                        } else {
                            self.state = PreloadScannerState::BogusComment;
                        }
                    } else if ch == '?' {
                        self.state = PreloadScannerState::BogusComment;
                    } else if ch == '/' {
                        self.current_is_end_tag = true;
                        self.state = PreloadScannerState::TagName;
                    } else if ch.is_ascii_alphabetic() {
                        self.current_tag.push(ch.to_ascii_lowercase());
                        self.state = PreloadScannerState::TagName;
                    } else {
                        self.state = PreloadScannerState::Data;
                    }
                }
                PreloadScannerState::TagName => {
                    if ch.is_whitespace() {
                        self.state = PreloadScannerState::BeforeAttributeName;
                    } else if ch == '>' {
                        self.emit_tag();
                    } else if ch != '/' {
                        self.current_tag.push(ch.to_ascii_lowercase());
                    }
                }
                PreloadScannerState::BeforeAttributeName => {
                    if ch.is_whitespace() {
                        continue;
                    }
                    if ch == '>' {
                        self.emit_tag();
                    } else if ch != '/' {
                        self.current_attr_name.clear();
                        self.current_attr_name.push(ch.to_ascii_lowercase());
                        self.current_attr_value.clear();
                        self.state = PreloadScannerState::AttributeName;
                    }
                }
                PreloadScannerState::AttributeName => {
                    if ch == '=' {
                        self.state = PreloadScannerState::BeforeAttributeValue;
                    } else if ch.is_whitespace() {
                        self.process_attribute();
                        self.state = PreloadScannerState::AfterAttributeName;
                    } else if ch == '>' {
                        self.process_attribute();
                        self.emit_tag();
                    } else if ch == '/' {
                        self.process_attribute();
                        self.state = PreloadScannerState::BeforeAttributeName;
                    } else {
                        self.current_attr_name.push(ch.to_ascii_lowercase());
                    }
                }
                PreloadScannerState::AfterAttributeName => {
                    if ch.is_whitespace() {
                        continue;
                    }
                    if ch == '=' {
                        self.state = PreloadScannerState::BeforeAttributeValue;
                    } else if ch == '>' {
                        self.emit_tag();
                    } else if ch != '/' {
                        self.current_attr_name.clear();
                        self.current_attr_name.push(ch.to_ascii_lowercase());
                        self.current_attr_value.clear();
                        self.state = PreloadScannerState::AttributeName;
                    }
                }
                PreloadScannerState::BeforeAttributeValue => {
                    if ch.is_whitespace() {
                        continue;
                    }
                    self.current_attr_value.clear();
                    match ch {
                        '"' => self.state = PreloadScannerState::AttributeValueDoubleQuoted,
                        '\'' => self.state = PreloadScannerState::AttributeValueSingleQuoted,
                        '>' => self.emit_tag(),
                        _ => {
                            self.current_attr_value.push(ch);
                            self.state = PreloadScannerState::AttributeValueUnquoted;
                        }
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
                    } else {
                        self.current_attr_value.push(ch);
                    }
                }
                PreloadScannerState::AfterAttributeValueQuoted => {
                    if ch.is_whitespace() {
                        self.state = PreloadScannerState::BeforeAttributeName;
                    } else if ch == '>' {
                        self.emit_tag();
                    } else if ch != '/' {
                        self.current_attr_name.clear();
                        self.current_attr_name.push(ch.to_ascii_lowercase());
                        self.current_attr_value.clear();
                        self.state = PreloadScannerState::AttributeName;
                    }
                }
                PreloadScannerState::Comment => {
                    if ch == '-' && chars.peek() == Some(&'-') {
                        chars.next();
                        if chars.peek() == Some(&'>') {
                            chars.next();
                            self.state = PreloadScannerState::Data;
                        }
                    }
                }
                PreloadScannerState::BogusComment => {
                    if ch == '>' {
                        self.state = PreloadScannerState::Data;
                    }
                }
            }
        }

        if matches!(
            self.state,
            PreloadScannerState::AttributeName
                | PreloadScannerState::AfterAttributeName
                | PreloadScannerState::AttributeValueDoubleQuoted
                | PreloadScannerState::AttributeValueSingleQuoted
                | PreloadScannerState::AttributeValueUnquoted
                | PreloadScannerState::AfterAttributeValueQuoted
                | PreloadScannerState::BeforeAttributeName
                | PreloadScannerState::TagName
        ) {
            self.process_attribute();
            self.emit_tag();
        }

        self.requests.clone()
    }

    fn process_attribute(&mut self) {
        if self.current_attr_name.is_empty() {
            return;
        }

        let value = self.current_attr_value.clone();
        self.current_attrs
            .insert(self.current_attr_name.clone(), value);
        self.current_attr_name.clear();
        self.current_attr_value.clear();
    }

    fn emit_tag(&mut self) {
        if !self.current_is_end_tag {
            self.emit_requests_for_current_tag();
        }
        self.reset_tag_state();
        self.state = PreloadScannerState::Data;
    }

    fn emit_requests_for_current_tag(&mut self) {
        match self.current_tag.as_str() {
            "script" => {
                if let Some(src) = self.current_attrs.get("src").cloned() {
                    let type_attr = self
                        .current_attrs
                        .get("type")
                        .map(|v| v.to_ascii_lowercase())
                        .unwrap_or_default();
                    let resource_type = if type_attr == "module" {
                        PreloadResourceType::ModuleScript
                    } else {
                        PreloadResourceType::Script
                    };

                    let mut req = PreloadRequest::new(src, resource_type);
                    req.is_module = resource_type == PreloadResourceType::ModuleScript;
                    req.is_async = self.current_attrs.contains_key("async");
                    req.is_defer = self.current_attrs.contains_key("defer");
                    self.apply_common_attrs(&mut req);
                    self.push_request(req);
                }
            }
            "link" => {
                let rel = self
                    .current_attrs
                    .get("rel")
                    .map(|v| v.to_ascii_lowercase())
                    .unwrap_or_default();
                if let Some(href) = self.current_attrs.get("href").cloned() {
                    let resource_type = if rel.contains("stylesheet") {
                        Some(PreloadResourceType::Stylesheet)
                    } else if rel.contains("manifest") {
                        Some(PreloadResourceType::Manifest)
                    } else if rel.contains("icon") {
                        Some(PreloadResourceType::Icon)
                    } else if rel.contains("preload") {
                        self.current_attrs
                            .get("as")
                            .and_then(|v| PreloadResourceType::from_as_attr(v))
                    } else if rel.contains("prefetch") {
                        Some(PreloadResourceType::Prefetch)
                    } else if rel.contains("dns-prefetch") {
                        Some(PreloadResourceType::DnsPrefetch)
                    } else if rel.contains("preconnect") {
                        Some(PreloadResourceType::Preconnect)
                    } else {
                        None
                    };

                    if let Some(resource_type) = resource_type {
                        let mut req = PreloadRequest::new(href, resource_type);
                        self.apply_common_attrs(&mut req);
                        req.rel = Some(rel);
                        req.as_attribute = self.current_attrs.get("as").cloned();
                        self.push_request(req);
                    }
                }
            }
            "img" => {
                if let Some(src) = self.current_attrs.get("src").cloned() {
                    let mut req = PreloadRequest::new(src, PreloadResourceType::Image);
                    self.apply_common_attrs(&mut req);
                    self.push_request(req);
                }

                if let Some(srcset) = self.current_attrs.get("srcset") {
                    for candidate in parse_srcset_urls(srcset) {
                        let mut req = PreloadRequest::new(candidate, PreloadResourceType::Image);
                        self.apply_common_attrs(&mut req);
                        self.push_request(req);
                    }
                }
            }
            "source" => {
                if let Some(src) = self.current_attrs.get("src").cloned() {
                    let mut req = PreloadRequest::new(src, PreloadResourceType::Source);
                    self.apply_common_attrs(&mut req);
                    self.push_request(req);
                }
                if let Some(srcset) = self.current_attrs.get("srcset") {
                    for candidate in parse_srcset_urls(srcset) {
                        let mut req = PreloadRequest::new(candidate, PreloadResourceType::Source);
                        self.apply_common_attrs(&mut req);
                        self.push_request(req);
                    }
                }
            }
            "video" => {
                if let Some(src) = self.current_attrs.get("src").cloned() {
                    let mut req = PreloadRequest::new(src, PreloadResourceType::Video);
                    self.apply_common_attrs(&mut req);
                    self.push_request(req);
                }
                if let Some(poster) = self.current_attrs.get("poster").cloned() {
                    let mut req = PreloadRequest::new(poster, PreloadResourceType::Image);
                    self.apply_common_attrs(&mut req);
                    self.push_request(req);
                }
            }
            "audio" => {
                if let Some(src) = self.current_attrs.get("src").cloned() {
                    let mut req = PreloadRequest::new(src, PreloadResourceType::Audio);
                    self.apply_common_attrs(&mut req);
                    self.push_request(req);
                }
            }
            _ => {}
        }
    }

    fn apply_common_attrs(&self, req: &mut PreloadRequest) {
        if let Some(fetchpriority) = self.current_attrs.get("fetchpriority").cloned() {
            req.priority = ResourcePriority::from_fetchpriority(&fetchpriority);
            req.fetchpriority = Some(fetchpriority);
        }
        if let Some(crossorigin) = self.current_attrs.get("crossorigin") {
            req.crossorigin = CrossOrigin::from_attr(crossorigin);
        }
        if let Some(integrity) = self.current_attrs.get("integrity").cloned() {
            req.integrity = Some(integrity);
        }
        if let Some(media) = self.current_attrs.get("media").cloned() {
            req.media = Some(media);
        }
        if let Some(loading) = self.current_attrs.get("loading").cloned() {
            req.loading = Some(loading);
        }
    }

    fn push_request(&mut self, req: PreloadRequest) {
        let key = normalize_request_url(&self.base_url, &req.url);
        if self.seen_urls.insert(key) {
            self.requests.push(req);
        }
    }

    fn reset_tag_state(&mut self) {
        self.current_tag.clear();
        self.current_attr_name.clear();
        self.current_attr_value.clear();
        self.current_attrs.clear();
        self.current_is_end_tag = false;
    }
}

impl Default for PreloadScanner {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_request_url(base_url: &str, url: &str) -> String {
    if base_url.is_empty()
        || url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("//")
        || url.starts_with("data:")
    {
        return url.to_string();
    }

    if base_url.ends_with('/') || url.starts_with('/') {
        format!("{base_url}{url}")
    } else {
        format!("{base_url}/{url}")
    }
}

fn parse_srcset_urls(srcset: &str) -> Vec<String> {
    srcset
        .split(',')
        .filter_map(|candidate| candidate.split_whitespace().next())
        .filter(|url| !url.is_empty())
        .map(str::to_string)
        .collect()
}
