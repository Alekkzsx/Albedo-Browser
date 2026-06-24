use super::*;
// ARQUIVO: src/graphics/gpu.rs

use std::borrow::Cow;
use wgpu::util::DeviceExt;

// Representa 1 retângulo HTML para ser instanciado na GPU

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QuadInstance {
    pub position: [f32; 2],      // x, y (top-left)
    pub size: [f32; 2],          // width, height
    pub color: [f32; 4],         // r, g, b, a
    pub border_radius: [f32; 4], // tl, tr, br, bl
    pub z_index: f32, // Opcional para profundidade, mas no WGPU com back-to-front array position é suficiente
    pub _pad: [f32; 3], // Padding de alinhamento 16-bytes
}

impl QuadInstance {
pub(crate) const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x2, // size
        2 => Float32x4, // color
        3 => Float32x4, // border_radius
        // z_index e _pad não transferidos ao Vertex (omitidos intencionalmente por performance)
    ];

pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<QuadInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance, // O SEGRED0: avança a cada Rect, não vértice
            attributes: &Self::ATTRIBS,
        }
    }
}
