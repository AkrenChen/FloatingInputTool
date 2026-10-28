use std::{ffi::c_void, mem};
use windows_sys::Win32::{
    Foundation::HWND,
    System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
};

#[repr(C)]
struct AccentPolicy {
    state: u32,
    flags: u32,
    tint: u32,
    animation: u32,
}

#[repr(C)]
struct CompositionAttribute {
    attribute: u32,
    data: *mut c_void,
    size: usize,
}

pub fn set_enabled(hwnd: HWND, enabled: bool) -> Result<(), String> {
    type SetAttribute = unsafe extern "system" fn(HWND, *mut CompositionAttribute) -> i32;
    let library_name: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    unsafe {
        let library = GetModuleHandleW(library_name.as_ptr());
        if library.is_null() {
            return Err("user32 is unavailable".into());
        }
        let address = GetProcAddress(library, b"SetWindowCompositionAttribute\0".as_ptr())
            .ok_or("window composition attributes are unavailable")?;
        let set_attribute: SetAttribute = mem::transmute(address);
        // Accent acrylic needs nonzero tint alpha; the visible tint lives in theme.css.
        let mut policy = AccentPolicy {
            state: if enabled { 4 } else { 0 },
            flags: 0,
            tint: 0x01000000,
            animation: 0,
        };
        let mut attribute = CompositionAttribute {
            attribute: 19,
            data: std::ptr::from_mut(&mut policy).cast(),
            size: mem::size_of::<AccentPolicy>(),
        };
        if set_attribute(hwnd, &mut attribute) == 0 {
            return Err(format!(
                "acrylic composition failed: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    Ok(())
}
