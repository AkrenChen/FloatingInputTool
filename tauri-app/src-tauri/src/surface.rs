use crate::window_behavior::{DockSide, StoredWindowBounds};

pub const SHADOW_INSET: u32 = 72;
pub const CAPTURE_INSET: u32 = 0;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceEffects {
    pub focused_blur: f32,
    pub idle_blur: f32,
    pub focused_tint: f32,
    pub idle_tint: f32,
    pub focused_opacity: f32,
    pub idle_opacity: f32,
    pub focused_material_opacity: f32,
    pub idle_material_opacity: f32,
    pub shadow_blur: f32,
    pub shadow_opacity: f32,
    pub idle_shadow_opacity: f32,
    pub shadow_y: f32,
    pub corner_radius: f32,
    pub preview: String,
}

impl Default for SurfaceEffects {
    fn default() -> Self {
        serde_json::from_str(include_str!("../../src/surfaceTheme.json"))
            .expect("bundled surface theme must be valid")
    }
}

impl SurfaceEffects {
    pub fn validate(&self) -> Result<(), String> {
        let values = [
            self.focused_blur,
            self.idle_blur,
            self.focused_tint,
            self.idle_tint,
            self.focused_opacity,
            self.idle_opacity,
            self.focused_material_opacity,
            self.idle_material_opacity,
            self.shadow_blur,
            self.shadow_opacity,
            self.idle_shadow_opacity,
            self.shadow_y,
            self.corner_radius,
        ];
        if values.iter().any(|value| !value.is_finite())
            || !(0.0..=64.0).contains(&self.focused_blur)
            || !(0.0..=64.0).contains(&self.idle_blur)
            || [
                self.focused_tint,
                self.idle_tint,
                self.focused_material_opacity,
                self.idle_material_opacity,
                self.shadow_opacity,
                self.idle_shadow_opacity,
            ]
            .iter()
            .any(|v| !(0.0..=1.0).contains(v))
            || [self.focused_opacity, self.idle_opacity]
                .iter()
                .any(|v| !(0.1..=1.0).contains(v))
            || !(0.0..=24.0).contains(&self.shadow_blur)
            || !(-12.0..=12.0).contains(&self.shadow_y)
            || !(0.0..=32.0).contains(&self.corner_radius)
            || !matches!(self.preview.as_str(), "follow" | "focused" | "idle")
        {
            return Err("surface parameter outside supported range".into());
        }
        Ok(())
    }
}

pub fn content_bounds(width: f32, height: f32, scale: f32) -> (f32, f32, f32) {
    let inset = CAPTURE_INSET as f32 * scale;
    (
        inset,
        (width - 2.0 * inset).max(1.0),
        (height - 2.0 * inset).max(1.0),
    )
}

pub fn visible_bounds(bounds: StoredWindowBounds, inset: i32) -> StoredWindowBounds {
    StoredWindowBounds {
        x: bounds.x + inset,
        y: bounds.y + inset,
        width: bounds.width.saturating_sub(2 * inset as u32).max(1),
        height: bounds.height.saturating_sub(2 * inset as u32).max(1),
    }
}

pub fn restore_x(
    side: Option<DockSide>,
    bounds: StoredWindowBounds,
    left: i32,
    right: i32,
    inset: i32,
) -> i32 {
    let minimum = left - inset;
    let maximum = (right - bounds.width as i32 + inset).max(minimum);
    match side {
        Some(DockSide::Left) => minimum,
        Some(DockSide::Right) => maximum,
        None => bounds.x.clamp(minimum, maximum),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_bounds_match_visible_panel_and_docked_restore_edges() {
        for scale in [1, 2] {
            let inset = CAPTURE_INSET as i32 * scale;
            let outer = StoredWindowBounds {
                x: -1920 - inset,
                y: -inset,
                width: 560 * scale as u32,
                height: 360 * scale as u32,
            };
            let visible = visible_bounds(outer, inset);
            assert_eq!(
                (visible.x, visible.y, visible.width, visible.height),
                (-1920, 0, 560 * scale as u32, 360 * scale as u32)
            );
            assert_eq!(
                restore_x(Some(DockSide::Left), outer, -1920, 0, inset) + inset,
                -1920
            );
            assert_eq!(
                restore_x(Some(DockSide::Right), outer, -1920, 0, inset) + outer.width as i32
                    - inset,
                0
            );
        }
    }
    #[test]
    fn final_theme_values_are_independent() {
        let effects = SurfaceEffects::default();
        effects.validate().unwrap();
        assert_eq!(effects.idle_shadow_opacity, 0.1);
        assert_eq!(
            (effects.idle_opacity, effects.idle_material_opacity),
            (0.34, 0.85)
        );
        assert_eq!(
            (
                effects.shadow_blur,
                effects.shadow_opacity,
                effects.corner_radius
            ),
            (5.0, 0.24, 24.0)
        );
    }
}
