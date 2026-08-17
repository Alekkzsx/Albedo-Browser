use ace_core::text::font::{FontDescriptor, FontStretch, FontStyle, FontWeight, GenericFontFamily};

#[test]
fn test_generic_font_family_keywords() {
    assert_eq!(
        GenericFontFamily::from_css_keyword("sans-serif"),
        Some(GenericFontFamily::SansSerif)
    );
    assert_eq!(
        GenericFontFamily::from_css_keyword("monospace"),
        Some(GenericFontFamily::Monospace)
    );
    assert_eq!(
        GenericFontFamily::from_css_keyword("system-ui"),
        Some(GenericFontFamily::SystemUi)
    );
    assert_eq!(GenericFontFamily::from_css_keyword("custom-font"), None);
}

#[test]
fn test_font_weight_parsing() {
    assert_eq!(
        FontWeight::from_css_value("normal"),
        Some(FontWeight::NORMAL)
    );
    assert_eq!(FontWeight::from_css_value("bold"), Some(FontWeight::BOLD));
    assert_eq!(FontWeight::from_css_value("700"), Some(FontWeight(700)));
    assert_eq!(FontWeight::from_css_value("950"), Some(FontWeight(950)));

    assert!(FontWeight::BOLD.is_bold());
    assert!(!FontWeight::NORMAL.is_bold());
}

#[test]
fn test_font_stretch_parsing() {
    assert_eq!(
        FontStretch::from_css_value("condensed"),
        Some(FontStretch::Condensed)
    );
    assert_eq!(
        FontStretch::from_css_value("expanded"),
        Some(FontStretch::Expanded)
    );
    assert_eq!(
        FontStretch::from_css_value("normal"),
        Some(FontStretch::Normal)
    );
}

#[test]
fn test_font_descriptor_creation() {
    let desc = FontDescriptor::new("sans-serif", 16.0);
    assert_eq!(desc.family, "sans-serif");
    assert_eq!(desc.generic_family, Some(GenericFontFamily::SansSerif));
    assert_eq!(desc.weight, FontWeight::NORMAL);
    assert_eq!(desc.style, FontStyle::Normal);
    assert_eq!(desc.stretch, FontStretch::Normal);
    assert_eq!(desc.size_px, 16.0);
}
