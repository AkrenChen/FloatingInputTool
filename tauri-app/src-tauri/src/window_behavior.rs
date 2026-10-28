#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DockSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MainWindowMode {
    #[default]
    Normal,
    DockedLeft,
    DockedRight,
    ExpandedFromDock,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct StoredWindowBounds {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Default)]
pub(crate) struct HoverDeadline(Option<u128>);

impl HoverDeadline {
    pub(crate) fn ready(&mut self, eligible: bool, now: u128, delay: u64) -> bool {
        if !eligible {
            self.0 = None;
            return false;
        }
        now.saturating_sub(*self.0.get_or_insert(now)) >= delay as u128
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MainWindowRuntimeState {
    pub(crate) mode: MainWindowMode,
    pub(crate) dock_side: Option<DockSide>,
    pub(crate) normal_bounds: Option<StoredWindowBounds>,
    pub(crate) docked_bounds: Option<StoredWindowBounds>,
}

impl DockSide {
    pub(crate) fn to_docked_mode(self) -> MainWindowMode {
        match self {
            Self::Left => MainWindowMode::DockedLeft,
            Self::Right => MainWindowMode::DockedRight,
        }
    }
}
impl MainWindowMode {
    pub(crate) fn is_docked(self) -> bool {
        matches!(self, Self::DockedLeft | Self::DockedRight)
    }
    pub(crate) fn is_resizable(self) -> bool {
        !self.is_docked()
    }
}
pub(crate) fn logical_px_to_physical(value: u32, scale: f64) -> u32 {
    (value as f64 * scale).round() as u32
}

pub(crate) fn finish_manual_drag(dragging: &std::sync::Mutex<bool>) -> Result<bool, String> {
    let mut active = dragging.lock().map_err(|error| error.to_string())?;
    Ok(std::mem::take(&mut *active))
}
impl MainWindowRuntimeState {
    pub(crate) fn dock(
        &self,
        side: DockSide,
        normal: StoredWindowBounds,
        docked: StoredWindowBounds,
    ) -> Self {
        if self.mode.is_docked() {
            return self.clone();
        }
        Self {
            mode: side.to_docked_mode(),
            dock_side: Some(side),
            normal_bounds: Some(normal),
            docked_bounds: Some(docked),
        }
    }
    pub(crate) fn capture_finished_dock(&self) -> Option<(DockSide, StoredWindowBounds)> {
        if self.mode == MainWindowMode::ExpandedFromDock {
            self.dock_side.zip(self.docked_bounds)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_settle_then_editor_mouseup_finishes_one_drag_only() {
        let dragging = std::sync::Mutex::new(true);
        assert!(finish_manual_drag(&dragging).unwrap());
        assert!(!finish_manual_drag(&dragging).unwrap());
        *dragging.lock().unwrap() = true;
        assert!(finish_manual_drag(&dragging).unwrap());
    }
    #[test]
    fn native_hover_delay_restarts_after_returning_inside() {
        let mut hover = HoverDeadline::default();
        assert!(!hover.ready(true, 100, 180));
        assert!(!hover.ready(true, 279, 180));
        assert!(!hover.ready(false, 280, 180));
        assert!(!hover.ready(true, 300, 180));
        assert!(hover.ready(true, 480, 180));
    }
    #[test]
    fn repeated_hide_preserves_docked_geometry() {
        let normal = StoredWindowBounds {
            x: 0,
            y: 80,
            width: 560,
            height: 360,
        };
        for side in [DockSide::Left, DockSide::Right] {
            let docked = StoredWindowBounds { x: -524, ..normal };
            let state = MainWindowRuntimeState::default().dock(side, normal, docked);
            assert_eq!(state.dock(side, docked, docked), state);
            assert_eq!(state.normal_bounds, Some(normal));
        }
    }
    #[test]
    fn capture_normal_mode_is_resizable() {
        assert!(MainWindowMode::Normal.is_resizable());
    }
    #[test]
    fn dpi_dimensions_are_physical() {
        for (scale, expected) in [(1.0, 36), (1.5, 54), (2.0, 72)] {
            assert_eq!(logical_px_to_physical(36, scale), expected);
        }
    }
    #[test]
    fn finish_capture_keeps_remembered_side_and_bounds() {
        let bounds = StoredWindowBounds {
            x: -500,
            y: 80,
            width: 560,
            height: 360,
        };
        let state = MainWindowRuntimeState {
            mode: MainWindowMode::ExpandedFromDock,
            dock_side: Some(DockSide::Left),
            normal_bounds: None,
            docked_bounds: Some(bounds),
        };
        assert_eq!(
            state.capture_finished_dock(),
            Some((DockSide::Left, bounds))
        );
        assert_eq!(
            MainWindowRuntimeState::default().capture_finished_dock(),
            None
        );
    }
}
