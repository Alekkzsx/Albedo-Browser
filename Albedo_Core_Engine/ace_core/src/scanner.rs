// ============================================================================
// Albedo Core Engine (ACE)
// File: scanner.rs
// Description: Scanner Léxico Vetorizado Nativo (Hardware SIMD).
//              Busca caracteres 32 vezes mais rápido agrupando via registradores YMM (AVX2).
//              Fallback transparente para 64-bits bitwise em arquiteturas sem AVX2.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::mem;

const USIZE_BYTES: usize = mem::size_of::<usize>();

/// Constante mágica para o truque Bitwise de encontrar bytes nulos.
const LSB_MASK: usize = usize::from_ne_bytes([0x01; 8]);
const MSB_MASK: usize = usize::from_ne_bytes([0x80; 8]);

/// Busca SIMD acelerada por hardware (AVX2).
/// Processa 32 bytes por ciclo de clock usando registradores de 256 bits.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn find_byte_avx2(haystack: &[u8], needle: u8) -> Option<usize> {
    use std::arch::x86_64::*;
    let len = haystack.len();
    let ptr = haystack.as_ptr();
    let mut offset = 0;

    // Replica o byte alvo para preencher o registrador YMM (32 bytes)
    let needle_mask = _mm256_set1_epi8(needle as i8);

    // Enquanto sobrar 32 bytes para ler...
    while offset + 32 <= len {
        // Lê 32 bytes sem restrição de alinhamento
        let chunk = _mm256_loadu_si256(ptr.add(offset).cast::<__m256i>());
        // Compara (CMPEQ) cada um dos 32 bytes simultaneamente. Se igual, o byte vira 0xFF.
        let eq = _mm256_cmpeq_epi8(chunk, needle_mask);
        // Extrai o bit mais significativo de cada byte para formar um inteiro de 32 bits
        let mask = _mm256_movemask_epi8(eq);

        if mask != 0 {
            // `trailing_zeros` magicamente nos dá o índice do primeiro bit '1'
            return Some(offset + mask.trailing_zeros() as usize);
        }
        offset += 32;
    }

    // Fallback para bytes restantes (cauda)
    while offset < len {
        if *ptr.add(offset) == needle {
            return Some(offset);
        }
        offset += 1;
    }
    None
}

/// Fallback escalar (Bitwise SIMD em u64)
#[inline]
fn find_byte_fallback(haystack: &[u8], needle: u8) -> Option<usize> {
    let len = haystack.len();
    let ptr = haystack.as_ptr();
    let mut offset = 0;

    let needle_mask = usize::from_ne_bytes([needle; 8]);

    while offset + USIZE_BYTES <= len {
        unsafe {
            let chunk = ptr.add(offset).cast::<usize>().read_unaligned();
            let xored = chunk ^ needle_mask;
            let has_zero_byte = (xored.wrapping_sub(LSB_MASK)) & !xored & MSB_MASK;

            if has_zero_byte != 0 {
                for i in 0..USIZE_BYTES {
                    if *ptr.add(offset + i) == needle {
                        return Some(offset + i);
                    }
                }
            }
        }
        offset += USIZE_BYTES;
    }

    while offset < len {
        unsafe {
            if *ptr.add(offset) == needle {
                return Some(offset);
            }
        }
        offset += 1;
    }

    None
}

/// API pública para busca vetorial. Roteia automaticamente para a instrução
/// mais avançada que a CPU do usuário suportar no tempo de execução.
#[inline]
pub fn find_byte_fast(haystack: &[u8], needle: u8) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // SAFETY: Já verificamos que a CPU suporta a instrução AVX2
            return unsafe { find_byte_avx2(haystack, needle) };
        }
    }
    find_byte_fallback(haystack, needle)
}
