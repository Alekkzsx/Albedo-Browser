use ace_core::features::{Feature, RuntimeFeatures};

#[test]
fn test_runtime_features_toggling() {
    let registry = RuntimeFeatures::new();
    assert!(!registry.is_enabled(Feature::CssGrid));

    registry.enable(Feature::CssGrid);
    assert!(registry.is_enabled(Feature::CssGrid));

    registry.disable(Feature::CssGrid);
    assert!(!registry.is_enabled(Feature::CssGrid));

    registry.set(Feature::CssSubgrid, true);
    assert!(registry.is_enabled(Feature::CssSubgrid));

    registry.reset_defaults();
    assert!(registry.is_enabled(Feature::CssGrid));
    assert!(registry.is_enabled(Feature::WebAssembly));
    assert!(!registry.is_enabled(Feature::CssSubgrid)); // Subgrid não é default
}

#[test]
fn test_global_runtime_features() {
    assert!(RuntimeFeatures::is_feature_enabled(Feature::CssFlexbox));
    assert!(RuntimeFeatures::is_feature_enabled(Feature::FetchApi));
}
