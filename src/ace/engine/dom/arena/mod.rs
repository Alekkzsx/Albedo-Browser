//! DomArena - Arena de alocação para AceDOM nodes
//! 
//! Implementa alocação em blocos de 4KB para melhor performance
//! e memória cache-friendly.

use std::alloc::{self, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// Tamanho do bloco da arena em bytes (4KB)

pub mod block_size; pub use block_size::*;
pub mod cache_line_align; pub use cache_line_align::*;
pub mod initial_capacity; pub use initial_capacity::*;
pub mod arenanode; pub use arenanode::*;
pub mod arenablock; pub use arenablock::*;
pub mod domarena; pub use domarena::*;
pub mod arenamemorystats; pub use arenamemorystats::*;
pub mod arenapoolsizes; pub use arenapoolsizes::*;
pub mod test_basic_allocation; pub use test_basic_allocation::*;
