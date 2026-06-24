    #[qjs(rename = "deriveBits")]
    pub fn derive_bits<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        base_key: Value<'js>,
        length: usize,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let alg_name = algorithm_name(algorithm.clone())?;
            if alg_name != "ECDH" {
                return Err(rquickjs::Error::new_from_js(
                    "Crypto",
                    "Only ECDH deriveBits is currently supported",
                ));
            }
            let public_val = algorithm_object(&algorithm)?
                .get::<_, Value>("public")
                .map_err(|_| {
                    rquickjs::Error::new_from_js("Crypto", "ECDH deriveBits requires public key")
                })?;
            let public_key = crypto_key_from_value(public_val)?;
            let base_key = crypto_key_from_value(base_key)?;
            let bytes_len = length / 8;

            let shared = match (&base_key.material, &public_key.material) {
                (CryptoKeyMaterial::X25519Private(sk), CryptoKeyMaterial::X25519Public(pk)) => {
                    X25519SecretKey::from_bytes(*sk)
                        .diffie_hellman(&X25519PublicKey::from_bytes(*pk))
                        .to_vec()
                }
                (CryptoKeyMaterial::P256Private(sk), CryptoKeyMaterial::P256Public(pk)) => {
                    let sk = P256SecretKey::from_bytes(*sk).map_err(to_js_crypto_err)?;
                    let pk =
                        P256PublicKey::from_uncompressed_bytes(pk).map_err(to_js_crypto_err)?;
                    sk.diffie_hellman(&pk).map_err(to_js_crypto_err)?.to_vec()
                }
                _ => {
                    return Err(rquickjs::Error::new_from_js(
                        "Crypto",
                        "ECDH key types do not match",
                    ));
                }
            };

            let mut out = shared;
            out.truncate(bytes_len.min(out.len()));
            Ok(ArrayBuffer::new(ctx, out)?.into_value())
        })
    }
}
