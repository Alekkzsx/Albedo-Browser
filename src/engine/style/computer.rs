use crate::engine::style::{Stylesheet, Style, Rule};
use crate::engine::style::selector_impl::AceElement;
use selectors::matching::{QuirksMode, MatchingMode, MatchingContext, MatchingForInvalidation, NeedsSelectorFlags};
use selectors::context::SelectorCaches;
use kuchiki::NodeRef;

/// Handles the CSS Cascade and computes the final style for an element.
pub struct StyleComputer<'a> {
    pub stylesheet: &'a Stylesheet,
}

impl<'a> StyleComputer<'a> {
    pub fn new(stylesheet: &'a Stylesheet) -> Self {
        Self { stylesheet }
    }

    /// Computes the final style for a given node.
    pub fn compute_style(&self, node: &NodeRef) -> Style {
        let element_data = match node.as_element() {
            Some(e) => e,
            None => return Style::default(), // Should not happen for styled nodes
        };

        let tag = element_data.name.local.to_string();
        let mut style = Style::default_for_tag(&tag);

        let element_wrapper = AceElement(node.clone());
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );

        // 1. Collect and sort matching rules by specificity
        let mut matching_rules: Vec<&Rule> = self.stylesheet.rules.iter()
            .filter(|rule| {
                selectors::matching::matches_selector_list(
                    &rule.selectors,
                    &element_wrapper,
                    &mut context
                )
            })
            .collect();

        // Sort by specificity (ascending, so higher specificity comes last and overrides)
        matching_rules.sort_by_key(|rule| rule.max_specificity());

        for rule in matching_rules {
            for decl in &rule.declarations {
                style.apply_declaration(decl);
            }
        }

        // 2. Apply inline style
        if let Some(s) = element_data.attributes.borrow().get("style") {
            let mut input = cssparser::ParserInput::new(s);
            let mut parser = cssparser::Parser::new(&mut input);
            let declarations = crate::engine::style::parser::parse_declarations(&mut parser);
            for decl in declarations {
                style.apply_declaration(&decl);
            }
        }

        style
    }
}
