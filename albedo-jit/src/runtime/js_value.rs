//! # JsValue
//!
//! Representação de 64-bits (NaN-boxing) para valores dinâmicos.
//!
//! Esta técnica explora o espaço dos NaNs no padrão IEEE 754 (float64)
//! para injetar tipos (inteiros, ponteiros, booleanos) num mesmo
//! registrador físico de máquina.
//!
//! Float64 canônico: `[sinal: 1] [expoente: 11] [mantissa: 52]`
//! O "espaço livre" de um NaN é quando o expoente é todo 1 (0x7FF).

#![allow(dead_code)]

// ---------------------------------------------------------------------------
// Constantes de Tagging (NaN-Boxing Space)
// ---------------------------------------------------------------------------
//
// Estrutura:
// [ TAG: 16 bits ] [ PAYLOAD: 48 bits ]
//
// Qualquer valor com top 16 bits >= 0xFFF9 é considerado "boxed".
// Float64 válidos (incluindo NaN canônico) ficam abaixo disso.

pub const FLOAT_NAN: u64 = 0x7FF8_0000_0000_0000;
pub const TAG_MASK: u64 = 0xFFFF_0000_0000_0000;
pub const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

pub const TAG_INT32: u64 = 0xFFFE_0000_0000_0000;
pub const TAG_BOOL: u64 = 0xFFFD_0000_0000_0000;
pub const TAG_UNDEFINED: u64 = 0xFFFC_0000_0000_0000;
pub const TAG_NULL: u64 = 0xFFFB_0000_0000_0000;
pub const TAG_OBJECT: u64 = 0xFFFA_0000_0000_0000;
pub const TAG_STRING: u64 = 0xFFF9_0000_0000_0000;
pub const TAG_BUILTIN: u64 = 0xFFF8_0000_0000_0000;
pub const TAG_MIN: u64 = TAG_BUILTIN;

/// Um valor dinâmico tipado no formato de 64 bits para o AlbedoJIT.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)] // Importante para otimizacao zero-cost na C-ABI
pub struct JsValue(pub u64);

impl JsValue {
    // === Empacotamento (Constructors) ===

    #[inline(always)]
    pub fn int32(v: i32) -> Self {
        // Tag int32 no header e payload de i32 estendido para u64 (lower 32 bits)
        Self(TAG_INT32 | (v as u32 as u64))
    }

    #[inline(always)]
    pub fn float64(v: f64) -> Self {
        // Toda vez que empacotarmos um Float, canonicamos os NaNs pra não encavalar nos Tags
        if v.is_nan() {
            Self(FLOAT_NAN)
        } else {
            Self(v.to_bits())
        }
    }

    #[inline(always)]
    pub fn bool(v: bool) -> Self {
        Self(TAG_BOOL | (v as u64))
    }

    #[inline(always)]
    pub fn undefined() -> Self {
        Self(TAG_UNDEFINED)
    }

    #[inline(always)]
    pub fn null() -> Self {
        Self(TAG_NULL)
    }

    #[inline(always)]
    pub fn string(id: u64) -> Self {
        Self(TAG_STRING | (id & PAYLOAD_MASK))
    }

    #[inline(always)]
    pub fn object(ptr: u64) -> Self {
        Self(TAG_OBJECT | (ptr & PAYLOAD_MASK))
    }

    #[inline(always)]
    pub fn builtin(id: u64) -> Self {
        Self(TAG_BUILTIN | (id & PAYLOAD_MASK))
    }

    // === Desempacotamento (Getters) ===

    #[inline(always)]
    pub fn as_int32(&self) -> i32 {
        // Mantém apenas os 32 bits inferiores e decodifica pra i32 assinado
        (self.0 & 0xFFFF_FFFF) as i32
    }

    #[inline(always)]
    pub fn as_float64(&self) -> f64 {
        f64::from_bits(self.0)
    }

    #[inline(always)]
    pub fn as_bool(&self) -> bool {
        (self.0 & 1) != 0
    }

    #[inline(always)]
    pub fn as_string_id(&self) -> u64 {
        self.0 & PAYLOAD_MASK
    }

    #[inline(always)]
    pub fn as_object_ptr(&self) -> u64 {
        self.0 & PAYLOAD_MASK
    }

    #[inline(always)]
    pub fn as_builtin_id(&self) -> u64 {
        self.0 & PAYLOAD_MASK
    }

    // === Testes de Tipo ===

    #[inline(always)]
    pub fn is_int32(&self) -> bool {
        (self.0 & TAG_MASK) == TAG_INT32
    }

    #[inline(always)]
    pub fn is_float64(&self) -> bool {
        // Valores com tag (top 16 bits) >= TAG_MIN são NaN-boxed.
        (self.0 & TAG_MASK) < TAG_MIN
    }

    #[inline(always)]
    pub fn is_bool(&self) -> bool {
        (self.0 & TAG_MASK) == TAG_BOOL
    }

    #[inline(always)]
    pub fn is_undefined(&self) -> bool {
        self.0 == TAG_UNDEFINED
    }

    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.0 == TAG_NULL
    }

    #[inline(always)]
    pub fn is_object(&self) -> bool {
        (self.0 & TAG_MASK) == TAG_OBJECT
    }

    #[inline(always)]
    pub fn is_string(&self) -> bool {
        (self.0 & TAG_MASK) == TAG_STRING
    }

    #[inline(always)]
    pub fn is_builtin(&self) -> bool {
        (self.0 & TAG_MASK) == TAG_BUILTIN
    }

    pub fn to_number(&self) -> f64 {
        if self.is_int32() {
            self.as_int32() as f64
        } else if self.is_float64() {
            self.as_float64()
        } else if self.is_bool() {
            if self.as_bool() {
                1.0
            } else {
                0.0
            }
        } else if self.is_null() {
            0.0
        } else {
            f64::NAN
        }
    }

    pub fn to_int32(&self) -> i32 {
        if self.is_int32() {
            self.as_int32()
        } else {
            self.to_number() as i32
        }
    }
}

// Para debug facilitado no console
impl std::fmt::Debug for JsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_int32() {
            write!(f, "Int32({})", self.as_int32())
        } else if self.is_bool() {
            write!(f, "Bool({})", self.as_bool())
        } else if self.is_undefined() {
            write!(f, "Undefined")
        } else if self.is_null() {
            write!(f, "Null")
        } else if self.is_string() {
            write!(f, "String(id={})", self.as_string_id())
        } else if self.is_builtin() {
            write!(f, "Builtin(id={})", self.as_builtin_id())
        } else if self.is_object() {
            write!(f, "Object(ptr=0x{:x})", self.as_object_ptr())
        } else {
            write!(f, "Float64({})", self.as_float64())
        }
    }
}

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nan_boxing_int32() {
        let val = JsValue::int32(42);
        assert!(val.is_int32());
        assert!(!val.is_float64());
        assert!(!val.is_bool());
        assert_eq!(val.as_int32(), 42);

        let negative = JsValue::int32(-10);
        assert!(negative.is_int32());
        assert_eq!(negative.as_int32(), -10);
    }

    #[test]
    fn test_nan_boxing_float64() {
        let val = JsValue::float64(3.1415);
        assert!(val.is_float64());
        assert!(!val.is_int32());
        assert_eq!(val.as_float64(), 3.1415);
    }

    #[test]
    fn test_nan_boxing_bool() {
        let val_true = JsValue::bool(true);
        let val_false = JsValue::bool(false);
        assert!(val_true.is_bool());
        assert!(val_false.is_bool());
        assert!(val_true.as_bool());
        assert!(!val_false.as_bool());
    }

    #[test]
    fn test_nan_boxing_specials() {
        let p_undef = JsValue::undefined();
        let p_null = JsValue::null();
        assert!(p_undef.is_undefined());
        assert!(p_null.is_null());
        assert!(!p_undef.is_null());
        assert!(!p_null.is_undefined());
    }
}
