use std::sync::OnceLock;

use crate::abi::generated_types::{SDL_Surface, SDL_Window, SDL_WindowShapeMode, SDL_bool, Uint32};

struct HostShapeApi {
    create_shaped_window: unsafe extern "C" fn(
        *const libc::c_char,
        libc::c_uint,
        libc::c_uint,
        libc::c_uint,
        libc::c_uint,
        Uint32,
    ) -> *mut SDL_Window,
    get_shaped_window_mode:
        unsafe extern "C" fn(*mut SDL_Window, *mut SDL_WindowShapeMode) -> libc::c_int,
    is_shaped_window: unsafe extern "C" fn(*const SDL_Window) -> SDL_bool,
    set_window_shape: unsafe extern "C" fn(
        *mut SDL_Window,
        *mut SDL_Surface,
        *mut SDL_WindowShapeMode,
    ) -> libc::c_int,
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

fn host_api() -> &'static HostShapeApi {
    static API: OnceLock<HostShapeApi> = OnceLock::new();
    API.get_or_init(|| HostShapeApi {
        create_shaped_window: load_host_symbol(b"SDL_CreateShapedWindow\0"),
        get_shaped_window_mode: load_host_symbol(b"SDL_GetShapedWindowMode\0"),
        is_shaped_window: load_host_symbol(b"SDL_IsShapedWindow\0"),
        set_window_shape: load_host_symbol(b"SDL_SetWindowShape\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateShapedWindow(
    title: *const libc::c_char,
    x: libc::c_uint,
    y: libc::c_uint,
    w: libc::c_uint,
    h: libc::c_uint,
    flags: Uint32,
) -> *mut SDL_Window {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().create_shaped_window)(title, x, y, w, h, flags);
    }
    crate::video::window::create_stub_window_internal(
        title,
        x as libc::c_int,
        y as libc::c_int,
        w as libc::c_int,
        h as libc::c_int,
        flags,
        true,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetShapedWindowMode(
    window: *mut SDL_Window,
    shape_mode: *mut SDL_WindowShapeMode,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_shaped_window_mode)(window, shape_mode);
    }
    crate::video::window::stub_window_get_shape_mode(window, shape_mode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IsShapedWindow(window: *const SDL_Window) -> SDL_bool {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().is_shaped_window)(window);
    }
    crate::video::window::stub_window_is_shaped(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowShape(
    window: *mut SDL_Window,
    shape: *mut SDL_Surface,
    shape_mode: *mut SDL_WindowShapeMode,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().set_window_shape)(window, shape, shape_mode);
    }
    crate::video::window::stub_window_set_shape(window, shape, shape_mode)
}
