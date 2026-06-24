use super::*;
use std::collections::{BTreeMap, HashMap};



#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    HtmlSyntax,
    InvalidDoctype,
    DecodeError,
    FosterParenting,
    UnexpectedEof,
    MissingSemicolonAfterCharacterReference,
    LexerParseError,
    CdataSectionOutsideForeignContent,
    NullCharacter,
    NestedComment,
    EofInComment,
    EofInDoctype,
    EofInTag,
}
