//! # CSSOM Variables, Cycles, Case Sensitivity & IACVT Adversarial Stress Test Suite
//!
//! Empirical stress tests covering:
//! 1. Complex cyclic dependency topologies (cycles of length 1, 2, 5, 20, disjoint cycles, branching, fallbacks).
//! 2. Deeply nested `var()` expressions (50-level nested fallbacks, 50-level dependency chains).
//! 3. Comma-containing values inside fallbacks (font families, box shadows, gradients, custom property values).
//! 4. Case sensitivity for custom properties vs case insensitivity for standard properties & keywords.
//! 5. IACVT (Invalid At Computed-Value Time) property resetting for non-inheritable vs inheriting for inheritable.
//! 6. Poisoning of multi-var declarations by a single invalid variable.
//! 7. Balanced quote/paren parsing with embedded semicolons.

use ace_dom::cssom::computed::{initial_value_for_property, is_inheritable_property};
use ace_dom::cssom::StyleResolver;
use ace_dom::parse_html;

// =========================================================================
// 1. Complex Cyclic Dependency Topologies
// =========================================================================

#[test]
fn test_adversarial_cycle_length_1() {
    // Length 1 self-cycle: --a: var(--a)
    let html = r#"<div id="target" style="--a: var(--a); color: var(--a);"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    // IACVT on root/target element -> inheritable 'color' falls back to initial value 'black'
    assert_eq!(computed.get_property_value("color"), Some("black"));
    assert_eq!(computed.get_custom_property("--a"), None);
}

#[test]
fn test_adversarial_cycle_length_2() {
    // Length 2 mutual cycle: --a -> --b -> --a
    let html = r#"<div id="target" style="--a: var(--b); --b: var(--a); color: var(--a); font-size: var(--b);"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
    assert_eq!(computed.get_property_value("font-size"), Some("16px"));
    assert_eq!(computed.get_custom_property("--a"), None);
    assert_eq!(computed.get_custom_property("--b"), None);
}

#[test]
fn test_adversarial_cycle_length_5() {
    // Length 5 cycle: --a -> --b -> --c -> --d -> --e -> --a
    let html = r#"
        <div id="target" style="
            --a: var(--b);
            --b: var(--c);
            --c: var(--d);
            --d: var(--e);
            --e: var(--a);
            color: var(--a);
            font-size: var(--c);
            visibility: var(--e);
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
    assert_eq!(computed.get_property_value("font-size"), Some("16px"));
    assert_eq!(computed.get_property_value("visibility"), Some("visible"));
    assert_eq!(computed.get_custom_property("--a"), None);
    assert_eq!(computed.get_custom_property("--e"), None);
}

#[test]
fn test_adversarial_cycle_length_20() {
    // Length 20 cycle: --node0 -> --node1 -> ... -> --node19 -> --node0
    let mut style_str = String::new();
    for i in 0..20 {
        let next = (i + 1) % 20;
        style_str.push_str(&format!("--node{}: var(--node{}); ", i, next));
    }
    style_str.push_str("color: var(--node0); display: var(--node10);");

    let html = format!(r#"<div id="target" style="{}"></div>"#, style_str);
    let doc = parse_html(&html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
    assert_eq!(computed.get_property_value("display"), Some("inline"));

    for i in 0..20 {
        assert_eq!(
            computed.get_custom_property(&format!("--node{}", i)),
            None,
            "Variable --node{} should be invalid due to 20-cycle",
            i
        );
    }
}

#[test]
fn test_adversarial_disjoint_multiple_cycles() {
    // Two independent cycles:
    // Cycle 1: --a1 -> --a2 -> --a1
    // Cycle 2: --b1 -> --b2 -> --b3 -> --b1
    // Independent valid variable: --valid: 42px
    let html = r#"
        <div id="target" style="
            --a1: var(--a2);
            --a2: var(--a1);
            --b1: var(--b2);
            --b2: var(--b3);
            --b3: var(--b1);
            --valid: 42px;
            color: var(--a1);
            font-size: var(--b1);
            width: var(--valid);
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
    assert_eq!(computed.get_property_value("font-size"), Some("16px"));
    assert_eq!(computed.get_property_value("width"), Some("42px"));
    assert_eq!(computed.get_custom_property("--valid"), Some("42px"));
    assert_eq!(computed.get_custom_property("--a1"), None);
    assert_eq!(computed.get_custom_property("--b1"), None);
}

#[test]
fn test_adversarial_cycle_with_downstream_branching() {
    // Cycle: --c1 -> --c2 -> --c1
    // Branch pointing into cycle: --branch1: var(--c2)
    // Downstream of branch: --branch2: var(--branch1)
    let html = r#"
        <div id="target" style="
            --c1: var(--c2);
            --c2: var(--c1);
            --branch1: var(--c2);
            --branch2: var(--branch1);
            color: var(--branch2);
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
    assert_eq!(computed.get_custom_property("--branch1"), None);
    assert_eq!(computed.get_custom_property("--branch2"), None);
}

#[test]
fn test_adversarial_cycle_inside_fallback_when_primary_missing() {
    // --missing is not declared.
    // Fallback references --b which forms cycle with --a: --a: var(--missing, var(--b)); --b: var(--a);
    let html = r#"
        <div id="target" style="
            --a: var(--missing, var(--b));
            --b: var(--a);
            color: var(--a);
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
    assert_eq!(computed.get_custom_property("--a"), None);
    assert_eq!(computed.get_custom_property("--b"), None);
}

#[test]
fn test_adversarial_cycle_inside_fallback_when_primary_defined() {
    // --defined is declared. Fallback contains a cycle, but fallback is never taken.
    let html = r#"
        <div id="target" style="
            --defined: #00ff00;
            --cycle1: var(--cycle2);
            --cycle2: var(--cycle1);
            --a: var(--defined, var(--cycle1));
            color: var(--a);
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("#00ff00"));
    assert_eq!(computed.get_custom_property("--a"), Some("#00ff00"));
    assert_eq!(computed.get_custom_property("--cycle1"), None);
}

// =========================================================================
// 2. Deeply Nested `var()` Expressions
// =========================================================================

#[test]
fn test_adversarial_deeply_nested_var_fallbacks_50_levels() {
    // var(--u1, var(--u2, ... var(--u50, #abcdef)...))
    let mut expr = String::from("#abcdef");
    for i in (1..=50).rev() {
        expr = format!("var(--u{}, {})", i, expr);
    }

    let html = format!(r#"<div id="target" style="color: {};"></div>"#, expr);
    let doc = parse_html(&html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("#abcdef"));
}

#[test]
fn test_adversarial_deeply_nested_var_chain_50_levels() {
    // --v1: 10px; --v2: var(--v1); --v3: var(--v2); ... --v50: var(--v49);
    let mut style_str = String::from("--v1: 10px; ");
    for i in 2..=50 {
        style_str.push_str(&format!("--v{}: var(--v{}); ", i, i - 1));
    }
    style_str.push_str("margin-top: var(--v50);");

    let html = format!(r#"<div id="target" style="{}"></div>"#, style_str);
    let doc = parse_html(&html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("margin-top"), Some("10px"));
    assert_eq!(computed.get_custom_property("--v50"), Some("10px"));
}

// =========================================================================
// 3. Comma-Containing Values Inside Fallbacks
// =========================================================================

#[test]
fn test_adversarial_comma_containing_fallbacks_font_family() {
    let html = r#"
        <div id="target" style="
            font-family: var(--missing-font, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif);
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(
        computed.get_property_value("font-family"),
        Some("-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif")
    );
}

#[test]
fn test_adversarial_comma_containing_fallbacks_box_shadow() {
    let html = r#"
        <div id="target" style="
            box-shadow: var(--shadow-elevation, 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05));
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(
        computed.get_property_value("box-shadow"),
        Some("0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)")
    );
}

#[test]
fn test_adversarial_comma_containing_fallbacks_gradients() {
    let html = r#"
        <div id="target" style="
            background: var(--bg-gradient, linear-gradient(90deg, #ff0000 0%, #00ff00 50%, #0000ff 100%));
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(
        computed.get_property_value("background"),
        Some("linear-gradient(90deg, #ff0000 0%, #00ff00 50%, #0000ff 100%)")
    );
}

#[test]
fn test_adversarial_comma_in_custom_property_value() {
    let html = r#"
        <div id="target" style="
            --font-stack: 'Fira Code', 'Cascadia Code', Consolas, monospace;
            font-family: var(--font-stack);
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(
        computed.get_property_value("font-family"),
        Some("'Fira Code', 'Cascadia Code', Consolas, monospace")
    );
}

// =========================================================================
// 4. Case Sensitivity: Custom Properties vs Standard Properties
// =========================================================================

#[test]
fn test_adversarial_case_sensitivity_custom_vs_standard() {
    let html = r#"
        <div id="target" style="
            --testVar: 10px;
            --testvar: 20px;
            --TESTVAR: 30px;
            --TestVar: 40px;
            width: var(--testVar);
            height: var(--testvar);
            margin: var(--TESTVAR);
            padding: var(--TestVar);
            COLOR: RED;
            DISPLAY: FLEX;
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);

    // Custom properties: strict case-sensitivity
    assert_eq!(computed.get_property_value("width"), Some("10px"));
    assert_eq!(computed.get_property_value("height"), Some("20px"));
    assert_eq!(computed.get_property_value("margin"), Some("30px"));
    assert_eq!(computed.get_property_value("padding"), Some("40px"));
    assert_eq!(computed.get_custom_property("--testVar"), Some("10px"));
    assert_eq!(computed.get_custom_property("--testvar"), Some("20px"));
    assert_eq!(computed.get_custom_property("--TESTVAR"), Some("30px"));
    assert_eq!(computed.get_custom_property("--TestVar"), Some("40px"));
    assert_eq!(computed.get_custom_property("--tEsTvAr"), None);

    // Standard properties: case-insensitive query and storage
    assert_eq!(computed.get_property_value("color"), Some("RED"));
    assert_eq!(computed.get_property_value("COLOR"), Some("RED"));
    assert_eq!(computed.get_property_value("Color"), Some("RED"));
    assert_eq!(computed.get_property_value("display"), Some("FLEX"));
    assert_eq!(computed.get_property_value("DISPLAY"), Some("FLEX"));
}

#[test]
fn test_adversarial_case_insensitive_keywords() {
    let html = r#"
        <div id="parent" style="color: blue; display: block;">
            <div id="child" style="color: INHERIT; display: INITIAL;"></div>
        </div>
    "#;
    let doc = parse_html(html);
    let child_id = doc.get_element_by_id("child").expect("child element");

    let computed = StyleResolver::resolve_element_style(&doc, child_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("blue"));
    assert_eq!(computed.get_property_value("display"), Some("inline"));
}

// =========================================================================
// 5. IACVT Property Resetting: Non-Inheritable vs Inheritable
// =========================================================================

#[test]
fn test_adversarial_iacvt_hierarchy_resetting() {
    let html = r#"
        <div id="root" style="color: #ff0000; font-size: 32px; display: block; margin: 50px; opacity: 0.5;">
            <div id="parent" style="
                --loop: var(--loop);
                color: var(--loop);
                font-size: var(--loop);
                display: var(--loop);
                margin: var(--loop);
                opacity: var(--loop);
            ">
                <div id="child">
                    <span id="grandchild" style="color: inherit; display: inherit; margin: unset;"></span>
                </div>
            </div>
        </div>
    "#;
    let doc = parse_html(html);
    let parent_id = doc.get_element_by_id("parent").expect("parent");
    let child_id = doc.get_element_by_id("child").expect("child");
    let grandchild_id = doc.get_element_by_id("grandchild").expect("grandchild");

    // Parent has IACVT on all 5 properties:
    // Inheritable (color, font-size) -> unset -> inherits from root (#ff0000, 32px)
    // Non-inheritable (display, margin, opacity) -> unset -> initial values ('inline', '0px', '1')
    let parent_computed = StyleResolver::resolve_element_style(&doc, parent_id, &[]);
    assert_eq!(parent_computed.get_property_value("color"), Some("#ff0000"));
    assert_eq!(parent_computed.get_property_value("font-size"), Some("32px"));
    assert_eq!(parent_computed.get_property_value("display"), Some("inline"));
    assert_eq!(parent_computed.get_property_value("margin"), Some("0px"));
    assert_eq!(parent_computed.get_property_value("opacity"), Some("1"));

    // Child declares nothing -> inherits inheritable from parent (#ff0000, 32px), non-inheritable are None
    let child_computed = StyleResolver::resolve_element_style(&doc, child_id, &[]);
    assert_eq!(child_computed.get_property_value("color"), Some("#ff0000"));
    assert_eq!(child_computed.get_property_value("font-size"), Some("32px"));
    assert_eq!(child_computed.get_property_value("display"), None);

    // Grandchild has explicit keywords:
    // color: inherit -> #ff0000
    // display: inherit -> parent's computed display is 'inline'
    // margin: unset -> margin is non-inheritable -> resets to initial '0px'
    let grandchild_computed = StyleResolver::resolve_element_style(&doc, grandchild_id, &[]);
    assert_eq!(grandchild_computed.get_property_value("color"), Some("#ff0000"));
    assert_eq!(grandchild_computed.get_property_value("display"), Some("inline"));
    assert_eq!(grandchild_computed.get_property_value("margin"), Some("0px"));
}

// =========================================================================
// 6. Multi-Var Poisoning & Corner Cases
// =========================================================================

#[test]
fn test_adversarial_multi_var_single_failure_poisons_property() {
    // If one variable in a multi-var declaration is cyclic/invalid, the entire property is IACVT
    let html = r#"
        <div id="target" style="
            --top: 10px;
            --bad: var(--bad);
            --bottom: 30px;
            --left: 40px;
            margin: var(--top) var(--bad) var(--bottom) var(--left);
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    // Margin is non-inheritable, so IACVT resets to '0px'
    assert_eq!(computed.get_property_value("margin"), Some("0px"));
}

#[test]
fn test_adversarial_empty_fallback() {
    // var(--missing, ) with empty fallback resolves to empty string
    // var(--missing) without fallback is invalid -> IACVT
    let html = r#"
        <div id="target" style="
            --empty: var(--non-existent, );
            --bad: var(--non-existent);
            content: var(--empty);
            display: var(--bad);
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_custom_property("--empty"), Some(""));
    assert_eq!(computed.get_custom_property("--bad"), None);
    assert_eq!(computed.get_property_value("content"), Some(""));
    assert_eq!(computed.get_property_value("display"), Some("inline"));
}

#[test]
fn test_adversarial_quotes_and_semicolons_in_declarations() {
    let html = r#"
        <div id="target" style="
            content: 'quoted; string; with; semicolons; !important';
            color: green;
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(
        computed.get_property_value("content"),
        Some("'quoted; string; with; semicolons; !important'")
    );
    assert_eq!(computed.get_property_value("color"), Some("green"));
}

#[test]
fn test_adversarial_is_inheritable_and_initial_values_coverage() {
    // Ensure all critical properties are registered in is_inheritable_property and initial_value_for_property
    assert!(is_inheritable_property("color"));
    assert!(is_inheritable_property("font-family"));
    assert!(is_inheritable_property("font-size"));
    assert!(is_inheritable_property("line-height"));
    assert!(is_inheritable_property("visibility"));
    assert!(is_inheritable_property("direction"));
    assert!(is_inheritable_property("cursor"));
    assert!(is_inheritable_property("--custom-var"));

    assert!(!is_inheritable_property("display"));
    assert!(!is_inheritable_property("margin"));
    assert!(!is_inheritable_property("padding"));
    assert!(!is_inheritable_property("border"));
    assert!(!is_inheritable_property("background-color"));
    assert!(!is_inheritable_property("opacity"));
    assert!(!is_inheritable_property("transform"));

    assert_eq!(initial_value_for_property("color"), "black");
    assert_eq!(initial_value_for_property("display"), "inline");
    assert_eq!(initial_value_for_property("margin"), "0px");
    assert_eq!(initial_value_for_property("visibility"), "visible");
    assert_eq!(initial_value_for_property("opacity"), "1");
}
