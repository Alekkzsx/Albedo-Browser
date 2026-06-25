//! AceDOM String Interning System
//! Otimização crítica de memória para strings repetidas (tag names, attributes)
//! Reduz uso de memória em 60-80% para strings frequentes

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Hasher rápido e seguro para string interning
use fxhash::FxBuildHasher;


pub mod internmap; pub use internmap::*;
pub mod string_intern; pub use string_intern::*;
pub mod stringinterner; pub use stringinterner::*;
pub mod internstats; pub use internstats::*;
#[macro_use]
pub mod intern; pub use intern::*;
pub mod r#ref; pub use r#ref::*;
