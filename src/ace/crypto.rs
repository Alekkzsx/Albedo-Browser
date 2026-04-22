pub mod sha1 {
    pub fn sha1(_data: &[u8]) -> [u8; 20] {
        [0; 20]
    }
}

pub mod sha2 {
    pub fn sha256(_data: &[u8]) -> [u8; 32] {
        [0; 32]
    }
    pub fn sha384(_data: &[u8]) -> [u8; 48] {
        [0; 48]
    }
    pub fn sha512(_data: &[u8]) -> [u8; 64] {
        [0; 64]
    }
}

pub mod ecdh {
    pub struct P256PublicKey;
    pub struct P256SecretKey;
    pub struct X25519PublicKey;
    pub struct X25519SecretKey;

    impl X25519SecretKey {
        pub fn generate() -> Self { Self }
        pub fn from_bytes(_bytes: [u8; 32]) -> Self { Self }
        pub fn to_bytes(&self) -> [u8; 32] { [0; 32] }
        pub fn public_key(&self) -> X25519PublicKey { X25519PublicKey }
        pub fn diffie_hellman(&self, _public: &X25519PublicKey) -> [u8; 32] { [0; 32] }
    }

    impl X25519PublicKey {
        pub fn from_bytes(_bytes: [u8; 32]) -> Self { Self }
        pub fn to_bytes(&self) -> [u8; 32] { [0; 32] }
    }

    impl P256SecretKey {
        pub fn from_bytes(_bytes: [u8; 32]) -> Result<Self, String> { Ok(Self) }
        pub fn to_bytes(&self) -> [u8; 32] { [0; 32] }
        pub fn diffie_hellman(&self, _public: &P256PublicKey) -> Result<[u8; 32], String> { Ok([0; 32]) }
    }

    impl P256PublicKey {
        pub fn from_uncompressed_bytes(_bytes: &[u8]) -> Result<Self, String> { Ok(Self) }
        pub fn to_uncompressed_bytes(&self) -> Vec<u8> { vec![] }
    }

    pub fn p256_generate_keypair() -> Result<([u8; 32], Vec<u8>), String> {
        Ok(([0; 32], vec![]))
    }
}

pub mod gcm {
    pub struct AesGcm;
    impl AesGcm {
        pub fn new(_key: &[u8]) -> Result<Self, String> { Ok(Self) }
        pub fn encrypt(&self, _iv: &[u8], _aad: &[u8], _plaintext: &[u8]) -> Result<(Vec<u8>, [u8; 16]), String> {
            Ok((vec![], [0; 16]))
        }
        pub fn decrypt(&self, _iv: &[u8], _aad: &[u8], _ciphertext: &[u8], _tag: &[u8; 16]) -> Result<Vec<u8>, String> {
            Ok(vec![])
        }
    }
}

pub mod hmac {
    pub fn hmac_sha1(_key: &[u8], _data: &[u8]) -> Vec<u8> { vec![] }
    pub fn hmac_sha256(_key: &[u8], _data: &[u8]) -> Vec<u8> { vec![] }
    pub fn hmac_sha512(_key: &[u8], _data: &[u8]) -> Vec<u8> { vec![] }
}

pub mod random {
    pub fn get_random_bytes(buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = 0;
        }
    }
}
