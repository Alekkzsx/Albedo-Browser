            for layer in layer_tree.get_all_layers_sorted() {
                let tiles = layer.build_tiles(512);

                for tile in tiles {
                    // Rasterize Tile via Skia on CPU (Caching would prevent this step if already rasterized)
                    let tile_w = 512;
                    let tile_h = 512;
                    let pixmap = tiny_skia::Pixmap::new(tile_w, tile_h).expect("Albedo Engine: internal invariant violated");
                    let mut fb_lock = Some(pixmap);

                    {
                        let mut font_system_lock = font_system_arc.lock().unwrap_or_else(|e| e.into_inner());
                        let mut swash_cache_lock = swash_cache_arc.lock().unwrap_or_else(|e| e.into_inner());
                        crate::renderer::paint_layout_tree(
                            &tile,
                            physical_w,
                            physical_h,
                            scale_factor,
                            &mut font_system_lock,
                            &mut swash_cache_lock,
                            &mut fb_lock,
                            &[],
                        );
                    }

                    let rasterized = fb_lock.expect("Albedo Engine: internal invariant violated");

                    // Upload to wgpu texture
                    let tile_texture = self.texture_pool.get_or_create(&self.device);
                    let tex_size = wgpu::Extent3d {
                        width: tile_w,
                        height: tile_h,
                        depth_or_array_layers: 1,
                    };

                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &tile_texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        rasterized.data(),
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * tile_w),
                            rows_per_image: Some(tile_h),
                        },
                        tex_size,
                    );

                    let view = tile_texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &self.texture_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                            },
                        ],
                        label: Some("Tile Bind Group"),
                    });

                    // Build Quad for Tile
                    let pos_x = (tile.x as f32) * scale_factor;
                    let pos_y = (tile.y as f32) * scale_factor;
                    let w = tile_w as f32;
                    let h = tile_h as f32;

                    let vertices = [
                        Vertex {
                            position: [pos_x, pos_y, 0.0],
                            uv: [0.0, 0.0],
                        },
                        Vertex {
                            position: [pos_x + w, pos_y, 0.0],
                            uv: [1.0, 0.0],
                        },
                        Vertex {
                            position: [pos_x + w, pos_y + h, 0.0],
                            uv: [1.0, 1.0],
                        },
                        Vertex {
                            position: [pos_x, pos_y, 0.0],
                            uv: [0.0, 0.0],
                        },
                        Vertex {
                            position: [pos_x + w, pos_y + h, 0.0],
                            uv: [1.0, 1.0],
                        },
                        Vertex {
                            position: [pos_x, pos_y + h, 0.0],
                            uv: [0.0, 1.0],
                        },
                    ];

                    let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Quad Vertex Buffer"),
                        size: (vertices.len() * std::mem::size_of::<Vertex>()) as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    self.queue
                        .write_buffer(&vertex_buffer, 0, slice_as_bytes(&vertices));

                    rpass.set_bind_group(1, &bind_group, &[]);
                    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    rpass.draw(0..6, 0..1);

                    // Recyble Texture
                    self.texture_pool.release(tile_texture);
                }
            }
