use std::sync::OnceLock;

use crate::abi::generated_types::{SDL_Surface, SDL_Window, SDL_WindowShapeMode, SDL_bool, Uint32};

struct ShapeApi {
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

fn api() -> &'static ShapeApi {
    static API: OnceLock<ShapeApi> = OnceLock::new();
    API.get_or_init(|| ShapeApi {
        create_shaped_window: crate::video::load_symbol(b"SDL_CreateShapedWindow\0"),
        get_shaped_window_mode: crate::video::load_symbol(b"SDL_GetShapedWindowMode\0"),
        is_shaped_window: crate::video::load_symbol(b"SDL_IsShapedWindow\0"),
        set_window_shape: crate::video::load_symbol(b"SDL_SetWindowShape\0"),
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
    crate::video::clear_real_error();
    (api().create_shaped_window)(title, x, y, w, h, flags)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetShapedWindowMode(
    window: *mut SDL_Window,
    shape_mode: *mut SDL_WindowShapeMode,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_shaped_window_mode)(window, shape_mode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IsShapedWindow(window: *const SDL_Window) -> SDL_bool {
    crate::video::clear_real_error();
    (api().is_shaped_window)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowShape(
    window: *mut SDL_Window,
    shape: *mut SDL_Surface,
    shape_mode: *mut SDL_WindowShapeMode,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_window_shape)(window, shape, shape_mode)
}
