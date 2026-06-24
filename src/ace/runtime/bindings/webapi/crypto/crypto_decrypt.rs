    /// TODO: add docs
    pub fn decrypt<'js>(
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
                    "Only AES-GCM decryption is currently supported",
                ));
            }
            let iv = algorithm_bytes(&algorithm, "iv")?;
            let aad = algorithm_optional_bytes(&algorithm, "additionalData").unwrap_or_default();
            let tag_length = algorithm_usize(&algorithm, "tagLength").unwrap_or(128);
            let all = extract_bytes(data)?;
            let tag_bytes = tag_length / 8;
            if all.len() < tag_bytes {
                return Err(rquickjs::Error::new_from_js(
                    "Crypto",
                    "Ciphertext too short for AES-GCM tag",
                ));
            }
            let ciphertext_len = all.len() - tag_bytes;
            let (ciphertext, tag_slice) = all.split_at(ciphertext_len);
            let tag: [u8; 16] = tag_slice.try_into().map_err(|_| {
                rquickjs::Error::new_from_js("Crypto", "AES-GCM tag must be 16 bytes")
            })?;

            let key = crypto_key_from_value(key)?;
            let secret = match &key.material {
                CryptoKeyMaterial::Secret(bytes) => bytes.as_slice(),
                _ => {
                    return Err(rquickjs::Error::new_from_js(
                        "Crypto",
                        "decrypt expects a secret CryptoKey",
                    ));
                }
            };

            let gcm = AesGcm::new(secret).map_err(to_js_crypto_err)?;
            let plaintext = gcm
                .decrypt(&iv, &aad, ciphertext, &tag)
                .map_err(to_js_crypto_err)?;
            Ok(ArrayBuffer::new(ctx, plaintext)?.into_value())
        })
    }

