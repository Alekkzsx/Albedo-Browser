use super::*;
use std::collections::{BTreeMap, HashMap};



/// TODO: add docs
pub fn is_void_element(tag: &str) -> bool {
    tag.eq_ignore_ascii_case("area")
        || tag.eq_ignore_ascii_case("base")
        || tag.eq_ignore_ascii_case("br")
        || tag.eq_ignore_ascii_case("col")
        || tag.eq_ignore_ascii_case("embed")
        || tag.eq_ignore_ascii_case("hr")
        || tag.eq_ignore_ascii_case("img")
        || tag.eq_ignore_ascii_case("input")
        || tag.eq_ignore_ascii_case("link")
        || tag.eq_ignore_ascii_case("meta")
        || tag.eq_ignore_ascii_case("param")
        || tag.eq_ignore_ascii_case("source")
        || tag.eq_ignore_ascii_case("track")
        || tag.eq_ignore_ascii_case("wbr")
}
