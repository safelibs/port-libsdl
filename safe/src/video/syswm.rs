use crate::abi::generated_types::{SDL_Window, SDL_bool, SDL_version, Uint8};

pub type SDL_SYSWM_TYPE = u32;

pub const SDL_SYSWM_UNKNOWN: SDL_SYSWM_TYPE = 0;
pub const SDL_SYSWM_WINDOWS: SDL_SYSWM_TYPE = 1;
pub const SDL_SYSWM_X11: SDL_SYSWM_TYPE = 2;
pub const SDL_SYSWM_DIRECTFB: SDL_SYSWM_TYPE = 3;
pub const SDL_SYSWM_COCOA: SDL_SYSWM_TYPE = 4;
pub const SDL_SYSWM_UIKIT: SDL_SYSWM_TYPE = 5;
pub const SDL_SYSWM_WAYLAND: SDL_SYSWM_TYPE = 6;
pub const SDL_SYSWM_MIR: SDL_SYSWM_TYPE = 7;
pub const SDL_SYSWM_WINRT: SDL_SYSWM_TYPE = 8;
pub const SDL_SYSWM_ANDROID: SDL_SYSWM_TYPE = 9;
pub const SDL_SYSWM_VIVANTE: SDL_SYSWM_TYPE = 10;
pub const SDL_SYSWM_OS2: SDL_SYSWM_TYPE = 11;
pub const SDL_SYSWM_HAIKU: SDL_SYSWM_TYPE = 12;
pub const SDL_SYSWM_KMSDRM: SDL_SYSWM_TYPE = 13;
pub const SDL_SYSWM_RISCOS: SDL_SYSWM_TYPE = 14;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDL_SysWMinfo_Windows {
    pub window: *mut libc::c_void,
    pub hdc: *mut libc::c_void,
    pub hinstance: *mut libc::c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDL_SysWMinfo_X11 {
    pub display: *mut libc::c_void,
    pub window: libc::c_ulong,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDL_SysWMinfo_Cocoa {
    pub window: *mut libc::c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDL_SysWMinfo_Wayland {
    pub display: *mut libc::c_void,
    pub surface: *mut libc::c_void,
    pub shell_surface: *mut libc::c_void,
    pub egl_window: *mut libc::c_void,
    pub xdg_surface: *mut libc::c_void,
    pub xdg_toplevel: *mut libc::c_void,
    pub xdg_popup: *mut libc::c_void,
    pub xdg_positioner: *mut libc::c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDL_SysWMinfo_OS2 {
    pub hwnd: *mut libc::c_void,
    pub hwndFrame: *mut libc::c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDL_SysWMinfo_KMSDRM {
    pub dev_index: libc::c_int,
    pub drm_fd: libc::c_int,
    pub gbm_dev: *mut libc::c_void,
}

#[repr(C)]
pub union SDL_SysWMinfoUnion {
    pub win: SDL_SysWMinfo_Windows,
    pub x11: SDL_SysWMinfo_X11,
    pub cocoa: SDL_SysWMinfo_Cocoa,
    pub wl: SDL_SysWMinfo_Wayland,
    pub os2: SDL_SysWMinfo_OS2,
    pub kmsdrm: SDL_SysWMinfo_KMSDRM,
    pub dummy: [Uint8; 64],
}

#[repr(C)]
pub struct SDL_SysWMinfo {
    pub version: SDL_version,
    pub subsystem: SDL_SYSWM_TYPE,
    pub info: SDL_SysWMinfoUnion,
}

struct HostSysWmApi {
    get_window_wm_info: unsafe extern "C" fn(*mut SDL_Window, *mut SDL_SysWMinfo) -> SDL_bool,
}

fn load_host_symbol<T>(name: &[u8]) -> T {
    let symbol = unsafe { libc::dlsym(crate::video::real_sdl_handle(), name.as_ptr().cast()) };
    assert!(
        !symbol.is_null(),
        "missing host SDL2 symbol {}",
        String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
    );
    unsafe { std::mem::transmute_copy(&symbol) }
}

fn host_api() -> &'static HostSysWmApi {
    static API: std::sync::OnceLock<HostSysWmApi> = std::sync::OnceLock::new();
    API.get_or_init(|| HostSysWmApi {
        get_window_wm_info: load_host_symbol(b"SDL_GetWindowWMInfo\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowWMInfo(
    window: *mut SDL_Window,
    info: *mut SDL_SysWMinfo,
) -> SDL_bool {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_wm_info)(window, info);
    }

    if !info.is_null() {
        (*info).subsystem = SDL_SYSWM_UNKNOWN;
        (*info).info.dummy = [0; 64];
    }
    0
}
