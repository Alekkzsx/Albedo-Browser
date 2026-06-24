use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeDirtyFlags(u32);

impl NodeDirtyFlags {
    pub const NONE: Self = Self(0);
    pub const STYLE: Self = Self(1 << 0);
    pub const LAYOUT: Self = Self(1 << 1);
    pub const CHILDREN: Self = Self(1 << 2);
    pub const SUBTREE: Self = Self(1 << 3);

    /// TODO: add docs
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// TODO: add docs
    pub fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    /// TODO: add docs
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// TODO: add docs
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl std::ops::BitOr for NodeDirtyFlags {
pub(crate) type Output = Self;

pub(crate) fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for NodeDirtyFlags {
pub(crate) fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
