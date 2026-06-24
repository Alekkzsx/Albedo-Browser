use super::*;
use crate::ace::crypto::ecdh::{
    p256_generate_keypair, P256PublicKey, P256SecretKey, X25519PublicKey, X25519SecretKey,
};
use crate::ace::crypto::gcm::AesGcm;
use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, IntoJs, Object, Result, Value};



pub(crate) fn copy_into_typed_array<T>(ta: &rquickjs::TypedArray<'_, T>, buf: &[u8]) {
    if let Some(raw) = ta.as_raw() {
        // SAFETY: The TypedArray's raw pointer is valid and points to allocated memory.
        // buf.len() is guaranteed to be within the TypedArray's bounds by the caller.
        unsafe {
            std::ptr::copy_nonoverlapping(buf.as_ptr(), raw.ptr.as_ptr() as *mut u8, buf.len());
        }
    }
}
