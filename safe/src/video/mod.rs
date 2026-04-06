pub mod blit;
pub mod bmp;
pub mod clipboard;
pub mod display;
pub mod dummy;
pub mod egl;
pub mod messagebox;
pub mod offscreen;
pub mod pixels;
pub mod rect;
pub mod shape;
pub mod surface;
pub mod syswm;
pub mod vulkan;
pub mod window;

pub mod linux {
    pub mod ime;
    pub mod kmsdrm;
    pub mod wayland;
    pub mod x11;
}

pub(crate) fn real_sdl_handle() -> *mut libc::c_void {
    panic!("host SDL2 compatibility runtime is unavailable outside perf validation")
}

pub(crate) fn open_real_sdl_with_flags(_flags: libc::c_int) -> *mut libc::c_void {
    real_sdl_handle()
}

pub(crate) fn try_real_sdl_handle() -> Option<*mut libc::c_void> {
    None
}

pub(crate) fn real_sdl_is_available() -> bool {
    false
}

pub(crate) fn real_sdl_is_loaded() -> bool {
    false
}

pub(crate) fn load_symbol<T>(name: &[u8]) -> T {
    let symbol = unsafe { libc::dlsym(real_sdl_handle(), name.as_ptr().cast()) };
    assert!(
        !symbol.is_null(),
        "missing host SDL2 symbol {}",
        String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
    );
    unsafe { std::mem::transmute_copy(&symbol) }
}

pub(crate) fn clear_real_error() {}

pub(crate) fn real_error_ptr() -> *const libc::c_char {
    std::ptr::null()
}
