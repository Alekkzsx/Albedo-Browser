//! # Minimal Object Model (JIT)
//!
//! Modelo mínimo de objetos/arrays com "hidden class" (shape) para suportar
//! inline caches e specialization no Tier 2.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::mem::offset_of;
use std::sync::OnceLock;

use crate::runtime::js_value::JsValue;
use serde_json::Value as JsonValue;

// ---------------------------------------------------------------------------
// Layout do Objeto (estável para geração de código)
// ---------------------------------------------------------------------------

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Object = 0,
    Array = 1,
    Function = 2,
}

#[repr(C)]
pub struct JsObject {
    pub shape_id: u64,
    pub kind: u32,
    /// Se kind == Function, contém o índice internado do FunctionId.
    pub func_id_idx: u32,
    pub props: *mut JsValue,
    pub props_len: u32,
    pub props_cap: u32,
}

pub const JSOBJ_SHAPE_OFFSET: u32 = offset_of!(JsObject, shape_id) as u32;
pub const JSOBJ_KIND_OFFSET: u32 = offset_of!(JsObject, kind) as u32;
pub const JSOBJ_PROPS_OFFSET: u32 = offset_of!(JsObject, props) as u32;
pub const JSOBJ_PROPS_LEN_OFFSET: u32 = offset_of!(JsObject, props_len) as u32;
pub const JSOBJ_PROPS_CAP_OFFSET: u32 = offset_of!(JsObject, props_cap) as u32;

// ---------------------------------------------------------------------------
// String Interning (payload = id)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct StringInterner {
    strings: Vec<String>,
    index: HashMap<String, u32>,
}

impl StringInterner {
    fn intern(&mut self, s: String) -> u32 {
        if let Some(id) = self.index.get(&s) {
            return *id;
        }
        let id = self.strings.len() as u32;
        self.strings.push(s.clone());
        self.index.insert(s, id);
        id
    }
}

static STRING_INTERNER: OnceLock<RwLock<StringInterner>> = OnceLock::new();
static LENGTH_PROP_ID: OnceLock<u32> = OnceLock::new();

pub fn intern_string(s: String) -> u32 {
    let interner = STRING_INTERNER.get_or_init(|| RwLock::new(StringInterner::default()));
    interner.write().intern(s)
}

pub fn get_string(id: u32) -> Option<String> {
    let interner = STRING_INTERNER.get_or_init(|| RwLock::new(StringInterner::default()));
    interner.read().strings.get(id as usize).cloned()
}

fn length_prop_id() -> u32 {
    *LENGTH_PROP_ID.get_or_init(|| {
        let interner = STRING_INTERNER.get_or_init(|| RwLock::new(StringInterner::default()));
        interner.write().intern("length".to_string())
    })
}

// ---------------------------------------------------------------------------
// Shape Registry (Hidden Classes)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Shape {
    id: u64, // hash of keys+order
    _parent: Option<u64>,
    _last_prop: Option<u32>,
    props: Vec<u32>, // property ids in order (offset = index)
    prop_index: HashMap<u32, usize>,
}

impl Shape {
    fn new_empty() -> Self {
        Self {
            id: fnv1a_seed(),
            _parent: None,
            _last_prop: None,
            props: Vec::new(),
            prop_index: HashMap::new(),
        }
    }
}

#[derive(Default)]
struct ShapeRegistry {
    shapes: HashMap<u64, Shape>,
    transitions: HashMap<(u64, u32), u64>, // (from_shape, prop_id) -> new_shape
}

impl ShapeRegistry {
    fn new() -> Self {
        let mut reg = Self::default();
        let empty = Shape::new_empty();
        reg.shapes.insert(empty.id, empty);
        reg
    }

    fn empty_shape(&self) -> u64 {
        fnv1a_seed()
    }

    fn get_offset(&self, shape_id: u64, prop_id: u32) -> Option<u32> {
        self.shapes
            .get(&shape_id)
            .and_then(|s| s.prop_index.get(&prop_id).copied())
            .map(|v| v as u32)
    }

    fn transition(&mut self, from_shape: u64, prop_id: u32) -> u64 {
        if let Some(next) = self.transitions.get(&(from_shape, prop_id)) {
            return *next;
        }

        let base = self.shapes.get(&from_shape).expect("Shape inválido");

        if base.prop_index.contains_key(&prop_id) {
            return from_shape;
        }

        let mut props = base.props.clone();
        let mut index = base.prop_index.clone();
        let offset = props.len();
        props.push(prop_id);
        index.insert(prop_id, offset);

        let new_hash = fnv1a_extend(base.id, prop_id);
        let new_shape = Shape {
            id: new_hash,
            _parent: Some(from_shape),
            _last_prop: Some(prop_id),
            props,
            prop_index: index,
        };

        // Colisões são extremamente improváveis com FNV-1a 64-bit.
        // Caso a mesma hash já exista com estrutura diferente, isso é um bug lógico.
        self.shapes.entry(new_hash).or_insert(new_shape);
        self.transitions.insert((from_shape, prop_id), new_hash);
        new_hash
    }
}

static SHAPE_REGISTRY: OnceLock<RwLock<ShapeRegistry>> = OnceLock::new();

fn shape_registry() -> &'static RwLock<ShapeRegistry> {
    SHAPE_REGISTRY.get_or_init(|| RwLock::new(ShapeRegistry::new()))
}

pub fn shape_offset(shape_id: u64, prop_id: u32) -> Option<u32> {
    shape_registry().read().get_offset(shape_id, prop_id)
}

// ---------------------------------------------------------------------------
// Heap de Objetos (ponteiros estáveis)
// ---------------------------------------------------------------------------

pub fn alloc_object() -> JsValue {
    let shape_id = shape_registry().read().empty_shape();
    let obj = Box::new(JsObject {
        shape_id,
        kind: ObjectKind::Object as u32,
        func_id_idx: 0,
        props: std::ptr::null_mut(),
        props_len: 0,
        props_cap: 0,
    });
    let ptr = Box::into_raw(obj) as u64;
    JsValue::object(ptr)
}

pub fn alloc_array() -> JsValue {
    let shape_id = shape_registry().read().empty_shape();
    let obj = Box::new(JsObject {
        shape_id,
        kind: ObjectKind::Array as u32,
        func_id_idx: 0,
        props: std::ptr::null_mut(),
        props_len: 0,
        props_cap: 0,
    });
    let ptr = Box::into_raw(obj) as u64;
    JsValue::object(ptr)
}

pub fn alloc_function(func_id_name: String) -> JsValue {
    let shape_id = shape_registry().read().empty_shape();
    let func_id_idx = intern_string(func_id_name);
    let obj = Box::new(JsObject {
        shape_id,
        kind: ObjectKind::Function as u32,
        func_id_idx,
        props: std::ptr::null_mut(),
        props_len: 0,
        props_cap: 0,
    });
    let ptr = Box::into_raw(obj) as u64;
    JsValue::object(ptr)
}

pub fn is_array(val: JsValue) -> bool {
    if !val.is_object() {
        return false;
    }
    unsafe {
        let obj = &*(val.as_object_ptr() as *const JsObject);
        obj.kind == ObjectKind::Array as u32
    }
}

fn ensure_props_capacity(obj: &mut JsObject, new_len: usize) {
    let mut vec: Vec<JsValue> = if obj.props.is_null() {
        Vec::new()
    } else {
        unsafe { Vec::from_raw_parts(obj.props, obj.props_len as usize, obj.props_cap as usize) }
    };

    if vec.capacity() < new_len {
        let mut new_cap = if vec.capacity() == 0 {
            4
        } else {
            vec.capacity() * 2
        };
        while new_cap < new_len {
            new_cap *= 2;
        }
        vec.reserve(new_cap - vec.capacity());
    }

    if vec.len() < new_len {
        vec.resize(new_len, JsValue::undefined());
    }

    obj.props_len = vec.len() as u32;
    obj.props_cap = vec.capacity() as u32;
    obj.props = vec.as_mut_ptr();
    std::mem::forget(vec);
}

pub fn set_prop(obj_val: JsValue, prop_val: JsValue, value: JsValue) {
    if !obj_val.is_object() || !prop_val.is_string() {
        return;
    }

    let prop_id = prop_val.as_string_id() as u32;
    unsafe {
        let obj = &mut *(obj_val.as_object_ptr() as *mut JsObject);
        if obj.kind == ObjectKind::Array as u32 {
            if let Some(idx) = parse_array_index(prop_val) {
                array_set_index(obj, idx, value);
                return;
            }
        }
        let mut reg = shape_registry().write();
        let new_shape = reg.transition(obj.shape_id, prop_id);
        obj.shape_id = new_shape;
        let offset = reg.get_offset(new_shape, prop_id).unwrap_or(0) as usize;
        ensure_props_capacity(obj, offset + 1);
        *obj.props.add(offset) = value;
    }
}

pub fn get_prop(obj_val: JsValue, prop_val: JsValue) -> JsValue {
    if !obj_val.is_object() || !prop_val.is_string() {
        return JsValue::undefined();
    }
    let prop_id = prop_val.as_string_id() as u32;
    unsafe {
        let obj = &mut *(obj_val.as_object_ptr() as *mut JsObject);
        if obj.kind == ObjectKind::Array as u32 {
            if prop_id == length_prop_id() {
                println!("[DEBUG-OBJ] Get length on array: len={}", obj.props_len);
                return JsValue::int32(obj.props_len as i32);
            }
            if let Some(idx) = parse_array_index(prop_val) {
                return array_get_index(obj, idx);
            }
        }
        println!("[DEBUG-OBJ] Get other prop: id={} (length_id={})", prop_id, length_prop_id());
        let reg = shape_registry().read();
        if let Some(offset) = reg.get_offset(obj.shape_id, prop_id) {
            if (offset as u32) < obj.props_len {
                return *obj.props.add(offset as usize);
            }
        }
        JsValue::undefined()
    }
}

pub fn has_prop(obj_val: JsValue, prop_val: JsValue) -> bool {
    if !obj_val.is_object() || !prop_val.is_string() {
        return false;
    }
    let prop_id = prop_val.as_string_id() as u32;
    unsafe {
        let obj = &*(obj_val.as_object_ptr() as *const JsObject);
        if obj.kind == ObjectKind::Array as u32 {
            if prop_id == length_prop_id() {
                return true;
            }
            if let Some(idx) = parse_array_index(prop_val) {
                return idx < obj.props_len as usize;
            }
        }
        let reg = shape_registry().read();
        reg.get_offset(obj.shape_id, prop_id).is_some()
    }
}

pub fn delete_prop(obj_val: JsValue, prop_val: JsValue) -> bool {
    if !obj_val.is_object() || !prop_val.is_string() {
        return true;
    }
    set_prop(obj_val, prop_val, JsValue::undefined());
    true
}

pub fn object_shape_id(val: JsValue) -> Option<u64> {
    if !val.is_object() {
        return None;
    }
    unsafe {
        let obj = &*(val.as_object_ptr() as *const JsObject);
        Some(obj.shape_id)
    }
}

fn fnv1a_seed() -> u64 {
    0xcbf29ce484222325
}

fn fnv1a_extend(hash: u64, prop_id: u32) -> u64 {
    let mut h = hash;
    let bytes = prop_id.to_le_bytes();
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn parse_array_index(prop_val: JsValue) -> Option<usize> {
    if !prop_val.is_string() {
        return None;
    }
    let id = prop_val.as_string_id() as usize;
    let interner = STRING_INTERNER.get_or_init(|| RwLock::new(StringInterner::default()));
    let strings = &interner.read().strings;
    strings.get(id).and_then(|s| s.parse::<usize>().ok())
}

pub fn array_len(arr: JsValue) -> Option<u32> {
    if !arr.is_object() {
        return None;
    }
    unsafe {
        let obj = &*(arr.as_object_ptr() as *const JsObject);
        if obj.kind == ObjectKind::Array as u32 {
            Some(obj.props_len)
        } else {
            None
        }
    }
}

pub fn array_push(arr: JsValue, items: &[JsValue]) -> JsValue {
    if !arr.is_object() {
        return JsValue::undefined();
    }
    unsafe {
        let obj = &mut *(arr.as_object_ptr() as *mut JsObject);
        if obj.kind != ObjectKind::Array as u32 {
            return JsValue::undefined();
        }
        let mut idx = obj.props_len as usize;
        for item in items {
            array_set_index(obj, idx, *item);
            idx += 1;
        }
        JsValue::int32(obj.props_len as i32)
    }
}

pub fn array_pop(arr: JsValue) -> JsValue {
    if !arr.is_object() {
        return JsValue::undefined();
    }
    unsafe {
        let obj = &mut *(arr.as_object_ptr() as *mut JsObject);
        if obj.kind != ObjectKind::Array as u32 || obj.props_len == 0 {
            return JsValue::undefined();
        }
        let idx = (obj.props_len - 1) as usize;
        let v = array_get_index(obj, idx);
        obj.props_len -= 1;
        v
    }
}

pub fn string_char_at(s: JsValue, idx: usize) -> JsValue {
    if !s.is_string() {
        return JsValue::undefined();
    }
    let id = s.as_string_id() as u32;
    if let Some(text) = get_string(id) {
        if let Some(ch) = text.chars().nth(idx) {
            let mut buf = String::new();
            buf.push(ch);
            let new_id = intern_string(buf);
            return JsValue::string(new_id as u64);
        }
        let empty_id = intern_string(String::new());
        return JsValue::string(empty_id as u64);
    }
    JsValue::undefined()
}

pub fn json_parse(s: JsValue) -> JsValue {
    if !s.is_string() {
        return JsValue::undefined();
    }
    let id = s.as_string_id() as u32;
    let text = match get_string(id) {
        Some(t) => t,
        None => return JsValue::undefined(),
    };

    match serde_json::from_str::<JsonValue>(&text) {
        Ok(v) => json_to_jsvalue(&v),
        Err(_) => JsValue::undefined(),
    }
}

fn json_to_jsvalue(v: &JsonValue) -> JsValue {
    match v {
        JsonValue::Null => JsValue::null(),
        JsonValue::Bool(b) => JsValue::bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    JsValue::int32(i as i32)
                } else {
                    JsValue::float64(i as f64)
                }
            } else if let Some(f) = n.as_f64() {
                JsValue::float64(f)
            } else {
                JsValue::float64(f64::NAN)
            }
        }
        JsonValue::String(s) => {
            let id = intern_string(s.clone());
            JsValue::string(id as u64)
        }
        JsonValue::Array(arr) => {
            let obj = alloc_array();
            for (i, item) in arr.iter().enumerate() {
                let key = intern_string(i.to_string());
                set_prop(obj, JsValue::string(key as u64), json_to_jsvalue(item));
            }
            obj
        }
        JsonValue::Object(map) => {
            let obj = alloc_object();
            for (k, v) in map {
                let key = intern_string(k.clone());
                set_prop(obj, JsValue::string(key as u64), json_to_jsvalue(v));
            }
            obj
        }
    }
}

fn array_get_index(obj: &JsObject, idx: usize) -> JsValue {
    if (idx as u32) < obj.props_len {
        unsafe { *obj.props.add(idx) }
    } else {
        JsValue::undefined()
    }
}

fn array_set_index(obj: &mut JsObject, idx: usize, value: JsValue) {
    let new_len = idx + 1;
    ensure_props_capacity(obj, new_len);
    unsafe {
        *obj.props.add(idx) = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_transitions() {
        let obj1 = alloc_object();
        let obj2 = alloc_object();

        // Inicialmente vazios, devem ter o mesmo shape
        assert_eq!(object_shape_id(obj1), object_shape_id(obj2));

        let prop_a = JsValue::string(intern_string("a".to_string()) as u64);
        let prop_b = JsValue::string(intern_string("b".to_string()) as u64);

        // obj1: {a: 1}
        set_prop(obj1, prop_a, JsValue::int32(1));
        let shape_a = object_shape_id(obj1).unwrap();

        // obj2: {a: 2} -> devem ter o mesmo shape (mesma estrutura)
        set_prop(obj2, prop_a, JsValue::int32(2));
        assert_eq!(object_shape_id(obj2).unwrap(), shape_a);

        // obj1: {a: 1, b: 3}
        set_prop(obj1, prop_b, JsValue::int32(3));
        let shape_ab = object_shape_id(obj1).unwrap();
        assert_ne!(shape_ab, shape_a);

        // obj2: {a: 2, b: 4} -> devem ter o mesmo shape
        set_prop(obj2, prop_b, JsValue::int32(4));
        assert_eq!(object_shape_id(obj2).unwrap(), shape_ab);
    }

    #[test]
    fn test_shape_divergence() {
        let obj_ab = alloc_object();
        let obj_ba = alloc_object();

        let prop_a = JsValue::string(intern_string("a".to_string()) as u64);
        let prop_b = JsValue::string(intern_string("b".to_string()) as u64);

        // Diferentes ordens de inserção geram diferentes shapes (como no V8)
        set_prop(obj_ab, prop_a, JsValue::int32(1));
        set_prop(obj_ab, prop_b, JsValue::int32(2));

        set_prop(obj_ba, prop_b, JsValue::int32(2));
        set_prop(obj_ba, prop_a, JsValue::int32(1));

        assert_ne!(object_shape_id(obj_ab), object_shape_id(obj_ba));
    }

    #[test]
    fn test_get_prop_offset() {
        let obj = alloc_object();
        let prop_x = JsValue::string(intern_string("x".to_string()) as u64);
        set_prop(obj, prop_x, JsValue::int32(100));

        let shape_id = object_shape_id(obj).unwrap();
        let prop_id = prop_x.as_string_id() as u32;

        let offset = shape_offset(shape_id, prop_id);
        assert!(offset.is_some());

        // O valor deve estar no offset correto
        unsafe {
            let internal_obj = &*(obj.as_object_ptr() as *const JsObject);
            let val = *internal_obj.props.add(offset.unwrap() as usize);
            assert_eq!(val.as_int32(), 100);
        }
    }
}
