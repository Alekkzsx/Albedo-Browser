use super::*;
use crate::ace::crypto::ecdh::{
    p256_generate_keypair, P256PublicKey, P256SecretKey, X25519PublicKey, X25519SecretKey,
};
use crate::ace::crypto::gcm::AesGcm;
use crate::ace::crypto::hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use crate::ace::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, IntoJs, Object, Result, Value};



pub(crate) fn resolved_promise<'js, F>(ctx: &Ctx<'js>, f: F) -> Result<Value<'js>>
where
    F: FnOnce(Ctx<'js>) -> Result<Value<'js>>,
{
    let (promise, resolve, reject) = rquickjs::Promise::new(ctx)?;
    match f(ctx.clone()) {
        Ok(value) => {
            let _ = resolve.call::<(Value,), ()>((value,));
        }
        Err(err) => {
            let _ = reject.call::<(String,), ()>((format!("{}", err),));
        }
    }
    Ok(promise.into_value())
}
