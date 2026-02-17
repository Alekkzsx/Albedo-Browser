
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::css_values::{CssLength, CssAlignItems};
    use super::super::parse_grid_track_list;
    use super::super::parse_align_items;

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
}
