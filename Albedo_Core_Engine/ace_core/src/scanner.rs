// ============================================================================
// Albedo Core Engine (ACE)
// File: scanner.rs
// Description: Scanner Léxico Vetorizado nativo.
//              Opera buscando caracteres 8 vezes mais rápido agrupando em u64.
// Author: Albedo Browser Engineering Team
// ============================================================================

use std::mem;

const USIZE_BYTES: usize = mem::size_of::<usize>();

/// Constante mágica para o truque Bitwise SIMD de encontrar bytes nulos/alvos.
/// Replica o byte `0x01` para todas as 8 posições.
const LSB_MASK: usize = usize::from_ne_bytes([0x01; 8]);
/// Replica o byte `0x80` para todas as 8 posições.
const MSB_MASK: usize = usize::from_ne_bytes([0x80; 8]);

/// Busca linearmente um byte (needle) num slice, lendo de 8 em 8 bytes 
/// sempre que possível (64-bits). Em computadores 32-bits, opera de 4 em 4 bytes.
#[inline]
pub fn find_byte_fast(haystack: &[u8], needle: u8) -> Option<usize> {
    let len = haystack.len();
    let mut ptr = haystack.as_ptr();
    let mut offset = 0;

    // A máscara que queremos buscar replicada para preencher um `usize`
    let needle_mask = usize::from_ne_bytes([needle; 8]);

    // Lê de bloco em bloco
    while offset + USIZE_BYTES <= len {
        unsafe {
            // Lemos 8 bytes da memória ignorando alinhamento estrito
            let chunk = ptr.add(offset).cast::<usize>().read_unaligned();
            
            // Fazemos um XOR. Onde o chunk for igual ao needle, o resultado do XOR será 0x00.
            let xored = chunk ^ needle_mask;
            
            // Truque clássico de Bitwise (O mesmo usado no glibc memchr):
            // Subtrair 0x01... faz o bit mais significativo (MSB) virar 1 se o byte original era 0.
            let has_zero_byte = (xored.wrapping_sub(LSB_MASK)) & !xored & MSB_MASK;

            if has_zero_byte != 0 {
                // Encontramos! Agora fazemos a busca exata nesses 8 bytes
                for i in 0..USIZE_BYTES {
                    if *ptr.add(offset + i) == needle {
                        return Some(offset + i);
                    }
                }
            }
        }
        offset += USIZE_BYTES;
    }

    // Processa os bytes restantes que não couberam em um bloco inteiro
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
