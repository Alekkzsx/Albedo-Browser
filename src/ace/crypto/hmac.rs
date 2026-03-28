use crate::ace::crypto::sha1::sha1;
use crate::ace::crypto::sha2::{sha256, sha512};

fn hmac<const N: usize, F>(block_size: usize, key: &[u8], data: &[u8], hash: F) -> [u8; N]
where
    F: Fn(&[u8]) -> [u8; N],
{
    let mut key_block = vec![0u8; block_size];
    if key.len() > block_size {
        let hashed = hash(key);
        key_block[..N].copy_from_slice(&hashed);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = key_block.clone();
    let mut opad = key_block;
    for byte in &mut ipad {
        *byte ^= 0x36;
    }
    for byte in &mut opad {
        *byte ^= 0x5c;
    }

    let mut inner = ipad;
    inner.extend_from_slice(data);
    let inner_hash = hash(&inner);

    let mut outer = opad;
    outer.extend_from_slice(&inner_hash);
    hash(&outer)
}

pub fn hmac_sha1(key: &[u8], data: &[u8]) -> [u8; 20] {
    hmac(64, key, data, sha1)
}

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    hmac(64, key, data, sha256)
}

pub fn hmac_sha512(key: &[u8], data: &[u8]) -> [u8; 64] {
    hmac(128, key, data, sha512)
}

#[cfg(test)]
mod tests {
    use super::{hmac_sha1, hmac_sha256};
    use crate::ace::util::hex::encode;

    #[test]
    fn hmac_sha256_rfc4231_vector() {
        let key = [0x0b; 20];
        let data = b"Hi There";
        assert_eq!(
            encode(&hmac_sha256(&key, data)),
            "b0344c61d8db38535ca8afceaf0bf12b\
             881dc200c9833da726e9376c2e32cff7"
                .replace(' ', "")
        );
    }

    #[test]
    fn hmac_sha1_rfc2202_vector() {
        let key = [0x0b; 20];
        let data = b"Hi There";
        assert_eq!(
            encode(&hmac_sha1(&key, data)),
            "b617318655057264e28bc0b6fb378c8ef146be00"
        );
    }
}
