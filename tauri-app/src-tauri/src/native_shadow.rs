use crate::{
    shadow_bitmap::{render, ShadowShape},
    surface::SurfaceEffects,
};
use std::{
    mem::size_of,
    ptr::{null, null_mut},
    sync::OnceLock,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

const SYNC: u32 = WM_APP + 41;
const SUBCLASS: usize = 0x53484457;

struct Bitmap {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
}

impl Bitmap {
    fn new(shape: ShadowShape) -> Result<Self, String> {
        let pixels = render(shape);
        let width = (shape.width + 2 * shape.padding) as i32;
        let height = (shape.height + 2 * shape.padding) as i32;
        unsafe {
            let dc = CreateCompatibleDC(null_mut());
            if dc.is_null() {
                return Err(last_error());
            }
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    ..std::mem::zeroed()
                },
                ..std::mem::zeroed()
            };
            let mut bits = null_mut();
            let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if bitmap.is_null() {
                DeleteDC(dc);
                return Err(last_error());
            }
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast(), pixels.len());
            let previous = SelectObject(dc, bitmap);
            Ok(Self {
                dc,
                bitmap,
                previous,
            })
        }
    }
}

impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

struct State {
    parent: HWND,
    hwnd: HWND,
    shape: Option<ShadowShape>,
    bitmap: Option<Bitmap>,
    alpha: u8,
    enabled: bool,
}

impl State {
    unsafe fn sync(&self) {
        if !self.enabled || IsWindowVisible(self.parent) == 0 || IsIconic(self.parent) != 0 {
            ShowWindow(self.hwnd, SW_HIDE);
            return;
        }
        let (Some(shape), Some(bitmap)) = (self.shape, self.bitmap.as_ref()) else {
            return;
        };
        let mut origin = POINT { x: 0, y: 0 };
        if ClientToScreen(self.parent, &mut origin) == 0 {
            return;
        }
        let position = POINT {
            x: origin.x - shape.padding as i32,
            y: origin.y - shape.padding as i32,
        };
        let size = SIZE {
            cx: (shape.width + 2 * shape.padding) as i32,
            cy: (shape.height + 2 * shape.padding) as i32,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: self.alpha,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        if UpdateLayeredWindow(
            self.hwnd,
            null_mut(),
            &position,
            &size,
            bitmap.dc,
            &POINT { x: 0, y: 0 },
            0,
            &blend,
            ULW_ALPHA,
        ) == 0
        {
            log::warn!("shadow update failed: {}", last_error());
            ShowWindow(self.hwnd, SW_HIDE);
            return;
        }
        SetWindowPos(
            self.hwnd,
            self.parent,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCCREATE => {
            let create = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        WM_NCHITTEST => return HTTRANSPARENT as isize,
        WM_MOUSEACTIVATE => return MA_NOACTIVATE as isize,
        SYNC => {
            let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
            if !state.is_null() {
                (*state).sync();
            }
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

unsafe extern "system" fn parent_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    shadow: usize,
) -> LRESULT {
    let result = DefSubclassProc(hwnd, message, wparam, lparam);
    if matches!(
        message,
        WM_WINDOWPOSCHANGED | WM_SHOWWINDOW | WM_SIZE | WM_DPICHANGED
    ) {
        PostMessageW(shadow as HWND, SYNC, 0, 0);
    }
    if message == WM_NCDESTROY {
        ShowWindow(shadow as HWND, SW_HIDE);
        RemoveWindowSubclass(hwnd, Some(parent_procedure), SUBCLASS);
    }
    result
}

pub(crate) struct ShadowHost {
    state: Box<State>,
}

impl ShadowHost {
    pub fn new(parent: HWND) -> Result<Self, String> {
        static CLASS: OnceLock<Result<u16, String>> = OnceLock::new();
        let class: Vec<u16> = "FloatingInputTool.InputTransparentShadow\0"
            .encode_utf16()
            .collect();
        unsafe {
            CLASS
                .get_or_init(|| {
                    let atom = RegisterClassW(&WNDCLASSW {
                        lpfnWndProc: Some(procedure),
                        hInstance: GetModuleHandleW(null()),
                        lpszClassName: class.as_ptr(),
                        ..std::mem::zeroed()
                    });
                    if atom == 0 {
                        Err(last_error())
                    } else {
                        Ok(atom)
                    }
                })
                .as_ref()
                .map_err(Clone::clone)?;
            let mut state = Box::new(State {
                parent,
                hwnd: null_mut(),
                shape: None,
                bitmap: None,
                alpha: 0,
                enabled: false,
            });
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                class.as_ptr(),
                class.as_ptr(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                std::ptr::from_mut(state.as_mut()).cast(),
            );
            if hwnd.is_null() {
                return Err(last_error());
            }
            state.hwnd = hwnd;
            if SetWindowSubclass(parent, Some(parent_procedure), SUBCLASS, hwnd as usize) == 0 {
                DestroyWindow(hwnd);
                return Err(last_error());
            }
            Ok(Self { state })
        }
    }

    pub fn update(
        &mut self,
        parameters: &SurfaceEffects,
        active: bool,
        scale: f32,
    ) -> Result<(), String> {
        let mut bounds = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if unsafe { GetClientRect(self.state.parent, &mut bounds) } == 0 {
            return Err(last_error());
        }
        let shape = ShadowShape {
            width: (bounds.right - bounds.left).max(1) as usize,
            height: (bounds.bottom - bounds.top).max(1) as usize,
            padding: (crate::surface::SHADOW_INSET as f32 * scale).round() as usize,
            corner: parameters.corner_radius * scale,
            blur: parameters.shadow_blur * scale,
            offset_y: parameters.shadow_y * scale,
        };
        if self.state.shape != Some(shape) {
            self.state.bitmap = Some(Bitmap::new(shape)?);
            self.state.shape = Some(shape);
        }
        self.state.alpha = (255.0
            * if active {
                parameters.shadow_opacity
            } else {
                parameters.idle_shadow_opacity
            })
        .round() as u8;
        self.schedule();
        Ok(())
    }

    pub fn set_docked(&mut self, docked: bool) {
        self.state.enabled = !docked && self.state.alpha > 0;
        self.schedule();
    }

    fn schedule(&self) {
        unsafe {
            PostMessageW(self.state.hwnd, SYNC, 0, 0);
        }
    }
}

impl Drop for ShadowHost {
    fn drop(&mut self) {
        unsafe {
            RemoveWindowSubclass(self.state.parent, Some(parent_procedure), SUBCLASS);
            DestroyWindow(self.state.hwnd);
        }
    }
}

fn last_error() -> String {
    std::io::Error::last_os_error().to_string()
}
