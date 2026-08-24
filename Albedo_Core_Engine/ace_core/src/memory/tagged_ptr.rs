//! # Ponteiros Etiquetados com Exploração de Nicho (TaggedPointer Pattern)
//!
//! Em arquiteturas de 64-bit (x86-64 e AArch64), alocações alinhadas a 8 bytes garantem que os
//! 3 bits menos significativos do endereço sejam sempre zeros (`0b000`).
//!
//! O `TaggedPointer<T>` armazena uma tag de metadados de 3 bits (`0..=7`) diretamente dentro do ponteiro,
//! eliminando o overhead de campos de discriminante ou enums adicionais em estruturas compactas da árvore DOM.

use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Tag de metadados compacta de 3 bits para nós e objetos da engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NodeTag {
    Element = 0b000,
    Text = 0b001,
    Comment = 0b010,
    Document = 0b011,
    DirtyLayout = 0b100,
    DirtyStyle = 0b101,
    CustomA = 0b110,
    CustomB = 0b111,
}

/// Ponteiro que embute uma tag de 3 bits nos bits inferiores de alinhamento de 8 bytes.
pub struct TaggedPointer<T> {
    bits: usize,
    _marker: PhantomData<*mut T>,
}

unsafe impl<T: Send> Send for TaggedPointer<T> {}
unsafe impl<T: Sync> Sync for TaggedPointer<T> {}

impl<T> Clone for TaggedPointer<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for TaggedPointer<T> {}

impl<T> TaggedPointer<T> {
    const TAG_MASK: usize = 0b111;
    const PTR_MASK: usize = !Self::TAG_MASK;

    /// Cria um novo `TaggedPointer`.
    ///
    /// # Panics
    /// Dispara pânico em debug se o ponteiro não for alinhado a 8 bytes.
    pub fn new(ptr: NonNull<T>, tag: NodeTag) -> Self {
        let addr = ptr.as_ptr() as usize;
        debug_assert_eq!(
            addr & Self::TAG_MASK,
            0,
            "Ponteiro deve ser estritamente alinhado a 8 bytes para TaggedPointer"
        );
        Self {
            bits: (addr & Self::PTR_MASK) | (tag as usize),
            _marker: PhantomData,
        }
    }

    /// Retorna o ponteiro limpo (sem os bits de tag).
    #[inline(always)]
    pub fn ptr(&self) -> NonNull<T> {
        let clean_addr = self.bits & Self::PTR_MASK;
        unsafe { NonNull::new_unchecked(clean_addr as *mut T) }
    }

    /// Retorna a tag de 3 bits embutida no ponteiro.
    #[inline(always)]
    pub fn tag(&self) -> u8 {
        (self.bits & Self::TAG_MASK) as u8
    }

    /// Atualiza a tag embutida preservando o endereço base do ponteiro.
    #[inline(always)]
    pub fn set_tag(&mut self, tag: NodeTag) {
        self.bits = (self.bits & Self::PTR_MASK) | (tag as usize);
    }
}

impl<T> fmt::Debug for TaggedPointer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaggedPointer")
            .field("ptr", &self.ptr())
            .field("tag", &self.tag())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tagged_pointer_ops() {
        #[repr(align(8))]
        struct AlignedNode(u64);

        let mut node = AlignedNode(42);
        let non_null = NonNull::new(&mut node as *mut AlignedNode).unwrap();

        let mut tagged = TaggedPointer::new(non_null, NodeTag::Text);
        assert_eq!(tagged.tag(), NodeTag::Text as u8);
        assert_eq!(unsafe { tagged.ptr().as_ref().0 }, 42);

        tagged.set_tag(NodeTag::DirtyLayout);
        assert_eq!(tagged.tag(), NodeTag::DirtyLayout as u8);
        assert_eq!(unsafe { tagged.ptr().as_ref().0 }, 42);
    }
}
