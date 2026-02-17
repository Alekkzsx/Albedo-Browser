// ARQUIVO: src/graphics/canvas2d.rs

use tiny_skia::{
    Color, FilterQuality, Paint, PathBuilder, Pixmap, 
    PixmapPaint, Rect, Stroke, Transform
};

/// Estado atual do desenho (Cores, Estilos de Linha)
#[derive(Clone, Debug)]
struct DrawState {
    fill_color: Color,
    stroke_color: Color,
    line_width: f32,
    global_alpha: f32,
    transform: Transform,
}

impl Default for DrawState {
    fn default() -> Self {
        Self {
            fill_color: Color::BLACK, // HTML5 default
            stroke_color: Color::BLACK,
            line_width: 1.0,
            global_alpha: 1.0,
            transform: Transform::identity(),
        }
    }
}

pub struct Canvas2D {
    width: u32,
    height: u32,
    pixmap: Pixmap,
    
    // Caminho atual sendo construído (beginPath -> ... -> fill)
    path_builder: PathBuilder,
    
    // Estado atual
    state: DrawState,
    
    // Pilha de estados para save()/restore()
    state_stack: Vec<DrawState>,
}

impl Canvas2D {
    /// Cria um novo Canvas em branco (Transparente)
    pub fn new(width: u32, height: u32) -> Self {
        let pixmap = Pixmap::new(width, height)
            .expect("ACE Error: Falha ao alocar memória para Canvas2D (RAM insuficiente?)");

        Self {
            width,
            height,
            pixmap,
            path_builder: PathBuilder::new(),
            state: DrawState::default(),
            state_stack: Vec::new(),
        }
    }

    /// Retorna os bytes crus da imagem (RGBA8 Premultiplied)
    /// Perfeito para o Slint exibir: slint::SharedPixelBuffer::clone_from_slice(...)
    pub fn get_pixels(&self) -> &[u8] {
        self.pixmap.data()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width != self.width || height != self.height {
            if let Some(new_pix) = Pixmap::new(width, height) {
                self.pixmap = new_pix;
                self.width = width;
                self.height = height;
                // HTML Canvas limpa o estado ao redimensionar, mas aqui mantemos o path builder
                self.path_builder = PathBuilder::new(); 
            }
        }
    }

    // --- GERENCIAMENTO DE ESTADO (Save/Restore) ---

    pub fn save(&mut self) {
        self.state_stack.push(self.state.clone());
    }

    pub fn restore(&mut self) {
        if let Some(saved_state) = self.state_stack.pop() {
            self.state = saved_state;
        }
    }

    // --- CONFIGURAÇÃO DE ESTILO ---

    pub fn set_fill_style(&mut self, hex: &str) {
        if let Some(c) = parse_hex_color(hex) {
            self.state.fill_color = c;
        }
    }

    pub fn set_stroke_style(&mut self, hex: &str) {
        if let Some(c) = parse_hex_color(hex) {
            self.state.stroke_color = c;
        }
    }

    pub fn set_line_width(&mut self, width: f32) {
        self.state.line_width = width;
    }

    pub fn set_global_alpha(&mut self, alpha: f32) {
        self.state.global_alpha = alpha.clamp(0.0, 1.0);
    }

    // --- MANIPULAÇÃO DE CAMINHOS (PATHS) ---

    pub fn begin_path(&mut self) {
        self.path_builder = PathBuilder::new();
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        self.path_builder.move_to(x, y);
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        self.path_builder.line_to(x, y);
    }

    /// Curva Bézier Quadrática (canvas.quadraticCurveTo)
    pub fn quadratic_curve_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.path_builder.quad_to(cx, cy, x, y);
    }

    /// Curva Bézier Cúbica (canvas.bezierCurveTo)
    pub fn bezier_curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
        self.path_builder.cubic_to(cx1, cy1, cx2, cy2, x, y);
    }

    pub fn close_path(&mut self) {
        self.path_builder.close();
    }

    /// Adiciona um retângulo ao path atual
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let r = Rect::from_xywh(x, y, w, h).unwrap_or(Rect::from_xywh(0.0,0.0,0.0,0.0).unwrap());
        self.path_builder.push_rect(r);
    }

    // --- OPERAÇÕES DE DESENHO IMEDIATO (Rects) ---

    /// clearRect: Limpa uma área (deixa transparente)
    pub fn clear_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let rect = Rect::from_xywh(x, y, w, h).unwrap_or(Rect::from_xywh(0.0,0.0,0.0,0.0).unwrap());
        // Modo Clear usa PorterDuff::Clear
        let mut paint = Paint::default();
        paint.blend_mode = tiny_skia::BlendMode::Clear;
        
        self.pixmap.fill_rect(
            rect, 
            &paint, 
            self.state.transform, 
            None
        );
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        if let Some(rect) = Rect::from_xywh(x, y, w, h) {
            let mut paint = Paint::default();
            paint.set_color(self.state.fill_color);
            paint.anti_alias = true; // Borda suave
            // Aplica transparência global se necessário
            if self.state.global_alpha < 1.0 {
                let current_alpha = self.state.fill_color.alpha();
                paint.set_color_rgba8(
                    (self.state.fill_color.red() * 255.0) as u8,
                    (self.state.fill_color.green() * 255.0) as u8,
                    (self.state.fill_color.blue() * 255.0) as u8,
                    (current_alpha * self.state.global_alpha * 255.0) as u8
                );
            }

            self.pixmap.fill_rect(rect, &paint, self.state.transform, None);
        }
    }

    // --- OPERAÇÕES FINAIS (Fill / Stroke) ---

    /// Pinta o caminho atual com a cor de preenchimento
    pub fn fill(&mut self) {
        if let Some(path) = self.path_builder.clone().finish() {
            let mut paint = Paint::default();
            paint.set_color(self.state.fill_color);
            paint.anti_alias = true;

            self.pixmap.fill_path(
                &path, 
                &paint, 
                Default::default(), // FillRule (Winding/EvenOdd)
                self.state.transform, 
                None
            );
        }
    }

    /// Desenha a borda do caminho atual
    pub fn stroke(&mut self) {
        if let Some(path) = self.path_builder.clone().finish() {
            let mut paint = Paint::default();
            paint.set_color(self.state.stroke_color);
            paint.anti_alias = true;

            let stroke = Stroke {
                width: self.state.line_width,
                ..Stroke::default() // Caps, Joins padrão
            };

            self.pixmap.stroke_path(
                &path, 
                &paint, 
                &stroke, 
                self.state.transform, 
                None
            );
        }
    }
}

// --- AUXILIAR: PARSER DE CORES LEVE ---

fn parse_hex_color(hex: &str) -> Option<Color> {
    if !hex.starts_with('#') { return None; }
    
    let hex = hex.trim_start_matches('#');
    let (r, g, b, a) = match hex.len() {
        3 => { // #RGB
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            (r * 17, g * 17, b * 17, 255)
        },
        6 => { // #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b, 255)
        },
        8 => { // #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            (r, g, b, a)
        }
        _ => return None,
    };

    Some(Color::from_rgba8(r, g, b, a))
}