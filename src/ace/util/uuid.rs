use std::fmt;

/// A 128-bit Universally Unique Identifier (UUID) v4.
/// 
/// Following RFC 4122, this is a random-based UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Uuid([u8; 16]);

impl Uuid {
    /// Generates a new UUID v4 using the `getrandom` library for entropy.
    /// 
    /// # Panics
    /// Panics if the system's random number generator fails.
    pub fn new_v4() -> Self {
        let mut bytes = [0u8; 16];
        crate::ace::crypto::random::get_random_bytes(&mut bytes);

        // Set version to 4: bits 4-7 of the 7th byte
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        // Set variant to 1: bits 6-7 of the 9th byte (0x80 = 10xxxxxx)
        bytes[8] = (bytes[8] & 0x3f) | 0x80;

        Uuid(bytes)
    }

    /// Returns the UUID as a formatted string: 8-4-4-4-12 hex digits.
    pub fn to_string(&self) -> String {
        format!("{}", self)
    }

    /// Returns the raw bytes of the UUID.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = &self.0;
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0], b[1], b[2], b[3],
            b[4], b[5],
            b[6], b[7],
            b[8], b[9],
            b[10], b[11], b[12], b[13], b[14], b[15]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_v4_format() {
        let uuid = Uuid::new_v4();
        let s = uuid.to_string();
        
        // Check length: 8 + 4 + 4 + 4 + 12 + 4 hyphens = 36
        assert_eq!(s.len(), 36);
        
        // Check hyphens at 8, 13, 18, 23
        let bytes = s.as_bytes();
        assert_eq!(bytes[8], b'-');
        assert_eq!(bytes[13], b'-');
        assert_eq!(bytes[18], b'-');
        assert_eq!(bytes[23], b'-');
        
        // Check version 4 (character at index 14)
        assert_eq!(bytes[14], b'4');
        
        // Check variant 1 (character at index 19 must be 8, 9, a, or b)
        let variant_char = bytes[19] as char;
        assert!(
            variant_char == '8' || variant_char == '9' || variant_char == 'a' || variant_char == 'b',
            "Invalid variant: {}", variant_char
        );
    }

    #[test]
    fn test_uuid_uniqueness() {
        let u1 = Uuid::new_v4();
        let u2 = Uuid::new_v4();
        assert_ne!(u1, u2);
    }
}
