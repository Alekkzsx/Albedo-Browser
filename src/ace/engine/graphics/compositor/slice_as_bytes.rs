use super::*;
/// Um Pool de Texturas dinâmicas para reciclar Tiles já rasterizados na VRAM.


pub(crate) fn slice_as_bytes<T>(slice: &[T]) -> &[u8] {
    // SAFETY: slice is a valid reference to a contiguous memory region.
    // The pointer and length are derived from a valid slice.
    // size_of_val gives the correct byte length for the slice.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}
