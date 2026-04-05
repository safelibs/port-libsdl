use std::sync::OnceLock;

use crate::abi::generated_types::{
    SDL_DisplayMode, SDL_FlashOperation, SDL_HitTest, SDL_Rect, SDL_Renderer, SDL_Surface,
    SDL_Window, SDL_bool, Uint16, Uint32,
};

struct WindowApi {
    create_window: unsafe extern "C" fn(
        *const libc::c_char,
        libc::c_int,
        libc::c_int,
        libc::c_int,
        libc::c_int,
        Uint32,
    ) -> *mut SDL_Window,
    create_window_and_renderer: unsafe extern "C" fn(
        libc::c_int,
        libc::c_int,
        Uint32,
        *mut *mut SDL_Window,
        *mut *mut SDL_Renderer,
    ) -> libc::c_int,
    create_window_from: unsafe extern "C" fn(*const libc::c_void) -> *mut SDL_Window,
    destroy_window: unsafe extern "C" fn(*mut SDL_Window),
    destroy_window_surface: unsafe extern "C" fn(*mut SDL_Window) -> libc::c_int,
    flash_window: unsafe extern "C" fn(*mut SDL_Window, SDL_FlashOperation) -> libc::c_int,
    get_window_borders_size: unsafe extern "C" fn(
        *mut SDL_Window,
        *mut libc::c_int,
        *mut libc::c_int,
        *mut libc::c_int,
        *mut libc::c_int,
    ) -> libc::c_int,
    get_window_brightness: unsafe extern "C" fn(*mut SDL_Window) -> f32,
    get_window_data:
        unsafe extern "C" fn(*mut SDL_Window, *const libc::c_char) -> *mut libc::c_void,
    get_window_display_index: unsafe extern "C" fn(*mut SDL_Window) -> libc::c_int,
    get_window_display_mode:
        unsafe extern "C" fn(*mut SDL_Window, *mut SDL_DisplayMode) -> libc::c_int,
    get_window_flags: unsafe extern "C" fn(*mut SDL_Window) -> Uint32,
    get_window_from_id: unsafe extern "C" fn(Uint32) -> *mut SDL_Window,
    get_window_gamma_ramp:
        unsafe extern "C" fn(*mut SDL_Window, *mut Uint16, *mut Uint16, *mut Uint16) -> libc::c_int,
    get_window_grab: unsafe extern "C" fn(*mut SDL_Window) -> SDL_bool,
    get_window_id: unsafe extern "C" fn(*mut SDL_Window) -> Uint32,
    get_window_keyboard_grab: unsafe extern "C" fn(*mut SDL_Window) -> SDL_bool,
    get_window_maximum_size:
        unsafe extern "C" fn(*mut SDL_Window, *mut libc::c_int, *mut libc::c_int),
    get_window_minimum_size:
        unsafe extern "C" fn(*mut SDL_Window, *mut libc::c_int, *mut libc::c_int),
    get_window_mouse_grab: unsafe extern "C" fn(*mut SDL_Window) -> SDL_bool,
    get_window_mouse_rect: unsafe extern "C" fn(*mut SDL_Window) -> *const SDL_Rect,
    get_window_opacity: unsafe extern "C" fn(*mut SDL_Window, *mut f32) -> libc::c_int,
    get_window_pixel_format: unsafe extern "C" fn(*mut SDL_Window) -> Uint32,
    get_window_position: unsafe extern "C" fn(*mut SDL_Window, *mut libc::c_int, *mut libc::c_int),
    get_window_size: unsafe extern "C" fn(*mut SDL_Window, *mut libc::c_int, *mut libc::c_int),
    get_window_size_in_pixels:
        unsafe extern "C" fn(*mut SDL_Window, *mut libc::c_int, *mut libc::c_int),
    get_window_surface: unsafe extern "C" fn(*mut SDL_Window) -> *mut SDL_Surface,
    get_window_title: unsafe extern "C" fn(*mut SDL_Window) -> *const libc::c_char,
    hide_window: unsafe extern "C" fn(*mut SDL_Window),
    maximize_window: unsafe extern "C" fn(*mut SDL_Window),
    minimize_window: unsafe extern "C" fn(*mut SDL_Window),
    raise_window: unsafe extern "C" fn(*mut SDL_Window),
    restore_window: unsafe extern "C" fn(*mut SDL_Window),
    set_window_always_on_top: unsafe extern "C" fn(*mut SDL_Window, SDL_bool),
    set_window_bordered: unsafe extern "C" fn(*mut SDL_Window, SDL_bool),
    set_window_brightness: unsafe extern "C" fn(*mut SDL_Window, f32) -> libc::c_int,
    set_window_data: unsafe extern "C" fn(
        *mut SDL_Window,
        *const libc::c_char,
        *mut libc::c_void,
    ) -> *mut libc::c_void,
    set_window_display_mode:
        unsafe extern "C" fn(*mut SDL_Window, *const SDL_DisplayMode) -> libc::c_int,
    set_window_fullscreen: unsafe extern "C" fn(*mut SDL_Window, Uint32) -> libc::c_int,
    set_window_grab: unsafe extern "C" fn(*mut SDL_Window, SDL_bool),
    set_window_hit_test:
        unsafe extern "C" fn(*mut SDL_Window, SDL_HitTest, *mut libc::c_void) -> libc::c_int,
    set_window_icon: unsafe extern "C" fn(*mut SDL_Window, *mut SDL_Surface),
    set_window_input_focus: unsafe extern "C" fn(*mut SDL_Window) -> libc::c_int,
    set_window_keyboard_grab: unsafe extern "C" fn(*mut SDL_Window, SDL_bool),
    set_window_maximum_size: unsafe extern "C" fn(*mut SDL_Window, libc::c_int, libc::c_int),
    set_window_minimum_size: unsafe extern "C" fn(*mut SDL_Window, libc::c_int, libc::c_int),
    set_window_modal_for: unsafe extern "C" fn(*mut SDL_Window, *mut SDL_Window) -> libc::c_int,
    set_window_mouse_grab: unsafe extern "C" fn(*mut SDL_Window, SDL_bool),
    set_window_mouse_rect: unsafe extern "C" fn(*mut SDL_Window, *const SDL_Rect) -> libc::c_int,
    set_window_opacity: unsafe extern "C" fn(*mut SDL_Window, f32) -> libc::c_int,
    set_window_position: unsafe extern "C" fn(*mut SDL_Window, libc::c_int, libc::c_int),
    set_window_resizable: unsafe extern "C" fn(*mut SDL_Window, SDL_bool),
    set_window_size: unsafe extern "C" fn(*mut SDL_Window, libc::c_int, libc::c_int),
    set_window_title: unsafe extern "C" fn(*mut SDL_Window, *const libc::c_char),
    show_window: unsafe extern "C" fn(*mut SDL_Window),
    update_window_surface: unsafe extern "C" fn(*mut SDL_Window) -> libc::c_int,
    update_window_surface_rects:
        unsafe extern "C" fn(*mut SDL_Window, *const SDL_Rect, libc::c_int) -> libc::c_int,
}

fn api() -> &'static WindowApi {
    static API: OnceLock<WindowApi> = OnceLock::new();
    API.get_or_init(|| WindowApi {
        create_window: crate::video::load_symbol(b"SDL_CreateWindow\0"),
        create_window_and_renderer: crate::video::load_symbol(b"SDL_CreateWindowAndRenderer\0"),
        create_window_from: crate::video::load_symbol(b"SDL_CreateWindowFrom\0"),
        destroy_window: crate::video::load_symbol(b"SDL_DestroyWindow\0"),
        destroy_window_surface: crate::video::load_symbol(b"SDL_DestroyWindowSurface\0"),
        flash_window: crate::video::load_symbol(b"SDL_FlashWindow\0"),
        get_window_borders_size: crate::video::load_symbol(b"SDL_GetWindowBordersSize\0"),
        get_window_brightness: crate::video::load_symbol(b"SDL_GetWindowBrightness\0"),
        get_window_data: crate::video::load_symbol(b"SDL_GetWindowData\0"),
        get_window_display_index: crate::video::load_symbol(b"SDL_GetWindowDisplayIndex\0"),
        get_window_display_mode: crate::video::load_symbol(b"SDL_GetWindowDisplayMode\0"),
        get_window_flags: crate::video::load_symbol(b"SDL_GetWindowFlags\0"),
        get_window_from_id: crate::video::load_symbol(b"SDL_GetWindowFromID\0"),
        get_window_gamma_ramp: crate::video::load_symbol(b"SDL_GetWindowGammaRamp\0"),
        get_window_grab: crate::video::load_symbol(b"SDL_GetWindowGrab\0"),
        get_window_id: crate::video::load_symbol(b"SDL_GetWindowID\0"),
        get_window_keyboard_grab: crate::video::load_symbol(b"SDL_GetWindowKeyboardGrab\0"),
        get_window_maximum_size: crate::video::load_symbol(b"SDL_GetWindowMaximumSize\0"),
        get_window_minimum_size: crate::video::load_symbol(b"SDL_GetWindowMinimumSize\0"),
        get_window_mouse_grab: crate::video::load_symbol(b"SDL_GetWindowMouseGrab\0"),
        get_window_mouse_rect: crate::video::load_symbol(b"SDL_GetWindowMouseRect\0"),
        get_window_opacity: crate::video::load_symbol(b"SDL_GetWindowOpacity\0"),
        get_window_pixel_format: crate::video::load_symbol(b"SDL_GetWindowPixelFormat\0"),
        get_window_position: crate::video::load_symbol(b"SDL_GetWindowPosition\0"),
        get_window_size: crate::video::load_symbol(b"SDL_GetWindowSize\0"),
        get_window_size_in_pixels: crate::video::load_symbol(b"SDL_GetWindowSizeInPixels\0"),
        get_window_surface: crate::video::load_symbol(b"SDL_GetWindowSurface\0"),
        get_window_title: crate::video::load_symbol(b"SDL_GetWindowTitle\0"),
        hide_window: crate::video::load_symbol(b"SDL_HideWindow\0"),
        maximize_window: crate::video::load_symbol(b"SDL_MaximizeWindow\0"),
        minimize_window: crate::video::load_symbol(b"SDL_MinimizeWindow\0"),
        raise_window: crate::video::load_symbol(b"SDL_RaiseWindow\0"),
        restore_window: crate::video::load_symbol(b"SDL_RestoreWindow\0"),
        set_window_always_on_top: crate::video::load_symbol(b"SDL_SetWindowAlwaysOnTop\0"),
        set_window_bordered: crate::video::load_symbol(b"SDL_SetWindowBordered\0"),
        set_window_brightness: crate::video::load_symbol(b"SDL_SetWindowBrightness\0"),
        set_window_data: crate::video::load_symbol(b"SDL_SetWindowData\0"),
        set_window_display_mode: crate::video::load_symbol(b"SDL_SetWindowDisplayMode\0"),
        set_window_fullscreen: crate::video::load_symbol(b"SDL_SetWindowFullscreen\0"),
        set_window_grab: crate::video::load_symbol(b"SDL_SetWindowGrab\0"),
        set_window_hit_test: crate::video::load_symbol(b"SDL_SetWindowHitTest\0"),
        set_window_icon: crate::video::load_symbol(b"SDL_SetWindowIcon\0"),
        set_window_input_focus: crate::video::load_symbol(b"SDL_SetWindowInputFocus\0"),
        set_window_keyboard_grab: crate::video::load_symbol(b"SDL_SetWindowKeyboardGrab\0"),
        set_window_maximum_size: crate::video::load_symbol(b"SDL_SetWindowMaximumSize\0"),
        set_window_minimum_size: crate::video::load_symbol(b"SDL_SetWindowMinimumSize\0"),
        set_window_modal_for: crate::video::load_symbol(b"SDL_SetWindowModalFor\0"),
        set_window_mouse_grab: crate::video::load_symbol(b"SDL_SetWindowMouseGrab\0"),
        set_window_mouse_rect: crate::video::load_symbol(b"SDL_SetWindowMouseRect\0"),
        set_window_opacity: crate::video::load_symbol(b"SDL_SetWindowOpacity\0"),
        set_window_position: crate::video::load_symbol(b"SDL_SetWindowPosition\0"),
        set_window_resizable: crate::video::load_symbol(b"SDL_SetWindowResizable\0"),
        set_window_size: crate::video::load_symbol(b"SDL_SetWindowSize\0"),
        set_window_title: crate::video::load_symbol(b"SDL_SetWindowTitle\0"),
        show_window: crate::video::load_symbol(b"SDL_ShowWindow\0"),
        update_window_surface: crate::video::load_symbol(b"SDL_UpdateWindowSurface\0"),
        update_window_surface_rects: crate::video::load_symbol(b"SDL_UpdateWindowSurfaceRects\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateWindow(
    title: *const libc::c_char,
    x: libc::c_int,
    y: libc::c_int,
    w: libc::c_int,
    h: libc::c_int,
    flags: Uint32,
) -> *mut SDL_Window {
    crate::video::clear_real_error();
    (api().create_window)(title, x, y, w, h, flags)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateWindowAndRenderer(
    width: libc::c_int,
    height: libc::c_int,
    window_flags: Uint32,
    window: *mut *mut SDL_Window,
    renderer: *mut *mut SDL_Renderer,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().create_window_and_renderer)(width, height, window_flags, window, renderer)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateWindowFrom(data: *const libc::c_void) -> *mut SDL_Window {
    crate::video::clear_real_error();
    (api().create_window_from)(data)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DestroyWindow(window: *mut SDL_Window) {
    crate::video::clear_real_error();
    (api().destroy_window)(window);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DestroyWindowSurface(window: *mut SDL_Window) -> libc::c_int {
    crate::video::clear_real_error();
    (api().destroy_window_surface)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FlashWindow(
    window: *mut SDL_Window,
    operation: SDL_FlashOperation,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().flash_window)(window, operation)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowBordersSize(
    window: *mut SDL_Window,
    top: *mut libc::c_int,
    left: *mut libc::c_int,
    bottom: *mut libc::c_int,
    right: *mut libc::c_int,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_window_borders_size)(window, top, left, bottom, right)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowBrightness(window: *mut SDL_Window) -> f32 {
    crate::video::clear_real_error();
    (api().get_window_brightness)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowData(
    window: *mut SDL_Window,
    name: *const libc::c_char,
) -> *mut libc::c_void {
    crate::video::clear_real_error();
    (api().get_window_data)(window, name)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowDisplayIndex(window: *mut SDL_Window) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_window_display_index)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowDisplayMode(
    window: *mut SDL_Window,
    mode: *mut SDL_DisplayMode,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_window_display_mode)(window, mode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowFlags(window: *mut SDL_Window) -> Uint32 {
    crate::video::clear_real_error();
    (api().get_window_flags)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowFromID(id: Uint32) -> *mut SDL_Window {
    crate::video::clear_real_error();
    (api().get_window_from_id)(id)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowGammaRamp(
    window: *mut SDL_Window,
    red: *mut Uint16,
    green: *mut Uint16,
    blue: *mut Uint16,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_window_gamma_ramp)(window, red, green, blue)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowGrab(window: *mut SDL_Window) -> SDL_bool {
    crate::video::clear_real_error();
    (api().get_window_grab)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowID(window: *mut SDL_Window) -> Uint32 {
    crate::video::clear_real_error();
    (api().get_window_id)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowKeyboardGrab(window: *mut SDL_Window) -> SDL_bool {
    crate::video::clear_real_error();
    (api().get_window_keyboard_grab)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowMaximumSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    crate::video::clear_real_error();
    (api().get_window_maximum_size)(window, w, h);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowMinimumSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    crate::video::clear_real_error();
    (api().get_window_minimum_size)(window, w, h);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowMouseGrab(window: *mut SDL_Window) -> SDL_bool {
    crate::video::clear_real_error();
    (api().get_window_mouse_grab)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowMouseRect(window: *mut SDL_Window) -> *const SDL_Rect {
    crate::video::clear_real_error();
    (api().get_window_mouse_rect)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowOpacity(
    window: *mut SDL_Window,
    out_opacity: *mut f32,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().get_window_opacity)(window, out_opacity)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowPixelFormat(window: *mut SDL_Window) -> Uint32 {
    crate::video::clear_real_error();
    (api().get_window_pixel_format)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowPosition(
    window: *mut SDL_Window,
    x: *mut libc::c_int,
    y: *mut libc::c_int,
) {
    crate::video::clear_real_error();
    (api().get_window_position)(window, x, y);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    crate::video::clear_real_error();
    (api().get_window_size)(window, w, h);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowSizeInPixels(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    crate::video::clear_real_error();
    (api().get_window_size_in_pixels)(window, w, h);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowSurface(window: *mut SDL_Window) -> *mut SDL_Surface {
    crate::video::clear_real_error();
    (api().get_window_surface)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowTitle(window: *mut SDL_Window) -> *const libc::c_char {
    crate::video::clear_real_error();
    (api().get_window_title)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HideWindow(window: *mut SDL_Window) {
    crate::video::clear_real_error();
    (api().hide_window)(window);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MaximizeWindow(window: *mut SDL_Window) {
    crate::video::clear_real_error();
    (api().maximize_window)(window);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MinimizeWindow(window: *mut SDL_Window) {
    crate::video::clear_real_error();
    (api().minimize_window)(window);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RaiseWindow(window: *mut SDL_Window) {
    crate::video::clear_real_error();
    (api().raise_window)(window);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RestoreWindow(window: *mut SDL_Window) {
    crate::video::clear_real_error();
    (api().restore_window)(window);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowAlwaysOnTop(window: *mut SDL_Window, on_top: SDL_bool) {
    crate::video::clear_real_error();
    (api().set_window_always_on_top)(window, on_top);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowBordered(window: *mut SDL_Window, bordered: SDL_bool) {
    crate::video::clear_real_error();
    (api().set_window_bordered)(window, bordered);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowBrightness(
    window: *mut SDL_Window,
    brightness: f32,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_window_brightness)(window, brightness)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowData(
    window: *mut SDL_Window,
    name: *const libc::c_char,
    userdata: *mut libc::c_void,
) -> *mut libc::c_void {
    crate::video::clear_real_error();
    (api().set_window_data)(window, name, userdata)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowDisplayMode(
    window: *mut SDL_Window,
    mode: *const SDL_DisplayMode,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_window_display_mode)(window, mode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowFullscreen(
    window: *mut SDL_Window,
    flags: Uint32,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_window_fullscreen)(window, flags)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowGrab(window: *mut SDL_Window, grabbed: SDL_bool) {
    crate::video::clear_real_error();
    (api().set_window_grab)(window, grabbed);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowHitTest(
    window: *mut SDL_Window,
    callback: SDL_HitTest,
    callback_data: *mut libc::c_void,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_window_hit_test)(window, callback, callback_data)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowIcon(window: *mut SDL_Window, icon: *mut SDL_Surface) {
    crate::video::clear_real_error();
    (api().set_window_icon)(window, icon);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowInputFocus(window: *mut SDL_Window) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_window_input_focus)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowKeyboardGrab(window: *mut SDL_Window, grabbed: SDL_bool) {
    crate::video::clear_real_error();
    (api().set_window_keyboard_grab)(window, grabbed);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowMaximumSize(
    window: *mut SDL_Window,
    max_w: libc::c_int,
    max_h: libc::c_int,
) {
    crate::video::clear_real_error();
    (api().set_window_maximum_size)(window, max_w, max_h);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowMinimumSize(
    window: *mut SDL_Window,
    min_w: libc::c_int,
    min_h: libc::c_int,
) {
    crate::video::clear_real_error();
    (api().set_window_minimum_size)(window, min_w, min_h);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowModalFor(
    modal_window: *mut SDL_Window,
    parent_window: *mut SDL_Window,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_window_modal_for)(modal_window, parent_window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowMouseGrab(window: *mut SDL_Window, grabbed: SDL_bool) {
    crate::video::clear_real_error();
    (api().set_window_mouse_grab)(window, grabbed);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowMouseRect(
    window: *mut SDL_Window,
    rect: *const SDL_Rect,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_window_mouse_rect)(window, rect)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowOpacity(
    window: *mut SDL_Window,
    opacity: f32,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_window_opacity)(window, opacity)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowPosition(
    window: *mut SDL_Window,
    x: libc::c_int,
    y: libc::c_int,
) {
    crate::video::clear_real_error();
    (api().set_window_position)(window, x, y);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowResizable(window: *mut SDL_Window, resizable: SDL_bool) {
    crate::video::clear_real_error();
    (api().set_window_resizable)(window, resizable);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowSize(
    window: *mut SDL_Window,
    w: libc::c_int,
    h: libc::c_int,
) {
    crate::video::clear_real_error();
    (api().set_window_size)(window, w, h);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowTitle(window: *mut SDL_Window, title: *const libc::c_char) {
    crate::video::clear_real_error();
    (api().set_window_title)(window, title);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ShowWindow(window: *mut SDL_Window) {
    crate::video::clear_real_error();
    (api().show_window)(window);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UpdateWindowSurface(window: *mut SDL_Window) -> libc::c_int {
    crate::video::clear_real_error();
    (api().update_window_surface)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UpdateWindowSurfaceRects(
    window: *mut SDL_Window,
    rects: *const SDL_Rect,
    numrects: libc::c_int,
) -> libc::c_int {
    crate::video::clear_real_error();
    (api().update_window_surface_rects)(window, rects, numrects)
}
