/// Um Pool de Texturas dinâmicas para reciclar Tiles já rasterizados na VRAM.
pub struct TexturePool {
    free_textures: Vec<wgpu::Texture>,
    pub tile_size: u32,
}

impl TexturePool {
    pub fn new(tile_size: u32) -> Self {
        Self {
            free_textures: Vec::new(),
            tile_size,
        }
    }

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

    pub fn release(&mut self, texture: wgpu::Texture) {
        self.free_textures.push(texture);
    }
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

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

impl GpuCompositor {
    pub async fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Nós pedimos por adaptador de Alta Performance (GPU dedicada se possível)
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("AceEngine GPU Compositor"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .ok()?;

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("WGSL Core Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING), // src_over (premul/unpremul as needed)
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Some(Self {
            instance,
            adapter,
            device,
            queue,
            texture_bind_group_layout,
            camera_bind_group_layout,
            render_pipeline,
            default_sampler,
            texture_pool: TexturePool::new(512),
        })
    }

    pub async fn render_tree(
        &mut self,
        layer_tree: &crate::engine::layer_tree::LayerTree,
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
            .write_buffer(&camera_uniform, 0, bytemuck::cast_slice(&[transform]));

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
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &camera_bind_group, &[]);

            // Process and Draw each Tile
            for layer in layer_tree.get_all_layers_sorted() {
                let tiles = layer.build_tiles(512);

                for tile in tiles {
                    // Rasterize Tile via Skia on CPU (Caching would prevent this step if already rasterized)
                    let tile_w = 512;
                    let tile_h = 512;
                    let pixmap = tiny_skia::Pixmap::new(tile_w, tile_h).unwrap();
                    let mut fb_lock = Some(pixmap);

                    {
                        let mut font_system_lock = font_system_arc.lock().unwrap();
                        let mut swash_cache_lock = swash_cache_arc.lock().unwrap();
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

                    let rasterized = fb_lock.unwrap();

                    // Upload to wgpu texture
                    let tile_texture = self.texture_pool.get_or_create(&self.device);
                    let tex_size = wgpu::Extent3d {
                        width: tile_w,
                        height: tile_h,
                        depth_or_array_layers: 1,
                    };

                    self.queue.write_texture(
                        wgpu::ImageCopyTexture {
                            texture: &tile_texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        rasterized.data(),
                        wgpu::ImageDataLayout {
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
                    let x = (tile.x as f32) * scale_factor;
                    let y = (tile.y as f32) * scale_factor;
                    let w = tile_w as f32;
                    let h = tile_h as f32;

                    let vertices = [
                        Vertex {
                            position: [x, y, 0.0],
                            uv: [0.0, 0.0],
                        },
                        Vertex {
                            position: [x + w, y, 0.0],
                            uv: [1.0, 0.0],
                        },
                        Vertex {
                            position: [x + w, y + h, 0.0],
                            uv: [1.0, 1.0],
                        },
                        Vertex {
                            position: [x, y, 0.0],
                            uv: [0.0, 0.0],
                        },
                        Vertex {
                            position: [x + w, y + h, 0.0],
                            uv: [1.0, 1.0],
                        },
                        Vertex {
                            position: [x, y + h, 0.0],
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
                        .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));

                    rpass.set_bind_group(1, &bind_group, &[]);
                    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    rpass.draw(0..6, 0..1);

                    // Recyble Texture
                    self.texture_pool.release(tile_texture);
                }
            }
        }

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &target_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
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

        self.device.poll(wgpu::Maintain::Wait); // Block until done
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
