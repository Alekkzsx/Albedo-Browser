use super::*;
use crate::ace::crypto::ecdh::{
    p256_generate_keypair, P256PublicKey, P256SecretKey, X25519PublicKey, X25519SecretKey,
};
use crate::ace::crypto::gcm::AesGcm;
use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, IntoJs, Object, Result, Value};



pub(crate) fn extract_bytes(data: Value<'_>) -> Result<Vec<u8>> {
    if let Some(s) = data.as_string() {
        return Ok(s.to_string()?.into_bytes());
    }
    if let Some(obj) = data.as_object() {
        if let Some(ab) = obj.as_array_buffer() {
            let slice: &[u8] = ab.as_ref();
            return Ok(slice.to_vec());
        }
        if let Some(ta) = obj.as_typed_array::<u8>() {
            return Ok(ta.as_bytes().expect("Albedo Engine: internal invariant violated").to_vec());
        }
    }
    Err(rquickjs::Error::new_from_js("Crypto", "Invalid data type"))
}
