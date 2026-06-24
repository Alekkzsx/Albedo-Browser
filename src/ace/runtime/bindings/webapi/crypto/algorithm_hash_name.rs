use super::*;
use crate::ace::crypto::ecdh::{
    p256_generate_keypair, P256PublicKey, P256SecretKey, X25519PublicKey, X25519SecretKey,
};
use crate::ace::crypto::gcm::AesGcm;
use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, IntoJs, Object, Result, Value};



pub(crate) fn algorithm_hash_name(value: &Value<'_>) -> Option<String> {
    let obj = value.as_object()?;
    let hash_value: Value = obj.get("hash").ok()?;
    if let Some(s) = hash_value.as_string() {
        s.to_string().ok()
    } else if let Some(hash_obj) = hash_value.as_object() {
        hash_obj.get("name").ok()
    } else {
        None
    }
}
