use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreloadResourceType {
    Script,
    Stylesheet,
    Image,
    Video,
    Audio,
    Source,
}

#[derive(Debug, Clone)]
pub struct PreloadRequest {
    pub url: String,
    pub resource_type: PreloadResourceType,
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
}

pub struct PreloadScanner {
    state: PreloadScannerState,
    current_tag: String,
    current_attr_name: String,
    current_attr_value: String,
    requests: Vec<PreloadRequest>,
}

impl PreloadScanner {
    pub fn new() -> Self {
        Self {
            state: PreloadScannerState::Data,
            current_tag: String::new(),
            current_attr_name: String::new(),
            current_attr_value: String::new(),
            requests: Vec::new(),
        }
    }

    pub fn scan(&mut self, input: &str) -> Vec<PreloadRequest> {
        self.requests.clear();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            match self.state {
                PreloadScannerState::Data => {
                    if ch == '<' {
                        self.state = PreloadScannerState::TagOpen;
                    }
                }
                PreloadScannerState::TagOpen => {
                    if ch.is_alphabetic() {
                        self.current_tag.clear();
                        self.current_tag.push(ch.to_ascii_lowercase());
                        self.state = PreloadScannerState::TagName;
                    } else if ch == '!' || ch == '?' || ch == '/' {
                         self.state = PreloadScannerState::Data; // Ignore
                    } else {
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
