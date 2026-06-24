use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::style::Stylesheet;
use crate::ace::engine::layout::ElementGeometry;
use crate::utils::time::unix_timestamp_secs_f64;
use taffy::geometry::MinMax;
use crate::ace::engine::core::AceEngine;



impl AceEngine {
    pub(crate) fn to_taffy_length_percentage(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::LengthPercentage {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::LengthPercentage::Points(*v),
            CssLength::Percent(v) => taffy::prelude::LengthPercentage::Percent(*v / 100.0),
            _ => taffy::prelude::LengthPercentage::Points(0.0),
        }
    }
    pub(crate) fn to_taffy_track_size(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::TrackSizingFunction {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Fr(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Auto,
                max: taffy::prelude::MaxTrackSizingFunction::Fraction(*v),
            }),
            CssLength::Px(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Points(*v),
                ),
                max: taffy::prelude::MaxTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Points(*v),
                ),
            }),
            CssLength::Percent(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Percent(*v / 100.0),
                ),
                max: taffy::prelude::MaxTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Percent(*v / 100.0),
                ),
            }),
            CssLength::MinMax(min, max) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: self.to_taffy_min_track(min),
                max: self.to_taffy_max_track(max),
            }),
            CssLength::Repeat(count, sub) => {
                let repetition = match count.as_str() {
                    "auto-fill" => taffy::prelude::GridTrackRepetition::AutoFill,
                    "auto-fit" => taffy::prelude::GridTrackRepetition::AutoFit,
                    _ => {
                        let c = count.parse::<u16>().unwrap_or(1);
                        taffy::prelude::GridTrackRepetition::Count(c)
                    }
                };
                let tracks = sub
                    .iter()
                    .map(|l| match self.to_taffy_track_size(l) {
                        taffy::prelude::TrackSizingFunction::Single(mm) => mm,
                        _ => MinMax {
                            min: taffy::prelude::MinTrackSizingFunction::Auto,
                            max: taffy::prelude::MaxTrackSizingFunction::Auto,
                        },
                    })
                    .collect();
                taffy::prelude::TrackSizingFunction::Repeat(repetition, tracks)
            }
            _ => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Auto,
                max: taffy::prelude::MaxTrackSizingFunction::Auto,
            }),
        }
    }
    pub(crate) fn to_taffy_min_track(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::MinTrackSizingFunction {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::MinTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Points(*v),
            ),
            CssLength::Percent(v) => taffy::prelude::MinTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Percent(*v / 100.0),
            ),
            CssLength::MinContent => taffy::prelude::MinTrackSizingFunction::MinContent,
            CssLength::MaxContent => taffy::prelude::MinTrackSizingFunction::MaxContent,
            _ => taffy::prelude::MinTrackSizingFunction::Auto,
        }
    }
    pub(crate) fn to_taffy_max_track(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::MaxTrackSizingFunction {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::MaxTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Points(*v),
            ),
            CssLength::Percent(v) => taffy::prelude::MaxTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Percent(*v / 100.0),
            ),
            CssLength::Fr(v) => taffy::prelude::MaxTrackSizingFunction::Fraction(*v),
            CssLength::MinContent => taffy::prelude::MaxTrackSizingFunction::MinContent,
            CssLength::MaxContent => taffy::prelude::MaxTrackSizingFunction::MaxContent,
            _ => taffy::prelude::MaxTrackSizingFunction::Auto,
        }
    }
}
