use super::*;
use std::fmt;


#[derive(Debug, Clone, PartialEq)]
pub enum CssLength {
    Px(f32),
    Percent(f32),
    Vw(f32),
    Vh(f32),
    Rem(f32),
    Em(f32),
    Fr(f32),
    Auto,
    Zero,
    Clamp(Box<CssLength>, Box<CssLength>, Box<CssLength>),
    Min(Vec<CssLength>),
    Max(Vec<CssLength>),
    MinMax(Box<CssLength>, Box<CssLength>),
    Repeat(String, Vec<CssLength>),
    MinContent,
    MaxContent,
    AutoFill,
    AutoFit,
    Calc(String),
    LineNames(Vec<String>),
    Name(String),
    Number(f32),
    Subgrid,
    Span(u16),
}

impl Default for CssLength {
pub(crate) fn default() -> Self {
        Self::Auto
    }
}

impl fmt::Display for CssLength {
pub(crate) fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CssLength::Px(v) => write!(f, "{}px", v),
            CssLength::Percent(v) => write!(f, "{}%", v),
            CssLength::Vw(v) => write!(f, "{}vw", v),
            CssLength::Vh(v) => write!(f, "{}vh", v),
            CssLength::Rem(v) => write!(f, "{}rem", v),
            CssLength::Em(v) => write!(f, "{}em", v),
            CssLength::Fr(v) => write!(f, "{}fr", v),
            CssLength::Auto => write!(f, "auto"),
            CssLength::Zero => write!(f, "0"),
            CssLength::Clamp(min, val, max) => write!(f, "clamp({}, {}, {})", min, val, max),
            CssLength::Min(vals) => {
                let s: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                write!(f, "min({})", s.join(", "))
            }
            CssLength::Max(vals) => {
                let s: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                write!(f, "max({})", s.join(", "))
            }
            CssLength::Calc(s) => write!(f, "calc({})", s),
            CssLength::MinMax(min, max) => write!(f, "minmax({}, {})", min, max),
            CssLength::Repeat(count, sub) => {
                let s: Vec<String> = sub.iter().map(|v| v.to_string()).collect();
                write!(f, "repeat({}, {})", count, s.join(", "))
            }
            CssLength::MinContent => write!(f, "min-content"),
            CssLength::MaxContent => write!(f, "max-content"),
            CssLength::AutoFill => write!(f, "auto-fill"),
            CssLength::AutoFit => write!(f, "auto-fit"),
            CssLength::LineNames(names) => write!(f, "[{}]", names.join(" ")),
            CssLength::Name(name) => write!(f, "{}", name),
            CssLength::Number(v) => write!(f, "{}", v),
            CssLength::Subgrid => write!(f, "subgrid"),
            CssLength::Span(v) => write!(f, "span {}", v),
        }
    }
}
