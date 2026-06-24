use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use cssparser::{CowRcStr, ParseError, Parser, ParserInput, SourceLocation, ToCss};
use precomputed_hash::PrecomputedHash;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext, MatchingMode};
use selectors::OpaqueElement;
use std::collections::HashMap;
pub mod css_values;
use self::css_values::{
    BackgroundImage, BoxShadow, ComputedStyle, CssAlignContent, CssAlignItems, CssBlendMode,
    CssBoxSizing, CssClear, CssColor, CssContent, CssCursor, CssDisplay, CssFlexDirection,
    CssFlexWrap, CssFloat, CssFontWeight, CssJustifyContent, CssLength, CssObjectFit,
    CssObjectPosition, CssOverflow, CssPointerEvents, CssPosition, CssTextAlign, CssTextOverflow,
    CssTextTransform, CssVisibility, CssWhiteSpace, Gradient, GradientStop, TextShadow,
};
use crate::ace::engine::style::css_values::CssFilter;
use crate::ace::engine::style::css_values::TransformFunction;

pub mod animation;



pub(crate) fn parse_display(val: &str) -> CssDisplay {
    match val.trim() {
        "none" => CssDisplay::None,
        "block" => CssDisplay::Block,
        "inline-block" => CssDisplay::InlineBlock,
        "inline" => CssDisplay::Inline,
        "flex" => CssDisplay::Flex,
        "inline-flex" => CssDisplay::InlineFlex,
        "grid" => CssDisplay::Grid,
        "contents" => CssDisplay::Contents,
        "table" => CssDisplay::Table,
        "table-row" => CssDisplay::TableRow,
        "table-cell" => CssDisplay::TableCell,
        "table-header-group" | "table-header" => CssDisplay::TableHeader,
        _ => CssDisplay::Inline,
    }
}
