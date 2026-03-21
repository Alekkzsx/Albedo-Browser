//! # Fast Builtins
//!
//! Implementações altamente otimizadas de builtins JS para uso direto pelo Tier 2.
//! Estas funções seguem a convenção de chamada C e operam diretamente sobre u64 (JsValue).

use crate::js_value::JsValue;
use crate::object_model;

/// Math.abs(x)
#[no_mangle]
pub extern "C" fn fast_math_abs(val: u64) -> u64 {
    let v = JsValue(val);
    if v.is_int32() {
        let x = v.as_int32();
        if x == i32::MIN {
            // Promoção para float se for o valor negativo mínimo
            return JsValue::float64((x as f64).abs()).0;
        }
        return JsValue::int32(x.abs()).0;
    } else if v.is_float64() {
        return JsValue::float64(v.as_float64().abs()).0;
    }
    // Fallback: se não for número, retorna NaN (simplificação para caminho rápido)
    JsValue::float64(f64::NAN).0
}

/// Math.sqrt(x)
#[no_mangle]
pub extern "C" fn fast_math_sqrt(val: u64) -> u64 {
    let v = JsValue(val);
    let n = to_number(v);
    JsValue::float64(n.sqrt()).0
}

/// Math.floor(x)
#[no_mangle]
pub extern "C" fn fast_math_floor(val: u64) -> u64 {
    let v = JsValue(val);
    if v.is_int32() {
        return val;
    }
    let n = to_number(v);
    JsValue::float64(n.floor()).0
}

/// Math.ceil(x)
#[no_mangle]
pub extern "C" fn fast_math_ceil(v: u64) -> u64 {
    let val = JsValue(v);
    if val.is_int32() {
        return v;
    }
    if val.is_float64() {
        return JsValue::float64(val.as_float64().ceil()).0;
    }
    JsValue::undefined().0
}

#[no_mangle]
pub extern "C" fn fast_array_push(arr: u64, val: u64) -> u64 {
    use std::io::Write;
    let arr_v = JsValue(arr);
    let val_v = JsValue(val);
    let res = object_model::array_push(arr_v, &[val_v]);
    println!(
        "[FAST-JIT] Array.push arr={:?} val={:?} -> len={}",
        arr_v,
        val_v,
        res.as_int32()
    );
    let _ = std::io::stdout().flush();
    res.0
}

#[no_mangle]
pub extern "C" fn fast_array_pop(arr: u64) -> u64 {
    object_model::array_pop(JsValue(arr)).0
}

#[no_mangle]
pub extern "C" fn fast_string_char_at(s: u64, idx: u64) -> u64 {
    let s_v = JsValue(s);
    let idx_v = JsValue(idx);
    let i = if idx_v.is_int32() {
        idx_v.as_int32() as usize
    } else {
        0
    };
    object_model::string_char_at(s_v, i).0
}

#[no_mangle]
pub extern "C" fn fast_json_parse(s: u64) -> u64 {
    object_model::json_parse(JsValue(s)).0
}

#[inline(always)]
fn to_number(v: JsValue) -> f64 {
    if v.is_int32() {
        v.as_int32() as f64
    } else if v.is_float64() {
        v.as_float64()
    } else {
        f64::NAN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_abs() {
        assert_eq!(JsValue(fast_math_abs(JsValue::int32(-42).0)).as_int32(), 42);
        assert_eq!(
            JsValue(fast_math_abs(JsValue::float64(-10.5).0)).as_float64(),
            10.5
        );
    }

    #[test]
    fn test_fast_sqrt() {
        assert_eq!(
            JsValue(fast_math_sqrt(JsValue::float64(16.0).0)).as_float64(),
            4.0
        );
    }
}
