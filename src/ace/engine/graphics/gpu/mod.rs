// ARQUIVO: src/graphics/gpu.rs

use std::borrow::Cow;
use wgpu::util::DeviceExt;

// Representa 1 retângulo HTML para ser instanciado na GPU

pub mod quadinstance; pub use quadinstance::*;
pub mod slice_as_bytes; pub use slice_as_bytes::*;
pub mod gpucontext; pub use gpucontext::*;
pub mod gpucontext_impl_1; pub use gpucontext_impl_1::*;
pub mod gpucontext_impl_2; pub use gpucontext_impl_2::*;
