use crate::runtime::core::runtime::JsRuntime;
use rquickjs::{ArrayBuffer, Class, Ctx, Object, Result, Value};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct SubtleCrypto {}

#[rquickjs::methods]
impl SubtleCrypto {
    #[qjs(rename = "digest")]
    pub fn digest<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: String,
        data: Value<'js>,
    ) -> Result<Value<'js>> {
        let (promise, resolve, _reject) = rquickjs::Promise::new(&ctx)?;
        let rt_val = ctx.globals().get::<_, Value>("__albedo_rt__")?;
        let _rt = Class::<JsRuntime>::from_object(rt_val.as_object().unwrap())
            .unwrap()
            .borrow()
            .clone();

        let bytes_vec = if let Some(s) = data.as_string() {
            s.to_string()?.into_bytes()
        } else if let Some(obj) = data.as_object() {
            if let Some(ab) = obj.as_array_buffer() {
                let slice: &[u8] = ab.as_ref();
                slice.to_vec()
            } else if let Some(ta) = obj.as_typed_array::<u8>() {
                ta.as_bytes().unwrap().to_vec()
            } else {
                return Err(rquickjs::Error::new_from_js("Crypto", "Invalid data type"));
            }
        } else {
            return Err(rquickjs::Error::new_from_js("Crypto", "Invalid data type"));
        };

        // Compute hash synchronously (data is in memory) and resolve immediately
        use sha2::{Digest, Sha256, Sha512};
        let hash = if algorithm == "SHA-256" {
            let mut hasher = Sha256::new();
            hasher.update(&bytes_vec);
            hasher.finalize().to_vec()
        } else if algorithm == "SHA-512" {
            let mut hasher = Sha512::new();
            hasher.update(&bytes_vec);
            hasher.finalize().to_vec()
        } else {
            Vec::new()
        };

        if let Ok(ab) = ArrayBuffer::new(ctx.clone(), hash) {
            let _ = resolve.call::<(ArrayBuffer,), ()>((ab,));
        }

        Ok(promise.into_value())
    }
}

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
            let _ = getrandom::getrandom(&mut buf);
            if let Some(raw) = ta.as_raw() {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buf.as_ptr(),
                        raw.ptr.as_ptr() as *mut u8,
                        buf.len(),
                    );
                }
            }
        } else if let Some(ta) = typed_array.as_typed_array::<u16>() {
            let mut buf = vec![0u8; ta.len() * 2];
            let _ = getrandom::getrandom(&mut buf);
            if let Some(raw) = ta.as_raw() {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buf.as_ptr(),
                        raw.ptr.as_ptr() as *mut u8,
                        buf.len(),
                    );
                }
            }
        } else if let Some(ta) = typed_array.as_typed_array::<u32>() {
            let mut buf = vec![0u8; ta.len() * 4];
            let _ = getrandom::getrandom(&mut buf);
            if let Some(raw) = ta.as_raw() {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buf.as_ptr(),
                        raw.ptr.as_ptr() as *mut u8,
                        buf.len(),
                    );
                }
            }
        } else if let Some(ta) = typed_array.as_typed_array::<i8>() {
            let mut buf = vec![0u8; ta.len()];
            let _ = getrandom::getrandom(&mut buf);
            if let Some(raw) = ta.as_raw() {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buf.as_ptr(),
                        raw.ptr.as_ptr() as *mut u8,
                        buf.len(),
                    );
                }
            }
        } else if let Some(ta) = typed_array.as_typed_array::<i16>() {
            let mut buf = vec![0u8; ta.len() * 2];
            let _ = getrandom::getrandom(&mut buf);
            if let Some(raw) = ta.as_raw() {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buf.as_ptr(),
                        raw.ptr.as_ptr() as *mut u8,
                        buf.len(),
                    );
                }
            }
        } else if let Some(ta) = typed_array.as_typed_array::<i32>() {
            let mut buf = vec![0u8; ta.len() * 4];
            let _ = getrandom::getrandom(&mut buf);
            if let Some(raw) = ta.as_raw() {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buf.as_ptr(),
                        raw.ptr.as_ptr() as *mut u8,
                        buf.len(),
                    );
                }
            }
        }

        Ok(typed_array)
    }

    pub fn random_uuid(&self) -> String {
        crate::ace::util::uuid::Uuid::new_v4().to_string()
    }
}

pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx| {
            Class::<SubtleCrypto>::register(&ctx)?;
            Class::<Crypto>::register(&ctx)?;
            let crypto = Class::instance(ctx.clone(), Crypto::new())?;
            ctx.globals().set("crypto", crypto)?;
            Ok(())
        })
    })
}
