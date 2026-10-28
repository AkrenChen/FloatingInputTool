pub const MARGIN: f32 = 72.0;

pub fn content_bounds(width: f32, height: f32, scale: f32) -> (f32, f32, f32) {
    let margin = MARGIN * scale;
    (
        margin,
        (width - margin * 2.0).max(1.0),
        (height - margin * 2.0).max(1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_bounds_match_the_css_content_at_different_dpi() {
        for scale in [1.0, 1.5, 2.0] {
            let (margin, width, height) = content_bounds(720.0 * scale, 820.0 * scale, scale);
            assert_eq!(margin / scale, 72.0);
            assert_eq!(width / scale, 576.0);
            assert_eq!(height / scale, 676.0);
        }
    }

    #[test]
    fn content_has_a_positive_extent_during_minimized_resize() {
        let (_, width, height) = content_bounds(0.0, 0.0, 2.0);
        assert!(width >= 1.0 && height >= 1.0);
    }
}
