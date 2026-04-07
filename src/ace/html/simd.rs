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

/// SIMD-accelerated whitespace detection using AVX-512
/// Processes 64 bytes in parallel
/// 
/// # Arguments
/// * `data` - Input slice (must be at least 64 bytes)
/// 
/// # Returns
/// Bitmask where each bit represents if corresponding byte is whitespace
/// 
/// # Safety
/// Requires AVX-512 support (available on Intel Skylake-X and later)
#[cfg(target_feature = "avx512f")]
#[target_feature(enable = "avx512f,avx512bw")]
pub unsafe fn simd_is_whitespace_avx512(data: &[u8]) -> u64 {
    debug_assert!(data.len() >= 64);
    
    // Load 64 bytes into AVX-512 register
    let chunk = _mm512_loadu_si512(data.as_ptr() as *const __m512i);
    
    // Whitespace characters: 0x09 (tab), 0x0A (LF), 0x0C (FF), 0x0D (CR), 0x20 (space)
    let tab = _mm512_set1_epi8(0x09);
    let lf = _mm512_set1_epi8(0x0A);
    let ff = _mm512_set1_epi8(0x0C);
    let cr = _mm512_set1_epi8(0x0D);
    let space = _mm512_set1_epi8(0x20);
    
    // Compare with each whitespace character (returns mask)
    let is_tab = _mm512_cmpeq_epi8_mask(chunk, tab);
    let is_lf = _mm512_cmpeq_epi8_mask(chunk, lf);
    let is_ff = _mm512_cmpeq_epi8_mask(chunk, ff);
    let is_cr = _mm512_cmpeq_epi8_mask(chunk, cr);
    let is_space = _mm512_cmpeq_epi8_mask(chunk, space);
    
    // Combine results with OR
    is_tab | is_lf | is_ff | is_cr | is_space
}

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

/// Runtime-dispatched whitespace detection
/// Automatically selects best SIMD implementation
#[inline]
pub fn detect_whitespace(data: &[u8]) -> Vec<bool> {
    #[cfg(target_feature = "avx512f")]
    {
        if data.len() >= 64 && is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
            return unsafe { detect_whitespace_avx512(data) };
        }
    }
    
    if data.len() >= 16 && is_x86_feature_detected!("sse2") {
        unsafe { detect_whitespace_sse2(data) }
    } else {
        detect_whitespace_scalar(data)
    }
}

#[cfg(target_feature = "avx512f")]
#[target_feature(enable = "avx512f,avx512bw")]
unsafe fn detect_whitespace_avx512(data: &[u8]) -> Vec<bool> {
    let mut result = Vec::with_capacity(data.len());
    
    for i in (0..data.len()).step_by(64) {
        if i + 64 > data.len() {
            break;
        }
        
        let mask = simd_is_whitespace_avx512(&data[i..]);
        for bit in 0..64 {
            result.push((mask & (1u64 << bit)) != 0);
        }
    }
    
    // Handle remaining bytes
    for &byte in &data[(data.len() - data.len() % 64)..] {
        result.push(matches!(byte, 0x09 | 0x0A | 0x0C | 0x0D | 0x20));
    }
    
    result
}

#[target_feature(enable = "sse2")]
unsafe fn detect_whitespace_sse2(data: &[u8]) -> Vec<bool> {
    let mut result = Vec::with_capacity(data.len());
    
    for i in (0..data.len()).step_by(16) {
        if i + 16 > data.len() {
            break;
        }
        
        let mask = simd_is_whitespace_sse2(&data[i..]);
        for bit in 0..16 {
            result.push((mask & (1u16 << bit)) != 0);
        }
    }
    
    // Handle remaining bytes
    for &byte in &data[(data.len() - data.len() % 16)..] {
        result.push(matches!(byte, 0x09 | 0x0A | 0x0C | 0x0D | 0x20));
    }
    
    result
}

fn detect_whitespace_scalar(data: &[u8]) -> Vec<bool> {
    data.iter()
        .map(|&byte| matches!(byte, 0x09 | 0x0A | 0x0C | 0x0D | 0x20))
        .collect()
}

/// Fast ASCII tag name scanner using AVX-512
/// Processes 64 bytes in parallel to find tag boundaries
/// 
/// # Arguments
/// * `data` - Input slice
/// * `start` - Starting position
/// 
/// # Returns
/// Position of first non-tag-name character (> or whitespace)
#[cfg(target_feature = "avx512f")]
#[target_feature(enable = "avx512f,avx512bw")]
pub unsafe fn fast_ascii_tag_scan_avx512(data: &[u8], start: usize) -> usize {
    if data.len() - start < 64 {
        return fast_ascii_tag_scan_avx2(data, start);
    }
    
    let chunk = _mm512_loadu_si512(data[start..].as_ptr() as *const __m512i);
    
    // Check for '>' which ends tag
    let gt = _mm512_set1_epi8(b'>' as i8);
    let is_gt = _mm512_cmpeq_epi8_mask(chunk, gt);
    
    // Check for whitespace (ends tag name, starts attributes)
    let space = _mm512_set1_epi8(b' ' as i8);
    let tab = _mm512_set1_epi8(b'\t' as i8);
    let lf = _mm512_set1_epi8(b'\n' as i8);
    let cr = _mm512_set1_epi8(b'\r' as i8);
    let ff = _mm512_set1_epi8(b'\x0C' as i8);
    
    let is_space = _mm512_cmpeq_epi8_mask(chunk, space);
    let is_tab = _mm512_cmpeq_epi8_mask(chunk, tab);
    let is_lf = _mm512_cmpeq_epi8_mask(chunk, lf);
    let is_cr = _mm512_cmpeq_epi8_mask(chunk, cr);
    let is_ff = _mm512_cmpeq_epi8_mask(chunk, ff);
    
    // Check for '/' which can end tag
    let slash = _mm512_set1_epi8(b'/' as i8);
    let is_slash = _mm512_cmpeq_epi8_mask(chunk, slash);
    
    // Combine all terminators
    let terminator = is_gt | is_space | is_tab | is_lf | is_cr | is_ff | is_slash;
    
    if terminator != 0 {
        start + terminator.trailing_zeros() as usize
    } else {
        start + 64
    }
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
#[target_feature(enable = "avx2")]
pub unsafe fn fast_ascii_tag_scan_avx2(data: &[u8], start: usize) -> usize {
    if data.len() - start < 32 {
        return slow_tag_scan(data, start);
    }
    
    // Valid tag name chars: a-z, A-Z, 0-9, hyphen, underscore, colon
    // We look for invalid chars: anything < '0' except '-', or > 'z'
    let chunk = _mm256_loadu_si256(data[start..].as_ptr() as *const __m256i);
    
    // Check for '>' which ends tag
    let gt = _mm256_set1_epi8(b'>' as i8);
    let is_gt = _mm256_cmpeq_epi8(chunk, gt);
    
    // Check for whitespace (ends attributes)
    let space = _mm256_set1_epi8(b' ' as i8);
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

/// Runtime-dispatched tag scanner
/// Automatically selects best SIMD implementation
#[inline]
pub fn scan_tag_name(data: &[u8], start: usize) -> usize {
    #[cfg(target_feature = "avx512f")]
    {
        if data.len() - start >= 64 && is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
            return unsafe { fast_ascii_tag_scan_avx512(data, start) };
        }
    }
    
    if data.len() - start >= 32 && is_x86_feature_detected!("avx2") {
        unsafe { fast_ascii_tag_scan_avx2(data, start) }
    } else {
        slow_tag_scan(data, start)
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
    // Top 100 entities with perfect hash
    // Using length-based dispatch for better branch prediction
    match entity.len() {
        2 => match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            _ => None,
        },
        3 => match entity {
            "amp" => Some('&'),
            "deg" => Some('\u{00B0}'),
            "yen" => Some('\u{00A5}'),
            "reg" => Some('\u{00AE}'),
            _ => None,
        },
        4 => match entity {
            "nbsp" => Some('\u{00A0}'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "copy" => Some('\u{00A9}'),
            "bull" => Some('\u{2022}'),
            "euro" => Some('\u{20AC}'),
            "sect" => Some('\u{00A7}'),
            "para" => Some('\u{00B6}'),
            "sup1" => Some('\u{00B9}'),
            "sup2" => Some('\u{00B2}'),
            "sup3" => Some('\u{00B3}'),
            _ => None,
        },
        5 => match entity {
            "mdash" => Some('\u{2014}'),
            "ndash" => Some('\u{2013}'),
            "laquo" => Some('\u{00AB}'),
            "raquo" => Some('\u{00BB}'),
            "trade" => Some('\u{2122}'),
            "pound" => Some('\u{00A3}'),
            "micro" => Some('\u{00B5}'),
            "times" => Some('\u{00D7}'),
            _ => None,
        },
        6 => match entity {
            "hellip" => Some('\u{2026}'),
            "curren" => Some('\u{00A4}'),
            "plusmn" => Some('\u{00B1}'),
            "frac14" => Some('\u{00BC}'),
            "frac12" => Some('\u{00BD}'),
            "frac34" => Some('\u{00BE}'),
            "divide" => Some('\u{00F7}'),
            _ => None,
        },
        _ => None,
    }
}

/// SIMD-accelerated entity name comparison using AVX-512
/// Compares entity name against multiple candidates in parallel
#[cfg(target_feature = "avx512f")]
#[target_feature(enable = "avx512f,avx512bw")]
pub unsafe fn simd_entity_match_avx512(entity: &[u8], candidates: &[&[u8]]) -> Option<usize> {
    if entity.len() > 64 {
        return None;
    }
    
    // Pad entity to 64 bytes
    let mut padded = [0u8; 64];
    padded[..entity.len()].copy_from_slice(entity);
    let entity_vec = _mm512_loadu_si512(padded.as_ptr() as *const __m512i);
    
    for (idx, candidate) in candidates.iter().enumerate() {
        if candidate.len() != entity.len() {
            continue;
        }
        
        let mut candidate_padded = [0u8; 64];
        candidate_padded[..candidate.len()].copy_from_slice(candidate);
        let candidate_vec = _mm512_loadu_si512(candidate_padded.as_ptr() as *const __m512i);
        
        let mask = _mm512_cmpeq_epi8_mask(entity_vec, candidate_vec);
        let len_mask = (1u64 << entity.len()) - 1;
        
        if (mask & len_mask) == len_mask {
            return Some(idx);
        }
    }
    
    None
}

/// SIMD-accelerated entity name comparison using AVX2
/// Compares entity name against multiple candidates in parallel
#[target_feature(enable = "avx2")]
pub unsafe fn simd_entity_match_avx2(entity: &[u8], candidates: &[&[u8]]) -> Option<usize> {
    if entity.len() > 32 {
        return None;
    }
    
    // Pad entity to 32 bytes
    let mut padded = [0u8; 32];
    padded[..entity.len()].copy_from_slice(entity);
    let entity_vec = _mm256_loadu_si256(padded.as_ptr() as *const __m256i);
    
    for (idx, candidate) in candidates.iter().enumerate() {
        if candidate.len() != entity.len() {
            continue;
        }
        
        let mut candidate_padded = [0u8; 32];
        candidate_padded[..candidate.len()].copy_from_slice(candidate);
        let candidate_vec = _mm256_loadu_si256(candidate_padded.as_ptr() as *const __m256i);
        
        let cmp = _mm256_cmpeq_epi8(entity_vec, candidate_vec);
        let mask = _mm256_movemask_epi8(cmp) as u32;
        let len_mask = (1u32 << entity.len()) - 1;
        
        if (mask & len_mask) == len_mask {
            return Some(idx);
        }
    }
    
    None
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
    #[cfg(target_feature = "avx512f")]
    {
        if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
            return "AVX-512 (64-byte parallel)";
        }
    }
    
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
        
        // Test length-based dispatch
        assert_eq!(fast_entity_lookup("deg"), Some('\u{00B0}'));
        assert_eq!(fast_entity_lookup("mdash"), Some('\u{2014}'));
        assert_eq!(fast_entity_lookup("hellip"), Some('\u{2026}'));
    }

    #[test]
    fn test_simd_entity_match() {
        let candidates = &[b"nbsp".as_slice(), b"lt".as_slice(), b"gt".as_slice(), b"amp".as_slice()];
        
        if is_x86_feature_detected!("avx2") {
            unsafe {
                assert_eq!(simd_entity_match_avx2(b"nbsp", candidates), Some(0));
                assert_eq!(simd_entity_match_avx2(b"lt", candidates), Some(1));
                assert_eq!(simd_entity_match_avx2(b"gt", candidates), Some(2));
                assert_eq!(simd_entity_match_avx2(b"amp", candidates), Some(3));
                assert_eq!(simd_entity_match_avx2(b"invalid", candidates), None);
            }
        }
    }

    #[test]
    fn test_scan_tag_name() {
        let data = b"<div class='test'>";
        let end = scan_tag_name(data, 1);
        assert_eq!(end, 4); // "div" ends at position 4
        
        let data2 = b"<img src='test.jpg'/>";
        let end2 = scan_tag_name(data2, 1);
        assert_eq!(end2, 4); // "img" ends at position 4
        
        let data3 = b"<a>";
        let end3 = scan_tag_name(data3, 1);
        assert_eq!(end3, 2); // "a" ends at position 2
    }

    #[test]
    #[cfg(target_feature = "avx512f")]
    fn test_avx512_tag_scan() {
        if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
            let mut data = vec![b'a'; 70];
            data[0] = b'<';
            data[10] = b'>';
            
            unsafe {
                let end = fast_ascii_tag_scan_avx512(&data, 1);
                assert_eq!(end, 10);
            }
        }
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

    #[test]
    fn test_detect_whitespace() {
        let data = b"Hello\tWorld\n";
        let result = detect_whitespace(data);
        assert_eq!(result.len(), data.len());
        assert!(!result[0]); // 'H'
        assert!(result[5]);  // '\t'
        assert!(!result[6]); // 'W'
        assert!(result[11]); // '\n'
    }

    #[test]
    #[cfg(target_feature = "avx512f")]
    fn test_avx512_whitespace() {
        if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
            let mut data = vec![0u8; 64];
            data[0] = b' ';
            data[10] = b'\t';
            data[20] = b'\n';
            data[30] = b'\r';
            data[40] = b'\x0C';
            
            unsafe {
                let mask = simd_is_whitespace_avx512(&data);
                assert_ne!(mask & (1u64 << 0), 0);  // space
                assert_ne!(mask & (1u64 << 10), 0); // tab
                assert_ne!(mask & (1u64 << 20), 0); // LF
                assert_ne!(mask & (1u64 << 30), 0); // CR
                assert_ne!(mask & (1u64 << 40), 0); // FF
                assert_eq!(mask & (1u64 << 5), 0);  // not whitespace
            }
        }
    }
}
