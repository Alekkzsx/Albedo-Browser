use ace_core::events::{KeyLocation, ModifiersState, PointerButton, PointerButtons, PointerType};

#[test]
fn test_pointer_buttons_bitmask() {
    let mut buttons = PointerButtons::PRIMARY;
    assert!(buttons.has_primary());
    assert!(!buttons.has_secondary());

    buttons.0 |= PointerButtons::SECONDARY.0;
    assert!(buttons.has_primary());
    assert!(buttons.has_secondary());
    assert!(!buttons.has_auxiliary());
}

#[test]
fn test_modifiers_state() {
    let mut mods = ModifiersState::default();
    assert!(mods.is_empty());

    mods.ctrl = true;
    mods.shift = true;
    assert!(!mods.is_empty());
    assert!(mods.ctrl);
    assert!(mods.shift);
    assert!(!mods.alt);
}

#[test]
fn test_pointer_types_and_w3c_indices() {
    assert_eq!(PointerType::default(), PointerType::Mouse);
    assert_eq!(PointerButton::Primary.to_w3c_index(), 0);
    assert_eq!(PointerButton::Auxiliary.to_w3c_index(), 1);
    assert_eq!(PointerButton::Secondary.to_w3c_index(), 2);
    assert_eq!(KeyLocation::default(), KeyLocation::Standard);
}
