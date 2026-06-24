    /// TODO: add docs
    pub fn encrypt<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        key: Value<'js>,
        data: Value<'js>,
    ) -> Result<Value<'js>> {
        resolved_promise(&ctx, |ctx| {
            let alg_name = algorithm_name(algorithm.clone())?;
            if alg_name != "AES-GCM" {
                return Err(rquickjs::Error::new_from_js(
                    "Crypto",
                    "Only AES-GCM encryption is currently supported",
                ));
            }
            let iv = algorithm_bytes(&algorithm, "iv")?;
            let aad = algorithm_optional_bytes(&algorithm, "additionalData").unwrap_or_default();
            let _tag_length = algorithm_usize(&algorithm, "tagLength").unwrap_or(128);
            let plaintext = extract_bytes(data)?;
            let key = crypto_key_from_value(key)?;
            let secret = match &key.material {
                CryptoKeyMaterial::Secret(bytes) => bytes.as_slice(),
                _ => {
                    return Err(rquickjs::Error::new_from_js(
                        "Crypto",
                        "encrypt expects a secret CryptoKey",
                    ));
                }
            };

            let gcm = AesGcm::new(secret).map_err(to_js_crypto_err)?;
            let (mut ciphertext, tag) = gcm
                .encrypt(&iv, &aad, &plaintext)
                .map_err(to_js_crypto_err)?;
            ciphertext.extend_from_slice(&tag);
            Ok(ArrayBuffer::new(ctx, ciphertext)?.into_value())
        })
    }

