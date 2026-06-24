use super::*;
// ARQUIVO: src/graphics/gpu.rs

use std::borrow::Cow;
use wgpu::util::DeviceExt;

// Representa 1 retângulo HTML para ser instanciado na GPU


pub(crate) fn slice_as_bytes<T>(slice: &[T]) -> &[u8] {
    // SAFETY: slice is a valid reference to a contiguous memory region.
    // The pointer and length are derived from a valid slice.
    // size_of_val gives the correct byte length for the slice.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}
