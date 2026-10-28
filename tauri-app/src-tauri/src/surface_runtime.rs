use crate::surface::SurfaceEffects;
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceRequest {
    pub effects: SurfaceEffects,
    pub color: [u8; 3],
}

impl Default for SurfaceRequest {
    fn default() -> Self {
        Self {
            effects: SurfaceEffects::default(),
            color: [246, 246, 251],
        }
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SurfaceStatus {
    available: bool,
    focused: bool,
    active: bool,
    interface_opacity: f32,
    inset: u32,
    #[cfg(windows)]
    applied: Option<crate::native_composition::Applied>,
}

#[cfg(windows)]
thread_local! {
    static SCENE: std::cell::RefCell<Option<crate::native_composition::Scene>> = const { std::cell::RefCell::new(None) };
    static REQUEST: std::cell::RefCell<SurfaceRequest> = std::cell::RefCell::new(SurfaceRequest::default());
}

pub fn refresh<R: Runtime>(app: &AppHandle<R>, request: Option<SurfaceRequest>) {
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || refresh_on_ui(&handle, request)) {
        log::warn!("failed to schedule main surface refresh: {error}");
    }
}

#[cfg(windows)]
fn refresh_on_ui<R: Runtime>(app: &AppHandle<R>, request: Option<SurfaceRequest>) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetForegroundWindow, GetSystemMetrics, GA_ROOT, SM_REMOTESESSION,
    };
    let Some(window) = app.get_webview_window(crate::WINDOW_LABEL_MAIN) else {
        return;
    };
    if let Some(request) = request {
        REQUEST.with_borrow_mut(|current| *current = request);
    }
    let request = REQUEST.with_borrow(Clone::clone);
    let docked = app
        .state::<crate::AppState>()
        .main_window_state
        .lock()
        .map(|state| state.mode.is_docked())
        .unwrap_or(false);
    let focused = window
        .hwnd()
        .map(|hwnd| unsafe { std::ptr::eq(GetAncestor(GetForegroundWindow(), GA_ROOT).0, hwnd.0) })
        .unwrap_or(false);
    let active = match request.effects.preview.as_str() {
        "focused" => true,
        "idle" => false,
        _ => focused,
    };
    let result = SCENE.with(|slot| -> Result<_, String> {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| "surface refresh already in progress")?;
        if unsafe { GetSystemMetrics(SM_REMOTESESSION) } != 0 {
            *slot = None;
            return Err("host backdrop disabled in remote session".into());
        }
        let size = window.inner_size().map_err(|e| e.to_string())?;
        let scale = window.scale_factor().map_err(|e| e.to_string())? as f32;
        if slot.is_none() {
            let hwnd = window.hwnd().map_err(|e| e.to_string())?;
            let _ = crate::acrylic::set_enabled(hwnd.0 as _, false);
            *slot = Some(
                crate::native_composition::Scene::new(
                    windows::Win32::Foundation::HWND(hwnd.0),
                    size.width as f32,
                    size.height as f32,
                    scale,
                )
                .map_err(|e| format!("composition initialization: {e}"))?,
            );
            log::info!("main native composition initialized");
        }
        let scene = slot.as_mut().unwrap();
        scene.set_color(request.color);
        scene
            .resize(size.width as f32, size.height as f32, scale)
            .map_err(|e| e.to_string())?;
        scene
            .apply(&request.effects, focused)
            .map_err(|e| e.to_string())?;
        scene.set_docked(docked).map_err(|e| e.to_string())?;
        scene.readback().map_err(|e| e.to_string())
    });
    let available = result.is_ok();
    if let Err(error) = &result {
        log::warn!("main native composition unavailable: {error}");
        SCENE.with(|slot| {
            if let Ok(mut scene) = slot.try_borrow_mut() {
                *scene = None;
            }
        });
    }
    if let Ok(mut state) = app.state::<crate::AppState>().native_backdrop.lock() {
        state.main = available;
    }
    let _ = window.emit(
        crate::EVENT_NATIVE_BACKDROP_CHANGED,
        crate::NativeBackdropStatus { available },
    );
    let _ = window.emit(
        "main-surface-state-changed",
        SurfaceStatus {
            available,
            focused,
            active,
            interface_opacity: if available && !docked {
                if active {
                    request.effects.focused_opacity
                } else {
                    request.effects.idle_opacity
                }
            } else {
                1.0
            },
            inset: if docked {
                0
            } else {
                crate::surface::SHADOW_INSET
            },
            applied: result.ok(),
        },
    );
}

#[cfg(not(windows))]
fn refresh_on_ui<R: Runtime>(app: &AppHandle<R>, _request: Option<SurfaceRequest>) {
    let _ = app.emit(
        "main-surface-state-changed",
        SurfaceStatus {
            available: false,
            focused: true,
            active: true,
            interface_opacity: 1.0,
            inset: 72,
        },
    );
}

pub fn clear() {
    #[cfg(windows)]
    SCENE.with_borrow_mut(|scene| *scene = None);
}
