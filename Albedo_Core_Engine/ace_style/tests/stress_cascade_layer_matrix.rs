//! # 🥊 Suíte 4: Matriz Completa de Origens da Cascata e Precedência de Camadas (@layer)
//!
//! Exercita:
//! - Precedência das 8 origens normativas da W3C
//! - Inversão de precedência de @layer para regras !important

use ace_core::intern::Atom;
use ace_dom::query::selector::Specificity;
use ace_style::cascade::layer_order::compare_layer_precedence;
use ace_style::cascade::origin::CascadeOrigin;
use ace_style::cascade::sorter::{CascadeDeclaration, CascadeSorter};
use ace_style::model::layer::LayerRegistry;
use smol_str::SmolStr;
use std::cmp::Ordering;

#[test]
fn test_stress_eight_normative_origins_matrix() {
    let origins = [
        CascadeOrigin::UserAgentNormal,
        CascadeOrigin::UserNormal,
        CascadeOrigin::AuthorNormal,
        CascadeOrigin::Animation,
        CascadeOrigin::AuthorImportant,
        CascadeOrigin::UserImportant,
        CascadeOrigin::UserAgentImportant,
        CascadeOrigin::Transition,
    ];

    // Valida que a ordem estrita é mantida: cada elemento subsequente vence os anteriores
    for i in 0..origins.len() {
        for j in 0..origins.len() {
            let decl_a = CascadeDeclaration {
                property: Atom::new("color"),
                value: SmolStr::new("red"),
                origin: origins[i],
                layer_order: None,
                specificity: Specificity::ZERO,
                sheet_index: 0,
                source_order: 0,
                is_inline: false,
            };

            let decl_b = CascadeDeclaration {
                property: Atom::new("color"),
                value: SmolStr::new("blue"),
                origin: origins[j],
                layer_order: None,
                specificity: Specificity::ZERO,
                sheet_index: 0,
                source_order: 0,
                is_inline: false,
            };

            let expected = i.cmp(&j);
            assert_eq!(decl_a.compare(&decl_b), expected, "Comparação entre {:?} e {:?}", origins[i], origins[j]);
        }
    }
}

#[test]
fn test_stress_layer_precedence_inversion_for_important() {
    let layer_base = Some(1);  // Primeira camada declarada
    let layer_theme = Some(2); // Segunda camada declarada

    // 1. Fluxo Normal: camada declarada mais tarde VENCE
    assert_eq!(
        compare_layer_precedence(layer_base, layer_theme, false),
        Ordering::Less
    );
    assert_eq!(
        compare_layer_precedence(layer_theme, layer_base, false),
        Ordering::Greater
    );

    // 2. Fluxo Normal: Unlayered (None) VENCE layered (Some)
    assert_eq!(
        compare_layer_precedence(None, layer_theme, false),
        Ordering::Greater
    );

    // 3. Fluxo !important: Ordem Invertida! Camada declarada ANTES VENCE!
    assert_eq!(
        compare_layer_precedence(layer_base, layer_theme, true),
        Ordering::Greater
    );
    assert_eq!(
        compare_layer_precedence(layer_theme, layer_base, true),
        Ordering::Less
    );

    // 4. Fluxo !important: Layered (Some) VENCE Unlayered (None)!
    assert_eq!(
        compare_layer_precedence(layer_base, None, true),
        Ordering::Greater
    );
    assert_eq!(
        compare_layer_precedence(None, layer_base, true),
        Ordering::Less
    );
}

#[test]
fn test_stress_full_layer_cascade_resolution() {
    let mut registry = LayerRegistry::new();
    let base_atom = Atom::new("base");
    let theme_atom = Atom::new("theme");

    registry.register_statement(&[base_atom.clone(), theme_atom.clone()]);
    let base_order = registry.get_layer_order(&base_atom);
    let theme_order = registry.get_layer_order(&theme_atom);

    assert!(base_order < theme_order);

    // Regra theme !important vs base !important: base vence!
    let mut decl_base_imp = CascadeDeclaration {
        property: Atom::new("margin"),
        value: SmolStr::new("10px"),
        origin: CascadeOrigin::AuthorImportant,
        layer_order: base_order,
        specificity: Specificity::ZERO,
        sheet_index: 0,
        source_order: 1,
        is_inline: false,
    };

    let decl_theme_imp = CascadeDeclaration {
        property: Atom::new("margin"),
        value: SmolStr::new("20px"),
        origin: CascadeOrigin::AuthorImportant,
        layer_order: theme_order,
        specificity: Specificity::new(0, 1, 0), // Maior especificidade, mas perde para precedência de layer em !important
        sheet_index: 0,
        source_order: 2,
        is_inline: false,
    };

    assert_eq!(decl_base_imp.compare(&decl_theme_imp), Ordering::Greater);
}
