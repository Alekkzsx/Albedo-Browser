//! # Subsistema de Amarrações WebIDL e Interoperabilidade com Motores JS
//!
//! Conexão normativa entre o DOM em arena do Rust e ambientes JavaScript (V8 / SpiderMonkey / ace_js).

pub mod interfaces;
pub mod webidl;
pub mod wrapper;

pub use interfaces::{DocumentBindings, ElementBindings, NodeBindings};
pub use webidl::{JSValue, WebIDLException, WebIDLResult};
pub use wrapper::{DOMDataStore, DOMWrapper, JSObjectId};
