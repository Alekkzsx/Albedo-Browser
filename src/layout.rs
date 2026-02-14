// ARQUIVO: src/layout.rs

#[derive(Debug, Clone)]
pub struct LayoutMetrics {
    pub width: f32,
    pub height: f32,
    pub line_height: f32,
}

/// Calcula o espaço que um texto ocupará na tela usando uma heurística rápida.
/// Não carrega fontes reais para economizar CPU (Filosofia Albedo).
pub fn measure_text(text: &str, font_size: f32, container_width: f32) -> LayoutMetrics {
    // 1. Definições Base
    // Fator 0.6 é uma média segura para fontes sans-serif (Arial/Roboto)
    let avg_char_width = font_size * 0.6; 
    let line_height = font_size * 1.3; // 1.3 dá um respiro bom para leitura

    // 2. Proteção contra texto vazio
    if text.trim().is_empty() {
        return LayoutMetrics { width: container_width, height: 0.0, line_height };
    }

    // 3. Matemática de Quebra de Linha
    // Quantos caracteres cabem em uma linha antes de bater na borda?
    let chars_per_line = (container_width / avg_char_width).floor().max(1.0) as usize;
    
    // Quantos caracteres o texto tem?
    let total_chars = text.chars().count();

    // Quantas linhas serão necessárias? (Arredondamento para cima)
    // Exemplo: 105 chars / 50 por linha = 2.1 -> 3 linhas
    let num_lines = (total_chars as f32 / chars_per_line as f32).ceil() as usize;
    let safe_num_lines = num_lines.max(1); // Mínimo 1 linha

    // 4. Resultado Final
    LayoutMetrics {
        width: container_width,
        height: safe_num_lines as f32 * line_height,
        line_height,
    }
}
