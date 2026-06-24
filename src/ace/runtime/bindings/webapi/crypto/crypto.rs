use super::*;
use crate::ace::crypto::ecdh::{
    p256_generate_keypair, P256PublicKey, P256SecretKey, X25519PublicKey, X25519SecretKey,
};
use crate::ace::crypto::gcm::AesGcm;
use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, IntoJs, Object, Result, Value};



#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Crypto {
    pub subtle: SubtleCrypto,
}

#[rquickjs::methods]
impl Crypto {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            subtle: SubtleCrypto {},
        }
    }

    #[qjs(get)]
    pub fn subtle(&self) -> SubtleCrypto {
        self.subtle.clone()
    }

    #[qjs(rename = "getRandomValues")]
    pub fn get_random_values<'js>(&self, typed_array: Object<'js>) -> Result<Object<'js>> {
        if let Some(ta) = typed_array.as_typed_array::<u8>() {
            let mut buf = vec![0u8; ta.len()];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        } else if let Some(ta) = typed_array.as_typed_array::<u16>() {
            let mut buf = vec![0u8; ta.len() * 2];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        } else if let Some(ta) = typed_array.as_typed_array::<u32>() {
            let mut buf = vec![0u8; ta.len() * 4];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        } else if let Some(ta) = typed_array.as_typed_array::<i8>() {
            let mut buf = vec![0u8; ta.len()];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        } else if let Some(ta) = typed_array.as_typed_array::<i16>() {
            let mut buf = vec![0u8; ta.len() * 2];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        } else if let Some(ta) = typed_array.as_typed_array::<i32>() {
            let mut buf = vec![0u8; ta.len() * 4];
            crate::ace::crypto::random::get_random_bytes(&mut buf);
            copy_into_typed_array(&ta, &buf);
        }

        Ok(typed_array)
    }

    /// TODO: add docs
    pub fn random_uuid(&self) -> String {
        crate::utils::uuid::Uuid::new_v4().to_string()
    }
}
