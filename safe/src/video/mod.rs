use std::sync::OnceLock;

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

fn open_real_sdl() -> *mut libc::c_void {
    const CANDIDATES: [&[u8]; 3] = [
        b"/lib/x86_64-linux-gnu/libSDL2-2.0.so.0\0",
        b"/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0\0",
        b"libSDL2-2.0.so.0\0",
    ];

    for candidate in CANDIDATES {
        let handle =
            unsafe { libc::dlopen(candidate.as_ptr().cast(), libc::RTLD_LOCAL | libc::RTLD_NOW) };
        if !handle.is_null() {
            return handle;
        }
    }

    panic!("unable to load the host SDL2 runtime");
}

pub(crate) fn real_sdl_handle() -> *mut libc::c_void {
    static HANDLE: OnceLock<usize> = OnceLock::new();
    *HANDLE.get_or_init(|| open_real_sdl() as usize) as *mut libc::c_void
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

type GetErrorFn = unsafe extern "C" fn() -> *const libc::c_char;
type ClearErrorFn = unsafe extern "C" fn();

fn real_get_error() -> GetErrorFn {
    static FN: OnceLock<GetErrorFn> = OnceLock::new();
    *FN.get_or_init(|| load_symbol(b"SDL_GetError\0"))
}

fn real_clear_error_fn() -> ClearErrorFn {
    static FN: OnceLock<ClearErrorFn> = OnceLock::new();
    *FN.get_or_init(|| load_symbol(b"SDL_ClearError\0"))
}

pub(crate) fn clear_real_error() {
    unsafe {
        real_clear_error_fn()();
    }
}

pub(crate) fn real_error_ptr() -> *const libc::c_char {
    unsafe { real_get_error()() }
}
