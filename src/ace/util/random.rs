/// Fast non-cryptographic PRNG based on Xoshiro256**.
///
/// Seeding is done through ACE-Crypto CSPRNG.
pub struct AceRandom {
    s: [u64; 4],
}

impl AceRandom {
    /// Creates a new RNG seeded from ACE-Crypto CSPRNG.
    pub fn new() -> Self {
        let mut seed = [0u8; 32];
        crate::ace::crypto::random::get_random_bytes(&mut seed);
        Self::from_seed(seed)
    }

    /// Creates a deterministic RNG from a fixed 32-byte seed.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        let mut chunks = [0u64; 4];
        for (i, chunk) in seed.chunks_exact(8).enumerate() {
            let mut b = [0u8; 8];
            b.copy_from_slice(chunk);
            chunks[i] = u64::from_le_bytes(b);
        }

        // Mix incoming seed words to avoid weak/zero internal states.
        let mut mixer = SplitMix64::new(
            chunks[0]
                ^ chunks[1].rotate_left(13)
                ^ chunks[2].rotate_left(29)
                ^ chunks[3].rotate_left(47),
        );
        let mut s = [0u64; 4];
        for slot in &mut s {
            *slot = mixer.next_u64();
        }

        // Xoshiro all-zero state is invalid.
        if s.iter().all(|&x| x == 0) {
            s[0] = 0x9e37_79b9_7f4a_7c15;
        }

        Self { s }
    }

    /// Returns the next random u32.
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Returns the next random u64.
    pub fn next_u64(&mut self) -> u64 {
        // xoshiro256** reference transition:
        // result = rotl(s1 * 5, 7) * 9
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);

        result
    }

    /// Returns the next random f64 in [0.0, 1.0).
    pub fn next_f64(&mut self) -> f64 {
        // Use top 53 bits to match f64 mantissa precision.
        let v = self.next_u64() >> 11;
        (v as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    /// Returns a random value in [min, max) (max exclusive).
    ///
    /// Panics if `min >= max`.
    pub fn gen_range(&mut self, min: u64, max: u64) -> u64 {
        assert!(min < max, "AceRandom::gen_range requires min < max");
        let span = max - min;
        min + self.uniform_u64(span)
    }

    fn uniform_u64(&mut self, upper_exclusive: u64) -> u64 {
        // Rejection sampling to avoid modulo bias.
        let threshold = u64::MAX - (u64::MAX % upper_exclusive);
        loop {
            let v = self.next_u64();
            if v < threshold {
                return v % upper_exclusive;
            }
        }
    }
}

impl Default for AceRandom {
    fn default() -> Self {
        Self::new()
    }
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::AceRandom;

    #[test]
    fn deterministic_seed_is_stable() {
        let mut a = AceRandom::from_seed([7u8; 32]);
        let mut b = AceRandom::from_seed([7u8; 32]);
        for _ in 0..64 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn f64_is_in_expected_range() {
        let mut rng = AceRandom::from_seed([3u8; 32]);
        for _ in 0..1000 {
            let v = rng.next_f64();
            assert!(v >= 0.0);
            assert!(v < 1.0);
        }
    }

    #[test]
    fn gen_range_respects_bounds() {
        let mut rng = AceRandom::from_seed([11u8; 32]);
        for _ in 0..1000 {
            let v = rng.gen_range(10, 20);
            assert!((10..20).contains(&v));
        }
    }
}
