use crate::{window_behavior::HoverDeadline, *};

#[tauri::command]
pub(crate) fn set_capture_interaction(
    state: State<'_, AppState>,
    composing: Option<bool>,
    saving: Option<bool>,
) {
    if let Some(value) = composing {
        state.composing.store(value, Ordering::SeqCst);
    }
    if let Some(value) = saving {
        state.saving.store(value, Ordering::SeqCst);
    }
}

pub(crate) fn start<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(windows)]
    {
        let app = app.clone();
        thread::spawn(move || {
            let mut deadline = HoverDeadline::default();
            let mut previous = MainWindowMode::Normal;
            let mut wait_for_leave = false;
            loop {
                thread::sleep(Duration::from_millis(50));
                let Some(window) = app.get_webview_window(WINDOW_LABEL_MAIN) else {
                    break;
                };
                let state = app.state::<AppState>();
                let runtime = match state.main_window_state.lock() {
                    Ok(value) => value.clone(),
                    Err(_) => break,
                };
                if runtime.mode != previous {
                    deadline = HoverDeadline::default();
                    wait_for_leave = runtime.mode.is_docked();
                    previous = runtime.mode;
                }
                if runtime.mode == MainWindowMode::Normal {
                    continue;
                }
                let enabled =
                    window.is_enabled().unwrap_or(false) && window.is_visible().unwrap_or(false);
                let Some((x, y)) = current_cursor_position() else {
                    continue;
                };
                let Ok(bounds) = capture_window_bounds(&window) else {
                    continue;
                };
                let Ok(scale) = window.scale_factor() else {
                    continue;
                };
                let config = match state.config.lock() {
                    Ok(value) => value.clone(),
                    Err(_) => break,
                };
                let inset = logical_px_to_physical(surface::CAPTURE_INSET, scale) as i32;
                let pointer_bounds = if runtime.mode.is_docked() {
                    let width =
                        logical_px_to_physical(config.visible_handle_width_px, scale).max(1);
                    StoredWindowBounds {
                        x: if runtime.dock_side == Some(DockSide::Left) {
                            bounds.x + bounds.width as i32 - width as i32
                        } else {
                            bounds.x
                        },
                        y: bounds.y + inset,
                        width,
                        height: bounds.height.saturating_sub(2 * inset as u32),
                    }
                } else {
                    surface::visible_bounds(bounds, inset)
                };
                let inside = x >= pointer_bounds.x
                    && x < pointer_bounds.x + pointer_bounds.width as i32
                    && y >= pointer_bounds.y
                    && y < pointer_bounds.y + pointer_bounds.height as i32;
                if !inside {
                    wait_for_leave = false;
                }
                let busy = state.composing.load(Ordering::SeqCst)
                    || state.saving.load(Ordering::SeqCst)
                    || is_left_mouse_button_down()
                    || state
                        .manual_drag_in_progress
                        .lock()
                        .map(|value| *value)
                        .unwrap_or(true);
                let eligible = enabled
                    && !busy
                    && match runtime.mode {
                        MainWindowMode::DockedLeft | MainWindowMode::DockedRight => {
                            inside && !wait_for_leave
                        }
                        MainWindowMode::ExpandedFromDock => !inside,
                        MainWindowMode::Normal => false,
                    };
                let delay = if runtime.mode.is_docked() {
                    config.hover_open_delay_ms
                } else {
                    config.hover_close_delay_ms
                };
                if !deadline.ready(eligible, current_time_millis(), delay) {
                    continue;
                }
                deadline = HoverDeadline::default();
                let result = if runtime.mode.is_docked() {
                    restore_docked_main_window_inner(&app, &state)
                } else {
                    begin_window_transition(&state)
                        .and_then(|transition| redock_remembered(&app, &state, &transition))
                };
                if let Err(error) = result {
                    log::warn!("native dock observer: {error}");
                }
            }
        });
    }
    #[cfg(not(windows))]
    let _ = app;
}
