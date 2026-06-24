use super::*;
use std::fmt;



#[derive(Debug, Clone, PartialEq)]
pub enum CssColor {
    Named(String),
    CurrentColor,
    Transparent,
    Rgba(u8, u8, u8, f32), // r, g, b, alpha
}

impl Default for CssColor {
pub(crate) fn default() -> Self {
        Self::Named("black".to_string()) // CanvasText equivalent
    }
}

impl CssColor {
    /// TODO: add docs
    pub fn to_rgba_string(&self) -> String {
        match self {
            CssColor::Named(name) => {
                let lower = name.to_lowercase();
                // Se já é hex (#rrggbb), retorna direto
                if lower.starts_with('#') {
                    return name.clone();
                }
                match lower.as_str() {
                    "white" => "#ffffff".to_string(),
                    "black" => "#000000".to_string(),
                    "red" => "#ff0000".to_string(),
                    "green" => "#008000".to_string(),
                    "blue" => "#0000ff".to_string(),
                    "yellow" => "#ffff00".to_string(),
                    "cyan" | "aqua" => "#00ffff".to_string(),
                    "magenta" | "fuchsia" => "#ff00ff".to_string(),
                    "orange" => "#ffa500".to_string(),
                    "pink" => "#ffc0cb".to_string(),
                    "purple" => "#800080".to_string(),
                    "brown" => "#a52a2a".to_string(),
                    "gray" | "grey" => "#808080".to_string(),
                    "silver" => "#c0c0c0".to_string(),
                    "navy" => "#000080".to_string(),
                    "teal" => "#008080".to_string(),
                    "maroon" => "#800000".to_string(),
                    "olive" => "#808000".to_string(),
                    "lime" => "#00ff00".to_string(),
                    "coral" => "#ff7f50".to_string(),
                    "gold" => "#ffd700".to_string(),
                    "crimson" => "#dc143c".to_string(),
                    "indigo" => "#4b0082".to_string(),
                    "violet" => "#ee82ee".to_string(),
                    "turquoise" => "#40e0d0".to_string(),
                    "salmon" => "#fa8072".to_string(),
                    "khaki" => "#f0e68c".to_string(),
                    "plum" => "#dda0dd".to_string(),
                    "orchid" => "#da70d6".to_string(),
                    "tomato" => "#ff6347".to_string(),
                    "beige" => "#f5f5dc".to_string(),
                    "ivory" => "#fffff0".to_string(),
                    "linen" => "#faf0e6".to_string(),
                    "wheat" => "#f5deb3".to_string(),
                    // Dark variants
                    "darkblue" => "#00008b".to_string(),
                    "darkgreen" => "#006400".to_string(),
                    "darkred" => "#8b0000".to_string(),
                    "darkcyan" => "#008b8b".to_string(),
                    "darkmagenta" => "#8b008b".to_string(),
                    "darkorange" => "#ff8c00".to_string(),
                    "darkviolet" => "#9400d3".to_string(),
                    "darkgray" | "darkgrey" => "#a9a9a9".to_string(),
                    "darkkhaki" => "#bdb76b".to_string(),
                    "darkolivegreen" => "#556b2f".to_string(),
                    "darkorchid" => "#9932cc".to_string(),
                    "darksalmon" => "#e9967a".to_string(),
                    "darkseagreen" => "#8fbc8f".to_string(),
                    "darkslateblue" => "#483d8b".to_string(),
                    "darkslategray" | "darkslategrey" => "#2f4f4f".to_string(),
                    "darkturquoise" => "#00ced1".to_string(),
                    "darkgoldenrod" => "#b8860b".to_string(),
                    // Light variants
                    "lightblue" => "#add8e6".to_string(),
                    "lightgreen" => "#90ee90".to_string(),
                    "lightgray" | "lightgrey" => "#d3d3d3".to_string(),
                    "lightcoral" => "#f08080".to_string(),
                    "lightcyan" => "#e0ffff".to_string(),
                    "lightyellow" => "#ffffe0".to_string(),
                    "lightpink" => "#ffb6c1".to_string(),
                    "lightsalmon" => "#ffa07a".to_string(),
                    "lightseagreen" => "#20b2aa".to_string(),
                    "lightskyblue" => "#87cefa".to_string(),
                    "lightsteelblue" => "#b0c4de".to_string(),
                    // Medium variants
                    "mediumblue" => "#0000cd".to_string(),
                    "mediumseagreen" => "#3cb371".to_string(),
                    "mediumslateblue" => "#7b68ee".to_string(),
                    "mediumspringgreen" => "#00fa9a".to_string(),
                    "mediumturquoise" => "#48d1cc".to_string(),
                    "mediumvioletred" => "#c71585".to_string(),
                    "mediumorchid" => "#ba55d3".to_string(),
                    "mediumpurple" => "#9370db".to_string(),
                    "mediumaquamarine" => "#66cdaa".to_string(),
                    // Others
                    "whitesmoke" => "#f5f5f5".to_string(),
                    "ghostwhite" => "#f8f8ff".to_string(),
                    "aliceblue" => "#f0f8ff".to_string(),
                    "lavender" => "#e6e6fa".to_string(),
                    "mistyrose" => "#ffe4e1".to_string(),
                    "snow" => "#fffafa".to_string(),
                    "seashell" => "#fff5ee".to_string(),
                    "mintcream" => "#f5fffa".to_string(),
                    "honeydew" => "#f0fff0".to_string(),
                    "azure" => "#f0ffff".to_string(),
                    "floralwhite" => "#fffaf0".to_string(),
                    "antiquewhite" => "#faebd7".to_string(),
                    "cornsilk" => "#fff8dc".to_string(),
                    "blanchedalmond" => "#ffebcd".to_string(),
                    "bisque" => "#ffe4c4".to_string(),
                    "navajowhite" => "#ffdead".to_string(),
                    "moccasin" => "#ffe4b5".to_string(),
                    "papayawhip" => "#ffefd5".to_string(),
                    "lemonchiffon" => "#fffacd".to_string(),
                    "oldlace" => "#fdf5e6".to_string(),
                    "peachpuff" => "#ffdab9".to_string(),
                    "powderblue" => "#b0e0e6".to_string(),
                    "rosybrown" => "#bc8f8f".to_string(),
                    "royalblue" => "#4169e1".to_string(),
                    "saddlebrown" => "#8b4513".to_string(),
                    "sandybrown" => "#f4a460".to_string(),
                    "seagreen" => "#2e8b57".to_string(),
                    "sienna" => "#a0522d".to_string(),
                    "skyblue" => "#87ceeb".to_string(),
                    "slateblue" => "#6a5acd".to_string(),
                    "slategray" | "slategrey" => "#708090".to_string(),
                    "springgreen" => "#00ff7f".to_string(),
                    "steelblue" => "#4682b4".to_string(),
                    "tan" => "#d2b48c".to_string(),
                    "thistle" => "#d8bfd8".to_string(),
                    "yellowgreen" => "#9acd32".to_string(),
                    "rebeccapurple" => "#663399".to_string(),
                    "transparent" => "transparent".to_string(),
                    "cornflowerblue" => "#6495ed".to_string(),
                    "cadetblue" => "#5f9ea0".to_string(),
                    "chartreuse" => "#7fff00".to_string(),
                    "chocolate" => "#d2691e".to_string(),
                    "deeppink" => "#ff1493".to_string(),
                    "deepskyblue" => "#00bfff".to_string(),
                    "dimgray" | "dimgrey" => "#696969".to_string(),
                    "dodgerblue" => "#1e90ff".to_string(),
                    "firebrick" => "#b22222".to_string(),
                    "forestgreen" => "#228b22".to_string(),
                    "gainsboro" => "#dcdcdc".to_string(),
                    "greenyellow" => "#adff2f".to_string(),
                    "hotpink" => "#ff69b4".to_string(),
                    "indianred" => "#cd5c5c".to_string(),
                    "lawngreen" => "#7cfc00".to_string(),
                    "limegreen" => "#32cd32".to_string(),
                    "midnightblue" => "#191970".to_string(),
                    "olivedrab" => "#6b8e23".to_string(),
                    "orangered" => "#ff4500".to_string(),
                    "palegreen" => "#98fb98".to_string(),
                    "paleturquoise" => "#afeeee".to_string(),
                    "palevioletred" => "#db7093".to_string(),
                    "peru" => "#cd853f".to_string(),
                    _ => "#000000".to_string(),
                }
            }
            CssColor::CurrentColor => "#000000".to_string(),
            CssColor::Transparent => "transparent".to_string(),
            CssColor::Rgba(r, g, b, a) => {
                if *a < 1.0 {
                    format!("rgba({},{},{},{})", r, g, b, a)
                } else {
                    format!("#{:02x}{:02x}{:02x}", r, g, b)
                }
            }
        }
    }
}
