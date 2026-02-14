use crate::js::bindings::element::Element;
use crate::js::bindings::element::mark_mutation;
use kuchiki::traits::*;
use kuchiki::NodeRef;

pub fn tag_name(el: &Element) -> String {
    el.node.as_element()
        .map(|data| data.name.local.to_string().to_uppercase())
        .unwrap_or_else(|| "".to_string())
}

pub fn text_content(el: &Element) -> String {
    el.node.text_contents()
}

pub fn set_text_content(el: &Element, text: String) {
    el.node.children().for_each(|child| child.detach());
    el.node.append(NodeRef::new_text(text));
    mark_mutation(el);
}

pub fn get_attribute(el: &Element, name: String) -> Option<String> {
    el.node.as_element().and_then(|data| {
        data.attributes.borrow().get(name.as_str()).map(|s| s.to_string())
    })
}

pub fn set_attribute(el: &Element, name: String, value: String) {
    if let Some(data) = el.node.as_element() {
        data.attributes.borrow_mut().insert(name, value);
        mark_mutation(el);
    }
}

pub fn has_attribute(el: &Element, name: String) -> bool {
    el.node.as_element()
        .map(|data| data.attributes.borrow().contains(name.as_str()))
        .unwrap_or(false)
}

pub fn remove_attribute(el: &Element, name: String) {
    if let Some(data) = el.node.as_element() {
        data.attributes.borrow_mut().remove(name);
        mark_mutation(el);
    }
}

pub fn inner_html(el: &Element) -> String {
    el.node.children().map(|c| c.to_string()).collect::<String>()
}

pub fn set_inner_html(el: &Element, html: String) {
    el.node.children().for_each(|child| child.detach());
    
    let document = kuchiki::parse_html().one(html);
    
    for section in &["head", "body"] {
        if let Ok(sec_node) = document.select_first(section) {
            let children: Vec<_> = sec_node.as_node().children().collect();
            for child in children {
                el.node.append(child);
            }
        }
    }
    mark_mutation(el);
}
