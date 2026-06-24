use super::*;
// ARQUIVO: src/graphics/gpu.rs

use std::borrow::Cow;
use wgpu::util::DeviceExt;

// Representa 1 retângulo HTML para ser instanciado na GPU


impl GpuContext {
    /// TODO: add docs
    pub async fn new(width: u32, height: u32) -> Option<Self> {
        tracing::info!("Initializing WGPU Instanced Renderer");

        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok()?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Albedo GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: Default::default(),
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    trace: wgpu::Trace::Off,

                },
            )
            .await
            .ok()?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Albedo Instanced Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(r#"
pub(crate) struct ScreenUniforms {
                    screen_width: f32,
                    screen_height: f32,
                };

                // Enviado por 'push_constants' se a placha gráfica aguentar, ou convertido puramente 
                // para resolver no JS. Como hardcode pra simplicidade:
                var<private> screen_width: f32 = 800.0;
                var<private> screen_height: f32 = 600.0;

pub(crate) struct InstanceInput {
                    @location(0) position: vec2<f32>,
                    @location(1) size: vec2<f32>,
                    @location(2) color: vec4<f32>,
                    @location(3) border_radius: vec4<f32>,
                };

pub(crate) struct VertexOutput {
                    @builtin(position) clip_position: vec4<f32>,
                    @location(0) color: vec4<f32>,
                    @location(1) local_pos: vec2<f32>,
                    @location(2) size: vec2<f32>,
                    @location(3) radii: vec4<f32>,
                };

                @vertex
pub(crate) fn vs_main(
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
pub(crate) fn sdRoundRect(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
                    let d = abs(p - b * 0.5) - b * 0.5 + vec2<f32>(r);
                    return min(max(d.x, d.y), 0.0) + length(max(d, vec2<f32>(0.0))) - r;
                }

                @fragment
pub(crate) fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
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
            multiview_mask: None,
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
}
