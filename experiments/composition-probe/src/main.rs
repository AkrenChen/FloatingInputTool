#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod composition;
mod effects;
mod layout;

use serde::{Deserialize, Serialize};
use std::{cell::RefCell, sync::Mutex};
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Parameters {
    focused_blur: f32,
    idle_blur: f32,
    focused_tint: f32,
    idle_tint: f32,
    focused_opacity: f32,
    idle_opacity: f32,
    focused_material_opacity: f32,
    idle_material_opacity: f32,
    shadow_blur: f32,
    shadow_opacity: f32,
    shadow_y: f32,
    corner_radius: f32,
    preview: String,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            focused_blur: 0.0,
            idle_blur: 0.0,
            focused_tint: 0.0,
            idle_tint: 0.0,
            focused_opacity: 1.0,
            idle_opacity: 0.34,
            focused_material_opacity: 1.0,
            idle_material_opacity: 0.85,
            shadow_blur: 5.0,
            shadow_opacity: 0.24,
            shadow_y: 2.0,
            corner_radius: 24.0,
            preview: "follow".into(),
        }
    }
}

struct ProbeState {
    parameters: Mutex<Parameters>,
    status: Mutex<String>,
}
thread_local! { static SCENE: RefCell<Option<composition::Scene>> = const { RefCell::new(None) }; }

fn log_status(app: &tauri::AppHandle, message: String) {
    eprintln!("{message}");
    *app.state::<ProbeState>().status.lock().unwrap() = message.clone();
    let _ = app.emit("probe-status", message);
}

#[tauri::command]
fn probe_status(state: tauri::State<ProbeState>) -> String {
    state.status.lock().unwrap().clone()
}

fn probe_focused(app: &tauri::AppHandle) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GetForegroundWindow, GA_ROOT};
    let Some(window) = app.get_webview_window("main") else {
        return false;
    };
    let Ok(hwnd) = window.hwnd() else {
        return false;
    };
    unsafe { std::ptr::eq(GetAncestor(GetForegroundWindow(), GA_ROOT).0, hwnd.0) }
}

#[tauri::command]
async fn update_parameters(
    app: tauri::AppHandle,
    parameters: Parameters,
) -> Result<composition::Applied, String> {
    let values = [
        parameters.focused_blur,
        parameters.idle_blur,
        parameters.focused_tint,
        parameters.idle_tint,
        parameters.focused_opacity,
        parameters.idle_opacity,
        parameters.focused_material_opacity,
        parameters.idle_material_opacity,
        parameters.shadow_blur,
        parameters.shadow_opacity,
        parameters.shadow_y,
        parameters.corner_radius,
    ];
    if values.iter().any(|v| !v.is_finite())
        || !(0.0..=64.0).contains(&parameters.focused_blur)
        || !(0.0..=64.0).contains(&parameters.idle_blur)
        || !(0.0..=1.0).contains(&parameters.focused_tint)
        || !(0.0..=1.0).contains(&parameters.idle_tint)
        || !(0.1..=1.0).contains(&parameters.focused_opacity)
        || !(0.1..=1.0).contains(&parameters.idle_opacity)
        || !(0.0..=1.0).contains(&parameters.focused_material_opacity)
        || !(0.0..=1.0).contains(&parameters.idle_material_opacity)
        || !(0.0..=24.0).contains(&parameters.shadow_blur)
        || !(0.0..=1.0).contains(&parameters.shadow_opacity)
        || !(-12.0..=12.0).contains(&parameters.shadow_y)
        || !(0.0..=32.0).contains(&parameters.corner_radius)
    {
        return Err("Parameter outside probe limits".into());
    }
    *app.state::<ProbeState>().parameters.lock().unwrap() = parameters.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let result = SCENE.with_borrow_mut(|scene| {
            let focused = probe_focused(&handle);
            let scene = scene
                .as_mut()
                .ok_or("Native scene not initialized".to_string())?;
            scene
                .apply(&parameters, focused)
                .and_then(|_| scene.readback())
                .map_err(|e| format!("Composition update: {e}"))
        });
        if let Err(message) = &result {
            log_status(&handle, message.clone());
        }
        let _ = tx.send(result);
    })
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rx.recv().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())??
}

fn main() {
    tauri::Builder::default()
        .manage(ProbeState {
            parameters: Mutex::new(Parameters::default()),
            status: Mutex::new("Starting native composition".into()),
        })
        .invoke_handler(tauri::generate_handler![probe_status, update_parameters])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let size = window.inner_size()?;
            let hwnd = windows::Win32::Foundation::HWND(window.hwnd()?.0 as _);
            match composition::Scene::new(
                hwnd,
                size.width as f32,
                size.height as f32,
                window.scale_factor()? as f32,
            ) {
                Ok(scene) => {
                    SCENE.with_borrow_mut(|slot| *slot = Some(scene));
                    log_status(
                        app.handle(),
                        "Native host-backdrop + Gaussian effect initialized (windowed WebView2)"
                            .into(),
                    );
                }
                Err(error) => log_status(
                    app.handle(),
                    format!("Native initialization failed: {error}"),
                ),
            }
            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                let result: windows::core::Result<()> = SCENE.with_borrow_mut(|slot| {
                    let Some(scene) = slot.as_mut() else {
                        return Ok(());
                    };
                    let focused = probe_focused(&handle);
                    match event {
                        tauri::WindowEvent::Resized(size) => {
                            let scale = handle
                                .get_webview_window("main")
                                .and_then(|w| w.scale_factor().ok())
                                .unwrap_or(1.0) as f32;
                            scene.resize(size.width as f32, size.height as f32, scale)?;
                            scene.apply(
                                &handle.state::<ProbeState>().parameters.lock().unwrap(),
                                focused,
                            )
                        }
                        tauri::WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                            ..
                        } => {
                            scene.resize(
                                new_inner_size.width as f32,
                                new_inner_size.height as f32,
                                *scale_factor as f32,
                            )?;
                            scene.apply(
                                &handle.state::<ProbeState>().parameters.lock().unwrap(),
                                focused,
                            )
                        }
                        tauri::WindowEvent::Focused(_) => scene.apply(
                            &handle.state::<ProbeState>().parameters.lock().unwrap(),
                            focused,
                        ),
                        _ => return Ok(()),
                    }?;
                    let _ = handle.emit("probe-render-state", scene.readback()?);
                    Ok(())
                });
                if let Err(error) = result {
                    log_status(&handle, format!("Native window event: {error}"));
                }
            });
            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                    handle.exit(0);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("composition probe runtime failed");
}
