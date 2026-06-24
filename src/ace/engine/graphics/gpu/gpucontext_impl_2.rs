use super::*;
// ARQUIVO: src/graphics/gpu.rs

use std::borrow::Cow;
use wgpu::util::DeviceExt;

// Representa 1 retângulo HTML para ser instanciado na GPU


impl GpuContext {

    /// TODO: add docs
    pub fn resize(&mut self, width: u32, height: u32) {
        self.texture_width = width;
        self.texture_height = height;
    }

    /// Renderiza um batch de instâncias
    pub async fn render_instanced_to_image(&self, instances: &[QuadInstance]) -> Vec<u8> {
        let size = wgpu::Extent3d {
            width: self.texture_width,
            height: self.texture_height,
            depth_or_array_layers: 1,
        };

        // Textura offscreen para Slint
        let texture_desc = wgpu::TextureDescriptor {
            label: Some("Offscreen WGPU Target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        let texture = self.device.create_texture(&texture_desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let u32_size = std::mem::size_of::<u32>() as u32;
        let output_buffer_size =
            (u32_size * self.texture_width * self.texture_height) as wgpu::BufferAddress;

        // Output Readback Buffer
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: Some("Readback Buffer"),
            mapped_at_creation: false,
        });

        // Vertex Buffer Dinâmico
        let instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ACE Rect Instance Buffer"),
                contents: slice_as_bytes(instances),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ACE Instance Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }), // Fundo Branco Page
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            // Ligações: instâncias + quantidade a desenhar
            render_pass.set_vertex_buffer(0, instance_buffer.slice(..));
            // 6 vértices (2 tris de formacao natural de quad) iterados pelo número de boxes (instancias)
            render_pass.draw(0..6, 0..instances.len() as u32);
        }

        // Texture To CPU RAM Buffer
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * self.texture_width), // 4 bytes (RGBA8) por pixel
                    rows_per_image: Some(self.texture_height),
                },
            },
            size,
        );

        self.queue.submit(Some(encoder.finish()));

        // Polling Seguro CPU Side (Non-blocking trait rustico)
        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            tx.send(v).expect("Albedo Engine: internal invariant violated");
        });
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).expect("Albedo Engine: internal invariant violated"); // Block until done

        if let Ok(Ok(())) = rx.recv() {
            let data = buffer_slice.get_mapped_range();
            let result = data.to_vec();
            drop(data);
            output_buffer.unmap();
            return result;
        }

        vec![]
    }
}
