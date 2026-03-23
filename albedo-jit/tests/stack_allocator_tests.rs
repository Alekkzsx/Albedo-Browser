//! Testes de Stack Allocator para AlbedoJIT

use albedo_jit::compiler::stack_allocator::StackAllocator;
use albedo_jit::bytecode::AirReg;

#[test]
fn test_allocate_object() {
    let mut allocator = StackAllocator::new();
    let alloc = allocator.allocate_object(AirReg(0), 3);

    assert_eq!(alloc.num_properties, 3);
    assert_eq!(alloc.size, 24); // 3 * 8 bytes
    assert_eq!(alloc.offset, 0);
    assert_eq!(allocator.get_frame_size(), 24);
}

#[test]
fn test_multiple_allocations() {
    let mut allocator = StackAllocator::new();
    let alloc1 = allocator.allocate_object(AirReg(0), 2);
    let alloc2 = allocator.allocate_object(AirReg(1), 3);

    assert_eq!(alloc1.offset, 0);
    assert_eq!(alloc1.size, 16); // 2 * 8

    assert_eq!(alloc2.offset, 16); // after alloc1
    assert_eq!(alloc2.size, 24); // 3 * 8

    assert_eq!(allocator.get_frame_size(), 40);
}

#[test]
fn test_get_property_offset() {
    let mut allocator = StackAllocator::new();
    allocator.allocate_object(AirReg(0), 5);

    assert_eq!(allocator.get_property_offset(AirReg(0), 0), Some(0));
    assert_eq!(allocator.get_property_offset(AirReg(0), 1), Some(8));
    assert_eq!(allocator.get_property_offset(AirReg(0), 2), Some(16));
    assert_eq!(allocator.get_property_offset(AirReg(0), 3), Some(24));
    assert_eq!(allocator.get_property_offset(AirReg(0), 4), Some(32));
}

#[test]
fn test_alignment() {
    let mut allocator = StackAllocator::new();
    // Alocar objeto com 1 propriedade (8 bytes)
    let alloc1 = allocator.allocate_object(AirReg(0), 1);
    // Alocar objeto com 2 propriedades (16 bytes)
    let alloc2 = allocator.allocate_object(AirReg(1), 2);

    // O segundo deve estar alinhado a 8 bytes
    assert_eq!(alloc1.size, 8);
    assert_eq!(alloc2.size, 16);
    assert_eq!(alloc2.offset, 8); // after alloc1
}
