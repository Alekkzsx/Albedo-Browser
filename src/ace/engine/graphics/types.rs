// engine/types.rs - Primitive Enums for Zero-Cost Layout and Rendering

// FASE 2: Otimização Zero-Cost (Remoção de Strings)

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ElementRenderType {
    Div,
    Input,
    Svg,
    Image,
    Textarea,
    Select,
    Button,
    Other,
}

impl Default for ElementRenderType {
    /// TODO: add docs
    fn default() -> Self {
        Self::Other
    }
}

impl ElementRenderType {
    /// TODO: add docs
    pub fn from_str(tag: &str) -> Self {
        match tag {
            "div" => Self::Div,
            "input" => Self::Input,
            "svg" => Self::Svg,
            "img" => Self::Image,
            "textarea" => Self::Textarea,
            "select" => Self::Select,
            "button" => Self::Button,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormInputType {
    Color,
    Range,
    Date,
    Time,
    Text,
    Checkbox,
    Radio,
    Number,
    Password,
    Email,
    None,
}

impl Default for FormInputType {
    /// TODO: add docs
    fn default() -> Self {
        Self::None
    }
}

impl FormInputType {
    /// TODO: add docs
    pub fn from_str(input_type: &str) -> Self {
        match input_type {
            "color" => Self::Color,
            "range" => Self::Range,
            "date" => Self::Date,
            "time" => Self::Time,
            "text" => Self::Text,
            "checkbox" => Self::Checkbox,
            "radio" => Self::Radio,
            "number" => Self::Number,
            "password" => Self::Password,
            "email" => Self::Email,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BorderStyle {
    Solid,
    Dashed,
    Dotted,
    None,
}

impl Default for BorderStyle {
    /// TODO: add docs
    fn default() -> Self {
        Self::None
    }
}

impl BorderStyle {
    /// TODO: add docs
    pub fn from_str(style: &str) -> Self {
        match style {
            "solid" => Self::Solid,
            "dashed" => Self::Dashed,
            "dotted" => Self::Dotted,
            _ => Self::None,
        }
    }
}
