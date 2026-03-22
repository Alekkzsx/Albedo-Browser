//! # Fast Builtins
//!
//! Implementações altamente otimizadas de builtins JS para uso direto pelo Tier 2.
//! Estas funções seguem a convenção de chamada C e operam diretamente sobre u64 (JsValue).

use crate::runtime::js_value::JsValue;
use crate::runtime::object_model;

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

/// Math.acos(x)
#[no_mangle]
pub extern "C" fn fast_math_acos(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).acos()).0
}

/// Math.acosh(x)
#[no_mangle]
pub extern "C" fn fast_math_acosh(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).acosh()).0
}

/// Math.asin(x)
#[no_mangle]
pub extern "C" fn fast_math_asin(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).asin()).0
}

/// Math.asinh(x)
#[no_mangle]
pub extern "C" fn fast_math_asinh(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).asinh()).0
}

/// Math.atan(x)
#[no_mangle]
pub extern "C" fn fast_math_atan(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).atan()).0
}

/// Math.atan2(y, x)
#[no_mangle]
pub extern "C" fn fast_math_atan2(y: u64, x: u64) -> u64 {
    JsValue::float64(to_number(JsValue(y)).atan2(to_number(JsValue(x)))).0
}

/// Math.atanh(x)
#[no_mangle]
pub extern "C" fn fast_math_atanh(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).atanh()).0
}

/// Math.cos(x)
#[no_mangle]
pub extern "C" fn fast_math_cos(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).cos()).0
}

/// Math.cosh(x)
#[no_mangle]
pub extern "C" fn fast_math_cosh(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).cosh()).0
}

/// Math.sin(x)
#[no_mangle]
pub extern "C" fn fast_math_sin(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).sin()).0
}

/// Math.sinh(x)
#[no_mangle]
pub extern "C" fn fast_math_sinh(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).sinh()).0
}

/// Math.tan(x)
#[no_mangle]
pub extern "C" fn fast_math_tan(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).tan()).0
}

/// Math.tanh(x)
#[no_mangle]
pub extern "C" fn fast_math_tanh(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).tanh()).0
}

/// Math.exp(x)
#[no_mangle]
pub extern "C" fn fast_math_exp(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).exp()).0
}

/// Math.expm1(x)
#[no_mangle]
pub extern "C" fn fast_math_expm1(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).exp_m1()).0
}

/// Math.log(x)
#[no_mangle]
pub extern "C" fn fast_math_log(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).ln()).0
}

/// Math.log1p(x)
#[no_mangle]
pub extern "C" fn fast_math_log1p(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).ln_1p()).0
}

/// Math.log10(x)
#[no_mangle]
pub extern "C" fn fast_math_log10(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).log10()).0
}

/// Math.log2(x)
#[no_mangle]
pub extern "C" fn fast_math_log2(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).log2()).0
}

/// Math.cbrt(x)
#[no_mangle]
pub extern "C" fn fast_math_cbrt(val: u64) -> u64 {
    JsValue::float64(to_number(JsValue(val)).cbrt()).0
}

/// Math.clz32(x)
#[no_mangle]
pub extern "C" fn fast_math_clz32(val: u64) -> u64 {
    let v = JsValue(val);
    let n = if v.is_int32() {
        v.as_int32() as u32
    } else {
        to_number(v) as u32
    };
    JsValue::int32(n.leading_zeros() as i32).0
}

/// Math.fround(x)
#[no_mangle]
pub extern "C" fn fast_math_fround(val: u64) -> u64 {
    let n = to_number(JsValue(val));
    JsValue::float64((n as f32) as f64).0
}

/// Math.hypot(x, y) - Versão bivariada para JIT
#[no_mangle]
pub extern "C" fn fast_math_hypot(x: u64, y: u64) -> u64 {
    let nx = to_number(JsValue(x));
    let ny = to_number(JsValue(y));
    JsValue::float64((nx * nx + ny * ny).sqrt()).0
}

/// Math.imul(x, y)
#[no_mangle]
pub extern "C" fn fast_math_imul(x: u64, y: u64) -> u64 {
    let vx = JsValue(x);
    let vy = JsValue(y);
    let ix = if vx.is_int32() { vx.as_int32() } else { to_number(vx) as i32 };
    let iy = if vy.is_int32() { vy.as_int32() } else { to_number(vy) as i32 };
    JsValue::int32(ix.wrapping_mul(iy)).0
}

/// Math.pow(x, y)
#[no_mangle]
pub extern "C" fn fast_math_pow(x: u64, y: u64) -> u64 {
    let nx = to_number(JsValue(x));
    let ny = to_number(JsValue(y));
    JsValue::float64(nx.powf(ny)).0
}

/// Math.random()
#[no_mangle]
pub extern "C" fn fast_math_random() -> u64 {
    JsValue::float64(rand::random::<f64>()).0
}

/// Math.round(x)
#[no_mangle]
pub extern "C" fn fast_math_round(val: u64) -> u64 {
    let x = to_number(JsValue(val));
    let r = if x >= 0.0 {
        (x + 0.5).floor()
    } else {
        (x - 0.5).ceil()
    };
    JsValue::float64(r).0
}

/// Math.sign(x)
#[no_mangle]
pub extern "C" fn fast_math_sign(val: u64) -> u64 {
    let x = to_number(JsValue(val));
    if x.is_nan() {
        JsValue::float64(f64::NAN).0
    } else if x == 0.0 {
        JsValue(val).0
    } else if x > 0.0 {
        JsValue::float64(1.0).0
    } else {
        JsValue::float64(-1.0).0
    }
}

/// Math.trunc(x)
#[no_mangle]
pub extern "C" fn fast_math_trunc(val: u64) -> u64 {
    let x = to_number(JsValue(val));
    let r = if x >= 0.0 { x.floor() } else { x.ceil() };
    JsValue::float64(r).0
}

/// Math.max(x, y) - Versão bivariada para JIT
#[no_mangle]
pub extern "C" fn fast_math_max(x: u64, y: u64) -> u64 {
    let nx = to_number(JsValue(x));
    let ny = to_number(JsValue(y));
    JsValue::float64(if nx > ny { nx } else { ny }).0
}

/// Math.min(x, y) - Versão bivariada para JIT
#[no_mangle]
pub extern "C" fn fast_math_min(x: u64, y: u64) -> u64 {
    let nx = to_number(JsValue(x));
    let ny = to_number(JsValue(y));
    JsValue::float64(if nx < ny { nx } else { ny }).0
}




#[no_mangle]
pub extern "C" fn fast_array_push(arr: u64, val: u64) -> u64 {
    let arr_v = JsValue(arr);
    let val_v = JsValue(val);
    object_model::array_push(arr_v, &[val_v]).0
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

    #[test]
    fn test_fast_trigo() {
        let val = JsValue::float64(0.0).0;
        assert_eq!(JsValue(fast_math_sin(val)).as_float64(), 0.0);
        assert_eq!(JsValue(fast_math_cos(val)).as_float64(), 1.0);
        assert_eq!(JsValue(fast_math_tan(val)).as_float64(), 0.0);
    }

    #[test]
    fn test_fast_log_exp() {
        assert_eq!(JsValue(fast_math_exp(JsValue::float64(0.0).0)).as_float64(), 1.0);
        assert_eq!(JsValue(fast_math_log(JsValue::float64(1.0).0)).as_float64(), 0.0);
    }


    #[test]
    fn test_fast_utility() {
        assert_eq!(JsValue(fast_math_imul(JsValue::int32(2).0, JsValue::int32(3).0)).as_int32(), 6);
        assert_eq!(JsValue(fast_math_pow(JsValue::float64(2.0).0, JsValue::float64(3.0).0)).as_float64(), 8.0);
        assert_eq!(JsValue(fast_math_max(JsValue::int32(10).0, JsValue::int32(20).0)).as_float64(), 20.0);
    }

    #[test]
    fn test_fast_array() {
        let arr = object_model::alloc_array();
        let val = JsValue::int32(123).0;
        let res = fast_array_push(arr.0, val);
        assert_eq!(JsValue(res).as_int32(), 1);
        let popped = fast_array_pop(arr.0);
        assert_eq!(JsValue(popped).as_int32(), 123);
    }
}

