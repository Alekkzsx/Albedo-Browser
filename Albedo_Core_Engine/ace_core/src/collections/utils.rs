//! # Utilitários de Manipulação de Bits e Coleções
//!
//! Funções auxiliares para contagem de bits, alinhamento de índices e particionamento de coleções.

/// Retorna a menor potência de 2 maior ou igual a `n`.
#[inline]
pub const fn next_power_of_two(n: usize) -> usize {
    if n <= 1 {
        1
    } else {
        1 << (usize::BITS - (n - 1).leading_zeros())
    }
}

/// Retorna o número de bits ativos (1s) em um valor `u64` (Population Count).
#[inline]
pub const fn popcount_u64(val: u64) -> u32 {
    val.count_ones()
}

/// Retorna o número de zeros à direita (menor bit ativo) em um `u64`.
#[inline]
pub const fn trailing_zeros_u64(val: u64) -> u32 {
    val.trailing_zeros()
}

/// Retorna o número de zeros à esquerda em um `u64`.
#[inline]
pub const fn leading_zeros_u64(val: u64) -> u32 {
    val.leading_zeros()
}

/// Retorna `true` se o valor possuir exatamente 1 bit ativo (é potência de 2).
#[inline]
pub const fn has_single_bit(val: u64) -> bool {
    val != 0 && (val & (val - 1)) == 0
}

/// Divide uma fatia (`slice`) em pedaços contíguos de tamanho máximo `chunk_size`.
#[inline]
pub fn chunk_slice<T>(slice: &[T], chunk_size: usize) -> impl Iterator<Item = &[T]> {
    slice.chunks(chunk_size)
}
