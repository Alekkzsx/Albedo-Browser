//! # Representação de Cores e Algoritmos de Blending CSS
//!
//! Implementação completa de cores para o CSSOM e Pipeline de Pintura GPU,
//! incluindo parsing de especificações CSS3/CSS4 e composição alfa Porter-Duff.

use crate::error::AceError;
use std::fmt;

/// Representa uma cor no espaço de cores sRGB com canal alfa de 8 bits por componente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    /// Componente Vermelho (0-255).
    pub r: u8,
    /// Componente Verde (0-255).
    pub g: u8,
    /// Componente Azul (0-255).
    pub b: u8,
    /// Canal Alfa de transparência (0 = Totalmente Transparente, 255 = Totalmente Opaco).
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::from_rgba(0, 0, 0, 0);
    pub const BLACK: Self = Self::from_rgb(0, 0, 0);
    pub const WHITE: Self = Self::from_rgb(255, 255, 255);
    pub const RED: Self = Self::from_rgb(255, 0, 0);
    pub const GREEN: Self = Self::from_rgb(0, 128, 0);
    pub const LIME: Self = Self::from_rgb(0, 255, 0);
    pub const BLUE: Self = Self::from_rgb(0, 0, 255);
    pub const YELLOW: Self = Self::from_rgb(255, 255, 0);
    pub const CYAN: Self = Self::from_rgb(0, 255, 255);
    pub const MAGENTA: Self = Self::from_rgb(255, 0, 255);
    pub const GRAY: Self = Self::from_rgb(128, 128, 128);
    pub const SILVER: Self = Self::from_rgb(192, 192, 192);

    /// Cria uma cor a partir de valores inteiros RGBA (0-255).
    #[inline]
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Cria uma cor opaca a partir de valores inteiros RGB (0-255).
    #[inline]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Cria uma cor a partir de floats normalizados (0.0 ..= 1.0).
    #[inline]
    pub fn from_rgba_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: (r.clamp(0.0, 1.0) * 255.0).round() as u8,
            g: (g.clamp(0.0, 1.0) * 255.0).round() as u8,
            b: (b.clamp(0.0, 1.0) * 255.0).round() as u8,
            a: (a.clamp(0.0, 1.0) * 255.0).round() as u8,
        }
    }

    /// Retorna os componentes normalizados como floats (0.0 ..= 1.0).
    #[inline]
    pub fn to_rgba_f32(self) -> (f32, f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }

    /// Retorna os componentes com canal alfa pré-multiplicado (necessário para Shaders GPU
    /// para evitar halos escuros em texturas semi-transparentes).
    #[inline]
    pub fn to_premultiplied_f32(self) -> (f32, f32, f32, f32) {
        let (r, g, b, a) = self.to_rgba_f32();
        (r * a, g * a, b * a, a)
    }

    /// Cria uma cor a partir do modelo HSLA:
    /// * `h`: Matiz (Hue em graus 0.0 ..= 360.0).
    /// * `s`: Saturação (0.0 ..= 1.0).
    /// * `l`: Luminosidade (0.0 ..= 1.0).
    /// * `a`: Alfa (0.0 ..= 1.0).
    pub fn from_hsla(h: f32, s: f32, l: f32, a: f32) -> Self {
        let s = s.clamp(0.0, 1.0);
        let l = l.clamp(0.0, 1.0);
        let a = a.clamp(0.0, 1.0);

        let h_norm = (h % 360.0 + 360.0) % 360.0;
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h_norm / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r_prime, g_prime, b_prime) = match (h_norm / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Self::from_rgba_f32(r_prime + m, g_prime + m, b_prime + m, a)
    }

    /// Parser de strings hexadecimais (`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`).
    pub fn from_hex(hex: &str) -> Result<Self, AceError> {
        let s = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
        let len = s.len();

        match len {
            3 => {
                // #RGB -> #RRGGBB
                let r = u8::from_str_radix(&s[0..1], 16).map_err(|_| invalid_color(hex))?;
                let g = u8::from_str_radix(&s[1..2], 16).map_err(|_| invalid_color(hex))?;
                let b = u8::from_str_radix(&s[2..3], 16).map_err(|_| invalid_color(hex))?;
                Ok(Self::from_rgba(r * 17, g * 17, b * 17, 255))
            }
            4 => {
                // #RGBA -> #RRGGBBAA
                let r = u8::from_str_radix(&s[0..1], 16).map_err(|_| invalid_color(hex))?;
                let g = u8::from_str_radix(&s[1..2], 16).map_err(|_| invalid_color(hex))?;
                let b = u8::from_str_radix(&s[2..3], 16).map_err(|_| invalid_color(hex))?;
                let a = u8::from_str_radix(&s[3..4], 16).map_err(|_| invalid_color(hex))?;
                Ok(Self::from_rgba(r * 17, g * 17, b * 17, a * 17))
            }
            6 => {
                // #RRGGBB
                let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| invalid_color(hex))?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| invalid_color(hex))?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| invalid_color(hex))?;
                Ok(Self::from_rgba(r, g, b, 255))
            }
            8 => {
                // #RRGGBBAA
                let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| invalid_color(hex))?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| invalid_color(hex))?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| invalid_color(hex))?;
                let a = u8::from_str_radix(&s[6..8], 16).map_err(|_| invalid_color(hex))?;
                Ok(Self::from_rgba(r, g, b, a))
            }
            _ => Err(invalid_color(hex)),
        }
    }

    /// Parser universal de especificações de cores CSS (hex, `rgb()`, `rgba()`, `hsl()`, `hsla()` e cores nomeadas).
    pub fn parse_css(input: &str) -> Result<Self, AceError> {
        let s = input.trim().to_ascii_lowercase();

        if s.starts_with('#') {
            return Self::from_hex(&s);
        }

        if let Some(color) = named_color(&s) {
            return Ok(color);
        }

        if s.starts_with("rgb(") || s.starts_with("rgba(") {
            return parse_rgb_functional(&s);
        }

        if s.starts_with("hsl(") || s.starts_with("hsla(") {
            return parse_hsl_functional(&s);
        }

        Err(invalid_color(input))
    }

    /// Composição alfa padrão Porter-Duff (`source-over`): mistura `self` (fonte) sobre `dst` (fundo).
    #[inline]
    pub fn blend_source_over(self, dst: Self) -> Self {
        let (src_r, src_g, src_b, src_a) = self.to_rgba_f32();
        let (dst_r, dst_g, dst_b, dst_a) = dst.to_rgba_f32();

        let out_a = src_a + dst_a * (1.0 - src_a);
        if out_a <= 0.0 {
            return Self::TRANSPARENT;
        }

        let out_r = (src_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a;
        let out_g = (src_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a;
        let out_b = (src_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a;

        Self::from_rgba_f32(out_r, out_g, out_b, out_a)
    }

    /// Interpolação linear entre duas cores (`t` entre 0.0 e 1.0).
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let (r1, g1, b1, a1) = self.to_rgba_f32();
        let (r2, g2, b2, a2) = other.to_rgba_f32();

        Self::from_rgba_f32(
            r1 + (r2 - r1) * t,
            g1 + (g2 - g1) * t,
            b1 + (b2 - b1) * t,
            a1 + (a2 - a1) * t,
        )
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
        } else {
            write!(
                f,
                "rgba({}, {}, {}, {:.3})",
                self.r,
                self.g,
                self.b,
                self.a as f32 / 255.0
            )
        }
    }
}

fn invalid_color(s: &str) -> AceError {
    AceError::style(format!("Valor de cor CSS inválido: '{}'", s))
}

fn parse_rgb_functional(s: &str) -> Result<Color, AceError> {
    let inner = s
        .trim_start_matches("rgba(")
        .trim_start_matches("rgb(")
        .trim_end_matches(')')
        .trim();

    // Suporta delimitador por vírgula ou espaço/barra (CSS Colors Level 4)
    let parts: Vec<&str> = if inner.contains(',') {
        inner.split(',').map(str::trim).collect()
    } else {
        inner.split_whitespace().filter(|p| *p != "/").collect()
    };

    if parts.len() < 3 || parts.len() > 4 {
        return Err(invalid_color(s));
    }

    let r = parse_component(parts[0], 255.0)?;
    let g = parse_component(parts[1], 255.0)?;
    let b = parse_component(parts[2], 255.0)?;
    let a = if parts.len() == 4 {
        parse_alpha(parts[3])?
    } else {
        1.0
    };

    Ok(Color::from_rgba_f32(r / 255.0, g / 255.0, b / 255.0, a))
}

fn parse_hsl_functional(s: &str) -> Result<Color, AceError> {
    let inner = s
        .trim_start_matches("hsla(")
        .trim_start_matches("hsl(")
        .trim_end_matches(')')
        .trim();

    let parts: Vec<&str> = if inner.contains(',') {
        inner.split(',').map(str::trim).collect()
    } else {
        inner.split_whitespace().filter(|p| *p != "/").collect()
    };

    if parts.len() < 3 || parts.len() > 4 {
        return Err(invalid_color(s));
    }

    let h: f32 = parts[0]
        .trim_end_matches("deg")
        .parse()
        .map_err(|_| invalid_color(s))?;
    let s_val = parse_percentage(parts[1])?;
    let l_val = parse_percentage(parts[2])?;
    let a = if parts.len() == 4 {
        parse_alpha(parts[3])?
    } else {
        1.0
    };

    Ok(Color::from_hsla(h, s_val, l_val, a))
}

fn parse_component(s: &str, max: f32) -> Result<f32, AceError> {
    if let Some(pct) = s.strip_suffix('%') {
        let val: f32 = pct.parse().map_err(|_| invalid_color(s))?;
        Ok((val / 100.0) * max)
    } else {
        s.parse::<f32>().map_err(|_| invalid_color(s))
    }
}

fn parse_percentage(s: &str) -> Result<f32, AceError> {
    if let Some(pct) = s.strip_suffix('%') {
        let val: f32 = pct.parse().map_err(|_| invalid_color(s))?;
        Ok(val / 100.0)
    } else {
        s.parse::<f32>().map_err(|_| invalid_color(s))
    }
}

fn parse_alpha(s: &str) -> Result<f32, AceError> {
    if let Some(pct) = s.strip_suffix('%') {
        let val: f32 = pct.parse().map_err(|_| invalid_color(s))?;
        Ok(val / 100.0)
    } else {
        s.parse::<f32>().map_err(|_| invalid_color(s))
    }
}

/// Dicionário de cores CSS nomeadas padrão (CSS Color Module Level 4).
fn named_color(name: &str) -> Option<Color> {
    match name {
        "transparent" => Some(Color::TRANSPARENT),
        "black" => Some(Color::BLACK),
        "white" => Some(Color::WHITE),
        "red" => Some(Color::RED),
        "green" => Some(Color::GREEN),
        "lime" => Some(Color::LIME),
        "blue" => Some(Color::BLUE),
        "yellow" => Some(Color::YELLOW),
        "cyan" | "aqua" => Some(Color::CYAN),
        "magenta" | "fuchsia" => Some(Color::MAGENTA),
        "gray" | "grey" => Some(Color::GRAY),
        "silver" => Some(Color::SILVER),
        "maroon" => Some(Color::from_rgb(128, 0, 0)),
        "olive" => Some(Color::from_rgb(128, 128, 0)),
        "navy" => Some(Color::from_rgb(0, 0, 128)),
        "purple" => Some(Color::from_rgb(128, 0, 128)),
        "teal" => Some(Color::from_rgb(0, 128, 128)),
        "orange" => Some(Color::from_rgb(255, 165, 0)),
        "aliceblue" => Some(Color::from_rgb(240, 248, 255)),
        "antiquewhite" => Some(Color::from_rgb(250, 235, 215)),
        "aquamarine" => Some(Color::from_rgb(127, 255, 212)),
        "azure" => Some(Color::from_rgb(240, 255, 255)),
        "beige" => Some(Color::from_rgb(245, 245, 220)),
        "bisque" => Some(Color::from_rgb(255, 228, 196)),
        "blanchedalmond" => Some(Color::from_rgb(255, 235, 205)),
        "blueviolet" => Some(Color::from_rgb(138, 43, 226)),
        "brown" => Some(Color::from_rgb(165, 42, 42)),
        "burlywood" => Some(Color::from_rgb(222, 184, 135)),
        "cadetblue" => Some(Color::from_rgb(95, 158, 160)),
        "chartreuse" => Some(Color::from_rgb(127, 255, 0)),
        "chocolate" => Some(Color::from_rgb(210, 105, 30)),
        "coral" => Some(Color::from_rgb(255, 127, 80)),
        "cornflowerblue" => Some(Color::from_rgb(100, 149, 237)),
        "cornsilk" => Some(Color::from_rgb(255, 248, 220)),
        "crimson" => Some(Color::from_rgb(220, 20, 60)),
        "darkblue" => Some(Color::from_rgb(0, 0, 139)),
        "darkcyan" => Some(Color::from_rgb(0, 139, 139)),
        "darkgoldenrod" => Some(Color::from_rgb(184, 134, 11)),
        "darkgray" | "darkgrey" => Some(Color::from_rgb(169, 169, 169)),
        "darkgreen" => Some(Color::from_rgb(0, 100, 0)),
        "darkkhaki" => Some(Color::from_rgb(189, 183, 107)),
        "darkmagenta" => Some(Color::from_rgb(139, 0, 139)),
        "darkolivegreen" => Some(Color::from_rgb(85, 107, 47)),
        "darkorange" => Some(Color::from_rgb(255, 140, 0)),
        "darkorchid" => Some(Color::from_rgb(153, 50, 204)),
        "darkred" => Some(Color::from_rgb(139, 0, 0)),
        "darksalmon" => Some(Color::from_rgb(233, 150, 122)),
        "darkseagreen" => Some(Color::from_rgb(143, 188, 143)),
        "darkslateblue" => Some(Color::from_rgb(72, 61, 139)),
        "darkslategray" | "darkslategrey" => Some(Color::from_rgb(47, 79, 79)),
        "darkturquoise" => Some(Color::from_rgb(0, 206, 209)),
        "darkviolet" => Some(Color::from_rgb(148, 0, 211)),
        "deeppink" => Some(Color::from_rgb(255, 20, 147)),
        "deepskyblue" => Some(Color::from_rgb(0, 191, 255)),
        "dimgray" | "dimgrey" => Some(Color::from_rgb(105, 105, 105)),
        "dodgerblue" => Some(Color::from_rgb(30, 144, 255)),
        "firebrick" => Some(Color::from_rgb(178, 34, 34)),
        "floralwhite" => Some(Color::from_rgb(255, 250, 240)),
        "forestgreen" => Some(Color::from_rgb(34, 139, 34)),
        "gainsboro" => Some(Color::from_rgb(220, 220, 220)),
        "ghostwhite" => Some(Color::from_rgb(248, 248, 255)),
        "gold" => Some(Color::from_rgb(255, 215, 0)),
        "goldenrod" => Some(Color::from_rgb(218, 165, 32)),
        "greenyellow" => Some(Color::from_rgb(173, 255, 47)),
        "honeydew" => Some(Color::from_rgb(240, 255, 240)),
        "hotpink" => Some(Color::from_rgb(255, 105, 180)),
        "indianred" => Some(Color::from_rgb(205, 92, 92)),
        "indigo" => Some(Color::from_rgb(75, 0, 130)),
        "ivory" => Some(Color::from_rgb(255, 255, 240)),
        "khaki" => Some(Color::from_rgb(240, 230, 140)),
        "lavender" => Some(Color::from_rgb(230, 230, 250)),
        "lavenderblush" => Some(Color::from_rgb(255, 240, 245)),
        "lawngreen" => Some(Color::from_rgb(124, 252, 0)),
        "lemonchiffon" => Some(Color::from_rgb(255, 250, 205)),
        "lightblue" => Some(Color::from_rgb(173, 216, 230)),
        "lightcoral" => Some(Color::from_rgb(240, 128, 128)),
        "lightcyan" => Some(Color::from_rgb(224, 255, 255)),
        "lightgoldenrodyellow" => Some(Color::from_rgb(250, 250, 210)),
        "lightgray" | "lightgrey" => Some(Color::from_rgb(211, 211, 211)),
        "lightgreen" => Some(Color::from_rgb(144, 238, 144)),
        "lightpink" => Some(Color::from_rgb(255, 182, 193)),
        "lightsalmon" => Some(Color::from_rgb(255, 160, 122)),
        "lightseagreen" => Some(Color::from_rgb(32, 178, 170)),
        "lightskyblue" => Some(Color::from_rgb(135, 206, 250)),
        "lightslategray" | "lightslategrey" => Some(Color::from_rgb(119, 136, 153)),
        "lightsteelblue" => Some(Color::from_rgb(176, 196, 222)),
        "lightyellow" => Some(Color::from_rgb(255, 255, 224)),
        "limegreen" => Some(Color::from_rgb(50, 205, 50)),
        "linen" => Some(Color::from_rgb(250, 240, 230)),
        "mediumaquamarine" => Some(Color::from_rgb(102, 205, 170)),
        "mediumblue" => Some(Color::from_rgb(0, 0, 205)),
        "mediumorchid" => Some(Color::from_rgb(186, 85, 211)),
        "mediumpurple" => Some(Color::from_rgb(147, 112, 219)),
        "mediumseagreen" => Some(Color::from_rgb(60, 179, 113)),
        "mediumslateblue" => Some(Color::from_rgb(123, 104, 238)),
        "mediumspringgreen" => Some(Color::from_rgb(0, 250, 154)),
        "mediumturquoise" => Some(Color::from_rgb(72, 209, 204)),
        "mediumvioletred" => Some(Color::from_rgb(199, 21, 133)),
        "midnightblue" => Some(Color::from_rgb(25, 25, 112)),
        "mintcream" => Some(Color::from_rgb(245, 255, 250)),
        "mistyrose" => Some(Color::from_rgb(255, 228, 225)),
        "moccasin" => Some(Color::from_rgb(255, 228, 181)),
        "navajowhite" => Some(Color::from_rgb(255, 222, 173)),
        "oldlace" => Some(Color::from_rgb(253, 245, 230)),
        "olivedrab" => Some(Color::from_rgb(107, 142, 35)),
        "orangered" => Some(Color::from_rgb(255, 69, 0)),
        "orchid" => Some(Color::from_rgb(218, 112, 214)),
        "palegoldenrod" => Some(Color::from_rgb(238, 232, 170)),
        "palegreen" => Some(Color::from_rgb(152, 251, 152)),
        "paleturquoise" => Some(Color::from_rgb(175, 238, 238)),
        "palevioletred" => Some(Color::from_rgb(219, 112, 147)),
        "papayawhip" => Some(Color::from_rgb(255, 239, 213)),
        "peachpuff" => Some(Color::from_rgb(255, 218, 185)),
        "peru" => Some(Color::from_rgb(205, 133, 63)),
        "pink" => Some(Color::from_rgb(255, 192, 203)),
        "plum" => Some(Color::from_rgb(221, 160, 221)),
        "powderblue" => Some(Color::from_rgb(176, 224, 230)),
        "rebeccapurple" => Some(Color::from_rgb(102, 51, 153)),
        "rosybrown" => Some(Color::from_rgb(188, 143, 143)),
        "royalblue" => Some(Color::from_rgb(65, 105, 225)),
        "saddlebrown" => Some(Color::from_rgb(139, 69, 19)),
        "salmon" => Some(Color::from_rgb(250, 128, 114)),
        "sandybrown" => Some(Color::from_rgb(244, 164, 96)),
        "seagreen" => Some(Color::from_rgb(46, 139, 87)),
        "seashell" => Some(Color::from_rgb(255, 245, 238)),
        "sienna" => Some(Color::from_rgb(160, 82, 45)),
        "skyblue" => Some(Color::from_rgb(135, 206, 235)),
        "slateblue" => Some(Color::from_rgb(106, 90, 205)),
        "slategray" | "slategrey" => Some(Color::from_rgb(112, 128, 144)),
        "snow" => Some(Color::from_rgb(255, 250, 250)),
        "springgreen" => Some(Color::from_rgb(0, 255, 127)),
        "steelblue" => Some(Color::from_rgb(70, 130, 180)),
        "tan" => Some(Color::from_rgb(210, 180, 140)),
        "thistle" => Some(Color::from_rgb(216, 191, 216)),
        "tomato" => Some(Color::from_rgb(255, 99, 71)),
        "turquoise" => Some(Color::from_rgb(64, 224, 208)),
        "violet" => Some(Color::from_rgb(238, 130, 238)),
        "wheat" => Some(Color::from_rgb(245, 222, 179)),
        "whitesmoke" => Some(Color::from_rgb(245, 245, 245)),
        "yellowgreen" => Some(Color::from_rgb(154, 205, 50)),
        _ => None,
    }
}

impl From<(u8, u8, u8)> for Color {
    #[inline]
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::from_rgb(r, g, b)
    }
}

impl From<(u8, u8, u8, u8)> for Color {
    #[inline]
    fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        Self::from_rgba(r, g, b, a)
    }
}
