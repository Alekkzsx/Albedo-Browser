//! # O(1) String Pooling & Static Atoms de Alta Performance
//!
//! Representação otimizada de átomos com identificadores estáticos de custo zero (zero atomic refcount)
//! para tags HTML, atributos e propriedades CSS, com fallback dinâmico para DefaultAtom.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use string_cache::DefaultAtom;

/// Representa uma string otimizada e única em toda a execução do navegador.
/// Para átomos estáticos conhecidos, a igualdade é $O(1)$ imediata (comparação de u32) sem atomic refcount.
#[derive(Clone, Eq)]
pub enum Atom {
    Static(&'static str, u32),
    Dynamic(DefaultAtom),
}

impl Atom {
    /// Interna uma string. Se for um átomo estático conhecido, resolve em $O(1)$ sem alocações.
    #[inline]
    pub fn new(text: &str) -> Self {
        if let Some(static_atom) = lookup_static(text) {
            static_atom
        } else {
            Self::Dynamic(DefaultAtom::from(text))
        }
    }

    /// Retorna a representação textual do Atom como `&str`.
    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Static(s, _) => s,
            Self::Dynamic(d) => d.as_ref(),
        }
    }

    /// Retorna `true` se o átomo for uma string vazia.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }

    /// Retorna o tamanho em bytes do átomo.
    #[inline]
    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    /// Retorna a fatia de bytes do átomo.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }

    /// Compara se o átomo é igual a uma string ignorando maiúsculas/minúsculas ASCII.
    #[inline]
    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        self.as_str().eq_ignore_ascii_case(other)
    }
}

impl PartialEq for Atom {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Static(_, id1), Self::Static(_, id2)) => id1 == id2,
            (Self::Dynamic(d1), Self::Dynamic(d2)) => d1 == d2,
            (Self::Static(s, _), Self::Dynamic(d)) => *s == d.as_ref(),
            (Self::Dynamic(d), Self::Static(s, _)) => d.as_ref() == *s,
        }
    }
}

impl PartialOrd for Atom {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Atom {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for Atom {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl From<&str> for Atom {
    #[inline]
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Atom {
    #[inline]
    fn from(s: String) -> Self {
        if let Some(static_atom) = lookup_static(&s) {
            static_atom
        } else {
            Self::Dynamic(DefaultAtom::from(s))
        }
    }
}

impl From<smol_str::SmolStr> for Atom {
    #[inline]
    fn from(s: smol_str::SmolStr) -> Self {
        Self::new(s.as_str())
    }
}

impl From<Atom> for smol_str::SmolStr {
    #[inline]
    fn from(atom: Atom) -> Self {
        smol_str::SmolStr::new(atom.as_str())
    }
}

impl From<DefaultAtom> for Atom {
    #[inline]
    fn from(atom: DefaultAtom) -> Self {
        if let Some(static_atom) = lookup_static(atom.as_ref()) {
            static_atom
        } else {
            Self::Dynamic(atom)
        }
    }
}

impl Deref for Atom {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for Atom {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for Atom {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Atom {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<Atom> for &str {
    #[inline]
    fn eq(&self, other: &Atom) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<String> for Atom {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<Atom> for String {
    #[inline]
    fn eq(&self, other: &Atom) -> bool {
        self.as_str() == other.as_str()
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Atom(\"{}\")", self.as_str())
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

macro_rules! define_static_atoms {
    ($($id:expr, $fn_name:ident, $str:expr);* $(;)?) => {
        pub mod atoms {
            use super::Atom;
            $(
                #[allow(non_snake_case)]
                #[inline(always)]
                pub const fn $fn_name() -> Atom {
                    Atom::Static($str, $id)
                }
            )*
        }

        #[inline]
        pub fn lookup_static(s: &str) -> Option<Atom> {
            match s {
                $( $str => Some(Atom::Static($str, $id)), )*
                _ => None,
            }
        }
    };
}

define_static_atoms! {
    0, HTML, "html";
    1, HEAD, "head";
    2, BODY, "body";
    3, DIV, "div";
    4, SPAN, "span";
    5, P, "p";
    6, A, "a";
    7, BUTTON, "button";
    8, INPUT, "input";
    9, IMG, "img";
    10, SCRIPT, "script";
    11, STYLE, "style";
    12, LINK, "link";
    13, META, "meta";
    14, TITLE, "title";
    15, IFRAME, "iframe";
    16, TABLE, "table";
    17, THEAD, "thead";
    18, TBODY, "tbody";
    19, TR, "tr";
    20, TH, "th";
    21, TD, "td";
    22, UL, "ul";
    23, OL, "ol";
    24, LI, "li";
    25, FORM, "form";
    26, LABEL, "label";
    27, TEXTAREA, "textarea";
    28, SELECT, "select";
    29, OPTION, "option";
    30, HEADER, "header";
    31, FOOTER, "footer";
    32, NAV, "nav";
    33, SECTION, "section";
    34, ARTICLE, "article";
    35, ASIDE, "aside";
    36, MAIN, "main";
    37, H1, "h1";
    38, H2, "h2";
    39, H3, "h3";
    40, H4, "h4";
    41, H5, "h5";
    42, H6, "h6";
    43, CANVAS, "canvas";
    44, SVG, "svg";
    45, PATH, "path";
    46, VIDEO, "video";
    47, AUDIO, "audio";
    48, SOURCE, "source";
    49, TEMPLATE, "template";
    50, SLOT, "slot";
    51, BR, "br";
    52, HR, "hr";
    53, PRE, "pre";
    54, CODE, "code";
    55, STRONG, "strong";
    56, EM, "em";
    57, ID, "id";
    58, CLASS, "class";
    11, STYLE_ATTR, "style";
    59, SRC, "src";
    60, HREF, "href";
    61, TYPE, "type";
    62, VALUE, "value";
    63, NAME, "name";
    64, REL, "rel";
    65, CONTENT, "content";
    66, CHARSET, "charset";
    67, ALT, "alt";
    68, WIDTH, "width";
    69, HEIGHT, "height";
    70, DISABLED, "disabled";
    71, CHECKED, "checked";
    72, SELECTED, "selected";
    73, READONLY, "readonly";
    74, PLACEHOLDER, "placeholder";
    75, ACTION, "action";
    76, METHOD, "method";
    77, TARGET, "target";
    78, DISPLAY, "display";
    79, POSITION, "position";
    80, TOP, "top";
    81, RIGHT, "right";
    82, BOTTOM, "bottom";
    83, LEFT, "left";
    68, WIDTH_PROP, "width";
    69, HEIGHT_PROP, "height";
    84, MIN_WIDTH, "min-width";
    85, MAX_WIDTH, "max-width";
    86, MIN_HEIGHT, "min-height";
    87, MAX_HEIGHT, "max-height";
    88, MARGIN, "margin";
    89, PADDING, "padding";
    90, BORDER, "border";
    91, BORDER_RADIUS, "border-radius";
    92, COLOR, "color";
    93, BACKGROUND_COLOR, "background-color";
    94, OPACITY, "opacity";
    95, Z_INDEX, "z-index";
    96, FONT_FAMILY, "font-family";
    97, FONT_SIZE, "font-size";
    98, FONT_WEIGHT, "font-weight";
    99, LINE_HEIGHT, "line-height";
    100, TEXT_ALIGN, "text-align";
    101, FLEX, "flex";
    102, FLEX_DIRECTION, "flex-direction";
    103, FLEX_WRAP, "flex-wrap";
    104, JUSTIFY_CONTENT, "justify-content";
    105, ALIGN_ITEMS, "align-items";
    106, GRID, "grid";
    107, GAP, "gap";
    108, OVERFLOW, "overflow";
    109, VISIBILITY, "visibility";
    110, TRANSFORM, "transform";
    111, BOX_SIZING, "box-sizing";
    112, CURSOR, "cursor";
}
