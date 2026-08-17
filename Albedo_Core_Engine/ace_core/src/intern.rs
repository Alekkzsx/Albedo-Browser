//! # O(1) String Pooling & Static Atoms
//!
//! A comparação de strings é o calcanhar de Aquiles de qualquer parser HTML/CSS.
//! No Albedo, representamos tags (`div`, `span`), atributos (`class`, `id`) e propriedades CSS (`color`)
//! como "Atoms" (átomos).
//!
//! Quando um Atom é criado, a string é internada em um pool global.
//! Comparar dois Atoms custa 1 ciclo de CPU, pois apenas o identificador numérico interno é comparado.

use std::fmt;
use std::ops::Deref;
use string_cache::DefaultAtom;

/// Representa uma string otimizada e única em toda a execução do navegador.
/// A comparação de igualdade `Atom == Atom` é $O(1)$.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Atom(DefaultAtom);

impl Atom {
    /// Interna uma string dinamicamente em tempo de execução.
    #[inline]
    pub fn new(text: &str) -> Self {
        Self(DefaultAtom::from(text))
    }

    /// Retorna a representação textual do Atom como `&str`.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
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

impl From<&str> for Atom {
    #[inline]
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Atom {
    #[inline]
    fn from(s: String) -> Self {
        Self(DefaultAtom::from(s))
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
        Self(atom)
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

/// Macro auxiliar para definir átomos pré-computados com lazy initialization global.
macro_rules! lazy_atom {
    ($name:ident, $str:expr) => {
        #[allow(non_snake_case)]
        #[inline]
        pub fn $name() -> $crate::intern::Atom {
            static ATOM: std::sync::LazyLock<$crate::intern::Atom> =
                std::sync::LazyLock::new(|| $crate::intern::Atom::new($str));
            ATOM.clone()
        }
    };
}

/// Conjunto de átomos pré-definidos de altíssima frequência para eliminar alocações
/// durante as fases de tokenização HTML e parsing CSS.
pub mod atoms {
    // Tags HTML
    lazy_atom!(HTML, "html");
    lazy_atom!(HEAD, "head");
    lazy_atom!(BODY, "body");
    lazy_atom!(DIV, "div");
    lazy_atom!(SPAN, "span");
    lazy_atom!(P, "p");
    lazy_atom!(A, "a");
    lazy_atom!(BUTTON, "button");
    lazy_atom!(INPUT, "input");
    lazy_atom!(IMG, "img");
    lazy_atom!(SCRIPT, "script");
    lazy_atom!(STYLE, "style");
    lazy_atom!(LINK, "link");
    lazy_atom!(META, "meta");
    lazy_atom!(TITLE, "title");
    lazy_atom!(IFRAME, "iframe");
    lazy_atom!(TABLE, "table");
    lazy_atom!(THEAD, "thead");
    lazy_atom!(TBODY, "tbody");
    lazy_atom!(TR, "tr");
    lazy_atom!(TH, "th");
    lazy_atom!(TD, "td");
    lazy_atom!(UL, "ul");
    lazy_atom!(OL, "ol");
    lazy_atom!(LI, "li");
    lazy_atom!(FORM, "form");
    lazy_atom!(LABEL, "label");
    lazy_atom!(TEXTAREA, "textarea");
    lazy_atom!(SELECT, "select");
    lazy_atom!(OPTION, "option");
    lazy_atom!(HEADER, "header");
    lazy_atom!(FOOTER, "footer");
    lazy_atom!(NAV, "nav");
    lazy_atom!(SECTION, "section");
    lazy_atom!(ARTICLE, "article");
    lazy_atom!(ASIDE, "aside");
    lazy_atom!(MAIN, "main");
    lazy_atom!(H1, "h1");
    lazy_atom!(H2, "h2");
    lazy_atom!(H3, "h3");
    lazy_atom!(H4, "h4");
    lazy_atom!(H5, "h5");
    lazy_atom!(H6, "h6");
    lazy_atom!(CANVAS, "canvas");
    lazy_atom!(SVG, "svg");
    lazy_atom!(PATH, "path");
    lazy_atom!(VIDEO, "video");
    lazy_atom!(AUDIO, "audio");
    lazy_atom!(SOURCE, "source");
    lazy_atom!(TEMPLATE, "template");
    lazy_atom!(SLOT, "slot");
    lazy_atom!(BR, "br");
    lazy_atom!(HR, "hr");
    lazy_atom!(PRE, "pre");
    lazy_atom!(CODE, "code");
    lazy_atom!(STRONG, "strong");
    lazy_atom!(EM, "em");

    // Atributos HTML
    lazy_atom!(ID, "id");
    lazy_atom!(CLASS, "class");
    lazy_atom!(STYLE_ATTR, "style");
    lazy_atom!(SRC, "src");
    lazy_atom!(HREF, "href");
    lazy_atom!(TYPE, "type");
    lazy_atom!(VALUE, "value");
    lazy_atom!(NAME, "name");
    lazy_atom!(REL, "rel");
    lazy_atom!(CONTENT, "content");
    lazy_atom!(CHARSET, "charset");
    lazy_atom!(ALT, "alt");
    lazy_atom!(WIDTH, "width");
    lazy_atom!(HEIGHT, "height");
    lazy_atom!(DISABLED, "disabled");
    lazy_atom!(CHECKED, "checked");
    lazy_atom!(SELECTED, "selected");
    lazy_atom!(READONLY, "readonly");
    lazy_atom!(PLACEHOLDER, "placeholder");
    lazy_atom!(ACTION, "action");
    lazy_atom!(METHOD, "method");
    lazy_atom!(TARGET, "target");

    // Propriedades CSS
    lazy_atom!(DISPLAY, "display");
    lazy_atom!(POSITION, "position");
    lazy_atom!(TOP, "top");
    lazy_atom!(RIGHT, "right");
    lazy_atom!(BOTTOM, "bottom");
    lazy_atom!(LEFT, "left");
    lazy_atom!(WIDTH_PROP, "width");
    lazy_atom!(HEIGHT_PROP, "height");
    lazy_atom!(MIN_WIDTH, "min-width");
    lazy_atom!(MAX_WIDTH, "max-width");
    lazy_atom!(MIN_HEIGHT, "min-height");
    lazy_atom!(MAX_HEIGHT, "max-height");
    lazy_atom!(MARGIN, "margin");
    lazy_atom!(PADDING, "padding");
    lazy_atom!(BORDER, "border");
    lazy_atom!(BORDER_RADIUS, "border-radius");
    lazy_atom!(COLOR, "color");
    lazy_atom!(BACKGROUND_COLOR, "background-color");
    lazy_atom!(OPACITY, "opacity");
    lazy_atom!(Z_INDEX, "z-index");
    lazy_atom!(FONT_FAMILY, "font-family");
    lazy_atom!(FONT_SIZE, "font-size");
    lazy_atom!(FONT_WEIGHT, "font-weight");
    lazy_atom!(LINE_HEIGHT, "line-height");
    lazy_atom!(TEXT_ALIGN, "text-align");
    lazy_atom!(FLEX, "flex");
    lazy_atom!(FLEX_DIRECTION, "flex-direction");
    lazy_atom!(FLEX_WRAP, "flex-wrap");
    lazy_atom!(JUSTIFY_CONTENT, "justify-content");
    lazy_atom!(ALIGN_ITEMS, "align-items");
    lazy_atom!(GRID, "grid");
    lazy_atom!(GAP, "gap");
    lazy_atom!(OVERFLOW, "overflow");
    lazy_atom!(VISIBILITY, "visibility");
    lazy_atom!(TRANSFORM, "transform");
    lazy_atom!(BOX_SIZING, "box-sizing");
    lazy_atom!(CURSOR, "cursor");
}
