#[cfg(test)]
mod tests {
    use super::super::css_values::{CssAlignItems, CssLength};
    use super::super::parse_align_items;
    use super::super::parse_grid_track_list;
    use super::*;

    #[test]
    fn test_parse_grid_track_list_simple() {
        let input = "100px 1fr auto";
        let tracks = parse_grid_track_list(input);
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0], CssLength::Px(100.0));
        assert_eq!(tracks[1], CssLength::Fr(1.0));
        assert_eq!(tracks[2], CssLength::Auto);
    }

    #[test]
    fn test_parse_grid_track_list_repeat() {
        let input = "repeat(3, 1fr)";
        let tracks = parse_grid_track_list(input);
        // Should expand to 3 tracks
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0], CssLength::Fr(1.0));
        assert_eq!(tracks[1], CssLength::Fr(1.0));
        assert_eq!(tracks[2], CssLength::Fr(1.0));
    }

    #[test]
    fn test_parse_grid_track_list_repeat_auto_fill() {
        let input = "repeat(auto-fill, 100px)";
        let tracks = parse_grid_track_list(input);
        assert_eq!(tracks.len(), 1);
        if let CssLength::Repeat(count, sub) = &tracks[0] {
            assert_eq!(count, "auto-fill");
            assert_eq!(sub.len(), 1);
            assert_eq!(sub[0], CssLength::Px(100.0));
        } else {
            panic!("Expected Repeat variant");
        }
    }

    #[test]
    fn test_parse_grid_track_list_minmax() {
        let input = "minmax(100px, 1fr)";
        let tracks = parse_grid_track_list(input);
        assert_eq!(tracks.len(), 1);
        if let CssLength::MinMax(min, max) = &tracks[0] {
            assert_eq!(**min, CssLength::Px(100.0));
            assert_eq!(**max, CssLength::Fr(1.0));
        } else {
            panic!("Expected MinMax variant");
        }
    }

    #[test]
    fn test_parse_align_items_auto() {
        assert_eq!(parse_align_items("auto"), CssAlignItems::Auto);
    }

    #[test]
    fn test_parse_grid_track_list_named_lines() {
        let input = "[header-start] 100px [header-end main-start] 1fr [main-end]";
        let tracks = parse_grid_track_list(input);
        assert_eq!(tracks.len(), 5);

        if let CssLength::LineNames(names) = &tracks[0] {
            assert_eq!(names, &vec!["header-start".to_string()]);
        } else {
            panic!("Expected LineNames");
        }

        assert_eq!(tracks[1], CssLength::Px(100.0));

        if let CssLength::LineNames(names) = &tracks[2] {
            assert_eq!(
                names,
                &vec!["header-end".to_string(), "main-start".to_string()]
            );
        } else {
            panic!("Expected LineNames");
        }

        assert_eq!(tracks[3], CssLength::Fr(1.0));

        if let CssLength::LineNames(names) = &tracks[4] {
            assert_eq!(names, &vec!["main-end".to_string()]);
        } else {
            panic!("Expected LineNames");
        }
    }

    #[test]
    fn test_parse_grid_template_areas() {
        use super::super::parse_grid_template_areas;
        let input = "\"header header\" \"main sidebar\" \"footer footer\"";
        let areas = parse_grid_template_areas(input);
        assert_eq!(areas.len(), 3);
        assert_eq!(areas[0], "header header");
        assert_eq!(areas[1], "main sidebar");
        assert_eq!(areas[2], "footer footer");
    }

    #[test]
    fn test_parse_display_contents() {
        use super::super::parse_display;
        use crate::engine::style::css_values::CssDisplay;
        assert_eq!(parse_display("contents"), CssDisplay::Contents);
    }

    #[test]
    fn test_parse_grid_template_subgrid() {
        let input = "subgrid";
        let tracks = parse_grid_track_list(input);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0], CssLength::Subgrid);
    }
}
