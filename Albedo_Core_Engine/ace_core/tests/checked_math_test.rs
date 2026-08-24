use ace_core::math::{Checked, CheckedSize};

#[test]
fn test_checked_arithmetic_sticky_error() {
    let a = Checked::new(100u32);
    let b = Checked::new(50u32);
    let c = a + b * 2u32;
    assert_eq!(c.value(), Some(200));
    assert!(c.is_valid());

    // Overflow na multiplicação
    let max = Checked::new(u32::MAX);
    let overflow = max * 2u32 + 10u32;
    assert!(!overflow.is_valid());
    assert_eq!(overflow.value(), None);
    assert_eq!(overflow.value_or(0), 0);

    // Divisão por zero
    let div_zero = Checked::new(100u32) / 0u32;
    assert!(!div_zero.is_valid());
}

#[test]
fn test_checked_size_and_cast() {
    let size = CheckedSize::new(1024);
    let doubled = size * 2usize;
    assert_eq!(doubled.value_or_max(), 2048);

    // Cast com overflow
    let large_val = Checked::new(1000u32);
    let byte_val: Checked<u8> = large_val.cast();
    assert!(!byte_val.is_valid());

    // Cast válido
    let small_val = Checked::new(120u32);
    let byte_val2: Checked<u8> = small_val.cast();
    assert_eq!(byte_val2.value(), Some(120u8));
}
