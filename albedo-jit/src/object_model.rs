//! # Minimal Object Model (JIT)
//!
//! Modelo mínimo de objetos/arrays com "hidden class" (shape) para suportar
//! inline caches e specialization no Tier 2.

use std::collections::HashMap;
use std::mem::offset_of;
use std::sync::OnceLock;
use parking_lot::RwLock;

use crate::js_value::JsValue;

// ---------------------------------------------------------------------------
// Layout do Objeto (estável para geração de código)
// ---------------------------------------------------------------------------

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Object = 0,
    Array = 1,
}

#[repr(C)]
pub struct JsObject {
    pub shape_id: u64,
    pub kind: u32,
    pub _pad: u32,
    pub props: *mut JsValue,
    pub props_len: u32,
    pub props_cap: u32,
}

pub const JSOBJ_SHAPE_OFFSET: u32 = offset_of!(JsObject, shape_id) as u32;
pub const JSOBJ_KIND_OFFSET: u32 = offset_of!(JsObject, kind) as u32;
pub const JSOBJ_PROPS_OFFSET: u32 = offset_of!(JsObject, props) as u32;
pub const JSOBJ_PROPS_LEN_OFFSET: u32 = offset_of!(JsObject, props_len) as u32;

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
    id: u64,         // hash of keys+order
    parent: Option<u64>,
    last_prop: Option<u32>,
    props: Vec<u32>, // property ids in order (offset = index)
    prop_index: HashMap<u32, usize>,
}

impl Shape {
    fn new_empty() -> Self {
        Self {
            id: fnv1a_seed(),
            parent: None,
            last_prop: None,
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
        self.shapes.get(&shape_id)
            .and_then(|s| s.prop_index.get(&prop_id).copied())
            .map(|v| v as u32)
    }

    fn transition(&mut self, from_shape: u64, prop_id: u32) -> u64 {
        if let Some(next) = self.transitions.get(&(from_shape, prop_id)) {
            return *next;
        }

        let base = self.shapes.get(&from_shape)
            .expect("Shape inválido");

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
            parent: Some(from_shape),
            last_prop: Some(prop_id),
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
        _pad: 0,
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
        _pad: 0,
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
    if obj.props_cap as usize >= new_len {
        return;
    }

    let mut new_cap = if obj.props_cap == 0 { 4 } else { obj.props_cap * 2 };
    while (new_cap as usize) < new_len {
        new_cap *= 2;
    }

    let mut vec: Vec<JsValue> = if obj.props.is_null() {
        Vec::with_capacity(new_cap as usize)
    } else {
        unsafe { Vec::from_raw_parts(obj.props, obj.props_len as usize, obj.props_cap as usize) }
    };

    vec.resize(new_len, JsValue::undefined());
    obj.props_len = new_len as u32;
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
                return JsValue::int32(obj.props_len as i32);
            }
            if let Some(idx) = parse_array_index(prop_val) {
                return array_get_index(obj, idx);
            }
        }
        let reg = shape_registry().read();
        if let Some(offset) = reg.get_offset(obj.shape_id, prop_id) {
            if (offset as u32) < obj.props_len {
                return *obj.props.add(offset as usize);
            }
        }
        JsValue::undefined()
    }
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
