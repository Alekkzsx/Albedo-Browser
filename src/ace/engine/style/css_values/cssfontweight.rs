use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssFontWeight {
    Normal,
    Bold,
    Lighter,
    Bolder,
    Weight(f32),
}

impl CssFontWeight {
    /// TODO: add docs
    pub fn to_cosmic(&self) -> cosmic_text::Weight {
        match self {
            CssFontWeight::Normal => cosmic_text::Weight::NORMAL,
            CssFontWeight::Bold => cosmic_text::Weight::BOLD,
            CssFontWeight::Lighter => cosmic_text::Weight::THIN,
            CssFontWeight::Bolder => cosmic_text::Weight::EXTRA_BOLD,
            CssFontWeight::Weight(w) => cosmic_text::Weight(*w as u16),
        }
    }
}

impl Default for CssFontWeight {
pub(crate) fn default() -> Self {
        Self::Normal
    }
}
