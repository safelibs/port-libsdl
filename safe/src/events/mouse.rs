use std::sync::OnceLock;

use crate::abi::generated_types::{
    SDL_Cursor, SDL_Surface, SDL_SystemCursor, SDL_Window, SDL_bool, Uint32, Uint8,
};

struct MouseApi {
    capture_mouse: unsafe extern "C" fn(SDL_bool) -> libc::c_int,
    create_color_cursor:
        unsafe extern "C" fn(*mut SDL_Surface, libc::c_int, libc::c_int) -> *mut SDL_Cursor,
    create_cursor: unsafe extern "C" fn(
        *const Uint8,
        *const Uint8,
        libc::c_int,
        libc::c_int,
        libc::c_int,
        libc::c_int,
    ) -> *mut SDL_Cursor,
    create_system_cursor: unsafe extern "C" fn(SDL_SystemCursor) -> *mut SDL_Cursor,
    free_cursor: unsafe extern "C" fn(*mut SDL_Cursor),
    get_cursor: unsafe extern "C" fn() -> *mut SDL_Cursor,
    get_default_cursor: unsafe extern "C" fn() -> *mut SDL_Cursor,
    get_global_mouse_state: unsafe extern "C" fn(*mut libc::c_int, *mut libc::c_int) -> Uint32,
    get_grabbed_window: unsafe extern "C" fn() -> *mut SDL_Window,
    get_mouse_focus: unsafe extern "C" fn() -> *mut SDL_Window,
    get_mouse_state: unsafe extern "C" fn(*mut libc::c_int, *mut libc::c_int) -> Uint32,
    get_relative_mouse_mode: unsafe extern "C" fn() -> SDL_bool,
    get_relative_mouse_state: unsafe extern "C" fn(*mut libc::c_int, *mut libc::c_int) -> Uint32,
    set_cursor: unsafe extern "C" fn(*mut SDL_Cursor),
    set_relative_mouse_mode: unsafe extern "C" fn(SDL_bool) -> libc::c_int,
    show_cursor: unsafe extern "C" fn(libc::c_int) -> libc::c_int,
    warp_mouse_global: unsafe extern "C" fn(libc::c_int, libc::c_int) -> libc::c_int,
    warp_mouse_in_window: unsafe extern "C" fn(*mut SDL_Window, libc::c_int, libc::c_int),
}

fn api() -> &'static MouseApi {
    static API: OnceLock<MouseApi> = OnceLock::new();
    API.get_or_init(|| MouseApi {
        capture_mouse: crate::video::load_symbol(b"SDL_CaptureMouse\0"),
        create_color_cursor: crate::video::load_symbol(b"SDL_CreateColorCursor\0"),
        create_cursor: crate::video::load_symbol(b"SDL_CreateCursor\0"),
        create_system_cursor: crate::video::load_symbol(b"SDL_CreateSystemCursor\0"),
        free_cursor: crate::video::load_symbol(b"SDL_FreeCursor\0"),
        get_cursor: crate::video::load_symbol(b"SDL_GetCursor\0"),
        get_default_cursor: crate::video::load_symbol(b"SDL_GetDefaultCursor\0"),
        get_global_mouse_state: crate::video::load_symbol(b"SDL_GetGlobalMouseState\0"),
        get_grabbed_window: crate::video::load_symbol(b"SDL_GetGrabbedWindow\0"),
        get_mouse_focus: crate::video::load_symbol(b"SDL_GetMouseFocus\0"),
        get_mouse_state: crate::video::load_symbol(b"SDL_GetMouseState\0"),
        get_relative_mouse_mode: crate::video::load_symbol(b"SDL_GetRelativeMouseMode\0"),
        get_relative_mouse_state: crate::video::load_symbol(b"SDL_GetRelativeMouseState\0"),
        set_cursor: crate::video::load_symbol(b"SDL_SetCursor\0"),
        set_relative_mouse_mode: crate::video::load_symbol(b"SDL_SetRelativeMouseMode\0"),
        show_cursor: crate::video::load_symbol(b"SDL_ShowCursor\0"),
        warp_mouse_global: crate::video::load_symbol(b"SDL_WarpMouseGlobal\0"),
        warp_mouse_in_window: crate::video::load_symbol(b"SDL_WarpMouseInWindow\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CaptureMouse(enabled: SDL_bool) -> libc::c_int {
    crate::video::clear_real_error();
    (api().capture_mouse)(enabled)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateColorCursor(
    surface: *mut SDL_Surface,
    hot_x: libc::c_int,
    hot_y: libc::c_int,
) -> *mut SDL_Cursor {
    crate::video::clear_real_error();
    (api().create_color_cursor)(surface, hot_x, hot_y)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateCursor(
    data: *const Uint8,
    mask: *const Uint8,
    w: libc::c_int,
    h: libc::c_int,
    hot_x: libc::c_int,
    hot_y: libc::c_int,
) -> *mut SDL_Cursor {
    crate::video::clear_real_error();
    (api().create_cursor)(data, mask, w, h, hot_x, hot_y)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateSystemCursor(id: SDL_SystemCursor) -> *mut SDL_Cursor {
    crate::video::clear_real_error();
    (api().create_system_cursor)(id)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreeCursor(cursor: *mut SDL_Cursor) {
    crate::video::clear_real_error();
    (api().free_cursor)(cursor);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetCursor() -> *mut SDL_Cursor {
    crate::video::clear_real_error();
    (api().get_cursor)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDefaultCursor() -> *mut SDL_Cursor {
    crate::video::clear_real_error();
    (api().get_default_cursor)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetGlobalMouseState(
    x: *mut libc::c_int,
    y: *mut libc::c_int,
) -> Uint32 {
    crate::video::clear_real_error();
    (api().get_global_mouse_state)(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetGrabbedWindow() -> *mut SDL_Window {
    crate::video::clear_real_error();
    (api().get_grabbed_window)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetMouseFocus() -> *mut SDL_Window {
    crate::video::clear_real_error();
    (api().get_mouse_focus)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetMouseState(x: *mut libc::c_int, y: *mut libc::c_int) -> Uint32 {
    crate::video::clear_real_error();
    (api().get_mouse_state)(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRelativeMouseMode() -> SDL_bool {
    crate::video::clear_real_error();
    (api().get_relative_mouse_mode)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRelativeMouseState(
    x: *mut libc::c_int,
    y: *mut libc::c_int,
) -> Uint32 {
    crate::video::clear_real_error();
    (api().get_relative_mouse_state)(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetCursor(cursor: *mut SDL_Cursor) {
    crate::video::clear_real_error();
    (api().set_cursor)(cursor);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetRelativeMouseMode(enabled: SDL_bool) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_relative_mouse_mode)(enabled)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ShowCursor(toggle: libc::c_int) -> libc::c_int {
    crate::video::clear_real_error();
    (api().show_cursor)(toggle)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WarpMouseGlobal(x: libc::c_int, y: libc::c_int) -> libc::c_int {
    crate::video::clear_real_error();
    (api().warp_mouse_global)(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WarpMouseInWindow(
    window: *mut SDL_Window,
    x: libc::c_int,
    y: libc::c_int,
) {
    crate::video::clear_real_error();
    (api().warp_mouse_in_window)(window, x, y);
}
