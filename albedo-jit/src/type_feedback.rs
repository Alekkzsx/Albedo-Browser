//! # Type Feedback & Inline Caches
//!
//! Coleta feedback de tipos em tempo de execução para guiar o Tier 2.

use std::sync::OnceLock;
use parking_lot::RwLock;

use hashbrown::HashMap;

use crate::js_value::JsValue;
use crate::object_model;

// ---------------------------------------------------------------------------
// Tipos e Estados
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    Int32,
    Float64,
    String,
    Object,
    Array,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcState {
    Uninitialized,
    Monomorphic,
    Polymorphic,
    Megamorphic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcKind {
    Add,
    GetProp,
    Call,
}

// ---------------------------------------------------------------------------
// Feedback Add
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypePair(pub ValueType, pub ValueType);

#[derive(Debug, Default, Clone)]
struct AddFeedback {
    total: u64,
    counts: HashMap<TypePair, u64>,
}

impl AddFeedback {
    fn record(&mut self, lhs: ValueType, rhs: ValueType) {
        let key = TypePair(lhs, rhs);
        *self.counts.entry(key).or_insert(0) += 1;
        self.total += 1;
    }

    fn snapshot(&self) -> AddFeedbackSnapshot {
        let unique = self.counts.len();
        let state = match unique {
            0 => IcState::Uninitialized,
            1 => IcState::Monomorphic,
            2..=4 => IcState::Polymorphic,
            _ => IcState::Megamorphic,
        };
        let mono = if unique == 1 {
            self.counts.keys().next().copied()
        } else {
            None
        };
        AddFeedbackSnapshot {
            total: self.total,
            state,
            monomorphic: mono,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddFeedbackSnapshot {
    pub total: u64,
    pub state: IcState,
    pub monomorphic: Option<TypePair>,
}

// ---------------------------------------------------------------------------
// Feedback GetProp
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropKey {
    pub shape_id: u64,
    pub prop_id: u32,
}

#[derive(Debug, Default, Clone)]
struct GetPropFeedback {
    total: u64,
    counts: HashMap<PropKey, u64>,
}

impl GetPropFeedback {
    fn record(&mut self, key: PropKey) {
        *self.counts.entry(key).or_insert(0) += 1;
        self.total += 1;
    }

    fn snapshot(&self) -> GetPropFeedbackSnapshot {
        let unique = self.counts.len();
        let state = match unique {
            0 => IcState::Uninitialized,
            1 => IcState::Monomorphic,
            2..=4 => IcState::Polymorphic,
            _ => IcState::Megamorphic,
        };
        let mono_key = if unique == 1 {
            self.counts.keys().next().copied()
        } else {
            None
        };
        let mono = mono_key.and_then(|k| {
            object_model::shape_offset(k.shape_id, k.prop_id).map(|offset| {
                GetPropMonomorphic {
                    shape_id: k.shape_id,
                    prop_id: k.prop_id,
                    offset,
                }
            })
        });
        GetPropFeedbackSnapshot {
            total: self.total,
            state,
            monomorphic: mono,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GetPropMonomorphic {
    pub shape_id: u64,
    pub prop_id: u32,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub struct GetPropFeedbackSnapshot {
    pub total: u64,
    pub state: IcState,
    pub monomorphic: Option<GetPropMonomorphic>,
}

// ---------------------------------------------------------------------------
// Feedback Call
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
struct CallFeedback {
    total: u64,
    counts: HashMap<ValueType, u64>,
}

impl CallFeedback {
    fn record(&mut self, callee: ValueType) {
        *self.counts.entry(callee).or_insert(0) += 1;
        self.total += 1;
    }

    fn snapshot(&self) -> CallFeedbackSnapshot {
        let unique = self.counts.len();
        let state = match unique {
            0 => IcState::Uninitialized,
            1 => IcState::Monomorphic,
            2..=4 => IcState::Polymorphic,
            _ => IcState::Megamorphic,
        };
        CallFeedbackSnapshot {
            total: self.total,
            state,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallFeedbackSnapshot {
    pub total: u64,
    pub state: IcState,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

struct IcSlot {
    kind: IcKind,
    add: RwLock<AddFeedback>,
    get_prop: RwLock<GetPropFeedback>,
    call: RwLock<CallFeedback>,
}

impl IcSlot {
    fn new(kind: IcKind) -> Self {
        Self {
            kind,
            add: RwLock::new(AddFeedback::default()),
            get_prop: RwLock::new(GetPropFeedback::default()),
            call: RwLock::new(CallFeedback::default()),
        }
    }
}

#[derive(Default)]
pub struct TypeFeedbackRegistry {
    slots: RwLock<Vec<IcSlot>>,
}

impl TypeFeedbackRegistry {
    pub fn global() -> &'static Self {
        static REG: OnceLock<TypeFeedbackRegistry> = OnceLock::new();
        REG.get_or_init(|| TypeFeedbackRegistry::default())
    }

    pub fn alloc_slot(kind: IcKind) -> u32 {
        let reg = Self::global();
        let mut slots = reg.slots.write();
        let id = slots.len() as u32;
        slots.push(IcSlot::new(kind));
        id
    }

    pub fn record_add(slot: u32, lhs: JsValue, rhs: JsValue) {
        if let Some(entry) = Self::global().slots.read().get(slot as usize) {
            if entry.kind != IcKind::Add {
                return;
            }
            let lhs_t = value_type(lhs);
            let rhs_t = value_type(rhs);
            entry.add.write().record(lhs_t, rhs_t);
        }
    }

    pub fn record_get_prop(slot: u32, obj: JsValue, prop: JsValue) {
        if let Some(entry) = Self::global().slots.read().get(slot as usize) {
            if entry.kind != IcKind::GetProp {
                return;
            }
            let shape_id = object_model::object_shape_id(obj);
            let prop_id = if prop.is_string() {
                Some(prop.as_string_id() as u32)
            } else {
                None
            };
            if let (Some(shape_id), Some(prop_id)) = (shape_id, prop_id) {
                entry.get_prop.write().record(PropKey { shape_id, prop_id });
            }
        }
    }

    pub fn record_call(slot: u32, callee: JsValue) {
        if let Some(entry) = Self::global().slots.read().get(slot as usize) {
            if entry.kind != IcKind::Call {
                return;
            }
            let callee_t = value_type(callee);
            entry.call.write().record(callee_t);
        }
    }

    pub fn add_snapshot(slot: u32) -> Option<AddFeedbackSnapshot> {
        Self::global().slots.read().get(slot as usize).map(|e| e.add.read().snapshot())
    }

    pub fn get_prop_snapshot(slot: u32) -> Option<GetPropFeedbackSnapshot> {
        Self::global().slots.read().get(slot as usize).map(|e| e.get_prop.read().snapshot())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn value_type(v: JsValue) -> ValueType {
    if v.is_int32() {
        ValueType::Int32
    } else if v.is_float64() {
        ValueType::Float64
    } else if v.is_string() {
        ValueType::String
    } else if v.is_builtin() {
        ValueType::Other
    } else if v.is_object() {
        if object_model::is_array(v) {
            ValueType::Array
        } else {
            ValueType::Object
        }
    } else {
        ValueType::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // AddFeedback
    // ---------------------------------------------------------------

    #[test]
    fn test_add_feedback_uninitialized() {
        let fb = AddFeedback::default();
        let snap = fb.snapshot();
        assert_eq!(snap.state, IcState::Uninitialized);
        assert_eq!(snap.total, 0);
        assert!(snap.monomorphic.is_none());
    }

    #[test]
    fn test_add_feedback_monomorphic_int32() {
        let mut fb = AddFeedback::default();
        for _ in 0..10 {
            fb.record(ValueType::Int32, ValueType::Int32);
        }
        let snap = fb.snapshot();
        assert_eq!(snap.state, IcState::Monomorphic);
        assert_eq!(snap.total, 10);
        assert_eq!(snap.monomorphic, Some(TypePair(ValueType::Int32, ValueType::Int32)));
    }

    #[test]
    fn test_add_feedback_polymorphic() {
        let mut fb = AddFeedback::default();
        fb.record(ValueType::Int32, ValueType::Int32);
        fb.record(ValueType::Float64, ValueType::Float64);
        let snap = fb.snapshot();
        assert_eq!(snap.state, IcState::Polymorphic);
        assert!(snap.monomorphic.is_none());
    }

    #[test]
    fn test_add_feedback_megamorphic() {
        let mut fb = AddFeedback::default();
        fb.record(ValueType::Int32, ValueType::Int32);
        fb.record(ValueType::Float64, ValueType::Float64);
        fb.record(ValueType::String, ValueType::String);
        fb.record(ValueType::Int32, ValueType::Float64);
        fb.record(ValueType::Float64, ValueType::Int32);
        let snap = fb.snapshot();
        assert_eq!(snap.state, IcState::Megamorphic);
        assert!(snap.monomorphic.is_none());
    }

    // ---------------------------------------------------------------
    // GetPropFeedback
    // ---------------------------------------------------------------

    #[test]
    fn test_getprop_feedback_monomorphic() {
        let mut fb = GetPropFeedback::default();
        let key = PropKey { shape_id: 1, prop_id: 42 };
        for _ in 0..5 {
            fb.record(key);
        }
        let snap = fb.snapshot();
        assert_eq!(snap.state, IcState::Monomorphic);
        assert_eq!(snap.total, 5);
        // monomorphic pode ser None se o shape não existir no registry,
        // o que é esperado num teste isolado
    }

    #[test]
    fn test_getprop_feedback_polymorphic() {
        let mut fb = GetPropFeedback::default();
        fb.record(PropKey { shape_id: 1, prop_id: 10 });
        fb.record(PropKey { shape_id: 2, prop_id: 10 });
        fb.record(PropKey { shape_id: 3, prop_id: 10 });
        let snap = fb.snapshot();
        assert_eq!(snap.state, IcState::Polymorphic);
    }

    // ---------------------------------------------------------------
    // CallFeedback
    // ---------------------------------------------------------------

    #[test]
    fn test_call_feedback_monomorphic() {
        let mut fb = CallFeedback::default();
        for _ in 0..8 {
            fb.record(ValueType::Object);
        }
        let snap = fb.snapshot();
        assert_eq!(snap.state, IcState::Monomorphic);
        assert_eq!(snap.total, 8);
    }

    #[test]
    fn test_call_feedback_megamorphic() {
        let mut fb = CallFeedback::default();
        fb.record(ValueType::Object);
        fb.record(ValueType::Int32);
        fb.record(ValueType::Float64);
        fb.record(ValueType::String);
        fb.record(ValueType::Array);
        let snap = fb.snapshot();
        assert_eq!(snap.state, IcState::Megamorphic);
    }

    // ---------------------------------------------------------------
    // value_type helper
    // ---------------------------------------------------------------

    #[test]
    fn test_value_type_classification() {
        assert_eq!(value_type(JsValue::int32(42)), ValueType::Int32);
        assert_eq!(value_type(JsValue::float64(3.14)), ValueType::Float64);
        assert_eq!(value_type(JsValue::string(0)), ValueType::String);
        assert_eq!(value_type(JsValue::bool(true)), ValueType::Other);
        assert_eq!(value_type(JsValue::undefined()), ValueType::Other);
        assert_eq!(value_type(JsValue::null()), ValueType::Other);
    }
}
