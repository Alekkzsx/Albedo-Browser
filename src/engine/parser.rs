use kuchiki::traits::*;
use crate::engine::dom::DomTree;

pub fn parse_html(html: &str) -> DomTree {
    let document = kuchiki::parse_html().one(html);
    DomTree::new(document)
}
