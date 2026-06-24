use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::style::Stylesheet;
use crate::ace::engine::layout::ElementGeometry;
use crate::utils::time::unix_timestamp_secs_f64;
use taffy::geometry::MinMax;
use crate::ace::engine::core::AceEngine;



impl AceEngine {
    pub(crate) fn convert_to_taffy_style(
        &self,
        style: &crate::ace::engine::style::css_values::ComputedStyle,
    ) -> taffy::prelude::Style {
        let mut t_style = taffy::prelude::Style::default();

        t_style.display = match style.display {
            crate::ace::engine::style::css_values::CssDisplay::Grid
            | crate::ace::engine::style::css_values::CssDisplay::Table => taffy::prelude::Display::Grid,
            crate::ace::engine::style::css_values::CssDisplay::Flex => taffy::prelude::Display::Flex,
            crate::ace::engine::style::css_values::CssDisplay::None => taffy::prelude::Display::None,
            crate::ace::engine::style::css_values::CssDisplay::Contents
            | crate::ace::engine::style::css_values::CssDisplay::TableRow
            | crate::ace::engine::style::css_values::CssDisplay::TableHeader => {
                taffy::prelude::Display::None
            } // Will be flattened/handled manually
            _ => taffy::prelude::Display::Flex,
        };

        t_style.flex_direction = match style.display {
            crate::ace::engine::style::css_values::CssDisplay::Block => {
                taffy::prelude::FlexDirection::Column
            }
            _ => match style.flex_direction {
                crate::ace::engine::style::css_values::CssFlexDirection::Row => {
                    taffy::prelude::FlexDirection::Row
                }
                crate::ace::engine::style::css_values::CssFlexDirection::Column => {
                    taffy::prelude::FlexDirection::Column
                }
                crate::ace::engine::style::css_values::CssFlexDirection::RowReverse => {
                    taffy::prelude::FlexDirection::RowReverse
                }
                crate::ace::engine::style::css_values::CssFlexDirection::ColumnReverse => {
                    taffy::prelude::FlexDirection::ColumnReverse
                }
            },
        };

        t_style.flex_wrap = match style.flex_wrap {
            crate::ace::engine::style::css_values::CssFlexWrap::NoWrap => {
                taffy::prelude::FlexWrap::NoWrap
            }
            crate::ace::engine::style::css_values::CssFlexWrap::Wrap => taffy::prelude::FlexWrap::Wrap,
            crate::ace::engine::style::css_values::CssFlexWrap::WrapReverse => {
                taffy::prelude::FlexWrap::WrapReverse
            }
        };

        t_style.position = match style.position {
            crate::ace::engine::style::css_values::CssPosition::Absolute
            | crate::ace::engine::style::css_values::CssPosition::Fixed => {
                taffy::prelude::Position::Absolute
            }
            _ => {
                if style.float != crate::ace::engine::style::css_values::CssFloat::None {
                    taffy::prelude::Position::Absolute
                } else {
                    taffy::prelude::Position::Relative
                }
            }
        };

        t_style.size = taffy::prelude::Size {
            width: self.to_taffy_dimension(&style.width),
            height: self.to_taffy_dimension(&style.height),
        };

        t_style.min_size = taffy::prelude::Size {
            width: self.to_taffy_dimension(&style.min_width),
            height: self.to_taffy_dimension(&style.min_height),
        };

        t_style.max_size = taffy::prelude::Size {
            width: self.to_taffy_dimension(&style.max_width),
            height: self.to_taffy_dimension(&style.max_height),
        };

        t_style.margin = taffy::prelude::Rect {
            left: self.to_taffy_length_percentage(&style.margin_left).into(),
            right: self.to_taffy_length_percentage(&style.margin_right).into(),
            top: self.to_taffy_length_percentage(&style.margin_top).into(),
            bottom: self.to_taffy_length_percentage(&style.margin_bottom).into(),
        };

        t_style.padding = taffy::prelude::Rect {
            left: self.to_taffy_length_percentage(&style.padding_left).into(),
            right: self.to_taffy_length_percentage(&style.padding_right).into(),
            top: self.to_taffy_length_percentage(&style.padding_top).into(),
            bottom: self
                .to_taffy_length_percentage(&style.padding_bottom)
                .into(),
        };

        t_style.gap = taffy::prelude::Size {
            width: self.to_taffy_length_percentage(&style.grid_column_gap),
            height: self.to_taffy_length_percentage(&style.grid_row_gap),
        };

        // Filter out LineNames before passing to Taffy
        t_style.grid_template_columns = style
            .grid_template_columns
            .iter()
            .filter(|l| !matches!(l, crate::ace::engine::style::css_values::CssLength::LineNames(_)))
            .map(|l| self.to_taffy_track_size(l))
            .collect();

        t_style.grid_template_rows = style
            .grid_template_rows
            .iter()
            .filter(|l| !matches!(l, crate::ace::engine::style::css_values::CssLength::LineNames(_)))
            .map(|l| self.to_taffy_track_size(l))
            .collect();

        // Resolve Placements
        t_style.grid_column.start = self.resolve_placement(&style.grid_column_start);
        t_style.grid_column.end = self.resolve_placement(&style.grid_column_end);
        t_style.grid_row.start = self.resolve_placement(&style.grid_row_start);
        t_style.grid_row.end = self.resolve_placement(&style.grid_row_end);

        t_style
    }
}
