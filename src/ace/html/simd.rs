//! SIMD Optimizations for ACE-HTML Parser
//! 
//! High-performance optimizations using SIMD instructions (SSE2, AVX2)
//! for whitespace detection, tag scanning, and entity lookup.
//! 
//! # Features
//! - `simd_is_whitespace_sse2()`: 16-byte parallel whitespace check
//! - `fast_ascii_tag_scan()`: 32-byte tag name processing
//! - `simd_find_byte()`: Fast byte search in buffers
//! - Entity lookup with perfect hashing

#![allow(dead_code)]

use std::arch::x86_64::*;

/// SIMD-accelerated whitespace detection using SSE2
/// Processes 16 bytes in parallel
/// 
/// # Arguments
/// * `data` - Input slice (must be at least 16 bytes)
/// 
/// # Returns
/// Bitmask where each bit represents if corresponding byte is whitespace
/// 
/// # Safety
/// Requires SSE2 support (available on all x86_64 since 2005)
#[inline(always)]
#[target_feature(enable = "sse2")]
pub unsafe fn simd_is_whitespace_sse2(data: &[u8]) -> u16 {
    debug_assert!(data.len() >= 16);
    
    // Load 16 bytes into SIMD register
    let chunk = _mm_loadu_si128(data.as_ptr() as *const __m128i);
    
    // Whitespace characters: 0x09 (tab), 0x0A (LF), 0x0C (FF), 0x0D (CR), 0x20 (space)
    let tab = _mm_set1_epi8(0x09);
    let lf = _mm_set1_epi8(0x0A);
    let ff = _mm_set1_epi8(0x0C);
    let cr = _mm_set1_epi8(0x0D);
    let space = _mm_set1_epi8(0x20);
    
    // Compare with each whitespace character
    let is_tab = _mm_cmpeq_epi8(chunk, tab);
    let is_lf = _mm_cmpeq_epi8(chunk, lf);
    let is_ff = _mm_cmpeq_epi8(chunk, ff);
    let is_cr = _mm_cmpeq_epi8(chunk, cr);
    let is_space = _mm_cmpeq_epi8(chunk, space);
    
    // Combine results with OR
    let mut result = _mm_or_si128(is_tab, is_lf);
    result = _mm_or_si128(result, is_ff);
    result = _mm_or_si128(result, is_cr);
    result = _mm_or_si128(result, is_space);
    
    // Convert to bitmask
    _mm_movemask_epi8(result) as u16
}

/// Fast ASCII tag name scanner using AVX2
/// Processes 32 bytes in parallel to find tag boundaries
/// 
/// # Arguments
/// * `data` - Input slice
/// * `start` - Starting position
/// 
/// # Returns
/// Position of first non-tag-name character (> or whitespace)
#[inline(always)]
#[target_feature(enable = "avx2")]
pub unsafe fn fast_ascii_tag_scan_avx2(data: &[u8], start: usize) -> usize {
    if data.len() - start < 32 {
        return slow_tag_scan(data, start);
    }
    
    // Valid tag name chars: a-z, A-Z, 0-9, hyphen, underscore, colon
    // We look for invalid chars: anything < '0' except '-', or > 'z'
    let chunk = _mm256_loadu_si256(data[start..].as_ptr() as *const __m256i);
    
    // Check for '>' which ends tag
    let gt = _mm256_set1_epi8(b'>');
    let is_gt = _mm256_cmpeq_epi8(chunk, gt);
    
    // Check for whitespace (ends attributes)
    let space = _mm256_set1_epi8(b' ');
    let is_space = _mm256_cmpeq_epi8(chunk, space);
    
    // Combine
    let terminator = _mm256_or_si256(is_gt, is_space);
    let mask = _mm256_movemask_epi8(terminator) as u32;
    
    if mask != 0 {
        start + mask.trailing_zeros() as usize
    } else {
        start + 32
    }
}

/// Fallback scalar tag scanner
fn slow_tag_scan(data: &[u8], start: usize) -> usize {
    for i in start..data.len() {
        let c = data[i];
        // Tag name can contain: a-z, A-Z, 0-9, -, _, :
        if c == b'>' || c <= b' ' {
            return i;
        }
    }
    data.len()
}

/// SIMD-accelerated byte finder
/// Finds the first occurrence of a byte in a buffer
/// 
/// # Arguments
/// * `data` - Input slice
/// * `byte` - Byte to search for
/// 
/// # Returns
/// Index of first occurrence, or None if not found
#[inline]
pub fn simd_find_byte(data: &[u8], byte: u8) -> Option<usize> {
    if data.len() >= 16 && is_x86_feature_detected!("sse2") {
        unsafe { simd_find_byte_sse2(data, byte) }
    } else {
        data.iter().position(|&b| b == byte)
    }
}

#[inline(always)]
#[target_feature(enable = "sse2")]
unsafe fn simd_find_byte_sse2(data: &[u8], byte: u8) -> Option<usize> {
    let needle = _mm_set1_epi8(byte as i8);
    
    for i in (0..data.len()).step_by(16) {
        if i + 16 > data.len() {
            break;
        }
        
        let chunk = _mm_loadu_si128(data[i..].as_ptr() as *const __m128i);
        let cmp = _mm_cmpeq_epi8(chunk, needle);
        let mask = _mm_movemask_epi8(cmp) as u16;
        
        if mask != 0 {
            return Some(i + mask.trailing_zeros() as usize);
        }
    }
    
    // Handle remaining bytes
    for i in (data.len() - data.len() % 16)..data.len() {
        if data[i] == byte {
            return Some(i);
        }
    }
    
    None
}

/// Fast HTML entity lookup using perfect hashing
/// Optimized for common entities: &nbsp; &lt; &gt; &amp; &quot;
#[inline]
pub fn fast_entity_lookup(entity: &str) -> Option<char> {
    match entity {
        "nbsp" => Some('\u{00A0}'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "copy" => Some('\u{00A9}'),
        "reg" => Some('\u{00AE}'),
        "mdash" => Some('\u{2014}'),
        "ndash" => Some('\u{2013}'),
        "hellip" => Some('\u{2026}'),
        "laquo" => Some('\u{00AB}'),
        "raquo" => Some('\u{00BB}'),
        "bull" => Some('\u{2022}'),
        "trade" => Some('\u{2122}'),
        "euro" => Some('\u{20AC}'),
        "yen" => Some('\u{00A5}'),
        "pound" => Some('\u{00A3}'),
        "curren" => Some('\u{00A4}'),
        "sect" => Some('\u{00A7}'),
        "para" => Some('\u{00B6}'),
        "micro" => Some('\u{00B5}'),
        "deg" => Some('\u{00B0}'),
        "plusmn" => Some('\u{00B1}'),
        "sup1" => Some('\u{00B9}'),
        "sup2" => Some('\u{00B2}'),
        "sup3" => Some('\u{00B3}'),
        "frac14" => Some('\u{00BC}'),
        "frac12" => Some('\u{00BD}'),
        "frac34" => Some('\u{00BE}'),
        "times" => Some('\u{00D7}'),
        "divide" => Some('\u{00F7}'),
        _ => None,
    }
}

/// Numeric entity decoder (&#123; or &#x1A;)
#[inline]
pub fn decode_numeric_entity(entity: &str) -> Option<char> {
    if entity.starts_with('x') || entity.starts_with('X') {
        // Hexadecimal
        u32::from_str_radix(&entity[1..], 16)
            .ok()
            .and_then(|cp| char::from_u32(cp))
    } else {
        // Decimal
        entity.parse::<u32>()
            .ok()
            .and_then(|cp| char::from_u32(cp))
    }
}

/// Batch whitespace normalization
/// Converts all whitespace chars to spaces in-place
/// Uses SIMD when available
pub fn normalize_whitespace_simd(data: &mut [u8]) {
    if data.len() >= 16 && is_x86_feature_detected!("sse2") {
        unsafe { normalize_whitespace_sse2(data) }
    } else {
        for byte in data.iter_mut() {
            if matches!(*byte, 0x09 | 0x0A | 0x0C | 0x0D) {
                *byte = 0x20;
            }
        }
    }
}

#[target_feature(enable = "sse2")]
unsafe fn normalize_whitespace_sse2(data: &mut [u8]) {
    let tab = _mm_set1_epi8(0x09);
    let lf = _mm_set1_epi8(0x0A);
    let ff = _mm_set1_epi8(0x0C);
    let cr = _mm_set1_epi8(0x0D);
    let space = _mm_set1_epi8(0x20);
    
    for i in (0..data.len()).step_by(16) {
        if i + 16 > data.len() {
            break;
        }
        
        let ptr = data[i..].as_mut_ptr() as *mut __m128i;
        let mut chunk = _mm_loadu_si128(ptr);
        
        // Check for each whitespace type
        let is_tab = _mm_cmpeq_epi8(chunk, tab);
        let is_lf = _mm_cmpeq_epi8(chunk, lf);
        let is_ff = _mm_cmpeq_epi8(chunk, ff);
        let is_cr = _mm_cmpeq_epi8(chunk, cr);
        
        // Combine masks
        let mut mask = _mm_or_si128(is_tab, is_lf);
        mask = _mm_or_si128(mask, is_ff);
        mask = _mm_or_si128(mask, is_cr);
        
        // Blend with space
        chunk = _mm_blendv_epi8(chunk, space, mask);
        
        _mm_storeu_si128(ptr, chunk);
    }
    
    // Handle remaining bytes
    for i in (data.len() - data.len() % 16)..data.len() {
        if matches!(data[i], 0x09 | 0x0A | 0x0C | 0x0D) {
            data[i] = 0x20;
        }
    }
}

/// Runtime CPU feature detection wrapper
#[inline]
pub fn has_simd_support() -> bool {
    is_x86_feature_detected!("sse2")
}

/// Get SIMD optimization level description
pub fn get_optimization_level() -> &'static str {
    if is_x86_feature_detected!("avx2") {
        "AVX2 (32-byte parallel)"
    } else if is_x86_feature_detected!("sse2") {
        "SSE2 (16-byte parallel)"
    } else {
        "Scalar (no SIMD)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_entity_lookup() {
        assert_eq!(fast_entity_lookup("nbsp"), Some('\u{00A0}'));
        assert_eq!(fast_entity_lookup("lt"), Some('<'));
        assert_eq!(fast_entity_lookup("gt"), Some('>'));
        assert_eq!(fast_entity_lookup("amp"), Some('&'));
        assert_eq!(fast_entity_lookup("quot"), Some('"'));
        assert_eq!(fast_entity_lookup("invalid"), None);
    }

    #[test]
    fn test_decode_numeric_entity() {
        assert_eq!(decode_numeric_entity("65"), Some('A'));
        assert_eq!(decode_numeric_entity("x41"), Some('A'));
        assert_eq!(decode_numeric_entity("X41"), Some('A'));
        assert_eq!(decode_numeric_entity("128"), Some('\u{0080}'));
        assert_eq!(decode_numeric_entity("invalid"), None);
    }

    #[test]
    fn test_simd_find_byte() {
        let data = b"Hello, World!";
        assert_eq!(simd_find_byte(data, b','), Some(5));
        assert_eq!(simd_find_byte(data, b'W'), Some(7));
        assert_eq!(simd_find_byte(data, b'!'), Some(12));
        assert_eq!(simd_find_byte(data, b'Z'), None);
    }

    #[test]
    fn test_normalize_whitespace() {
        let mut data = b"Hello\tWorld\nTest\r\nEnd".to_vec();
        normalize_whitespace_simd(&mut data);
        assert_eq!(&data, b"Hello World Test  End");
    }

    #[test]
    fn test_optimization_level() {
        let level = get_optimization_level();
        println!("SIMD Optimization: {}", level);
        assert!(!level.is_empty());
    }
}
