use super::*;
// AceDOM String Interning System
// Otimização crítica de memória para strings repetidas (tag names, attributes)
// Reduz uso de memória em 60-80% para strings frequentes

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Hasher rápido e seguro para string interning
use fxhash::FxBuildHasher;


pub(crate) type InternMap = HashMap<Arc<str>, usize, FxBuildHasher>;
