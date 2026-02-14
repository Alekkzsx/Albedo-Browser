use selectors::{Element, SelectorImpl};
use common_impls::{LocalName, NamespaceUrl, AttrValue};
use kuchiki::NodeRef;
use cssparser::ToCss;
use precomputed_hash::PrecomputedHash;

pub mod common_impls {
    use std::fmt;
    use cssparser::ToCss;
    use precomputed_hash::PrecomputedHash;
    use std::hash::{Hash, Hasher};

    #[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
    pub struct LocalName(pub String);
    
    impl ToCss for LocalName {
        fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
            dest.write_str(&self.0)
        }
    }
    impl From<&str> for LocalName {
        fn from(s: &str) -> Self { LocalName(s.to_string()) }
    }
    impl PrecomputedHash for LocalName {
        fn precomputed_hash(&self) -> u32 {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.0.hash(&mut hasher);
            hasher.finish() as u32
        }
    }
    
    #[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
    pub struct NamespaceUrl(pub String);
    impl ToCss for NamespaceUrl {
       fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
            dest.write_str(&self.0)
        }
    }
    impl From<&str> for NamespaceUrl {
        fn from(s: &str) -> Self { NamespaceUrl(s.to_string()) }
    }
    impl PrecomputedHash for NamespaceUrl {
        fn precomputed_hash(&self) -> u32 {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.0.hash(&mut hasher);
            hasher.finish() as u32
        }
    }
    
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub struct AttrValue(pub String);
    impl ToCss for AttrValue {
        fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
             dest.write_str(&self.0)
        }
    }
     impl From<&str> for AttrValue {
        fn from(s: &str) -> Self { AttrValue(s.to_string()) }
    }
    impl AsRef<str> for AttrValue {
        fn as_ref(&self) -> &str { &self.0 }
    }
}

#[derive(Debug, Clone)]
pub struct Helper;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PseudoClass;
impl ToCss for PseudoClass {
    fn to_css<W>(&self, _dest: &mut W) -> std::fmt::Result where W: std::fmt::Write { Ok(()) }
}
impl selectors::parser::NonTSPseudoClass for PseudoClass {
    type Impl = Helper;
    fn is_active_or_hover(&self) -> bool { false }
    fn is_user_action_state(&self) -> bool { false }
}
impl From<&str> for PseudoClass {
    fn from(_: &str) -> Self { PseudoClass }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PseudoElement;
impl ToCss for PseudoElement {
    fn to_css<W>(&self, _dest: &mut W) -> std::fmt::Result where W: std::fmt::Write { Ok(()) }
}
impl selectors::parser::PseudoElement for PseudoElement {
     type Impl = Helper;
}
impl From<&str> for PseudoElement {
    fn from(_: &str) -> Self { PseudoElement }
}

impl SelectorImpl for Helper {
    type ExtraMatchingData<'a> = ();
    type AttrValue = AttrValue;
    type Identifier = LocalName; 
    type LocalName = LocalName;
    type NamespaceUrl = NamespaceUrl;
    type NamespacePrefix = LocalName;
    type BorrowedNamespaceUrl = NamespaceUrl;
    type BorrowedLocalName = LocalName;
    type NonTSPseudoClass = PseudoClass;
    type PseudoElement = PseudoElement;
}




#[derive(Debug, Clone)]
pub struct AceElement(pub NodeRef);

impl Element for AceElement {
    type Impl = Helper;

    fn opaque(&self) -> selectors::OpaqueElement {
        selectors::OpaqueElement::new(&self.0)
    }

    fn parent_element(&self) -> Option<Self> {
        self.0.parent().and_then(|n| {
            if n.as_element().is_some() {
                Some(AceElement(n))
            } else {
                None
            }
        })
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        let mut node = self.0.previous_sibling();
        while let Some(n) = node {
            if n.as_element().is_some() {
                return Some(AceElement(n));
            }
            node = n.previous_sibling();
        }
        None
    }

    fn next_sibling_element(&self) -> Option<Self> {
        let mut node = self.0.next_sibling();
        while let Some(n) = node {
            if n.as_element().is_some() {
                return Some(AceElement(n));
            }
            node = n.next_sibling();
        }
        None
    }

    fn is_html_element_in_html_document(&self) -> bool {
        true
    }

    fn has_local_name(&self, local_name: &LocalName) -> bool {
        if let Some(data) = self.0.as_element() {
            data.name.local.to_string() == local_name.0
        } else {
            false
        }
    }

    fn has_namespace(&self, _ns: &NamespaceUrl) -> bool {
        false 
    }

    fn is_same_type(&self, other: &Self) -> bool {
        if let (Some(d1), Some(d2)) = (self.0.as_element(), other.0.as_element()) {
            d1.name.local == d2.name.local
        } else {
            false
        }
    }

    fn attr_matches(
        &self,
        _ns: &selectors::attr::NamespaceConstraint<&NamespaceUrl>,
        local_name: &LocalName,
        operation: &selectors::attr::AttrSelectorOperation<&AttrValue>,
    ) -> bool {
        if let Some(data) = self.0.as_element() {
            let attrs = data.attributes.borrow();
            if let Some(val) = attrs.get(local_name.0.as_str()) {
                let attr_val = AttrValue(val.to_string());
                return operation.eval_str(attr_val.as_ref());
            }
        }
        false
    }
    
    fn match_non_ts_pseudo_class(
        &self,
        _pc: &PseudoClass,
        _context: &mut selectors::context::MatchingContext<Self::Impl>,
    ) -> bool
    {
        false
    }

    fn match_pseudo_element(
        &self,
        _pe: &PseudoElement,
        _context: &mut selectors::context::MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }
    
    fn is_link(&self) -> bool {
        if let Some(data) = self.0.as_element() {
            data.name.local.to_string() == "a"
        } else {
            false
        }
    }
    
    fn is_html_slot_element(&self) -> bool {
        false
    }
    
    fn is_part(&self, _name: &LocalName) -> bool {
        false
    }
    
    fn imported_part(
        &self,
        _name: &LocalName,
    ) -> Option<LocalName> {
        None
    }
    
     fn is_empty(&self) -> bool {
        !self.0.children().any(|_| true)
    }
    
    fn is_root(&self) -> bool {
        self.0.parent().is_none()
    }
    
    fn parent_node_is_shadow_root(&self) -> bool { false }
    fn containing_shadow_host(&self) -> Option<Self> { None }
    fn is_pseudo_element(&self) -> bool { false }
    fn first_element_child(&self) -> Option<Self> {
        let mut node = self.0.first_child();
         while let Some(n) = node {
            if n.as_element().is_some() {
                return Some(AceElement(n));
            }
            node = n.next_sibling();
        }
        None
    }
    fn apply_selector_flags(&self, _flags: selectors::matching::ElementSelectorFlags) {}
    
    fn has_id(&self, id: &LocalName, _case_sensitivity: selectors::attr::CaseSensitivity) -> bool {
         if let Some(data) = self.0.as_element() {
             if let Some(val) = data.attributes.borrow().get("id") {
                 return val == id.0;
             }
         }
         false
    }
    
    fn has_class(&self, name: &LocalName, _case_sensitivity: selectors::attr::CaseSensitivity) -> bool {
         if let Some(data) = self.0.as_element() {
              if let Some(val) = data.attributes.borrow().get("class") {
                  return val.split_whitespace().any(|c| c == name.0);
              }
         }
         false
    }
    
    fn has_custom_state(&self, _name: &LocalName) -> bool { false }
    fn add_element_unique_hashes(&self, _filter: &mut selectors::bloom::BloomFilter) -> bool { false }
}
