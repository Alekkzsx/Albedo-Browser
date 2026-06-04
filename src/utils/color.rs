use tiny_skia::Color;

/// Faz o parsing de uma string de cor hexadecimal para `tiny_skia::Color`.
/// Suporta formatos `#RGB`, `#RRGGBB` e `#RRGGBBAA` (com canal alpha).
pub fn parse_hex_color(hex: &str) -> Result<Color, ()> {
    if !hex.starts_with('#') {
        return Err(());
    }

    let hex_digits = hex.trim_start_matches('#');
    let (r, g, b, a) = match hex_digits.len() {
        3 => {
            // #RGB
            let r = u8::from_str_radix(&hex_digits[0..1], 16).map_err(|_| ())?;
            let g = u8::from_str_radix(&hex_digits[1..2], 16).map_err(|_| ())?;
            let b = u8::from_str_radix(&hex_digits[2..3], 16).map_err(|_| ())?;
            (r * 17, g * 17, b * 17, 255)
        }
        6 => {
            // #RRGGBB
            let r = u8::from_str_radix(&hex_digits[0..2], 16).map_err(|_| ())?;
            let g = u8::from_str_radix(&hex_digits[2..4], 16).map_err(|_| ())?;
            let b = u8::from_str_radix(&hex_digits[4..6], 16).map_err(|_| ())?;
            (r, g, b, 255)
        }
        8 => {
            // #RRGGBBAA
            let r = u8::from_str_radix(&hex_digits[0..2], 16).map_err(|_| ())?;
            let g = u8::from_str_radix(&hex_digits[2..4], 16).map_err(|_| ())?;
            let b = u8::from_str_radix(&hex_digits[4..6], 16).map_err(|_| ())?;
            let a = u8::from_str_radix(&hex_digits[6..8], 16).map_err(|_| ())?;
            (r, g, b, a)
        }
        _ => return Err(()),
    };

    Ok(Color::from_rgba8(r, g, b, a))
}
