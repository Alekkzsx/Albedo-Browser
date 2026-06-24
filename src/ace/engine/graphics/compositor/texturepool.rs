use super::*;
/// Um Pool de Texturas dinâmicas para reciclar Tiles já rasterizados na VRAM.

pub struct TexturePool {
    free_textures: Vec<wgpu::Texture>,
    pub tile_size: u32,
}

impl TexturePool {
    /// TODO: add docs
    pub fn new(tile_size: u32) -> Self {
        Self {
            free_textures: Vec::new(),
            tile_size,
        }
    }

    /// TODO: add docs
    pub fn get_or_create(&mut self, device: &wgpu::Device) -> wgpu::Texture {
        self.free_textures.pop().unwrap_or_else(|| {
            let size = wgpu::Extent3d {
                width: self.tile_size,
                height: self.tile_size,
                depth_or_array_layers: 1,
            };
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Tile Texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        })
    }

    /// TODO: add docs
    pub fn release(&mut self, texture: wgpu::Texture) {
        self.free_textures.push(texture);
    }
}
