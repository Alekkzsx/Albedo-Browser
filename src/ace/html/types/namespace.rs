use super::*;
use std::collections::{BTreeMap, HashMap};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Namespace {
    Html,
    Svg,
    MathMl,
}
