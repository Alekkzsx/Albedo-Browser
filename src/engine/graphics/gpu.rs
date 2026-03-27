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

fn slice_as_bytes<T>(slice: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            std::mem::size_of_val(slice),
        )
    }
}

impl QuadInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x2, // size
        2 => Float32x4, // color
        3 => Float32x4, // border_radius
        // z_index e _pad não transferidos ao Vertex (omitidos intencionalmente por performance)
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<QuadInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance, // O SEGRED0: avança a cada Rect, não vértice
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    texture_width: u32,
    texture_height: u32,
}

impl GpuContext {
    pub async fn new(width: u32, height: u32) -> Option<Self> {
        println!("GPU: Inicializando WGPU Instanced Renderer...");

        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Albedo GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .ok()?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Albedo Instanced Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(r#"
                struct ScreenUniforms {
                    screen_width: f32,
                    screen_height: f32,
                };

                // Enviado por 'push_constants' se a placha gráfica aguentar, ou convertido puramente 
                // para resolver no JS. Como hardcode pra simplicidade:
                var<private> screen_width: f32 = 800.0;
                var<private> screen_height: f32 = 600.0;

                struct InstanceInput {
                    @location(0) position: vec2<f32>,
                    @location(1) size: vec2<f32>,
                    @location(2) color: vec4<f32>,
                    @location(3) border_radius: vec4<f32>,
                };

                struct VertexOutput {
                    @builtin(position) clip_position: vec4<f32>,
                    @location(0) color: vec4<f32>,
                    @location(1) local_pos: vec2<f32>,
                    @location(2) size: vec2<f32>,
                    @location(3) radii: vec4<f32>,
                };

                @vertex
                fn vs_main(
                    @builtin(vertex_index) vertex_index: u32,
                    instance: InstanceInput
                ) -> VertexOutput {
                    var out: VertexOutput;

                    // Um quad plano (Triângulo duplo 0..6 vértices)
                    var pos = array<vec2<f32>, 6>(
                        vec2<f32>(0.0, 0.0), // top-left
                        vec2<f32>(1.0, 0.0), // top-right
                        vec2<f32>(1.0, 1.0), // bottom-right
                        vec2<f32>(0.0, 0.0), // top-left
                        vec2<f32>(1.0, 1.0), // bottom-right
                        vec2<f32>(0.0, 1.0)  // bottom-left
                    );
                    
                    let p = pos[vertex_index];
                    
                    // Pixels -> Clip Space (-1 a 1)
                    let absolute_x = instance.position.x + p.x * instance.size.x;
                    let absolute_y = instance.position.y + p.y * instance.size.y;
                    
                    // Assumiremos default de textura temporário para cálculo global
                    let clip_x = (absolute_x / 800.0) * 2.0 - 1.0;
                    let clip_y = 1.0 - (absolute_y / 600.0) * 2.0;

                    out.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
                    out.color = instance.color;
                    out.local_pos = p * instance.size; // Pixel coords relativos à caixa
                    out.size = instance.size;
                    out.radii = instance.border_radius; // tl, tr, br, bl
                    
                    return out;
                }

                // Função de Rounded Box (Signed Distance)
                fn sdRoundRect(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
                    let d = abs(p - b * 0.5) - b * 0.5 + vec2<f32>(r);
                    return min(max(d.x, d.y), 0.0) + length(max(d, vec2<f32>(0.0))) - r;
                }

                @fragment
                fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                    // Escolher raio apropriado baseado no quadrante do pixel. (Simples aproximação)
                    // Para precisão milimétrica o SDF seria feito por quadrante.
                    // Aqui usamos o topLeft por padrão para testes:
                    let r = in.radii.x; 

                    if (r > 0.0) {
                        let center_bias = in.size * 0.5;
                        // local_pos do meio = (0,0) ate in.size. Movemos a origem pro meio:
                        let dist = sdRoundRect(in.local_pos, in.size, r);
                        if (dist > 0.5) {
                            discard; // Anti-Aliasing rudimentar (borda suavizada na versao avancada usar smoothstep)
                        }
                    }

                    return in.color;
                }
            "#)),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Albedo Instanced Pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[QuadInstance::desc()], // Array instanciado no pipeline
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Transparencia permitida
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Some(Self {
            device,
            queue,
            render_pipeline,
            texture_width: width,
            texture_height: height,
        })
    }

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
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            // Ligações: instâncias + quantidade a desenhar
            render_pass.set_vertex_buffer(0, instance_buffer.slice(..));
            // 6 vértices (2 tris de formacao natural de quad) iterados pelo número de boxes (instancias)
            render_pass.draw(0..6, 0..instances.len() as u32);
        }

        // Texture To CPU RAM Buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
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
            tx.send(v).unwrap();
        });

        self.device.poll(wgpu::Maintain::Wait);

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
