use super::*;
/// Um Pool de Texturas dinâmicas para reciclar Tiles já rasterizados na VRAM.


impl GpuCompositor {

    /// TODO: add docs
    pub async fn render_tree(
        &mut self,
        layer_tree: &crate::ace::engine::layer_tree::LayerTree,
        physical_w: u32,
        physical_h: u32,
        scale_factor: f32,
        font_system_arc: std::sync::Arc<std::sync::Mutex<cosmic_text::FontSystem>>,
        swash_cache_arc: std::sync::Arc<std::sync::Mutex<cosmic_text::SwashCache>>,
    ) -> Option<Vec<u8>> {
        if physical_w == 0 || physical_h == 0 {
            return None;
        }

        let extent = wgpu::Extent3d {
            width: physical_w,
            height: physical_h,
            depth_or_array_layers: 1,
        };

        let target_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Target Texture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Buffer to read back the pixels
        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = physical_w * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: (padded_bytes_per_row * physical_h) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Generate Ortho Camera Matrix
        let transform = [
            [2.0 / physical_w as f32, 0.0, 0.0, 0.0],
            [0.0, -2.0 / physical_h as f32, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ];

        let camera_uniform = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&camera_uniform, 0, slice_as_bytes(&[transform]));

        let camera_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &camera_bind_group, &[]);

            // Process and Draw each Tile
            include!("compositor_tiles.rs");
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(physical_h),
                },
            },
            extent,
        );

        self.queue.submit(Some(encoder.finish()));

        // Map and Read
        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), wgpu::BufferAsyncError>>();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });

        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).expect("Albedo Engine: internal invariant violated"); // Block until done
        if let Ok(Ok(())) = rx.await {
            let data = buffer_slice.get_mapped_range();
            let mut final_pixels = vec![0; (physical_w * physical_h * 4) as usize];

            for y in 0..physical_h {
                let src_start = (y * padded_bytes_per_row) as usize;
                let src_end = src_start + unpadded_bytes_per_row as usize;
                let dst_start = (y * unpadded_bytes_per_row) as usize;
                let dst_end = dst_start + unpadded_bytes_per_row as usize;

                final_pixels[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
            }

            drop(data);
            output_buffer.unmap();

            // Swap BGRA to RGBA if required depending on target texture format
            /* format is Rgba8UnormSrgb, no swapping needed! */

            return Some(final_pixels);
        }

        None
    }
}
