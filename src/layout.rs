#[derive(Clone, Debug)]
pub struct TextLayout {
    pub text_lines: Vec<String>,
    pub total_height: f32,
}

pub struct FontMetrics {
    pub char_width_ratio: f32, // Largura média de um char (0.6 para fontes padrão)
    pub line_height_ratio: f32, // Altura da linha (1.2 é um bom respiro)
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self {
            char_width_ratio: 0.6,
            line_height_ratio: 1.4,
        }
    }
}

pub fn calculate_text_wrapping(text: &str, font_size: f32, max_width: f32) -> TextLayout {
    let metrics = FontMetrics::default();
    
    // 1. Estimar largura de um caractere em pixels
    // Ex: Fonte 16px -> char_width ~= 9.6px
    let avg_char_width = font_size * metrics.char_width_ratio;
    
    // 2. Calcular quantos caracteres cabem em uma linha
    let chars_per_line = (max_width / avg_char_width).floor() as usize;
    
    // 3. O algoritmo de "Word Wrap" (Quebra por palavra)
    let mut lines = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    
    let mut current_line = String::new();
    let mut current_len = 0;

    for word in words {
        let word_len = word.chars().count();
        
        // Se a palavra sozinha é maior que a linha, corta ela (edge case)
        // Se a linha atual + palavra passar do limite -> Nova Linha
        if current_len + word_len + 1 > chars_per_line && current_len > 0 {
            lines.push(current_line);
            current_line = String::from(word);
            current_len = word_len;
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
                current_len += 1;
            }
            current_line.push_str(word);
            current_len += word_len;
        }
    }
    // Empurra a última linha que sobrou
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    // 4. Calcular Altura Total (Baseado no número de linhas geradas)
    let line_height = font_size * metrics.line_height_ratio;
    let total_height = (lines.len() as f32 * line_height).max(line_height); // Mínimo 1 linha

    TextLayout {
        text_lines: lines, // Retornamos o texto já dividido (ou unido para o Slint lidar com wrap)
        total_height,      // IMPORTANTE: Isso diz pro próximo elemento onde começar!
    }
}
