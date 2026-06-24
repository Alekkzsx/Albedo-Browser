/// Inline Formatting Context (IFC) - Algorithm for laying out inline elements and text
/// Based on CSS 2.2 / CSS 3 specification for inline layout
use crate::ace::engine::style::css_values::ComputedStyle;
use crate::ace::engine::text::TextMetrics;

/// Represents an inline box (can be text, inline element, or inline-block)

pub mod inlinebox; pub use inlinebox::*;
pub mod layoutline; pub use layoutline::*;
pub mod inlineformattingcontext; pub use inlineformattingcontext::*;
pub mod inlineformattingcontext_impl_1; pub use inlineformattingcontext_impl_1::*;
pub mod inlineformattingcontext_impl_2; pub use inlineformattingcontext_impl_2::*;
pub mod inlineformattingcontext_impl_3; pub use inlineformattingcontext_impl_3::*;
pub mod linebreaker_; pub use linebreaker_::*;
pub mod linebreaker; pub use linebreaker::*;
pub mod apply_text_transform; pub use apply_text_transform::*;
pub mod test_inline_box_dimensions; pub use test_inline_box_dimensions::*;
