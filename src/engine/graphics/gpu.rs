// ARQUIVO: src/graphics/gpu.rs

use wgpu::util::DeviceExt;
use std::borrow::Cow;

pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    texture_width: u32,
    texture_height: u32,
}

impl GpuContext {
    // Inicializa a GPU Real. Isso pode demorar 1 ou 2 segundos na primeira vez.
    pub async fn new(width: u32, height: u32) -> Option<Self> {
        println!("GPU: Inicializando WGPU (Hardware)...");

        // 1. Instância (Vulkan/DX12/Metal)
        let instance = wgpu::Instance::default();

        // 2. Adaptador (A Placa de Vídeo física)
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower, // Prioriza bateria/leveza
            force_fallback_adapter: false,
            compatible_surface: None,
        }).await?;

        // 3. Dispositivo Lógico e Fila de Comandos
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Albedo GPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: Default::default(),
            },
            None,
        ).await.ok()?;

        // 4. Shader (Código WGSL que roda NA PLACA DE VÍDEO)
        // Isso desenha um triângulo colorido real.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Albedo Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(r#"
                struct VertexOutput {
                    @builtin(position) clip_position: vec4<f32>,
                    @location(0) color: vec3<f32>,
                };

                @vertex
                fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
                    var out: VertexOutput;
                    // Coordenadas do Triângulo
                    var pos = array<vec2<f32>, 3>(
                        vec2<f32>( 0.0,  0.5),
                        vec2<f32>(-0.5, -0.5),
                        vec2<f32>( 0.5, -0.5)
                    );
                    var colors = array<vec3<f32>, 3>(
                        vec3<f32>(1.0, 0.0, 0.0), // Vermelho
                        vec3<f32>(0.0, 1.0, 0.0), // Verde
                        vec3<f32>(0.0, 0.0, 1.0)  // Azul
                    );
                    out.clip_position = vec4<f32>(pos[in_vertex_index], 0.0, 1.0);
                    out.color = colors[in_vertex_index];
                    return out;
                }

                @fragment
                fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                    return vec4<f32>(in.color, 1.0);
                }
            "#)),
        });

        // 5. Pipeline de Renderização (Como os dados fluem)
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Albedo Pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
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

    /// Renderiza um frame e retorna os bytes brutos da imagem (RGBA)
    /// O Slint vai pegar esses bytes e exibir como uma imagem normal.
    pub async fn render_to_image(&self) -> Vec<u8> {
        let size = wgpu::Extent3d {
            width: self.texture_width,
            height: self.texture_height,
            depth_or_array_layers: 1,
        };

        // 1. Textura na GPU (Alvo de Renderização)
        let texture_desc = wgpu::TextureDescriptor {
            label: Some("Target Texture"),
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

        // 2. Buffer para leitura (Staging Buffer - CPU side)
        // Precisamos alinhar bytes (WGPU exige múltiplos de 256)
        let u32_size = std::mem::size_of::<u32>() as u32;
        let output_buffer_size = (u32_size * self.texture_width * self.texture_height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: Some("Output Buffer"),
            mapped_at_creation: false,
        };
        let output_buffer = self.device.create_buffer(&output_buffer_desc);

        // 3. Encoder de Comandos
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 }), // Fundo Cinza Escuro
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..3, 0..1); // Desenha 3 vértices (Triângulo)
        }

        // Copia Textura GPU -> Buffer Leitura
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
                    bytes_per_row: Some(4 * self.texture_width),
                    rows_per_image: Some(self.texture_height),
                },
            },
            size,
        );

        // Submete comando para GPU Real
        self.queue.submit(Some(encoder.finish()));

        // 4. Mapear memória e ler bytes
        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            tx.send(v).unwrap();
        });
        
        // Espera GPU terminar (poll)
        self.device.poll(wgpu::Maintain::Wait);
        
        // Bloqueia thread leve até receber confirmação
        if let Ok(Ok(())) = rx.recv() {
            let data = buffer_slice.get_mapped_range();
            let result = data.to_vec();
            drop(data);
            output_buffer.unmap();
            return result;
        }

        vec![] // Falha
    }
}