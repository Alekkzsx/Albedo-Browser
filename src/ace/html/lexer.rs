use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartTagToken {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub self_closing: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctypeToken {
    pub name: Option<String>,
    pub public_id: Option<String>,
    pub system_id: Option<String>,
    pub force_quirks: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlToken {
    pub kind: HtmlTokenKind,
    pub line: usize,
    pub column: usize,
}

impl HtmlToken {
    /// Convenience constructor for the EOF sentinel token.
    pub fn eof() -> Self {
        Self { kind: HtmlTokenKind::Eof, line: 0, column: 0 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HtmlTokenKind {
    StartTag(StartTagToken),
    EndTag(String),
    Comment(String),
    Doctype(DoctypeToken),
    Character(String),
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexerState {
    Data,
    RcData,
    RawText,
    ScriptData,
    PlainText,

    TagOpen,
    EndTagOpen,
    TagName,

    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,

    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    BogusComment,
    CommentLessThanSign,
    CommentLessThanSignBang,
    CommentLessThanSignBangDash,
    CommentLessThanSignBangDashDash,

    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    AfterDoctypePublicKeyword,
    BeforeDoctypePublicIdentifier,
    DoctypePublicIdentifierDoubleQuoted,
    DoctypePublicIdentifierSingleQuoted,
    AfterDoctypePublicIdentifier,
    BetweenDoctypePublicAndSystemIdentifiers,
    AfterDoctypeSystemKeyword,
    BeforeDoctypeSystemIdentifier,
    DoctypeSystemIdentifierDoubleQuoted,
    DoctypeSystemIdentifierSingleQuoted,
    AfterDoctypeSystemIdentifier,
    BogusDoctype,

    RcDataLessThanSign,
    RcDataEndTagOpen,
    RcDataEndTagName,

    RawTextLessThanSign,
    RawTextEndTagOpen,
    RawTextEndTagName,

    ScriptDataLessThanSign,
    ScriptDataEndTagOpen,
    ScriptDataEndTagName,
    ScriptDataEscapeStart,
    ScriptDataEscapeStartDash,
    ScriptDataEscaped,
    ScriptDataEscapedDash,
    ScriptDataEscapedDashDash,
    ScriptDataEscapedLessThanSign,
    ScriptDataEscapedEndTagOpen,
    ScriptDataEscapedEndTagName,
    ScriptDataDoubleEscapeStart,
    ScriptDataDoubleEscaped,
    ScriptDataDoubleEscapedDash,
    ScriptDataDoubleEscapedDashDash,
    ScriptDataDoubleEscapedLessThanSign,
    ScriptDataDoubleEscapeEnd,

    // Missing 16 states for character references, comments and CDATA
    CharacterReference,
    NamedCharacterReference,
    AmbiguousAmpersand,
    NumericCharacterReference,
    HexadecimalCharacterReferenceStart,
    DecimalCharacterReferenceStart,
    HexadecimalCharacterReference,
    DecimalCharacterReference,
    NumericCharacterReferenceEnd,
    CdataSection,
    CdataSectionBracket,
    CdataSectionEnd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LexerErrorKind {
    InvalidTagName,
    InvalidEndTag,
    InvalidDoctype,
    BogusComment,

    UnexpectedNullCharacter,
    UnexpectedQuestionMarkInsteadOfTagName,
    InvalidFirstCharacterOfTagName,
    MissingEndTagName,
    EofBeforeTagName,
    EofInTag,
    EofInComment,
    EofInDoctype,
    EofInScriptHtmlCommentLikeText,
    AbruptClosingOfEmptyComment,
    UnexpectedSolidusInTag,
    UnexpectedEqualsSignBeforeAttributeName,
    UnexpectedCharacterInAttributeName,
    UnexpectedCharacterInUnquotedAttributeValue,
    MissingAttributeValue,
    MissingWhitespaceBetweenAttributes,
    IncorrectlyOpenedComment,
    MissingDoctypeName,
    InvalidCharacterSequenceAfterDoctypeName,
    MissingWhitespaceAfterDoctypePublicKeyword,
    MissingWhitespaceAfterDoctypeSystemKeyword,
    MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
    MissingQuoteBeforeDoctypePublicIdentifier,
    MissingQuoteBeforeDoctypeSystemIdentifier,
    MissingDoctypePublicIdentifier,
    MissingDoctypeSystemIdentifier,
    AbruptDoctypePublicIdentifier,
    AbruptDoctypeSystemIdentifier,
    UnknownNamedCharacterReference,
    MissingSemicolonAfterCharacterReference,
    AbsenceOfDigitsInNumericCharacterReference,
    AmbiguousAmpersand,
    NullCharacterReference,
    NestedComment,
    EofInCdata,
    MissingWhitespaceBeforeDoctypeName,
    NoncharacterInInputStream,
    SurrogateInInputStream,
    CharacterReferenceOutsideUnicodeRange,
    NoncharacterCharacterReference,
    SurrogateCharacterReference,
    EndTagWithTrailingSolidus,
    /// §13.2.5.8: whitespace after tag name when current token is an end tag
    EndTagWithAttributes,
    CdataSectionOutsideForeignContent,
    DuplicateAttribute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexerError {
    pub kind: LexerErrorKind,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl LexerError {
    fn new(kind: LexerErrorKind, message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            kind,
            message: message.into(),
            line,
            column,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct TagTokenBuilder {
    name: String,
    is_end_tag: bool,
    self_closing: bool,
    attributes: HashMap<String, String>,
    current_attr_name: String,
    current_attr_value: String,
}

impl TagTokenBuilder {
    fn new(is_end_tag: bool) -> Self {
        Self {
            is_end_tag,
            ..Default::default()
        }
    }

    fn push_name(&mut self, ch: char) {
        self.name.push(ch.to_ascii_lowercase());
    }

    fn begin_attribute(&mut self, first_char: Option<char>) {
        let _ = self.finish_attribute_if_needed();
        self.current_attr_name.clear();
        self.current_attr_value.clear();
        if let Some(ch) = first_char {
            self.current_attr_name.push(ch.to_ascii_lowercase());
        }
    }

    fn push_attribute_name(&mut self, ch: char) {
        self.current_attr_name.push(ch.to_ascii_lowercase());
    }

    fn push_attribute_value_char(&mut self, ch: char) {
        self.current_attr_value.push(ch);
    }

    #[allow(dead_code)]
    fn push_attribute_value_str(&mut self, value: &str) {
        self.current_attr_value.push_str(value);
    }

    fn finish_attribute_if_needed(&mut self) -> bool {
        if self.current_attr_name.is_empty() {
            self.current_attr_value.clear();
            return false;
        }

        let name = std::mem::take(&mut self.current_attr_name);
        let value = std::mem::take(&mut self.current_attr_value);
        let is_duplicate = self.attributes.contains_key(&name);
        self.attributes.entry(name).or_insert(value);
        is_duplicate
    }

    fn finish(mut self, line: usize, column: usize) -> HtmlToken {
        let _ = self.finish_attribute_if_needed();

        let kind = if self.is_end_tag {
            HtmlTokenKind::EndTag(self.name)
        } else {
            HtmlTokenKind::StartTag(StartTagToken {
                name: self.name,
                attributes: self.attributes,
                self_closing: self.self_closing,
            })
        };

        HtmlToken {
            kind,
            line,
            column,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct DoctypeBuilder {
    name: Option<String>,
    public_id: Option<String>,
    system_id: Option<String>,
    force_quirks: bool,
}

impl DoctypeBuilder {
    fn finish(self, line: usize, column: usize) -> HtmlToken {
        HtmlToken {
            kind: HtmlTokenKind::Doctype(DoctypeToken {
                name: self.name,
                public_id: self.public_id,
                system_id: self.system_id,
                force_quirks: self.force_quirks,
            }),
            line,
            column,
        }
    }
}

pub struct HtmlLexer {
    chars: VecDeque<char>,
    pos: usize,
    state: LexerState,

    pending: VecDeque<HtmlToken>,
    text_buffer: String,
    text_buffer_line: usize,
    text_buffer_column: usize,

    current_tag: Option<TagTokenBuilder>,
    current_comment: String,
    current_doctype: Option<DoctypeBuilder>,

    temporary_buffer: String,
    raw_text_tag: Option<String>,

    errors: Vec<LexerError>,
    eof_emitted: bool,
    input_ended: bool,

    // Position Tracking
    line: usize,
    column: usize,
    last_column: usize,

    // Compliance Fields
    return_state: LexerState,
    character_reference_code: u32,
    cdata_section_allowed: bool,
}

impl HtmlLexer {
    pub fn new(input: &str) -> Self {
        let mut lexer = Self {
            chars: VecDeque::new(),
            pos: 0,
            state: LexerState::Data,
            pending: VecDeque::new(),
            text_buffer: String::new(),
            text_buffer_line: 1,
            text_buffer_column: 1,
            current_tag: None,
            current_comment: String::new(),
            current_doctype: None,
            temporary_buffer: String::new(),
            raw_text_tag: None,
            errors: Vec::new(),
            eof_emitted: false,
            input_ended: false,

            line: 1,
            column: 1,
            last_column: 1,

            return_state: LexerState::Data,
            character_reference_code: 0,
            cdata_section_allowed: false,
        };
        lexer.feed(input);
        lexer
    }

    pub fn empty() -> Self {
        Self {
            chars: VecDeque::new(),
            pos: 0,
            state: LexerState::Data,
            pending: VecDeque::new(),
            text_buffer: String::new(),
            text_buffer_line: 1,
            text_buffer_column: 1,
            current_tag: None,
            current_comment: String::new(),
            current_doctype: None,
            temporary_buffer: String::new(),
            raw_text_tag: None,
            errors: Vec::new(),
            eof_emitted: false,
            input_ended: false,

            line: 1,
            column: 1,
            last_column: 1,

            return_state: LexerState::Data,
            character_reference_code: 0,
            cdata_section_allowed: false,
        }
    }

    pub fn feed(&mut self, input: &str) {
        self.chars.extend(input.chars());
    }

    pub fn end(&mut self) {
        self.input_ended = true;
    }

    pub fn set_raw_text_tag(&mut self, tag: Option<String>) {
        self.raw_text_tag = tag.map(|value| value.to_ascii_lowercase());
        self.state = match self.raw_text_tag.as_deref() {
            Some("title") | Some("textarea") => LexerState::RcData,
            Some("script") => LexerState::ScriptData,
            Some(_) => LexerState::RawText,
            None => LexerState::Data,
        };
    }

    pub fn set_cdata_allowed(&mut self, allowed: bool) {
        self.cdata_section_allowed = allowed;
    }

    pub fn errors(&self) -> &[LexerError] {
        &self.errors
    }

    pub fn take_errors(&mut self) -> Vec<LexerError> {
        std::mem::take(&mut self.errors)
    }

    pub fn next_token(&mut self) -> Option<HtmlToken> {
        loop {
            if let Some(token) = self.pending.pop_front() {
                return Some(token);
            }

            if self.eof_emitted {
                return Some(HtmlToken { kind: HtmlTokenKind::Eof, line: self.line, column: self.column });
            }

            let was_at_end = self.pos >= self.chars.len();
            if was_at_end && !self.input_ended {
                return None; // Need more data from feed()
            }

            self.step();

            if self.pos >= self.chars.len() && self.input_ended {
                self.flush_text();
                if let Some(token) = self.pending.pop_front() {
                    return Some(token);
                }

                if !was_at_end {
                    continue;
                }

                self.eof_emitted = true;
                return Some(HtmlToken { kind: HtmlTokenKind::Eof, line: self.line, column: self.column });
            }
            
            // If we're not at the end, or we just emitted tokens (like a tag), return them
            if let Some(token) = self.pending.pop_front() {
                return Some(token);
            }
            
            // If we didn't emit a token and we are at the end of current chunk, return None
            if self.pos >= self.chars.len() && !self.input_ended {
                return None;
            }
        }
    }

    fn step(&mut self) {
        let ch = self.consume_next_input_character();

        match self.state {
            LexerState::Data => self.state_data(ch),
            LexerState::RcData => self.state_rcdata(ch),
            LexerState::RawText => self.state_rawtext(ch),
            LexerState::ScriptData => self.state_scriptdata(ch),
            LexerState::PlainText => self.state_plaintext(ch),

            LexerState::TagOpen => self.state_tag_open(ch),
            LexerState::EndTagOpen => self.state_end_tag_open(ch),
            LexerState::TagName => self.state_tag_name(ch),

            LexerState::BeforeAttributeName => self.state_before_attribute_name(ch),
            LexerState::AttributeName => self.state_attribute_name(ch),
            LexerState::AfterAttributeName => self.state_after_attribute_name(ch),
            LexerState::BeforeAttributeValue => self.state_before_attribute_value(ch),
            LexerState::AttributeValueDoubleQuoted => self.state_attribute_value_double_quoted(ch),
            LexerState::AttributeValueSingleQuoted => self.state_attribute_value_single_quoted(ch),
            LexerState::AttributeValueUnquoted => self.state_attribute_value_unquoted(ch),
            LexerState::AfterAttributeValueQuoted => self.state_after_attribute_value_quoted(ch),
            LexerState::SelfClosingStartTag => self.state_self_closing_start_tag(ch),

            LexerState::MarkupDeclarationOpen => self.state_markup_declaration_open(ch),
            LexerState::CommentStart => self.state_comment_start(ch),
            LexerState::CommentStartDash => self.state_comment_start_dash(ch),
            LexerState::Comment => self.state_comment(ch),
            LexerState::CommentEndDash => self.state_comment_end_dash(ch),
            LexerState::CommentEnd => self.state_comment_end(ch),
            LexerState::CommentEndBang => self.state_comment_end_bang(ch),
            LexerState::BogusComment => self.state_bogus_comment(ch),

            LexerState::Doctype => self.state_doctype(ch),
            LexerState::BeforeDoctypeName => self.state_before_doctype_name(ch),
            LexerState::DoctypeName => self.state_doctype_name(ch),
            LexerState::AfterDoctypeName => self.state_after_doctype_name(ch),
            LexerState::AfterDoctypePublicKeyword => self.state_after_doctype_public_keyword(ch),
            LexerState::BeforeDoctypePublicIdentifier => {
                self.state_before_doctype_public_identifier(ch)
            }
            LexerState::DoctypePublicIdentifierDoubleQuoted => {
                self.state_doctype_public_identifier_double_quoted(ch)
            }
            LexerState::DoctypePublicIdentifierSingleQuoted => {
                self.state_doctype_public_identifier_single_quoted(ch)
            }
            LexerState::AfterDoctypePublicIdentifier => {
                self.state_after_doctype_public_identifier(ch)
            }
            LexerState::BetweenDoctypePublicAndSystemIdentifiers => {
                self.state_between_doctype_public_and_system_identifiers(ch)
            }
            LexerState::AfterDoctypeSystemKeyword => self.state_after_doctype_system_keyword(ch),
            LexerState::BeforeDoctypeSystemIdentifier => {
                self.state_before_doctype_system_identifier(ch)
            }
            LexerState::DoctypeSystemIdentifierDoubleQuoted => {
                self.state_doctype_system_identifier_double_quoted(ch)
            }
            LexerState::DoctypeSystemIdentifierSingleQuoted => {
                self.state_doctype_system_identifier_single_quoted(ch)
            }
            LexerState::AfterDoctypeSystemIdentifier => {
                self.state_after_doctype_system_identifier(ch)
            }
            LexerState::BogusDoctype => self.state_bogus_doctype(ch),

            LexerState::RcDataLessThanSign => self.state_rcdata_less_than_sign(ch),
            LexerState::RcDataEndTagOpen => self.state_rcdata_end_tag_open(ch),
            LexerState::RcDataEndTagName => self.state_rcdata_end_tag_name(ch),

            LexerState::RawTextLessThanSign => self.state_rawtext_less_than_sign(ch),
            LexerState::RawTextEndTagOpen => self.state_rawtext_end_tag_open(ch),
            LexerState::RawTextEndTagName => self.state_rawtext_end_tag_name(ch),

            LexerState::ScriptDataLessThanSign => self.state_scriptdata_less_than_sign(ch),
            LexerState::ScriptDataEndTagOpen => self.state_scriptdata_end_tag_open(ch),
            LexerState::ScriptDataEndTagName => self.state_scriptdata_end_tag_name(ch),
            LexerState::ScriptDataEscapeStart => self.state_scriptdata_escape_start(ch),
            LexerState::ScriptDataEscapeStartDash => self.state_scriptdata_escape_start_dash(ch),
            LexerState::ScriptDataEscaped => self.state_scriptdata_escaped(ch),
            LexerState::ScriptDataEscapedDash => self.state_scriptdata_escaped_dash(ch),
            LexerState::ScriptDataEscapedDashDash => self.state_scriptdata_escaped_dash_dash(ch),
            LexerState::ScriptDataEscapedLessThanSign => {
                self.state_scriptdata_escaped_less_than_sign(ch)
            }
            LexerState::ScriptDataEscapedEndTagOpen => {
                self.state_scriptdata_escaped_end_tag_open(ch)
            }
            LexerState::ScriptDataEscapedEndTagName => {
                self.state_scriptdata_escaped_end_tag_name(ch)
            }
            LexerState::ScriptDataDoubleEscapeStart => {
                self.state_scriptdata_double_escape_start(ch)
            }
            LexerState::ScriptDataDoubleEscaped => self.state_scriptdata_double_escaped(ch),
            LexerState::ScriptDataDoubleEscapedDash => {
                self.state_scriptdata_double_escaped_dash(ch)
            }
            LexerState::ScriptDataDoubleEscapedDashDash => {
                self.state_scriptdata_double_escaped_dash_dash(ch)
            }
            LexerState::ScriptDataDoubleEscapedLessThanSign => {
                self.state_scriptdata_double_escaped_less_than_sign(ch)
            }
            LexerState::ScriptDataDoubleEscapeEnd => self.state_scriptdata_double_escape_end(ch),

            LexerState::CharacterReference => self.state_character_reference(ch),
            LexerState::NamedCharacterReference => self.state_named_character_reference(ch),
            LexerState::AmbiguousAmpersand => self.state_ambiguous_ampersand(ch),
            LexerState::NumericCharacterReference => self.state_numeric_character_reference(ch),
            LexerState::HexadecimalCharacterReferenceStart => {
                self.state_hexadecimal_character_reference_start(ch)
            }
            LexerState::DecimalCharacterReferenceStart => {
                self.state_decimal_character_reference_start(ch)
            }
            LexerState::HexadecimalCharacterReference => {
                self.state_hexadecimal_character_reference(ch)
            }
            LexerState::DecimalCharacterReference => self.state_decimal_character_reference(ch),
            LexerState::NumericCharacterReferenceEnd => self.state_numeric_character_reference_end(ch),
            LexerState::CommentLessThanSign => self.state_comment_less_than_sign(ch),
            LexerState::CommentLessThanSignBang => self.state_comment_less_than_sign_bang(ch),
            LexerState::CommentLessThanSignBangDash => self.state_comment_less_than_sign_bang_dash(ch),
            LexerState::CommentLessThanSignBangDashDash => {
                self.state_comment_less_than_sign_bang_dash_dash(ch)
            }
            LexerState::CdataSection => self.state_cdata_section(ch),
            LexerState::CdataSectionBracket => self.state_cdata_section_bracket(ch),
            LexerState::CdataSectionEnd => self.state_cdata_section_end(ch),
        }
    }

    fn state_data(&mut self, ch: Option<char>) {
        match ch {
            Some('&') => {
                self.return_state = LexerState::Data;
                self.state = LexerState::CharacterReference;
            }
            Some('<') => {
                self.flush_text();
                self.state = LexerState::TagOpen;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in data state",
                );
                self.push_text_char('\u{FFFD}');
            }
            Some(c) => self.push_text_char(c),
            None => self.flush_text(),
        }
    }

    fn state_rcdata(&mut self, ch: Option<char>) {
        match ch {
            Some('&') => {
                self.return_state = LexerState::RcData;
                self.state = LexerState::CharacterReference;
            }
            Some('<') => {
                self.flush_text();
                self.state = LexerState::RcDataLessThanSign;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in RCDATA state",
                );
                self.push_text_char('\u{FFFD}');
            }
            Some(c) => self.push_text_char(c),
            None => self.flush_text(),
        }
    }

    fn state_rawtext(&mut self, ch: Option<char>) {
        match ch {
            Some('<') => {
                self.flush_text();
                self.state = LexerState::RawTextLessThanSign;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in RAWTEXT state",
                );
                self.push_text_char('\u{FFFD}');
            }
            Some(c) => self.push_text_char(c),
            None => self.flush_text(),
        }
    }

    fn state_scriptdata(&mut self, ch: Option<char>) {
        match ch {
            Some('<') => {
                self.flush_text();
                self.state = LexerState::ScriptDataLessThanSign;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in ScriptData state",
                );
                self.push_text_char('\u{FFFD}');
            }
            Some(c) => self.push_text_char(c),
            None => {
                self.parse_error(
                    LexerErrorKind::EofInScriptHtmlCommentLikeText,
                    "EOF in script data",
                );
                self.flush_text();
            }
        }
    }

    fn state_plaintext(&mut self, ch: Option<char>) {
        match ch {
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in PLAINTEXT state",
                );
                self.push_text_char('\u{FFFD}');
            }
            Some(c) => self.push_text_char(c),
            None => self.flush_text(),
        }
    }

    fn state_tag_open(&mut self, ch: Option<char>) {
        match ch {
            Some('!') => self.state = LexerState::MarkupDeclarationOpen,
            Some('/') => self.state = LexerState::EndTagOpen,
            Some(c) if is_ascii_alpha(c) => {
                self.begin_tag(false);
                self.reconsume_in(LexerState::TagName);
            }
            Some('?') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedQuestionMarkInsteadOfTagName,
                    "unexpected '?' after '<'",
                );
                self.current_comment.clear();
                // Spec §13.2.5.6: reconsumir '?' em BogusComment para incluí-lo no dado do comentário
                self.reconsume_in(LexerState::BogusComment);
            }
            Some(other) => {
                self.parse_error(
                    LexerErrorKind::InvalidFirstCharacterOfTagName,
                    format!("invalid first tag char '{}'", other),
                );
                self.push_text_char('<');
                self.reconsume_in(LexerState::Data);
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofBeforeTagName,
                    "EOF right after '<'",
                );
                self.push_text_char('<');
                self.state = LexerState::Data;
            }
        }
    }

    fn state_end_tag_open(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_ascii_alpha(c) => {
                self.begin_tag(true);
                self.reconsume_in(LexerState::TagName);
            }
            Some('>') => {
                self.parse_error(LexerErrorKind::MissingEndTagName, "missing end tag name");
                self.state = LexerState::Data;
            }
            Some(_) => {
                self.parse_error(
                    LexerErrorKind::InvalidFirstCharacterOfTagName,
                    "invalid character after '</'",
                );
                self.current_comment.clear();
                self.state = LexerState::BogusComment;
                self.reconsume();
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofBeforeTagName,
                    "EOF after '</'",
                );
                self.push_text_str("</");
                self.state = LexerState::Data;
            }
        }
    }

    fn state_tag_name(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {
                if self.current_tag.as_ref().map_or(false, |t| t.is_end_tag) {
                    self.parse_error(LexerErrorKind::EndTagWithAttributes, "end tag with attributes");
                }
                self.state = LexerState::BeforeAttributeName;
            }
            Some('/') => {
                if self.current_tag.as_ref().map_or(false, |t| t.is_end_tag) {
                    self.parse_error(LexerErrorKind::EndTagWithTrailingSolidus, "end tag with trailing solidus");
                }
                self.state = LexerState::SelfClosingStartTag;
            }
            Some('>') => self.emit_current_tag_and_switch_to_content_state(),
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in tag name",
                );
                self.with_current_tag_mut(|tag| tag.push_name('\u{FFFD}'));
            }
            Some(c) => self.with_current_tag_mut(|tag| tag.push_name(c)),
            None => {
                self.parse_error(LexerErrorKind::EofInTag, "EOF in tag name");
                self.state = LexerState::Data;
            }
        }
    }

    fn state_before_attribute_name(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {}
            Some('/') => self.state = LexerState::SelfClosingStartTag,
            Some('>') => self.emit_current_tag_and_switch_to_content_state(),
            Some('=') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedEqualsSignBeforeAttributeName,
                    "unexpected '=' before attribute name",
                );
                self.with_current_tag_mut(|tag| tag.begin_attribute(Some('=')));
                self.state = LexerState::AttributeName;
            }
            Some(c) => {
                self.with_current_tag_mut(|tag| tag.begin_attribute(Some(c)));
                self.state = LexerState::AttributeName;
            }
            None => {
                self.parse_error(LexerErrorKind::EofInTag, "EOF before attribute name");
                self.state = LexerState::Data;
            }
        }
    }

    fn state_attribute_name(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => self.state = LexerState::AfterAttributeName,
            Some('/') => {
                self.finish_attribute();
                self.state = LexerState::SelfClosingStartTag;
            }
            Some('=') => self.state = LexerState::BeforeAttributeValue,
            Some('>') => self.emit_current_tag_and_switch_to_content_state(),
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in attribute name",
                );
                self.with_current_tag_mut(|tag| tag.push_attribute_name('\u{FFFD}'));
            }
            Some(c @ ('"' | '\'' | '<')) => {
                self.parse_error(
                    LexerErrorKind::UnexpectedCharacterInAttributeName,
                    format!("unexpected '{}' in attribute name", c),
                );
                self.with_current_tag_mut(|tag| tag.push_attribute_name(c));
            }
            Some(c) => self.with_current_tag_mut(|tag| tag.push_attribute_name(c)),
            None => {
                self.parse_error(LexerErrorKind::EofInTag, "EOF in attribute name");
                self.state = LexerState::Data;
            }
        }
    }

    fn state_after_attribute_name(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {}
            Some('/') => self.state = LexerState::SelfClosingStartTag,
            Some('=') => self.state = LexerState::BeforeAttributeValue,
            Some('>') => self.emit_current_tag_and_switch_to_content_state(),
            Some(c) => {
                self.finish_attribute();
                self.with_current_tag_mut(|tag| {
                    tag.begin_attribute(Some(c));
                });
                self.state = LexerState::AttributeName;
            }
            None => {
                self.parse_error(LexerErrorKind::EofInTag, "EOF after attribute name");
                self.state = LexerState::Data;
            }
        }
    }

    fn state_before_attribute_value(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {}
            Some('"') => self.state = LexerState::AttributeValueDoubleQuoted,
            Some('\'') => self.state = LexerState::AttributeValueSingleQuoted,
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::MissingAttributeValue,
                    "missing attribute value before '>'",
                );
                self.emit_current_tag_and_switch_to_content_state();
            }
            Some(c) => {
                self.reconsume_in_with_char(LexerState::AttributeValueUnquoted, c);
            }
            None => {
                self.parse_error(LexerErrorKind::EofInTag, "EOF before attribute value");
                self.state = LexerState::Data;
            }
        }
    }

    fn state_attribute_value_double_quoted(&mut self, ch: Option<char>) {
        match ch {
            Some('"') => {
                self.finish_attribute();
                self.state = LexerState::AfterAttributeValueQuoted;
            }
            Some('&') => {
                self.return_state = LexerState::AttributeValueDoubleQuoted;
                self.state = LexerState::CharacterReference;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in double quoted attribute value",
                );
                self.with_current_tag_mut(|tag| tag.push_attribute_value_char('\u{FFFD}'));
            }
            Some(c) => self.with_current_tag_mut(|tag| tag.push_attribute_value_char(c)),
            None => {
                self.parse_error(
                    LexerErrorKind::EofInTag,
                    "EOF in double quoted attribute value",
                );
                self.state = LexerState::Data;
            }
        }
    }

    fn state_attribute_value_single_quoted(&mut self, ch: Option<char>) {
        match ch {
            Some('\'') => {
                self.finish_attribute();
                self.state = LexerState::AfterAttributeValueQuoted;
            }
            Some('&') => {
                self.return_state = LexerState::AttributeValueSingleQuoted;
                self.state = LexerState::CharacterReference;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in single quoted attribute value",
                );
                self.with_current_tag_mut(|tag| tag.push_attribute_value_char('\u{FFFD}'));
            }
            Some(c) => self.with_current_tag_mut(|tag| tag.push_attribute_value_char(c)),
            None => {
                self.parse_error(
                    LexerErrorKind::EofInTag,
                    "EOF in single quoted attribute value",
                );
                self.state = LexerState::Data;
            }
        }
    }

    fn state_attribute_value_unquoted(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {
                self.finish_attribute();
                self.state = LexerState::BeforeAttributeName;
            }
            Some('&') => {
                self.return_state = LexerState::AttributeValueUnquoted;
                self.state = LexerState::CharacterReference;
            }
            Some('>') => self.emit_current_tag_and_switch_to_content_state(),
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in unquoted attribute value",
                );
                self.with_current_tag_mut(|tag| tag.push_attribute_value_char('\u{FFFD}'));
            }
            Some(c @ ('"' | '\'' | '<' | '=' | '`')) => {
                self.parse_error(
                    LexerErrorKind::UnexpectedCharacterInUnquotedAttributeValue,
                    format!("unexpected '{}' in unquoted attribute value", c),
                );
                self.with_current_tag_mut(|tag| tag.push_attribute_value_char(c));
            }
            Some(c) => self.with_current_tag_mut(|tag| tag.push_attribute_value_char(c)),
            None => {
                self.parse_error(
                    LexerErrorKind::EofInTag,
                    "EOF in unquoted attribute value",
                );
                self.state = LexerState::Data;
            }
        }
    }

    fn state_after_attribute_value_quoted(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => self.state = LexerState::BeforeAttributeName,
            Some('/') => self.state = LexerState::SelfClosingStartTag,
            Some('>') => self.emit_current_tag_and_switch_to_content_state(),
            Some(c) => {
                self.parse_error(
                    LexerErrorKind::MissingWhitespaceBetweenAttributes,
                    format!("missing whitespace after quoted attribute before '{}'", c),
                );
                self.reconsume_in_with_char(LexerState::BeforeAttributeName, c);
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInTag,
                    "EOF after quoted attribute value",
                );
                self.state = LexerState::Data;
            }
        }
    }

    fn state_self_closing_start_tag(&mut self, ch: Option<char>) {
        match ch {
            Some('>') => {
                self.finish_attribute();
                self.with_current_tag_mut(|tag| {
                    tag.self_closing = true;
                });
                self.emit_current_tag_and_switch_to_content_state();
            }
            Some(c) => {
                self.parse_error(
                    LexerErrorKind::UnexpectedSolidusInTag,
                    format!("unexpected '{}' after '/' in tag", c),
                );
                self.reconsume_in_with_char(LexerState::BeforeAttributeName, c);
            }
            None => {
                self.parse_error(LexerErrorKind::EofInTag, "EOF in self closing start tag");
                self.state = LexerState::Data;
            }
        }
    }

    fn state_markup_declaration_open(&mut self, ch: Option<char>) {
        if ch.is_some() {
            self.reconsume();
        }

        if self.starts_with("--") {
            self.pos += 2;
            self.current_comment.clear();
            self.state = LexerState::CommentStart;
            return;
        }

        if self.starts_with_case_insensitive("doctype") {
            self.pos += "doctype".chars().count();
            self.state = LexerState::Doctype;
            return;
        }

        if self.starts_with("[CDATA[") {
            if !self.cdata_section_allowed {
                self.parse_error(
                    LexerErrorKind::CdataSectionOutsideForeignContent,
                    "CDATA section outside foreign content",
                );
            }
            self.pos += "[CDATA[".chars().count();
            self.state = LexerState::CdataSection;
            return;
        }

        self.parse_error(
            LexerErrorKind::IncorrectlyOpenedComment,
            "incorrectly opened comment; entering bogus comment state",
        );
        self.current_comment.clear();
        self.state = LexerState::BogusComment;
    }

    fn state_comment_start(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => self.state = LexerState::CommentStartDash,
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::AbruptClosingOfEmptyComment,
                    "abrupt closing of empty comment",
                );
                self.emit_comment();
                self.state = LexerState::Data;
            }
            Some(c) => {
                self.reconsume_in_with_char(LexerState::Comment, c);
            }
            None => {
                self.parse_error(LexerErrorKind::EofInComment, "EOF in comment start");
                self.emit_comment();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_comment_start_dash(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => self.state = LexerState::CommentEnd,
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::AbruptClosingOfEmptyComment,
                    "abrupt closing of empty comment",
                );
                self.emit_comment();
                self.state = LexerState::Data;
            }
            Some(c) => {
                self.current_comment.push('-');
                self.reconsume_in_with_char(LexerState::Comment, c);
            }
            None => {
                self.parse_error(LexerErrorKind::EofInComment, "EOF in comment start dash");
                self.emit_comment();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_comment(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => self.state = LexerState::CommentEndDash,
            Some('<') => {
                self.current_comment.push('<');
                self.state = LexerState::CommentLessThanSign;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in comment",
                );
                self.current_comment.push('\u{FFFD}');
            }
            Some(c) => self.current_comment.push(c),
            None => {
                self.parse_error(LexerErrorKind::EofInComment, "EOF in comment");
                self.emit_comment();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_comment_less_than_sign(&mut self, ch: Option<char>) {
        match ch {
            Some('!') => {
                self.current_comment.push('!');
                self.state = LexerState::CommentLessThanSignBang;
            }
            Some('<') => {
                self.current_comment.push('<');
            }
            Some(c) => {
                self.reconsume_in_with_char(LexerState::Comment, c);
            }
            None => {
                self.reconsume_in(LexerState::Comment);
            }
        }
    }

    fn state_comment_less_than_sign_bang(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => {
                self.state = LexerState::CommentLessThanSignBangDash;
            }
            Some(c) => {
                self.reconsume_in_with_char(LexerState::Comment, c);
            }
            None => {
                self.reconsume_in(LexerState::Comment);
            }
        }
    }

    fn state_comment_less_than_sign_bang_dash(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => {
                self.state = LexerState::CommentLessThanSignBangDashDash;
            }
            Some(c) => {
                self.reconsume_in_with_char(LexerState::CommentStartDash, c);
            }
            None => {
                self.reconsume_in(LexerState::CommentStartDash);
            }
        }
    }

    fn state_comment_less_than_sign_bang_dash_dash(&mut self, ch: Option<char>) {
        match ch {
            Some('>') | None => {
                self.reconsume_in(LexerState::CommentEnd);
            }
            Some(c) => {
                self.parse_error(LexerErrorKind::NestedComment, "nested comment start pattern");
                self.reconsume_in_with_char(LexerState::CommentEnd, c);
            }
        }
    }

    fn state_cdata_section(&mut self, ch: Option<char>) {
        match ch {
            Some(']') => {
                self.state = LexerState::CdataSectionBracket;
            }
            Some('\0') => {
                self.parse_error(LexerErrorKind::UnexpectedNullCharacter, "null in CDATA section");
                self.push_text_char('\u{FFFD}');
            }
            Some(c) => {
                self.push_text_char(c);
            }
            None => {
                self.parse_error(LexerErrorKind::EofInCdata, "EOF in CDATA section");
                self.state = LexerState::Data;
            }
        }
    }

    fn state_cdata_section_bracket(&mut self, ch: Option<char>) {
        match ch {
            Some(']') => {
                self.state = LexerState::CdataSectionEnd;
            }
            Some(c) => {
                self.push_text_char(']');
                self.reconsume_in_with_char(LexerState::CdataSection, c);
            }
            None => {
                self.push_text_char(']');
                self.state = LexerState::CdataSection;
            }
        }
    }

    fn state_cdata_section_end(&mut self, ch: Option<char>) {
        match ch {
            Some('>') => {
                self.state = LexerState::Data;
            }
            Some(']') => {
                self.push_text_char(']');
            }
            Some(c) => {
                self.push_text_str("]]");
                self.reconsume_in_with_char(LexerState::CdataSection, c);
            }
            None => {
                self.push_text_str("]]");
                self.state = LexerState::CdataSection;
            }
        }
    }

    fn state_comment_end_dash(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => self.state = LexerState::CommentEnd,
            Some(c) => {
                self.current_comment.push('-');
                self.reconsume_in_with_char(LexerState::Comment, c);
            }
            None => {
                self.parse_error(LexerErrorKind::EofInComment, "EOF in comment end dash");
                self.emit_comment();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_comment_end(&mut self, ch: Option<char>) {
        match ch {
            Some('>') => {
                self.emit_comment();
                self.state = LexerState::Data;
            }
            Some('!') => self.state = LexerState::CommentEndBang,
            Some('-') => self.current_comment.push('-'),
            Some(c) => {
                self.current_comment.push_str("--");
                self.reconsume_in_with_char(LexerState::Comment, c);
            }
            None => {
                self.parse_error(LexerErrorKind::EofInComment, "EOF in comment end");
                self.emit_comment();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_comment_end_bang(&mut self, ch: Option<char>) {
        match ch {
            Some('>') => {
                self.emit_comment();
                self.state = LexerState::Data;
            }
            Some('-') => {
                self.current_comment.push_str("--!");
                self.state = LexerState::CommentEndDash;
            }
            Some(c) => {
                self.current_comment.push_str("--!");
                self.reconsume_in_with_char(LexerState::Comment, c);
            }
            None => {
                self.parse_error(LexerErrorKind::EofInComment, "EOF in comment end bang");
                self.emit_comment();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_bogus_comment(&mut self, ch: Option<char>) {
        match ch {
            Some('>') => {
                self.emit_comment();
                self.state = LexerState::Data;
            }
            Some('\0') => self.current_comment.push('\u{FFFD}'),
            Some(c) => self.current_comment.push(c),
            None => {
                self.parse_error(LexerErrorKind::EofInComment, "EOF in bogus comment");
                self.emit_comment();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_doctype(&mut self, ch: Option<char>) {
        self.current_doctype = Some(DoctypeBuilder::default());
        match ch {
            Some(c) if is_whitespace(c) => self.state = LexerState::BeforeDoctypeName,
            Some(c) => {
                self.parse_error(LexerErrorKind::MissingWhitespaceBeforeDoctypeName, "missing whitespace before DOCTYPE name");
                self.reconsume_in_with_char(LexerState::BeforeDoctypeName, c);
            }
            None => {
                self.parse_error(LexerErrorKind::EofInDoctype, "EOF in DOCTYPE");
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_before_doctype_name(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {}
            Some('>') => {
                self.parse_error(LexerErrorKind::MissingDoctypeName, "missing doctype name");
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null in doctype name",
                );
                self.with_current_doctype_mut(|dt| dt.name = Some("\u{FFFD}".to_string()));
                self.state = LexerState::DoctypeName;
            }
            Some(c) => {
                self.with_current_doctype_mut(|dt| dt.name = Some(c.to_ascii_lowercase().to_string()));
                self.state = LexerState::DoctypeName;
            }
            None => {
                self.parse_error(LexerErrorKind::EofInDoctype, "EOF before doctype name");
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_doctype_name(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => self.state = LexerState::AfterDoctypeName,
            Some('>') => {
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null in doctype name",
                );
                self.with_current_doctype_mut(|dt| {
                    let name = dt.name.get_or_insert_with(String::new);
                    name.push('\u{FFFD}');
                });
            }
            Some(c) => {
                self.with_current_doctype_mut(|dt| {
                    let name = dt.name.get_or_insert_with(String::new);
                    name.push(c.to_ascii_lowercase());
                });
            }
            None => {
                self.parse_error(LexerErrorKind::EofInDoctype, "EOF in doctype name");
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_after_doctype_name(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {}
            Some('>') => {
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some('p' | 'P') if self.starts_with_case_insensitive("UBLIC") => {
                self.pos += "UBLIC".chars().count();
                self.state = LexerState::AfterDoctypePublicKeyword;
            }
            Some('s' | 'S') if self.starts_with_case_insensitive("YSTEM") => {
                self.pos += "YSTEM".chars().count();
                self.state = LexerState::AfterDoctypeSystemKeyword;
            }
            Some(_) => {
                self.parse_error(
                    LexerErrorKind::InvalidCharacterSequenceAfterDoctypeName,
                    "invalid characters after doctype name",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.state = LexerState::BogusDoctype;
            }
            None => {
                self.parse_error(LexerErrorKind::EofInDoctype, "EOF after doctype name");
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_after_doctype_public_keyword(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => self.state = LexerState::BeforeDoctypePublicIdentifier,
            Some('"') => {
                self.parse_error(
                    LexerErrorKind::MissingWhitespaceAfterDoctypePublicKeyword,
                    "missing whitespace after DOCTYPE PUBLIC keyword",
                );
                self.with_current_doctype_mut(|dt| dt.public_id = Some(String::new()));
                self.state = LexerState::DoctypePublicIdentifierDoubleQuoted;
            }
            Some('\'') => {
                self.parse_error(
                    LexerErrorKind::MissingWhitespaceAfterDoctypePublicKeyword,
                    "missing whitespace after DOCTYPE PUBLIC keyword",
                );
                self.with_current_doctype_mut(|dt| dt.public_id = Some(String::new()));
                self.state = LexerState::DoctypePublicIdentifierSingleQuoted;
            }
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::MissingDoctypePublicIdentifier,
                    "missing DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some(_) => {
                self.parse_error(
                    LexerErrorKind::MissingQuoteBeforeDoctypePublicIdentifier,
                    "missing quote before DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.state = LexerState::BogusDoctype;
            }
            None => {
                self.parse_error(LexerErrorKind::EofInDoctype, "EOF after DOCTYPE PUBLIC keyword");
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_before_doctype_public_identifier(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {}
            Some('"') => {
                self.with_current_doctype_mut(|dt| dt.public_id = Some(String::new()));
                self.state = LexerState::DoctypePublicIdentifierDoubleQuoted;
            }
            Some('\'') => {
                self.with_current_doctype_mut(|dt| dt.public_id = Some(String::new()));
                self.state = LexerState::DoctypePublicIdentifierSingleQuoted;
            }
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::MissingDoctypePublicIdentifier,
                    "missing DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some(_) => {
                self.parse_error(
                    LexerErrorKind::MissingQuoteBeforeDoctypePublicIdentifier,
                    "missing quote before DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.state = LexerState::BogusDoctype;
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInDoctype,
                    "EOF before DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_doctype_public_identifier_double_quoted(&mut self, ch: Option<char>) {
        match ch {
            Some('"') => self.state = LexerState::AfterDoctypePublicIdentifier,
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null in DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| {
                    dt.public_id
                        .get_or_insert_with(String::new)
                        .push('\u{FFFD}');
                });
            }
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::AbruptDoctypePublicIdentifier,
                    "abrupt end of DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some(c) => {
                self.with_current_doctype_mut(|dt| {
                    dt.public_id.get_or_insert_with(String::new).push(c);
                });
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInDoctype,
                    "EOF in DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_doctype_public_identifier_single_quoted(&mut self, ch: Option<char>) {
        match ch {
            Some('\'') => self.state = LexerState::AfterDoctypePublicIdentifier,
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null in DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| {
                    dt.public_id
                        .get_or_insert_with(String::new)
                        .push('\u{FFFD}');
                });
            }
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::AbruptDoctypePublicIdentifier,
                    "abrupt end of DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some(c) => {
                self.with_current_doctype_mut(|dt| {
                    dt.public_id.get_or_insert_with(String::new).push(c);
                });
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInDoctype,
                    "EOF in DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_after_doctype_public_identifier(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {
                self.state = LexerState::BetweenDoctypePublicAndSystemIdentifiers;
            }
            Some('>') => {
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some('"') => {
                self.parse_error(
                    LexerErrorKind::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
                    "missing whitespace between DOCTYPE public and system identifiers",
                );
                self.with_current_doctype_mut(|dt| dt.system_id = Some(String::new()));
                self.state = LexerState::DoctypeSystemIdentifierDoubleQuoted;
            }
            Some('\'') => {
                self.parse_error(
                    LexerErrorKind::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
                    "missing whitespace between DOCTYPE public and system identifiers",
                );
                self.with_current_doctype_mut(|dt| dt.system_id = Some(String::new()));
                self.state = LexerState::DoctypeSystemIdentifierSingleQuoted;
            }
            Some(_) => {
                self.parse_error(
                    LexerErrorKind::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
                    "invalid character between DOCTYPE public and system identifiers",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.state = LexerState::BogusDoctype;
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInDoctype,
                    "EOF after DOCTYPE public identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_between_doctype_public_and_system_identifiers(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {}
            Some('>') => {
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some('"') => {
                self.with_current_doctype_mut(|dt| dt.system_id = Some(String::new()));
                self.state = LexerState::DoctypeSystemIdentifierDoubleQuoted;
            }
            Some('\'') => {
                self.with_current_doctype_mut(|dt| dt.system_id = Some(String::new()));
                self.state = LexerState::DoctypeSystemIdentifierSingleQuoted;
            }
            Some(_) => {
                self.parse_error(
                    LexerErrorKind::MissingQuoteBeforeDoctypeSystemIdentifier,
                    "missing quote before DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.state = LexerState::BogusDoctype;
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInDoctype,
                    "EOF between DOCTYPE public and system identifiers",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_after_doctype_system_keyword(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => self.state = LexerState::BeforeDoctypeSystemIdentifier,
            Some('"') => {
                self.parse_error(
                    LexerErrorKind::MissingWhitespaceAfterDoctypeSystemKeyword,
                    "missing whitespace after DOCTYPE SYSTEM keyword",
                );
                self.with_current_doctype_mut(|dt| dt.system_id = Some(String::new()));
                self.state = LexerState::DoctypeSystemIdentifierDoubleQuoted;
            }
            Some('\'') => {
                self.parse_error(
                    LexerErrorKind::MissingWhitespaceAfterDoctypeSystemKeyword,
                    "missing whitespace after DOCTYPE SYSTEM keyword",
                );
                self.with_current_doctype_mut(|dt| dt.system_id = Some(String::new()));
                self.state = LexerState::DoctypeSystemIdentifierSingleQuoted;
            }
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::MissingDoctypeSystemIdentifier,
                    "missing DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some(_) => {
                self.parse_error(
                    LexerErrorKind::MissingQuoteBeforeDoctypeSystemIdentifier,
                    "missing quote before DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.state = LexerState::BogusDoctype;
            }
            None => {
                self.parse_error(LexerErrorKind::EofInDoctype, "EOF after DOCTYPE SYSTEM keyword");
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_before_doctype_system_identifier(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {}
            Some('"') => {
                self.with_current_doctype_mut(|dt| dt.system_id = Some(String::new()));
                self.state = LexerState::DoctypeSystemIdentifierDoubleQuoted;
            }
            Some('\'') => {
                self.with_current_doctype_mut(|dt| dt.system_id = Some(String::new()));
                self.state = LexerState::DoctypeSystemIdentifierSingleQuoted;
            }
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::MissingDoctypeSystemIdentifier,
                    "missing DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some(_) => {
                self.parse_error(
                    LexerErrorKind::MissingQuoteBeforeDoctypeSystemIdentifier,
                    "missing quote before DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.state = LexerState::BogusDoctype;
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInDoctype,
                    "EOF before DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_doctype_system_identifier_double_quoted(&mut self, ch: Option<char>) {
        match ch {
            Some('"') => self.state = LexerState::AfterDoctypeSystemIdentifier,
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null in DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| {
                    dt.system_id
                        .get_or_insert_with(String::new)
                        .push('\u{FFFD}');
                });
            }
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::AbruptDoctypeSystemIdentifier,
                    "abrupt end of DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some(c) => {
                self.with_current_doctype_mut(|dt| {
                    dt.system_id.get_or_insert_with(String::new).push(c);
                });
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInDoctype,
                    "EOF in DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_doctype_system_identifier_single_quoted(&mut self, ch: Option<char>) {
        match ch {
            Some('\'') => self.state = LexerState::AfterDoctypeSystemIdentifier,
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null in DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| {
                    dt.system_id
                        .get_or_insert_with(String::new)
                        .push('\u{FFFD}');
                });
            }
            Some('>') => {
                self.parse_error(
                    LexerErrorKind::AbruptDoctypeSystemIdentifier,
                    "abrupt end of DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some(c) => {
                self.with_current_doctype_mut(|dt| {
                    dt.system_id.get_or_insert_with(String::new).push(c);
                });
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInDoctype,
                    "EOF in DOCTYPE system identifier",
                );
                self.with_current_doctype_mut(|dt| dt.force_quirks = true);
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_after_doctype_system_identifier(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) => {}
            Some('>') => {
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some(_) => {
                self.parse_error(
                    LexerErrorKind::InvalidCharacterSequenceAfterDoctypeName,
                    "invalid characters after DOCTYPE system identifier",
                );
                self.state = LexerState::BogusDoctype;
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInDoctype,
                    "EOF after DOCTYPE system identifier",
                );
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_bogus_doctype(&mut self, ch: Option<char>) {
        match ch {
            Some('>') => {
                self.emit_doctype();
                self.state = LexerState::Data;
            }
            Some(_) => {}
            None => {
                self.parse_error(LexerErrorKind::EofInDoctype, "EOF in bogus doctype");
                self.emit_doctype();
                self.state = LexerState::Data;
            }
        }
    }

    fn state_rcdata_less_than_sign(&mut self, ch: Option<char>) {
        match ch {
            Some('/') => {
                self.temporary_buffer.clear();
                self.begin_tag(true);
                self.state = LexerState::RcDataEndTagOpen;
            }
            Some(c) => {
                self.push_text_char('<');
                self.reconsume_in_with_char(LexerState::RcData, c);
            }
            None => {
                self.push_text_char('<');
                self.state = LexerState::RcData;
            }
        }
    }

    fn state_rcdata_end_tag_open(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_ascii_alpha(c) => {
                self.with_current_tag_mut(|tag| tag.push_name(c));
                self.temporary_buffer.push(c.to_ascii_lowercase());
                self.state = LexerState::RcDataEndTagName;
            }
            Some(c) => {
                self.current_tag = None;
                self.push_text_str("</");
                self.reconsume_in_with_char(LexerState::RcData, c);
            }
            None => {
                self.current_tag = None;
                self.push_text_str("</");
                self.state = LexerState::RcData;
            }
        }
    }

    fn state_rcdata_end_tag_name(&mut self, ch: Option<char>) {
        self.state_raw_end_tag_name_common(ch, LexerState::RcDataEndTagName, LexerState::RcData);
    }

    fn state_rawtext_less_than_sign(&mut self, ch: Option<char>) {
        match ch {
            Some('/') => {
                self.temporary_buffer.clear();
                self.begin_tag(true);
                self.state = LexerState::RawTextEndTagOpen;
            }
            Some(c) => {
                self.push_text_char('<');
                self.reconsume_in_with_char(LexerState::RawText, c);
            }
            None => {
                self.push_text_char('<');
                self.state = LexerState::RawText;
            }
        }
    }

    fn state_rawtext_end_tag_open(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_ascii_alpha(c) => {
                self.with_current_tag_mut(|tag| tag.push_name(c));
                self.temporary_buffer.push(c.to_ascii_lowercase());
                self.state = LexerState::RawTextEndTagName;
            }
            Some(c) => {
                self.current_tag = None;
                self.push_text_str("</");
                self.reconsume_in_with_char(LexerState::RawText, c);
            }
            None => {
                self.current_tag = None;
                self.push_text_str("</");
                self.state = LexerState::RawText;
            }
        }
    }

    fn state_rawtext_end_tag_name(&mut self, ch: Option<char>) {
        self.state_raw_end_tag_name_common(ch, LexerState::RawTextEndTagName, LexerState::RawText);
    }

    fn state_scriptdata_less_than_sign(&mut self, ch: Option<char>) {
        match ch {
            Some('/') => {
                self.temporary_buffer.clear();
                self.begin_tag(true);
                self.state = LexerState::ScriptDataEndTagOpen;
            }
            Some('!') => {
                self.push_text_str("<!");
                self.state = LexerState::ScriptDataEscapeStart;
            }
            Some(c) => {
                self.push_text_char('<');
                self.reconsume_in_with_char(LexerState::ScriptData, c);
            }
            None => {
                self.push_text_char('<');
                self.state = LexerState::ScriptData;
            }
        }
    }

    fn state_scriptdata_end_tag_open(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_ascii_alpha(c) => {
                self.with_current_tag_mut(|tag| tag.push_name(c));
                self.temporary_buffer.push(c.to_ascii_lowercase());
                self.state = LexerState::ScriptDataEndTagName;
            }
            Some(c) => {
                self.current_tag = None;
                self.push_text_str("</");
                self.reconsume_in_with_char(LexerState::ScriptData, c);
            }
            None => {
                self.current_tag = None;
                self.push_text_str("</");
                self.state = LexerState::ScriptData;
            }
        }
    }

    fn state_scriptdata_end_tag_name(&mut self, ch: Option<char>) {
        self.state_raw_end_tag_name_common(
            ch,
            LexerState::ScriptDataEndTagName,
            LexerState::ScriptData,
        );
    }

    fn state_scriptdata_escape_start(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => {
                self.push_text_char('-');
                self.state = LexerState::ScriptDataEscapeStartDash;
            }
            Some(c) => self.reconsume_in_with_char(LexerState::ScriptData, c),
            None => self.state = LexerState::ScriptData,
        }
    }

    fn state_scriptdata_escape_start_dash(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => {
                self.push_text_char('-');
                self.state = LexerState::ScriptDataEscapedDashDash;
            }
            Some(c) => self.reconsume_in_with_char(LexerState::ScriptData, c),
            None => self.state = LexerState::ScriptData,
        }
    }

    fn state_scriptdata_escaped(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => {
                self.push_text_char('-');
                self.state = LexerState::ScriptDataEscapedDash;
            }
            Some('<') => self.state = LexerState::ScriptDataEscapedLessThanSign,
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in ScriptData escaped state",
                );
                self.push_text_char('\u{FFFD}');
            }
            Some(c) => self.push_text_char(c),
            None => {
                self.parse_error(
                    LexerErrorKind::EofInScriptHtmlCommentLikeText,
                    "EOF in script data escaped state",
                );
                self.state = LexerState::Data;
            }
        }
    }

    fn state_scriptdata_escaped_dash(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => {
                self.push_text_char('-');
                self.state = LexerState::ScriptDataEscapedDashDash;
            }
            Some('<') => self.state = LexerState::ScriptDataEscapedLessThanSign,
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in ScriptData escaped dash state",
                );
                self.push_text_char('\u{FFFD}');
                self.state = LexerState::ScriptDataEscaped;
            }
            Some(c) => {
                self.push_text_char(c);
                self.state = LexerState::ScriptDataEscaped;
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInScriptHtmlCommentLikeText,
                    "EOF in script data escaped dash state",
                );
                self.state = LexerState::Data;
            }
        }
    }

    fn state_scriptdata_escaped_dash_dash(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => self.push_text_char('-'),
            Some('<') => self.state = LexerState::ScriptDataEscapedLessThanSign,
            Some('>') => {
                self.push_text_char('>');
                self.state = LexerState::ScriptData;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in ScriptData escaped dash dash state",
                );
                self.push_text_char('\u{FFFD}');
                self.state = LexerState::ScriptDataEscaped;
            }
            Some(c) => {
                self.push_text_char(c);
                self.state = LexerState::ScriptDataEscaped;
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInScriptHtmlCommentLikeText,
                    "EOF in script data escaped dash dash state",
                );
                self.state = LexerState::Data;
            }
        }
    }

    fn state_scriptdata_escaped_less_than_sign(&mut self, ch: Option<char>) {
        match ch {
            Some('/') => {
                self.temporary_buffer.clear();
                self.push_text_str("</");
                self.begin_tag(true);
                self.state = LexerState::ScriptDataEscapedEndTagOpen;
            }
            Some(c) if is_ascii_alpha(c) => {
                self.temporary_buffer.clear();
                self.temporary_buffer.push(c.to_ascii_lowercase());
                self.push_text_char('<');
                self.push_text_char(c);
                self.state = LexerState::ScriptDataDoubleEscapeStart;
            }
            Some(c) => {
                self.push_text_char('<');
                self.reconsume_in_with_char(LexerState::ScriptDataEscaped, c);
            }
            None => {
                self.push_text_char('<');
                self.state = LexerState::ScriptDataEscaped;
            }
        }
    }

    fn state_scriptdata_escaped_end_tag_open(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_ascii_alpha(c) => {
                self.with_current_tag_mut(|tag| tag.push_name(c));
                self.temporary_buffer.push(c.to_ascii_lowercase());
                self.state = LexerState::ScriptDataEscapedEndTagName;
            }
            Some(c) => {
                self.current_tag = None;
                self.reconsume_in_with_char(LexerState::ScriptDataEscaped, c);
            }
            None => {
                self.current_tag = None;
                self.state = LexerState::ScriptDataEscaped;
            }
        }
    }

    fn state_scriptdata_escaped_end_tag_name(&mut self, ch: Option<char>) {
        self.state_raw_end_tag_name_common(
            ch,
            LexerState::ScriptDataEscapedEndTagName,
            LexerState::ScriptDataEscaped,
        );
    }

    fn state_scriptdata_double_escape_start(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) || matches!(c, '/' | '>') => {
                if self.temporary_buffer == "script" {
                    self.state = LexerState::ScriptDataDoubleEscaped;
                } else {
                    self.state = LexerState::ScriptDataEscaped;
                }
                self.reconsume_in_with_char(self.state, c);
            }
            Some(c) if is_ascii_alpha(c) => {
                self.temporary_buffer.push(c.to_ascii_lowercase());
                self.push_text_char(c);
            }
            Some(c) => {
                self.state = LexerState::ScriptDataEscaped;
                self.reconsume_in_with_char(LexerState::ScriptDataEscaped, c);
            }
            None => self.state = LexerState::Data,
        }
    }

    fn state_scriptdata_double_escaped(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => {
                self.push_text_char('-');
                self.state = LexerState::ScriptDataDoubleEscapedDash;
            }
            Some('<') => {
                self.push_text_char('<');
                self.state = LexerState::ScriptDataDoubleEscapedLessThanSign;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in ScriptData double escaped state",
                );
                self.push_text_char('\u{FFFD}');
            }
            Some(c) => self.push_text_char(c),
            None => {
                self.parse_error(
                    LexerErrorKind::EofInScriptHtmlCommentLikeText,
                    "EOF in script data double escaped state",
                );
                self.state = LexerState::Data;
            }
        }
    }

    fn state_scriptdata_double_escaped_dash(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => {
                self.push_text_char('-');
                self.state = LexerState::ScriptDataDoubleEscapedDashDash;
            }
            Some('<') => {
                self.push_text_char('<');
                self.state = LexerState::ScriptDataDoubleEscapedLessThanSign;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in ScriptData double escaped dash state",
                );
                self.push_text_char('\u{FFFD}');
                self.state = LexerState::ScriptDataDoubleEscaped;
            }
            Some(c) => {
                self.push_text_char(c);
                self.state = LexerState::ScriptDataDoubleEscaped;
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInScriptHtmlCommentLikeText,
                    "EOF in script data double escaped dash state",
                );
                self.state = LexerState::Data;
            }
        }
    }

    fn state_scriptdata_double_escaped_dash_dash(&mut self, ch: Option<char>) {
        match ch {
            Some('-') => self.push_text_char('-'),
            Some('<') => {
                self.push_text_char('<');
                self.state = LexerState::ScriptDataDoubleEscapedLessThanSign;
            }
            Some('>') => {
                self.push_text_char('>');
                self.state = LexerState::ScriptData;
            }
            Some('\0') => {
                self.parse_error(
                    LexerErrorKind::UnexpectedNullCharacter,
                    "null character in ScriptData double escaped dash dash state",
                );
                self.push_text_char('\u{FFFD}');
                self.state = LexerState::ScriptDataDoubleEscaped;
            }
            Some(c) => {
                self.push_text_char(c);
                self.state = LexerState::ScriptDataDoubleEscaped;
            }
            None => {
                self.parse_error(
                    LexerErrorKind::EofInScriptHtmlCommentLikeText,
                    "EOF in script data double escaped dash dash state",
                );
                self.state = LexerState::Data;
            }
        }
    }

    fn state_scriptdata_double_escaped_less_than_sign(&mut self, ch: Option<char>) {
        match ch {
            Some('/') => {
                self.temporary_buffer.clear();
                self.push_text_char('/');
                self.state = LexerState::ScriptDataDoubleEscapeEnd;
            }
            Some(c) => self.reconsume_in_with_char(LexerState::ScriptDataDoubleEscaped, c),
            None => self.state = LexerState::Data,
        }
    }

    fn state_scriptdata_double_escape_end(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if is_whitespace(c) || matches!(c, '/' | '>') => {
                if self.temporary_buffer == "script" {
                    self.state = LexerState::ScriptDataEscaped;
                } else {
                    self.state = LexerState::ScriptDataDoubleEscaped;
                }
                self.reconsume_in_with_char(self.state, c);
            }
            Some(c) if is_ascii_alpha(c) => {
                self.temporary_buffer.push(c.to_ascii_lowercase());
                self.push_text_char(c);
            }
            Some(c) => self.reconsume_in_with_char(LexerState::ScriptDataDoubleEscaped, c),
            None => self.state = LexerState::Data,
        }
    }

    fn state_raw_end_tag_name_common(
        &mut self,
        ch: Option<char>,
        _continue_state: LexerState,
        fallback_state: LexerState,
    ) {
        match ch {
            Some(c) if is_ascii_alpha(c) => {
                self.with_current_tag_mut(|tag| tag.push_name(c));
                self.temporary_buffer.push(c.to_ascii_lowercase());
            }
            Some(c) if is_whitespace(c) => {
                if self.is_appropriate_end_tag_token() {
                    self.state = LexerState::BeforeAttributeName;
                } else {
                    self.fallback_from_raw_end_tag(c, fallback_state);
                }
            }
            Some('/') => {
                if self.is_appropriate_end_tag_token() {
                    self.state = LexerState::SelfClosingStartTag;
                } else {
                    self.fallback_from_raw_end_tag('/', fallback_state);
                }
            }
            Some('>') => {
                if self.is_appropriate_end_tag_token() {
                    self.emit_current_tag_and_switch_to_content_state();
                } else {
                    self.fallback_from_raw_end_tag('>', fallback_state);
                }
            }
            Some(c) => self.fallback_from_raw_end_tag(c, fallback_state),
            None => {
                self.parse_error(LexerErrorKind::EofInTag, "EOF in end tag name");
                self.state = LexerState::Data;
            }
        }
    }

    // --- CHARACTER REFERENCE STATES ---

    fn state_character_reference(&mut self, ch: Option<char>) {
        self.temporary_buffer.clear();
        self.temporary_buffer.push('&');
        match ch {
            Some(c) if is_ascii_alnum(c) => {
                self.reconsume_in_with_char(LexerState::NamedCharacterReference, c);
            }
            Some('#') => {
                self.temporary_buffer.push('#');
                self.state = LexerState::NumericCharacterReference;
            }
            Some(c) => {
                if is_whitespace(c) && self.peek_char().is_some_and(is_ascii_alpha) {
                    self.parse_error(
                        LexerErrorKind::MissingSemicolonAfterCharacterReference,
                        "missing semicolon after character reference-like ampersand",
                    );
                }
                self.emit_character('&');
                self.reconsume_in_with_char(self.return_state, c);
            }
            None => {
                self.emit_character('&');
                self.reconsume_in(self.return_state);
            }
        }
    }

    fn state_named_character_reference(&mut self, ch: Option<char>) {
        if ch.is_some() {
            self.reconsume();
            
            self.temporary_buffer.clear();
            self.temporary_buffer.push('&');
            
            let mut max_match_len = 0;
            let mut max_match_str: Option<&'static str> = None;
            
            let pos_backup = self.pos;
            let col_backup = self.column;
            
            let mut consumed_chars = 0;
            
            while let Some(next_c) = self.consume_next_input_character() {
                self.temporary_buffer.push(next_c);
                consumed_chars += 1;
                
                if let Some(decoded) = self.lookup_exact_entity(&self.temporary_buffer) {
                    max_match_len = consumed_chars;
                    max_match_str = Some(decoded);
                }
                
                if !self.has_entity_starting_with(&self.temporary_buffer) {
                    break;
                }
            }
            
            if max_match_len > 0 {
                let over_consumed = consumed_chars - max_match_len;
                self.pos -= over_consumed;
                self.column -= over_consumed;
                
                let is_attribute = matches!(
                    self.return_state,
                    LexerState::AttributeValueDoubleQuoted
                        | LexerState::AttributeValueSingleQuoted
                        | LexerState::AttributeValueUnquoted
                );
                
                let last_char = self.temporary_buffer.chars().nth(max_match_len).unwrap();
                let next_char = self.peek_char().unwrap_or('\0');
                
                if is_attribute && last_char != ';' && (next_char == '=' || is_ascii_alnum(next_char)) {
                    self.pos = pos_backup;
                    self.column = col_backup;
                    self.emit_character('&');
                    
                    if let Some(next) = self.peek_char() {
                        if is_ascii_alnum(next) {
                            let c1 = self.consume_next_input_character().unwrap();
                            if self.peek_char() == Some(';') {
                                self.parse_error(LexerErrorKind::UnknownNamedCharacterReference, "ambiguous ampersand before semicolon");
                            }
                            self.reconsume_in_with_char(self.return_state, c1);
                            return;
                        }
                    }
                    self.state = self.return_state;
                    return;
                }
                
                if last_char != ';' {
                    self.parse_error(LexerErrorKind::MissingSemicolonAfterCharacterReference, "missing semicolon");
                }
                
                let decoded = max_match_str.unwrap();
                for dec_char in decoded.chars() {
                    self.emit_character(dec_char);
                }
                self.state = self.return_state;
                
            } else {
                self.pos = pos_backup;
                self.column = col_backup;
                
                self.emit_character('&');
                
                if let Some(next) = self.peek_char() {
                    if is_ascii_alnum(next) {
                        let c1 = self.consume_next_input_character().unwrap();
                        if self.peek_char() == Some(';') {
                            self.parse_error(LexerErrorKind::UnknownNamedCharacterReference, "ambiguous ampersand before semicolon");
                        }
                        self.reconsume_in_with_char(self.return_state, c1);
                        return;
                    }
                }
                self.state = self.return_state;
            }
        } else {
            self.emit_character('&');
            self.state = self.return_state;
        }
    }

    fn state_ambiguous_ampersand(&mut self, ch: Option<char>) {
        // WHATWG §13.2.5.72: Ambiguous ampersand state
        match ch {
            Some(c) if is_ascii_alnum(c) => {
                // If consumed as part of an attribute, append to attribute value
                if matches!(
                    self.return_state,
                    LexerState::AttributeValueDoubleQuoted
                        | LexerState::AttributeValueSingleQuoted
                        | LexerState::AttributeValueUnquoted
                ) {
                    self.with_current_tag_mut(|tag| tag.push_attribute_value_char(c));
                } else {
                    self.push_text_char(c);
                }
            }
            Some(';') => {
                self.parse_error(
                    LexerErrorKind::AmbiguousAmpersand,
                    "ambiguous ampersand followed by semicolon",
                );
                self.reconsume_in(self.return_state);
            }
            Some(c) => {
                self.reconsume_in_with_char(self.return_state, c);
            }
            None => {
                self.reconsume_in(self.return_state);
            }
        }
    }

    fn state_numeric_character_reference(&mut self, ch: Option<char>) {
        self.character_reference_code = 0;
        match ch {
            Some('x' | 'X') => {
                self.temporary_buffer.push(ch.expect("temporary buffer must not be empty after peek"));
                self.state = LexerState::HexadecimalCharacterReferenceStart;
            }
            Some(c) => {
                self.reconsume_in_with_char(LexerState::DecimalCharacterReferenceStart, c);
            }
            None => {
                self.reconsume_in(LexerState::NumericCharacterReferenceEnd);
            }
        }
    }

    fn state_hexadecimal_character_reference_start(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if c.is_ascii_hexdigit() => {
                self.reconsume_in_with_char(LexerState::HexadecimalCharacterReference, c);
            }
            Some(c) => {
                self.parse_error(LexerErrorKind::AbsenceOfDigitsInNumericCharacterReference, "hex numeric reference without digits");
                self.flush_temporary_buffer();
                self.reconsume_in_with_char(self.return_state, c);
            }
            None => {
                self.reconsume_in(LexerState::NumericCharacterReferenceEnd);
            }
        }
    }

    fn state_decimal_character_reference_start(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if c.is_ascii_digit() => {
                self.reconsume_in_with_char(LexerState::DecimalCharacterReference, c);
            }
            Some(c) => {
                self.parse_error(LexerErrorKind::AbsenceOfDigitsInNumericCharacterReference, "decimal numeric reference without digits");
                self.flush_temporary_buffer();
                self.reconsume_in_with_char(self.return_state, c);
            }
            None => {
                self.reconsume_in(LexerState::NumericCharacterReferenceEnd);
            }
        }
    }

    fn state_hexadecimal_character_reference(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if c.is_ascii_hexdigit() => {
                // Use saturating arithmetic: values > 0x10FFFF are handled in
                // NumericCharacterReferenceEnd (they become U+FFFD per spec §13.2.5.79).
                self.character_reference_code = self.character_reference_code
                    .saturating_mul(16)
                    .saturating_add(c.to_digit(16).expect("already validated as hex digit"));
            }
            Some(';') => {
                self.state = LexerState::NumericCharacterReferenceEnd;
            }
            Some(c) => {
                self.parse_error(LexerErrorKind::MissingSemicolonAfterCharacterReference, "hex reference without semicolon");
                self.reconsume_in_with_char(LexerState::NumericCharacterReferenceEnd, c);
            }
            None => {
                self.state = LexerState::NumericCharacterReferenceEnd;
            }
        }
    }

    fn state_decimal_character_reference(&mut self, ch: Option<char>) {
        match ch {
            Some(c) if c.is_ascii_digit() => {
                // Use saturating arithmetic: values > 0x10FFFF are handled in
                // NumericCharacterReferenceEnd (they become U+FFFD per spec §13.2.5.78).
                self.character_reference_code = self.character_reference_code
                    .saturating_mul(10)
                    .saturating_add(c.to_digit(10).expect("already validated as decimal digit") as u32);
            }
            Some(';') => {
                self.state = LexerState::NumericCharacterReferenceEnd;
            }
            Some(c) => {
                self.parse_error(LexerErrorKind::MissingSemicolonAfterCharacterReference, "decimal reference without semicolon");
                self.reconsume_in_with_char(LexerState::NumericCharacterReferenceEnd, c);
            }
            None => {
                self.state = LexerState::NumericCharacterReferenceEnd;
            }
        }
    }

    fn state_numeric_character_reference_end(&mut self, _ch: Option<char>) {
        // Validate code point
        let mut code = self.character_reference_code;
        
        // Check for surrogate pairs (0xD800-0xDFFF)
        if code >= 0xD800 && code <= 0xDFFF {
            self.parse_error(
                LexerErrorKind::SurrogateCharacterReference,
                "surrogate character reference",
            );
            code = 0xFFFD;
        } else if code == 0 {
            self.parse_error(
                LexerErrorKind::NullCharacterReference,
                "null character reference",
            );
            code = 0xFFFD;
        } else if code > 0x10FFFF {
            self.parse_error(
                LexerErrorKind::CharacterReferenceOutsideUnicodeRange,
                "character reference outside unicode range",
            );
            code = 0xFFFD;
        }
        
        // Handle specific replacements from the spec table (omitted for brevity in initial draft but should be here)
        let replacement = match code {
            0x80 => 0x20AC, 0x82 => 0x201A, 0x83 => 0x0192, 0x84 => 0x201E,
            0x85 => 0x2026, 0x86 => 0x2020, 0x87 => 0x2021, 0x88 => 0x02C6,
            0x89 => 0x2030, 0x8A => 0x0160, 0x8B => 0x2039, 0x8C => 0x0152,
            0x8E => 0x017D, 0x91 => 0x2018, 0x92 => 0x2019, 0x93 => 0x201C,
            0x94 => 0x201D, 0x95 => 0x2022, 0x96 => 0x2013, 0x97 => 0x2014,
            0x98 => 0x02DC, 0x99 => 0x2122, 0x9A => 0x0161, 0x9B => 0x203A,
            0x9C => 0x0153, 0x9E => 0x017E, 0x9F => 0x0178,
            _ => code,
        };

        if let Some(decoded) = std::char::from_u32(replacement) {
            self.emit_character(decoded);
        } else {
            self.emit_character('\u{FFFD}');
        }
        
        self.state = self.return_state;
    }

    fn emit_character(&mut self, c: char) {
        match self.return_state {
            LexerState::Data | LexerState::RcData => self.push_text_char(c),
            LexerState::AttributeValueDoubleQuoted | LexerState::AttributeValueSingleQuoted | LexerState::AttributeValueUnquoted => {
                self.with_current_tag_mut(|tag| tag.push_attribute_value_char(c));
            }
            _ => self.push_text_char(c),
        }
    }

    #[allow(dead_code)]
    fn is_attribute_return_state(&self) -> bool {
        matches!(
            self.return_state,
            LexerState::AttributeValueDoubleQuoted
                | LexerState::AttributeValueSingleQuoted
                | LexerState::AttributeValueUnquoted
        )
    }

    #[allow(dead_code)]
    fn has_entity_starting_with(&self, prefix: &str) -> bool {
        use crate::ace::html::entities::HTML_ENTITIES;
        match HTML_ENTITIES.binary_search_by(|(name, _)| {
            if name.starts_with(prefix) {
                std::cmp::Ordering::Equal
            } else {
                name.cmp(&prefix)
            }
        }) {
            Ok(_) => true,
            Err(idx) => {
                if idx < HTML_ENTITIES.len() {
                    HTML_ENTITIES[idx].0.starts_with(prefix)
                } else {
                    false
                }
            }
        }
    }

    fn lookup_exact_entity(&self, name: &str) -> Option<&'static str> {
        use crate::ace::html::entities::lookup_named_entity;
        lookup_named_entity(name)
    }

    fn flush_temporary_buffer(&mut self) {
        let buf = std::mem::take(&mut self.temporary_buffer);
        for c in buf.chars() {
            self.emit_character(c);
        }
    }

    fn fallback_from_raw_end_tag(&mut self, ch: char, fallback_state: LexerState) {
        self.current_tag = None;
        self.push_text_str("</");
        let buffered = self.temporary_buffer.clone();
        self.push_text_str(&buffered);
        self.temporary_buffer.clear();
        self.reconsume_in_with_char(fallback_state, ch);
    }

    fn begin_tag(&mut self, is_end_tag: bool) {
        self.current_tag = Some(TagTokenBuilder::new(is_end_tag));
    }

    fn finish_attribute(&mut self) {
        if let Some(tag) = self.current_tag.as_mut() {
            if tag.finish_attribute_if_needed() {
                self.parse_error(LexerErrorKind::DuplicateAttribute, "duplicate attribute name");
            }
        }
    }

    fn emit_current_tag_and_switch_to_content_state(&mut self) {
        let line = self.line;
        let column = self.column;
        let Some(builder) = self.current_tag.take() else {
            self.state = LexerState::Data;
            return;
        };

        let token = builder.finish(line, column);

        match &token.kind {
            HtmlTokenKind::StartTag(tag) => {
                self.flush_text();
                let raw_state = tag_to_content_state(&tag.name, tag.self_closing);
                if let Some((raw_tag, state)) = raw_state {
                    self.raw_text_tag = Some(raw_tag);
                    self.state = state;
                } else {
                    self.state = LexerState::Data;
                }
            }
            HtmlTokenKind::EndTag(name) => {
                self.flush_text();
                if self
                    .raw_text_tag
                    .as_ref()
                    .is_some_and(|raw| raw.eq_ignore_ascii_case(name))
                {
                    self.raw_text_tag = None;
                }
                self.state = LexerState::Data;
            }
            _ => self.state = LexerState::Data,
        }

        self.pending.push_back(token);
    }

    fn emit_comment(&mut self) {
        let line = self.line;
        let column = self.column;
        self.flush_text();
        let comment = std::mem::take(&mut self.current_comment);
        self.pending.push_back(HtmlToken {
            kind: HtmlTokenKind::Comment(comment),
            line,
            column,
        });
    }

    fn emit_doctype(&mut self) {
        let line = self.line;
        let column = self.column;
        self.flush_text();
        let dt = self
            .current_doctype
            .take()
            .unwrap_or_default()
            .finish(line, column);
        self.pending.push_back(dt);
    }

    fn is_appropriate_end_tag_token(&self) -> bool {
        let Some(raw_tag) = self.raw_text_tag.as_ref() else {
            return false;
        };

        self.current_tag
            .as_ref()
            .is_some_and(|tag| tag.is_end_tag && tag.name.eq_ignore_ascii_case(raw_tag))
    }

    fn parse_error(&mut self, kind: LexerErrorKind, message: impl Into<String>) {
        self.errors.push(LexerError::new(kind, message, self.line, self.column));
    }

    fn with_current_tag_mut<F: FnOnce(&mut TagTokenBuilder)>(&mut self, f: F) {
        if let Some(tag) = self.current_tag.as_mut() {
            f(tag);
        }
    }

    fn with_current_doctype_mut<F: FnOnce(&mut DoctypeBuilder)>(&mut self, f: F) {
        if let Some(dt) = self.current_doctype.as_mut() {
            f(dt);
        }
    }

    fn push_text_char(&mut self, ch: char) {
        if self.text_buffer.is_empty() {
            self.text_buffer_line = self.line;
            self.text_buffer_column = self.column;
        }
        self.text_buffer.push(ch);
    }

    fn push_text_str(&mut self, text: &str) {
        if self.text_buffer.is_empty() {
            self.text_buffer_line = self.line;
            self.text_buffer_column = self.column;
        }
        self.text_buffer.push_str(text);
    }

    fn flush_text(&mut self) {
        if self.text_buffer.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.text_buffer);
        self.pending.push_back(HtmlToken {
            kind: HtmlTokenKind::Character(text),
            line: self.text_buffer_line,
            column: self.text_buffer_column,
        });
    }

    fn consume_next_input_character(&mut self) -> Option<char> {
        if self.pos >= self.chars.len() {
            return None;
        }
        let ch = self.chars[self.pos];
        self.pos += 1;

        self.last_column = self.column;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(ch)
    }

    #[allow(dead_code)]
    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn reconsume(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
            let ch = self.chars[self.pos];
            if ch == '\n' {
                self.line -= 1;
                self.column = self.last_column; // This is a simplification but works for 1-char reconsume
            } else {
                self.column -= 1;
            }
        }
    }

    pub fn set_state(&mut self, state: LexerState) {
        self.state = state;
    }

    pub fn remaining_input(&self) -> String {
        self.chars.iter().skip(self.pos).copied().collect()
    }

    fn reconsume_in(&mut self, new_state: LexerState) {
        self.state = new_state;
        self.reconsume();
    }

    fn reconsume_in_with_char(&mut self, new_state: LexerState, _just_consumed: char) {
        self.state = new_state;
        self.reconsume();
    }

    fn starts_with(&self, needle: &str) -> bool {
        let len = needle.chars().count();
        if self.pos + len > self.chars.len() {
            return false;
        }

        self.chars
            .iter()
            .skip(self.pos)
            .take(len)
            .copied()
            .zip(needle.chars())
            .all(|(a, b)| a == b)
    }

    fn starts_with_case_insensitive(&self, needle: &str) -> bool {
        let needle_chars: Vec<char> = needle.chars().collect();
        let len = needle_chars.len();
        if self.pos + len > self.chars.len() {
            return false;
        }

        self.chars.iter().skip(self.pos).zip(needle_chars.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    }
}

fn tag_to_content_state(tag_name: &str, self_closing: bool) -> Option<(String, LexerState)> {
    if self_closing {
        return None;
    }

    let name = tag_name.to_ascii_lowercase();
    let state = match name.as_str() {
        "title" | "textarea" => LexerState::RcData,
        "script" => LexerState::ScriptData,
        "style" | "xmp" | "iframe" | "noembed" | "noframes" | "noscript" => {
            LexerState::RawText
        }
        "plaintext" => LexerState::PlainText,
        _ => return None,
    };

    Some((name, state))
}

fn is_ascii_alpha(ch: char) -> bool {
    ch.is_ascii_alphabetic()
}

fn is_ascii_alnum(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
}

fn is_whitespace(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\u{000C}' | ' ')
}

fn normalize_numeric_codepoint(mut cp: u32) -> char {
    if cp == 0 {
        return '\u{FFFD}';
    }

    // C1 control replacement table per HTML spec.
    cp = match cp {
        0x80 => 0x20AC,
        0x82 => 0x201A,
        0x83 => 0x0192,
        0x84 => 0x201E,
        0x85 => 0x2026,
        0x86 => 0x2020,
        0x87 => 0x2021,
        0x88 => 0x02C6,
        0x89 => 0x2030,
        0x8A => 0x0160,
        0x8B => 0x2039,
        0x8C => 0x0152,
        0x8E => 0x017D,
        0x91 => 0x2018,
        0x92 => 0x2019,
        0x93 => 0x201C,
        0x94 => 0x201D,
        0x95 => 0x2022,
        0x96 => 0x2013,
        0x97 => 0x2014,
        0x98 => 0x02DC,
        0x99 => 0x2122,
        0x9A => 0x0161,
        0x9B => 0x203A,
        0x9C => 0x0153,
        0x9E => 0x017E,
        0x9F => 0x0178,
        _ => cp,
    };

    if (0xD800..=0xDFFF).contains(&cp) || cp > 0x10FFFF {
        return '\u{FFFD}';
    }

    if matches!(cp, 0x000D | 0x000B) {
        return '\u{FFFD}';
    }

    char::from_u32(cp).unwrap_or('\u{FFFD}')
}

pub fn decode_character_references(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '&' {
            out.push(ch);
            continue;
        }

        let mut consumed = String::new();
        let mut lookahead = chars.clone();

        while let Some(next) = lookahead.peek().copied() {
            if next == ';' {
                break;
            }
            if !(next.is_ascii_alphanumeric() || matches!(next, '#' | 'x' | 'X')) {
                break;
            }
            consumed.push(next);
            lookahead.next();
            if consumed.len() > 64 {
                break;
            }
        }

        let has_semicolon = lookahead.peek() == Some(&';');
        if !consumed.is_empty() && has_semicolon {
            // advance original iterator by consumed chars and semicolon
            for _ in 0..consumed.chars().count() {
                chars.next();
            }
            chars.next();

            if let Some(decoded) = decode_reference_from_text(&consumed) {
                out.push_str(&decoded);
                continue;
            }

            out.push('&');
            out.push_str(&consumed);
            out.push(';');
            continue;
        }

        out.push('&');
    }

    out
}

fn decode_reference_from_text(text: &str) -> Option<String> {
    if let Some(number) = text.strip_prefix("#x").or_else(|| text.strip_prefix("#X")) {
        let value = u32::from_str_radix(number, 16).ok()?;
        return Some(normalize_numeric_codepoint(value).to_string());
    }

    if let Some(number) = text.strip_prefix('#') {
        let value = number.parse::<u32>().ok()?;
        return Some(normalize_numeric_codepoint(value).to_string());
    }

    super::entities::decode_named_entity(text).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::{decode_character_references, HtmlLexer, HtmlToken, HtmlTokenKind, LexerErrorKind};

    fn build_lexer(input: &str) -> HtmlLexer {
        let mut lexer = HtmlLexer::new(input);
        lexer.end();
        lexer
    }

    fn next_non_eof(lexer: &mut HtmlLexer) -> Vec<HtmlToken> {
        let mut out = Vec::new();
        while let Some(token) = lexer.next_token() {
            if matches!(token.kind, HtmlTokenKind::Eof) {
                break;
            }
            out.push(token);
        }
        out
    }

    #[test]
    fn tokenizes_basic_markup() {
        let mut lexer = build_lexer(r#"<div class="hero">Hi &amp; bye</div>"#);

        let tokens = next_non_eof(&mut lexer);
        assert_eq!(tokens.len(), 3);

        let HtmlTokenKind::StartTag(tag) = &tokens[0].kind else {
            panic!("expected start tag");
        };
        assert_eq!(tag.name, "div");
        assert_eq!(tag.attributes.get("class"), Some(&"hero".to_string()));

        assert_eq!(tokens[1].kind, HtmlTokenKind::Character("Hi & bye".to_string()));
        assert_eq!(tokens[2].kind, HtmlTokenKind::EndTag("div".to_string()));
    }

    #[test]
    fn tokenizes_comment_and_doctype() {
        let mut lexer = HtmlLexer::new(r#"<!DOCTYPE html><!-- note -->"#);
        let tokens = next_non_eof(&mut lexer);
        assert!(matches!(tokens[0].kind, HtmlTokenKind::Doctype(_)));
        assert_eq!(tokens[1].kind, HtmlTokenKind::Comment(" note ".to_string()));
    }

    #[test]
    fn tokenizes_doctype_with_public_and_system_identifiers() {
        let mut lexer =
            HtmlLexer::new(r#"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "about:legacy-compat">"#);

        let tokens = next_non_eof(&mut lexer);
        let HtmlTokenKind::Doctype(dt) = &tokens[0].kind else {
            panic!("expected doctype token");
        };

        assert_eq!(dt.name.as_deref(), Some("html"));
        assert_eq!(
            dt.public_id.as_deref(),
            Some("-//W3C//DTD XHTML 1.0 Transitional//EN")
        );
        assert_eq!(dt.system_id.as_deref(), Some("about:legacy-compat"));
        assert!(!dt.force_quirks);
    }

    #[test]
    fn decodes_numeric_entities() {
        assert_eq!(
            decode_character_references("Tom &amp; Jerry &#x1F63A;"),
            "Tom & Jerry 😺"
        );
    }

    #[test]
    fn records_recoverable_lexer_errors() {
        let mut lexer = HtmlLexer::new("</>");
        let _ = next_non_eof(&mut lexer);
        assert!(lexer
            .errors()
            .iter()
            .any(|err| err.kind == LexerErrorKind::MissingEndTagName));
    }

    #[test]
    fn enters_rcdata_and_decodes_entities() {
        let mut lexer = HtmlLexer::new("Hello &amp; <b>");
        lexer.set_raw_text_tag(Some("textarea".to_string()));
        let tokens = next_non_eof(&mut lexer);
        assert_eq!(tokens[0].kind, HtmlTokenKind::Character("Hello & ".to_string()));
    }

    #[test]
    fn keeps_rawtext_without_entity_decoding() {
        let mut lexer = HtmlLexer::new("A &amp; B</style>");
        lexer.set_raw_text_tag(Some("style".to_string()));
        let tokens = next_non_eof(&mut lexer);
        assert_eq!(tokens[0].kind, HtmlTokenKind::Character("A &amp; B".to_string()));
        assert_eq!(tokens[1].kind, HtmlTokenKind::EndTag("style".to_string()));
    }

    #[test]
    fn emits_comment_token_when_comment_ends_at_eof() {
        let mut lexer = HtmlLexer::new("<!--x");
        let tokens = next_non_eof(&mut lexer);

        assert_eq!(tokens.iter().map(|t| t.kind.clone()).collect::<Vec<_>>(), vec![HtmlTokenKind::Comment("x".to_string())]);
        assert!(lexer
            .errors()
            .iter()
            .any(|err| err.kind == LexerErrorKind::EofInComment));
    }

    #[test]
    fn keeps_script_text_until_matching_end_tag() {
        let mut lexer = HtmlLexer::new("<script>if (a < b) call();</script><p>x</p>");
        let tokens = next_non_eof(&mut lexer);

        let script_start = tokens
            .iter()
            .position(|token| matches!(token.kind, HtmlTokenKind::StartTag(ref tag) if tag.name == "script"))
            .expect("expected script start tag");
        let script_end = tokens
            .iter()
            .position(|token| matches!(token.kind, HtmlTokenKind::EndTag(ref name) if name == "script"))
            .expect("expected script end tag");
        assert!(script_end > script_start);

        let mut script_text = String::new();
        for token in &tokens[script_start + 1..script_end] {
            if let HtmlTokenKind::Character(data) = &token.kind {
                script_text.push_str(data);
            }
        }
        assert_eq!(script_text, "if (a < b) call();");

        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, HtmlTokenKind::StartTag(ref tag) if tag.name == "p")));
        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, HtmlTokenKind::EndTag(ref name) if name == "p")));
    }

    #[test]
    fn keeps_script_escaped_sequences_as_text() {
        let mut lexer = build_lexer("<script><!-- alert(1) //--></script>");
        let tokens = next_non_eof(&mut lexer);

        let script_text: String = tokens
            .iter()
            .filter_map(|token| match &token.kind {
                HtmlTokenKind::Character(data) => Some(data.as_str()),
                _ => None,
            })
            .collect();

        assert!(script_text.contains("<!-- alert(1) //-->"));
        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, HtmlTokenKind::EndTag(ref name) if name == "script")));
    }

    #[test]
    fn keeps_script_double_escaped_sequences_as_text() {
        let mut lexer = build_lexer("<script><!--<script>var a = 1;</script>--></script>");
        let tokens = next_non_eof(&mut lexer);

        let script_text: String = tokens
            .iter()
            .filter_map(|token| match &token.kind {
                HtmlTokenKind::Character(data) => Some(data.as_str()),
                _ => None,
            })
            .collect();

        assert!(script_text.contains("<script>var a = 1;"));
        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, HtmlTokenKind::EndTag(ref name) if name == "script")));
    }

    #[test]
    fn no_panic_on_short_malformed_fuzz_inputs() {
        fn next_u64(seed: &mut u64) -> u64 {
            let mut x = *seed;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *seed = x;
            x
        }

        let alphabet: [char; 22] = [
            '<', '>', '/', '!', '&', ';', '"', '\'', '=', 'a', 'b', 'c', '0', '9', ' ', '\n',
            '\t', '\0', '#', 'x', '-', '?',
        ];

        let mut seed = 0xD0C0_A11E_u64;
        for _ in 0..1024 {
            let len = (next_u64(&mut seed) % 24) as usize;
            let mut input = String::new();
            for _ in 0..len {
                let idx = (next_u64(&mut seed) % alphabet.len() as u64) as usize;
                input.push(alphabet[idx]);
            }

            let mut lexer = build_lexer(&input);
            let mut guard = 0usize;
            while let Some(token) = lexer.next_token() {
                guard += 1;
                assert!(guard < 4096, "lexer did not terminate for input {:?}", input);
                if matches!(token.kind, HtmlTokenKind::Eof) {
                    break;
                }
            }
        }
    }
}
