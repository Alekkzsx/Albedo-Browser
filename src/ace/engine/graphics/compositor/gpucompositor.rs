use super::*;
/// Um Pool de Texturas dinâmicas para reciclar Tiles já rasterizados na VRAM.


pub struct GpuCompositor {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    // Core Render Pass State
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub render_pipeline: wgpu::RenderPipeline,
    pub default_sampler: wgpu::Sampler,

    pub texture_pool: TexturePool,
}
