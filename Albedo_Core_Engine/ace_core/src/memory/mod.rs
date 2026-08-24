//! # Gestão de Memória, Pressão do Sistema e IPC
//!
//! Barramentos de evento de pressão de memória (`MemoryPressureListener`), ponteiros fracos anti-ciclo (`WeakPtr`),
//! telemetria recursiva de heap (`MallocSizeOf`), memória compartilhada (`SharedMemoryRegion`),
//! amostragem protegida contra corrupção (`GwpAsanPool`) e ponteiros etiquetados com nicho (`TaggedPointer`).

pub mod gwp_asan;
pub mod pressure;
pub mod shared_memory;
pub mod size_of;
pub mod tagged_ptr;
pub mod tracing;
pub mod weak_ptr;

pub use gwp_asan::GwpAsanPool;
pub use pressure::{MemoryPressureLevel, MemoryPressureListener};
pub use shared_memory::{
    ReadOnlySharedMemoryMapping, ReadOnlySharedMemoryRegion, WritableSharedMemoryMapping,
    WritableSharedMemoryRegion,
};
pub use size_of::MallocSizeOf;
pub use tagged_ptr::{NodeTag, TaggedPointer};
pub use tracing::{GCRoot, RootSet, Traceable, Visitor};
pub use weak_ptr::{LocalWeakPtr, LocalWeakPtrFactory, WeakPtr, WeakPtrFactory};
