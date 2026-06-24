use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


#[cfg(test)]
mod tests {
    use super::*;

pub(crate) fn make_element_dom() -> AceDOM {
        let mut dom = AceDOM::new();
        let elem_id = dom.create_element("div", vec![]);
        let _text_id = dom.create_text_node("hello");
        dom.append_child(elem_id, 1);
        dom
    }

    #[test]
pub(crate) fn get_element_returns_some_for_element_node() {
        let dom = make_element_dom();
        assert!(dom.get_element(0).is_some());
    }

    #[test]
pub(crate) fn get_element_returns_none_for_text_node() {
        let dom = make_element_dom();
        assert!(dom.get_element(1).is_none());
    }

    #[test]
pub(crate) fn get_element_returns_none_for_out_of_bounds() {
        let dom = make_element_dom();
        assert!(dom.get_element(999).is_none());
    }

    #[test]
pub(crate) fn get_element_mut_allows_modification() {
        let mut dom = make_element_dom();
        if let Some(el) = dom.get_element_mut(0) {
            el.tag = "span".to_string();
        }
        assert_eq!(dom.get_element(0).expect("Albedo Engine: internal invariant violated").tag, "span");
    }
}
