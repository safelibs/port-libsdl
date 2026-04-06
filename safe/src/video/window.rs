use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_DisplayMode, SDL_Event, SDL_EventType_SDL_WINDOWEVENT, SDL_FlashOperation, SDL_HitTest,
    SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888, SDL_Rect, SDL_Renderer, SDL_Surface, SDL_Window,
    SDL_WindowEvent, SDL_WindowEventID_SDL_WINDOWEVENT_FOCUS_GAINED,
    SDL_WindowFlags_SDL_WINDOW_ALWAYS_ON_TOP, SDL_WindowFlags_SDL_WINDOW_BORDERLESS,
    SDL_WindowFlags_SDL_WINDOW_FULLSCREEN, SDL_WindowFlags_SDL_WINDOW_FULLSCREEN_DESKTOP,
    SDL_WindowFlags_SDL_WINDOW_HIDDEN, SDL_WindowFlags_SDL_WINDOW_INPUT_FOCUS,
    SDL_WindowFlags_SDL_WINDOW_KEYBOARD_GRABBED, SDL_WindowFlags_SDL_WINDOW_MAXIMIZED,
    SDL_WindowFlags_SDL_WINDOW_MINIMIZED, SDL_WindowFlags_SDL_WINDOW_MOUSE_FOCUS,
    SDL_WindowFlags_SDL_WINDOW_MOUSE_GRABBED, SDL_WindowFlags_SDL_WINDOW_RESIZABLE,
    SDL_WindowFlags_SDL_WINDOW_SHOWN, SDL_WindowShapeMode, SDL_bool, SDL_bool_SDL_FALSE, Uint16,
    Uint32, SDL_HINT_GRAB_KEYBOARD, SDL_WINDOWPOS_CENTERED_MASK, SDL_WINDOWPOS_UNDEFINED_MASK,
};

struct HostWindowApi {
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

fn load_host_symbol<T>(name: &[u8]) -> T {
    let symbol = unsafe { libc::dlsym(crate::video::real_sdl_handle(), name.as_ptr().cast()) };
    assert!(
        !symbol.is_null(),
        "missing host SDL2 symbol {}",
        String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
    );
    unsafe { std::mem::transmute_copy(&symbol) }
}

fn host_api() -> &'static HostWindowApi {
    static API: OnceLock<HostWindowApi> = OnceLock::new();
    API.get_or_init(|| HostWindowApi {
        create_window: load_host_symbol(b"SDL_CreateWindow\0"),
        create_window_and_renderer: load_host_symbol(b"SDL_CreateWindowAndRenderer\0"),
        create_window_from: load_host_symbol(b"SDL_CreateWindowFrom\0"),
        destroy_window: load_host_symbol(b"SDL_DestroyWindow\0"),
        destroy_window_surface: load_host_symbol(b"SDL_DestroyWindowSurface\0"),
        flash_window: load_host_symbol(b"SDL_FlashWindow\0"),
        get_window_borders_size: load_host_symbol(b"SDL_GetWindowBordersSize\0"),
        get_window_brightness: load_host_symbol(b"SDL_GetWindowBrightness\0"),
        get_window_data: load_host_symbol(b"SDL_GetWindowData\0"),
        get_window_display_index: load_host_symbol(b"SDL_GetWindowDisplayIndex\0"),
        get_window_display_mode: load_host_symbol(b"SDL_GetWindowDisplayMode\0"),
        get_window_flags: load_host_symbol(b"SDL_GetWindowFlags\0"),
        get_window_from_id: load_host_symbol(b"SDL_GetWindowFromID\0"),
        get_window_gamma_ramp: load_host_symbol(b"SDL_GetWindowGammaRamp\0"),
        get_window_grab: load_host_symbol(b"SDL_GetWindowGrab\0"),
        get_window_id: load_host_symbol(b"SDL_GetWindowID\0"),
        get_window_keyboard_grab: load_host_symbol(b"SDL_GetWindowKeyboardGrab\0"),
        get_window_maximum_size: load_host_symbol(b"SDL_GetWindowMaximumSize\0"),
        get_window_minimum_size: load_host_symbol(b"SDL_GetWindowMinimumSize\0"),
        get_window_mouse_grab: load_host_symbol(b"SDL_GetWindowMouseGrab\0"),
        get_window_mouse_rect: load_host_symbol(b"SDL_GetWindowMouseRect\0"),
        get_window_opacity: load_host_symbol(b"SDL_GetWindowOpacity\0"),
        get_window_pixel_format: load_host_symbol(b"SDL_GetWindowPixelFormat\0"),
        get_window_position: load_host_symbol(b"SDL_GetWindowPosition\0"),
        get_window_size: load_host_symbol(b"SDL_GetWindowSize\0"),
        get_window_size_in_pixels: load_host_symbol(b"SDL_GetWindowSizeInPixels\0"),
        get_window_surface: load_host_symbol(b"SDL_GetWindowSurface\0"),
        get_window_title: load_host_symbol(b"SDL_GetWindowTitle\0"),
        hide_window: load_host_symbol(b"SDL_HideWindow\0"),
        maximize_window: load_host_symbol(b"SDL_MaximizeWindow\0"),
        minimize_window: load_host_symbol(b"SDL_MinimizeWindow\0"),
        raise_window: load_host_symbol(b"SDL_RaiseWindow\0"),
        restore_window: load_host_symbol(b"SDL_RestoreWindow\0"),
        set_window_always_on_top: load_host_symbol(b"SDL_SetWindowAlwaysOnTop\0"),
        set_window_bordered: load_host_symbol(b"SDL_SetWindowBordered\0"),
        set_window_brightness: load_host_symbol(b"SDL_SetWindowBrightness\0"),
        set_window_data: load_host_symbol(b"SDL_SetWindowData\0"),
        set_window_display_mode: load_host_symbol(b"SDL_SetWindowDisplayMode\0"),
        set_window_fullscreen: load_host_symbol(b"SDL_SetWindowFullscreen\0"),
        set_window_grab: load_host_symbol(b"SDL_SetWindowGrab\0"),
        set_window_hit_test: load_host_symbol(b"SDL_SetWindowHitTest\0"),
        set_window_icon: load_host_symbol(b"SDL_SetWindowIcon\0"),
        set_window_input_focus: load_host_symbol(b"SDL_SetWindowInputFocus\0"),
        set_window_keyboard_grab: load_host_symbol(b"SDL_SetWindowKeyboardGrab\0"),
        set_window_maximum_size: load_host_symbol(b"SDL_SetWindowMaximumSize\0"),
        set_window_minimum_size: load_host_symbol(b"SDL_SetWindowMinimumSize\0"),
        set_window_modal_for: load_host_symbol(b"SDL_SetWindowModalFor\0"),
        set_window_mouse_grab: load_host_symbol(b"SDL_SetWindowMouseGrab\0"),
        set_window_mouse_rect: load_host_symbol(b"SDL_SetWindowMouseRect\0"),
        set_window_opacity: load_host_symbol(b"SDL_SetWindowOpacity\0"),
        set_window_position: load_host_symbol(b"SDL_SetWindowPosition\0"),
        set_window_resizable: load_host_symbol(b"SDL_SetWindowResizable\0"),
        set_window_size: load_host_symbol(b"SDL_SetWindowSize\0"),
        set_window_title: load_host_symbol(b"SDL_SetWindowTitle\0"),
        show_window: load_host_symbol(b"SDL_ShowWindow\0"),
        update_window_surface: load_host_symbol(b"SDL_UpdateWindowSurface\0"),
        update_window_surface_rects: load_host_symbol(b"SDL_UpdateWindowSurfaceRects\0"),
    })
}

struct StubWindow {
    id: Uint32,
    title: CString,
    x: libc::c_int,
    y: libc::c_int,
    w: libc::c_int,
    h: libc::c_int,
    flags: Uint32,
    brightness: f32,
    opacity: f32,
    surface: *mut SDL_Surface,
    window_data: HashMap<Vec<u8>, usize>,
    display_mode: Option<SDL_DisplayMode>,
    min_size: (libc::c_int, libc::c_int),
    max_size: (libc::c_int, libc::c_int),
    mouse_rect: Option<SDL_Rect>,
    hit_test: SDL_HitTest,
    hit_test_data: usize,
    shaped: bool,
    shape_mode: Option<SDL_WindowShapeMode>,
    restore_bounds: Option<WindowBounds>,
}

unsafe impl Send for StubWindow {}

#[derive(Clone, Copy)]
struct WindowBounds {
    x: libc::c_int,
    y: libc::c_int,
    w: libc::c_int,
    h: libc::c_int,
}

struct WindowRegistry {
    next_id: Uint32,
    windows: HashMap<usize, Box<StubWindow>>,
    by_id: HashMap<Uint32, usize>,
}

impl Default for WindowRegistry {
    fn default() -> Self {
        Self {
            next_id: 1,
            windows: HashMap::new(),
            by_id: HashMap::new(),
        }
    }
}

fn window_registry() -> &'static Mutex<WindowRegistry> {
    static REGISTRY: OnceLock<Mutex<WindowRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(WindowRegistry::default()))
}

fn lock_window_registry() -> std::sync::MutexGuard<'static, WindowRegistry> {
    match window_registry().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn cstring_from_ptr(text: *const libc::c_char) -> Result<CString, ()> {
    if text.is_null() {
        return Ok(CString::new("").unwrap());
    }
    CString::new(unsafe { CStr::from_ptr(text).to_bytes() }).map_err(|_| {
        let _ = crate::core::error::set_error_message("Window title is invalid");
    })
}

fn with_stub_window_mut<T>(
    window: *mut SDL_Window,
    callback: impl FnOnce(&mut StubWindow) -> T,
) -> Result<T, ()> {
    if window.is_null() {
        let _ = crate::core::error::set_error_message("Invalid window");
        return Err(());
    }
    let mut registry = lock_window_registry();
    registry
        .windows
        .get_mut(&(window as usize))
        .map(|entry| callback(entry))
        .ok_or_else(|| {
            let _ = crate::core::error::set_error_message("Invalid window");
        })
}

fn with_stub_window<T>(
    window: *mut SDL_Window,
    callback: impl FnOnce(&StubWindow) -> T,
) -> Result<T, ()> {
    if window.is_null() {
        let _ = crate::core::error::set_error_message("Invalid window");
        return Err(());
    }
    let registry = lock_window_registry();
    registry
        .windows
        .get(&(window as usize))
        .map(|entry| callback(entry))
        .ok_or_else(|| {
            let _ = crate::core::error::set_error_message("Invalid window");
        })
}

fn alloc_surface(width: libc::c_int, height: libc::c_int) -> *mut SDL_Surface {
    unsafe {
        crate::video::surface::SDL_CreateRGBSurfaceWithFormat(
            0,
            width.max(1),
            height.max(1),
            32,
            SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888,
        )
    }
}

fn free_surface(surface: *mut SDL_Surface) {
    if !surface.is_null() {
        unsafe {
            crate::video::surface::SDL_FreeSurface(surface);
        }
    }
}

fn recreate_surface(window: &mut StubWindow) -> libc::c_int {
    free_surface(window.surface);
    window.surface = alloc_surface(window.w, window.h);
    if window.surface.is_null() {
        -1
    } else {
        0
    }
}

fn normalize_window_flags(flags: Uint32) -> Uint32 {
    if flags & SDL_WindowFlags_SDL_WINDOW_HIDDEN != 0 {
        flags & !SDL_WindowFlags_SDL_WINDOW_SHOWN
    } else {
        (flags | SDL_WindowFlags_SDL_WINDOW_SHOWN) & !SDL_WindowFlags_SDL_WINDOW_HIDDEN
    }
}

fn window_position_display_index(value: libc::c_int) -> Option<libc::c_int> {
    let encoded = value as u32;
    let mask = encoded & 0xFFFF_0000;
    if mask == SDL_WINDOWPOS_CENTERED_MASK || mask == SDL_WINDOWPOS_UNDEFINED_MASK {
        Some((encoded & 0xFFFF) as libc::c_int)
    } else {
        None
    }
}

fn clamp_stub_display_index(display_index: libc::c_int) -> libc::c_int {
    let display_count = unsafe { crate::video::display::SDL_GetNumVideoDisplays() }.max(1);
    if display_index < 0 || display_index >= display_count {
        0
    } else {
        display_index
    }
}

fn display_bounds_for_window_position(x: libc::c_int, y: libc::c_int) -> SDL_Rect {
    let requested_display = window_position_display_index(x)
        .or_else(|| window_position_display_index(y))
        .unwrap_or(0);
    let mut bounds = SDL_Rect {
        x: 0,
        y: 0,
        w: 1024,
        h: 768,
    };
    unsafe {
        let _ = crate::video::display::SDL_GetDisplayBounds(
            clamp_stub_display_index(requested_display),
            &mut bounds,
        );
    }
    bounds
}

fn resolve_stub_window_bounds(
    x: libc::c_int,
    y: libc::c_int,
    w: libc::c_int,
    h: libc::c_int,
    flags: Uint32,
) -> WindowBounds {
    let mut bounds = WindowBounds {
        x,
        y,
        w: w.max(1),
        h: h.max(1),
    };
    let display_bounds = display_bounds_for_window_position(x, y);

    if flags & SDL_WindowFlags_SDL_WINDOW_FULLSCREEN_DESKTOP != 0 {
        bounds.x = display_bounds.x;
        bounds.y = display_bounds.y;
        bounds.w = display_bounds.w;
        bounds.h = display_bounds.h;
        return bounds;
    }

    if (x as u32) & 0xFFFF_0000 == SDL_WINDOWPOS_CENTERED_MASK {
        bounds.x = display_bounds.x + ((display_bounds.w - bounds.w) / 2);
    } else if (x as u32) & 0xFFFF_0000 == SDL_WINDOWPOS_UNDEFINED_MASK {
        bounds.x = display_bounds.x;
    }

    if (y as u32) & 0xFFFF_0000 == SDL_WINDOWPOS_CENTERED_MASK {
        bounds.y = display_bounds.y + ((display_bounds.h - bounds.h) / 2);
    } else if (y as u32) & 0xFFFF_0000 == SDL_WINDOWPOS_UNDEFINED_MASK {
        bounds.y = display_bounds.y;
    }

    bounds
}

fn set_stub_window_bounds(window: &mut StubWindow, bounds: WindowBounds) {
    window.x = bounds.x;
    window.y = bounds.y;
    window.w = bounds.w.max(1);
    window.h = bounds.h.max(1);
}

fn push_stub_window_event(window_id: Uint32, event: u8, data1: libc::c_int, data2: libc::c_int) {
    let mut sdl_event = SDL_Event {
        window: SDL_WindowEvent {
            type_: SDL_EventType_SDL_WINDOWEVENT,
            timestamp: 0,
            windowID: window_id,
            event,
            padding1: 0,
            padding2: 0,
            padding3: 0,
            data1,
            data2,
        },
    };
    unsafe {
        let _ = crate::events::queue::SDL_PushEvent(&mut sdl_event);
    }
}

fn focus_stub_window(window: *mut SDL_Window, emit_focus_event: bool) {
    if window.is_null() {
        return;
    }

    let mut focused_window_id = 0;
    {
        let target = window as usize;
        let mut registry = lock_window_registry();
        for (key, entry) in registry.windows.iter_mut() {
            if *key == target {
                entry.flags |=
                    SDL_WindowFlags_SDL_WINDOW_INPUT_FOCUS | SDL_WindowFlags_SDL_WINDOW_MOUSE_FOCUS;
                focused_window_id = entry.id;
            } else {
                entry.flags &= !(SDL_WindowFlags_SDL_WINDOW_INPUT_FOCUS
                    | SDL_WindowFlags_SDL_WINDOW_MOUSE_FOCUS);
            }
        }
    }

    if focused_window_id == 0 {
        return;
    }

    crate::events::keyboard::set_keyboard_focus(window);
    crate::events::mouse::set_mouse_focus(window);
    if emit_focus_event {
        push_stub_window_event(
            focused_window_id,
            SDL_WindowEventID_SDL_WINDOWEVENT_FOCUS_GAINED as u8,
            0,
            0,
        );
    }
}

fn fill_linear_gamma_ramp(channel: *mut Uint16) {
    if channel.is_null() {
        return;
    }
    for (index, value) in (0u32..256).enumerate() {
        unsafe {
            *channel.add(index) = ((value * 0xFFFF) / 0xFF) as Uint16;
        }
    }
}

pub(crate) fn create_stub_window_internal(
    title: *const libc::c_char,
    x: libc::c_int,
    y: libc::c_int,
    w: libc::c_int,
    h: libc::c_int,
    flags: Uint32,
    shaped: bool,
) -> *mut SDL_Window {
    if crate::video::display::require_video_driver().is_err() {
        return std::ptr::null_mut();
    }

    let title = match cstring_from_ptr(title) {
        Ok(title) => title,
        Err(()) => return std::ptr::null_mut(),
    };

    let mut registry = lock_window_registry();
    let id = registry.next_id;
    registry.next_id = registry.next_id.saturating_add(1).max(1);
    let bounds = resolve_stub_window_bounds(x, y, w, h, flags);

    let mut entry = Box::new(StubWindow {
        id,
        title,
        x: bounds.x,
        y: bounds.y,
        w: bounds.w,
        h: bounds.h,
        flags: normalize_window_flags(flags),
        brightness: 1.0,
        opacity: 1.0,
        surface: std::ptr::null_mut(),
        window_data: HashMap::new(),
        display_mode: None,
        min_size: (0, 0),
        max_size: (0, 0),
        mouse_rect: None,
        hit_test: None,
        hit_test_data: 0,
        shaped,
        shape_mode: None,
        restore_bounds: None,
    });
    let ptr = entry.as_mut() as *mut StubWindow as *mut SDL_Window;
    registry.by_id.insert(id, ptr as usize);
    registry.windows.insert(ptr as usize, entry);
    drop(registry);

    if flags & SDL_WindowFlags_SDL_WINDOW_HIDDEN == 0 {
        focus_stub_window(ptr, false);
    }
    ptr
}

pub(crate) fn is_stub_window(window: *mut SDL_Window) -> bool {
    if window.is_null() {
        return false;
    }

    let registry = lock_window_registry();
    registry.windows.contains_key(&(window as usize))
}

pub(crate) fn stub_window_size(window: *mut SDL_Window) -> Result<(libc::c_int, libc::c_int), ()> {
    with_stub_window(window, |entry| (entry.w, entry.h))
}

pub(crate) fn stub_window_has_surface(window: *mut SDL_Window) -> Result<bool, ()> {
    with_stub_window(window, |entry| !entry.surface.is_null())
}

pub(crate) fn stub_window_is_shaped(window: *const SDL_Window) -> SDL_bool {
    let Ok(shaped) = with_stub_window(window as *mut SDL_Window, |entry| entry.shaped) else {
        return 0;
    };
    shaped as SDL_bool
}

pub(crate) fn stub_window_set_shape(
    window: *mut SDL_Window,
    _shape: *mut SDL_Surface,
    shape_mode: *mut SDL_WindowShapeMode,
) -> libc::c_int {
    with_stub_window_mut(window, |entry| {
        if !entry.shaped {
            return crate::core::error::set_error_message("Window is not shapeable");
        }
        entry.shape_mode = if shape_mode.is_null() {
            None
        } else {
            Some(unsafe { *shape_mode })
        };
        0
    })
    .unwrap_or(-1)
}

pub(crate) fn stub_window_get_shape_mode(
    window: *mut SDL_Window,
    shape_mode: *mut SDL_WindowShapeMode,
) -> libc::c_int {
    with_stub_window(window, |entry| {
        if !entry.shaped {
            return crate::core::error::set_error_message("Window is not shapeable");
        }
        if let Some(mode) = entry.shape_mode {
            if !shape_mode.is_null() {
                unsafe {
                    *shape_mode = mode;
                }
            }
            0
        } else {
            crate::core::error::set_error_message("Window currently has no shape")
        }
    })
    .unwrap_or(-1)
}

pub(crate) fn reset_video_state() {
    unsafe {
        crate::render::core::reset_managed_window_renderers();
    }
    let mut registry = lock_window_registry();
    for entry in registry.windows.values_mut() {
        free_surface(entry.surface);
        entry.surface = std::ptr::null_mut();
    }
    *registry = WindowRegistry::default();
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
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().create_window)(title, x, y, w, h, flags);
    }
    create_stub_window_internal(title, x, y, w, h, flags, false)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateWindowAndRenderer(
    width: libc::c_int,
    height: libc::c_int,
    window_flags: Uint32,
    window: *mut *mut SDL_Window,
    renderer: *mut *mut SDL_Renderer,
) -> libc::c_int {
    if window.is_null() {
        return crate::core::error::invalid_param_error("window");
    }
    *window = SDL_CreateWindow(std::ptr::null(), 0, 0, width, height, window_flags);
    if !renderer.is_null() {
        *renderer = std::ptr::null_mut();
    }
    if (*window).is_null() {
        return -1;
    }

    if renderer.is_null() {
        0
    } else {
        *renderer = crate::render::core::SDL_CreateRenderer(*window, -1, 0);
        if (*renderer).is_null() {
            SDL_DestroyWindow(*window);
            *window = std::ptr::null_mut();
            -1
        } else {
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateWindowFrom(data: *const libc::c_void) -> *mut SDL_Window {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().create_window_from)(data);
    }
    let _ = crate::core::error::set_error_message("SDL_CreateWindowFrom() is not supported");
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DestroyWindow(window: *mut SDL_Window) {
    if window.is_null() {
        return;
    }
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().destroy_window)(window);
        return;
    }

    crate::render::core::destroy_window_renderer(window);
    let mut registry = lock_window_registry();
    if let Some(mut entry) = registry.windows.remove(&(window as usize)) {
        registry.by_id.remove(&entry.id);
        free_surface(entry.surface);
        entry.surface = std::ptr::null_mut();
    }
    drop(registry);
    crate::events::keyboard::clear_keyboard_focus(window);
    crate::events::mouse::clear_window_references(window);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_DestroyWindowSurface(window: *mut SDL_Window) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().destroy_window_surface)(window);
    }
    with_stub_window_mut(window, |entry| {
        free_surface(entry.surface);
        entry.surface = std::ptr::null_mut();
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FlashWindow(
    window: *mut SDL_Window,
    operation: SDL_FlashOperation,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().flash_window)(window, operation);
    }
    let _ = operation;
    with_stub_window(window, |_| 0).unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowBordersSize(
    window: *mut SDL_Window,
    top: *mut libc::c_int,
    left: *mut libc::c_int,
    bottom: *mut libc::c_int,
    right: *mut libc::c_int,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_borders_size)(window, top, left, bottom, right);
    }
    with_stub_window(window, |_| {
        if !top.is_null() {
            *top = 0;
        }
        if !left.is_null() {
            *left = 0;
        }
        if !bottom.is_null() {
            *bottom = 0;
        }
        if !right.is_null() {
            *right = 0;
        }
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowBrightness(window: *mut SDL_Window) -> f32 {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_brightness)(window);
    }
    with_stub_window(window, |entry| entry.brightness).unwrap_or(1.0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowData(
    window: *mut SDL_Window,
    name: *const libc::c_char,
) -> *mut libc::c_void {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_data)(window, name);
    }
    if name.is_null() {
        let _ = crate::core::error::invalid_param_error("name");
        return std::ptr::null_mut();
    }
    let key = CStr::from_ptr(name).to_bytes().to_vec();
    if key.is_empty() {
        let _ = crate::core::error::invalid_param_error("name");
        return std::ptr::null_mut();
    }
    with_stub_window(window, |entry| {
        entry.window_data.get(&key).copied().unwrap_or(0) as *mut libc::c_void
    })
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowDisplayIndex(window: *mut SDL_Window) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_display_index)(window);
    }
    with_stub_window(window, |_| 0).unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowDisplayMode(
    window: *mut SDL_Window,
    mode: *mut SDL_DisplayMode,
) -> libc::c_int {
    if mode.is_null() {
        return crate::core::error::invalid_param_error("mode");
    }
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_display_mode)(window, mode);
    }
    with_stub_window(window, |entry| {
        *mode = entry
            .display_mode
            .unwrap_or_else(crate::video::display::stub_display_mode);
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowFlags(window: *mut SDL_Window) -> Uint32 {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_flags)(window);
    }
    with_stub_window(window, |entry| entry.flags).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowFromID(id: Uint32) -> *mut SDL_Window {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_from_id)(id);
    }
    let registry = lock_window_registry();
    registry.by_id.get(&id).copied().unwrap_or(0) as *mut SDL_Window
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowGammaRamp(
    window: *mut SDL_Window,
    red: *mut Uint16,
    green: *mut Uint16,
    blue: *mut Uint16,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_gamma_ramp)(window, red, green, blue);
    }
    with_stub_window(window, |_| {
        fill_linear_gamma_ramp(red);
        fill_linear_gamma_ramp(green);
        fill_linear_gamma_ramp(blue);
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowGrab(window: *mut SDL_Window) -> SDL_bool {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_grab)(window);
    }
    with_stub_window(window, |entry| {
        ((entry.flags
            & (SDL_WindowFlags_SDL_WINDOW_MOUSE_GRABBED
                | SDL_WindowFlags_SDL_WINDOW_KEYBOARD_GRABBED))
            != 0) as SDL_bool
    })
    .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowID(window: *mut SDL_Window) -> Uint32 {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_id)(window);
    }
    with_stub_window(window, |entry| entry.id).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowKeyboardGrab(window: *mut SDL_Window) -> SDL_bool {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_keyboard_grab)(window);
    }
    with_stub_window(window, |entry| {
        (entry.flags & SDL_WindowFlags_SDL_WINDOW_KEYBOARD_GRABBED != 0) as SDL_bool
    })
    .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowMaximumSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().get_window_maximum_size)(window, w, h);
        return;
    }
    let _ = with_stub_window(window, |entry| {
        if !w.is_null() {
            *w = entry.max_size.0;
        }
        if !h.is_null() {
            *h = entry.max_size.1;
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowMinimumSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().get_window_minimum_size)(window, w, h);
        return;
    }
    let _ = with_stub_window(window, |entry| {
        if !w.is_null() {
            *w = entry.min_size.0;
        }
        if !h.is_null() {
            *h = entry.min_size.1;
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowMouseGrab(window: *mut SDL_Window) -> SDL_bool {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_mouse_grab)(window);
    }
    with_stub_window(window, |entry| {
        (entry.flags & SDL_WindowFlags_SDL_WINDOW_MOUSE_GRABBED != 0) as SDL_bool
    })
    .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowMouseRect(window: *mut SDL_Window) -> *const SDL_Rect {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_mouse_rect)(window);
    }
    let registry = lock_window_registry();
    registry
        .windows
        .get(&(window as usize))
        .and_then(|entry| entry.mouse_rect.as_ref())
        .map(|rect| rect as *const SDL_Rect)
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowOpacity(
    window: *mut SDL_Window,
    out_opacity: *mut f32,
) -> libc::c_int {
    if out_opacity.is_null() {
        return crate::core::error::invalid_param_error("out_opacity");
    }
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_opacity)(window, out_opacity);
    }
    with_stub_window(window, |entry| {
        *out_opacity = entry.opacity;
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowPixelFormat(window: *mut SDL_Window) -> Uint32 {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_pixel_format)(window);
    }
    with_stub_window(window, |_| SDL_PixelFormatEnum_SDL_PIXELFORMAT_ARGB8888).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowPosition(
    window: *mut SDL_Window,
    x: *mut libc::c_int,
    y: *mut libc::c_int,
) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().get_window_position)(window, x, y);
        return;
    }
    let _ = with_stub_window(window, |entry| {
        if !x.is_null() {
            *x = entry.x;
        }
        if !y.is_null() {
            *y = entry.y;
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowSize(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().get_window_size)(window, w, h);
        return;
    }
    let _ = with_stub_window(window, |entry| {
        if !w.is_null() {
            *w = entry.w;
        }
        if !h.is_null() {
            *h = entry.h;
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowSizeInPixels(
    window: *mut SDL_Window,
    w: *mut libc::c_int,
    h: *mut libc::c_int,
) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().get_window_size_in_pixels)(window, w, h);
        return;
    }
    SDL_GetWindowSize(window, w, h);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowSurface(window: *mut SDL_Window) -> *mut SDL_Surface {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_surface)(window);
    }
    if crate::render::core::window_has_renderer(window) {
        let _ = crate::core::error::set_error_message("Renderer already associated with window");
        return std::ptr::null_mut();
    }
    with_stub_window_mut(window, |entry| {
        if entry.surface.is_null() && recreate_surface(entry) != 0 {
            return std::ptr::null_mut();
        }
        entry.surface
    })
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetWindowTitle(window: *mut SDL_Window) -> *const libc::c_char {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().get_window_title)(window);
    }
    with_stub_window(window, |entry| entry.title.as_ptr()).unwrap_or(std::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HideWindow(window: *mut SDL_Window) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().hide_window)(window);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        entry.flags |= SDL_WindowFlags_SDL_WINDOW_HIDDEN;
        entry.flags &= !SDL_WindowFlags_SDL_WINDOW_SHOWN;
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MaximizeWindow(window: *mut SDL_Window) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().maximize_window)(window);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        entry.flags |= SDL_WindowFlags_SDL_WINDOW_MAXIMIZED;
        entry.flags &= !SDL_WindowFlags_SDL_WINDOW_MINIMIZED;
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MinimizeWindow(window: *mut SDL_Window) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().minimize_window)(window);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        entry.flags |= SDL_WindowFlags_SDL_WINDOW_MINIMIZED;
        entry.flags &= !SDL_WindowFlags_SDL_WINDOW_MAXIMIZED;
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RaiseWindow(window: *mut SDL_Window) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().raise_window)(window);
    } else {
        focus_stub_window(window, true);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RestoreWindow(window: *mut SDL_Window) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().restore_window)(window);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        entry.flags &= !SDL_WindowFlags_SDL_WINDOW_MINIMIZED;
        entry.flags &= !SDL_WindowFlags_SDL_WINDOW_MAXIMIZED;
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowAlwaysOnTop(window: *mut SDL_Window, on_top: SDL_bool) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_always_on_top)(window, on_top);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        if on_top != 0 {
            entry.flags |= SDL_WindowFlags_SDL_WINDOW_ALWAYS_ON_TOP;
        } else {
            entry.flags &= !SDL_WindowFlags_SDL_WINDOW_ALWAYS_ON_TOP;
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowBordered(window: *mut SDL_Window, bordered: SDL_bool) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_bordered)(window, bordered);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        if bordered != 0 {
            entry.flags &= !SDL_WindowFlags_SDL_WINDOW_BORDERLESS;
        } else {
            entry.flags |= SDL_WindowFlags_SDL_WINDOW_BORDERLESS;
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowBrightness(
    window: *mut SDL_Window,
    brightness: f32,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().set_window_brightness)(window, brightness);
    }
    with_stub_window_mut(window, |entry| {
        entry.brightness = brightness;
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowData(
    window: *mut SDL_Window,
    name: *const libc::c_char,
    userdata: *mut libc::c_void,
) -> *mut libc::c_void {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().set_window_data)(window, name, userdata);
    }
    if name.is_null() {
        let _ = crate::core::error::invalid_param_error("name");
        return std::ptr::null_mut();
    }
    let key = CStr::from_ptr(name).to_bytes().to_vec();
    if key.is_empty() {
        let _ = crate::core::error::invalid_param_error("name");
        return std::ptr::null_mut();
    }
    with_stub_window_mut(window, |entry| {
        if userdata.is_null() {
            entry.window_data.remove(&key).unwrap_or(0) as *mut libc::c_void
        } else {
            entry
                .window_data
                .insert(key, userdata as usize)
                .unwrap_or(0) as *mut libc::c_void
        }
    })
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowDisplayMode(
    window: *mut SDL_Window,
    mode: *const SDL_DisplayMode,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().set_window_display_mode)(window, mode);
    }
    with_stub_window_mut(window, |entry| {
        entry.display_mode = if mode.is_null() { None } else { Some(*mode) };
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowFullscreen(
    window: *mut SDL_Window,
    flags: Uint32,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().set_window_fullscreen)(window, flags);
    }
    with_stub_window_mut(window, |entry| {
        let had_fullscreen_desktop =
            entry.flags & SDL_WindowFlags_SDL_WINDOW_FULLSCREEN_DESKTOP != 0;
        entry.flags &= !(SDL_WindowFlags_SDL_WINDOW_FULLSCREEN
            | SDL_WindowFlags_SDL_WINDOW_FULLSCREEN_DESKTOP);
        entry.flags |= flags
            & (SDL_WindowFlags_SDL_WINDOW_FULLSCREEN
                | SDL_WindowFlags_SDL_WINDOW_FULLSCREEN_DESKTOP);
        if flags & SDL_WindowFlags_SDL_WINDOW_FULLSCREEN_DESKTOP != 0 {
            if !had_fullscreen_desktop {
                entry.restore_bounds = Some(WindowBounds {
                    x: entry.x,
                    y: entry.y,
                    w: entry.w,
                    h: entry.h,
                });
            }
            let fullscreen_bounds =
                resolve_stub_window_bounds(entry.x, entry.y, entry.w, entry.h, flags);
            set_stub_window_bounds(entry, fullscreen_bounds);
        } else if had_fullscreen_desktop {
            if let Some(windowed_bounds) = entry.restore_bounds.take() {
                set_stub_window_bounds(entry, windowed_bounds);
            }
        }
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowGrab(window: *mut SDL_Window, grabbed: SDL_bool) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_grab)(window, grabbed);
        return;
    }
    SDL_SetWindowMouseGrab(window, grabbed);
    if grabbed == 0
        || crate::core::hints::SDL_GetHintBoolean(
            SDL_HINT_GRAB_KEYBOARD.as_ptr().cast(),
            SDL_bool_SDL_FALSE,
        ) != 0
    {
        SDL_SetWindowKeyboardGrab(window, grabbed);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowHitTest(
    window: *mut SDL_Window,
    callback: SDL_HitTest,
    callback_data: *mut libc::c_void,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().set_window_hit_test)(window, callback, callback_data);
    }
    with_stub_window_mut(window, |entry| {
        entry.hit_test = callback;
        entry.hit_test_data = callback_data as usize;
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowIcon(window: *mut SDL_Window, icon: *mut SDL_Surface) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_icon)(window, icon);
    } else {
        let _ = (window, icon);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowInputFocus(window: *mut SDL_Window) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().set_window_input_focus)(window);
    }
    if with_stub_window(window, |_| ()).is_err() {
        return -1;
    }
    focus_stub_window(window, true);
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowKeyboardGrab(window: *mut SDL_Window, grabbed: SDL_bool) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_keyboard_grab)(window, grabbed);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        if grabbed != 0 {
            entry.flags |= SDL_WindowFlags_SDL_WINDOW_KEYBOARD_GRABBED;
        } else {
            entry.flags &= !SDL_WindowFlags_SDL_WINDOW_KEYBOARD_GRABBED;
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowMaximumSize(
    window: *mut SDL_Window,
    max_w: libc::c_int,
    max_h: libc::c_int,
) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_maximum_size)(window, max_w, max_h);
        return;
    }
    if window.is_null() {
        let _ = crate::core::error::set_error_message("Invalid window");
        return;
    }
    if max_w <= 0 {
        let _ = crate::core::error::invalid_param_error("max_w");
        return;
    }
    if max_h <= 0 {
        let _ = crate::core::error::invalid_param_error("max_h");
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        entry.max_size = (max_w, max_h);
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowMinimumSize(
    window: *mut SDL_Window,
    min_w: libc::c_int,
    min_h: libc::c_int,
) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_minimum_size)(window, min_w, min_h);
        return;
    }
    if window.is_null() {
        let _ = crate::core::error::set_error_message("Invalid window");
        return;
    }
    if min_w <= 0 {
        let _ = crate::core::error::invalid_param_error("min_w");
        return;
    }
    if min_h <= 0 {
        let _ = crate::core::error::invalid_param_error("min_h");
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        entry.min_size = (min_w, min_h);
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowModalFor(
    modal_window: *mut SDL_Window,
    parent_window: *mut SDL_Window,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().set_window_modal_for)(modal_window, parent_window);
    }
    let _ = parent_window;
    with_stub_window(modal_window, |_| 0).unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowMouseGrab(window: *mut SDL_Window, grabbed: SDL_bool) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_mouse_grab)(window, grabbed);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        if grabbed != 0 {
            entry.flags |= SDL_WindowFlags_SDL_WINDOW_MOUSE_GRABBED;
            crate::events::mouse::set_grabbed_window(window);
        } else {
            entry.flags &= !SDL_WindowFlags_SDL_WINDOW_MOUSE_GRABBED;
            crate::events::mouse::set_grabbed_window(std::ptr::null_mut());
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowMouseRect(
    window: *mut SDL_Window,
    rect: *const SDL_Rect,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().set_window_mouse_rect)(window, rect);
    }
    with_stub_window_mut(window, |entry| {
        entry.mouse_rect = if rect.is_null() { None } else { Some(*rect) };
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowOpacity(
    window: *mut SDL_Window,
    opacity: f32,
) -> libc::c_int {
    if !(0.0..=1.0).contains(&opacity) {
        return crate::core::error::set_error_message("Window opacity must be between 0.0 and 1.0");
    }
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().set_window_opacity)(window, opacity);
    }
    with_stub_window_mut(window, |entry| {
        entry.opacity = opacity;
        0
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowPosition(
    window: *mut SDL_Window,
    x: libc::c_int,
    y: libc::c_int,
) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_position)(window, x, y);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        let bounds = resolve_stub_window_bounds(x, y, entry.w, entry.h, entry.flags);
        entry.x = bounds.x;
        entry.y = bounds.y;
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowResizable(window: *mut SDL_Window, resizable: SDL_bool) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_resizable)(window, resizable);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        if resizable != 0 {
            entry.flags |= SDL_WindowFlags_SDL_WINDOW_RESIZABLE;
        } else {
            entry.flags &= !SDL_WindowFlags_SDL_WINDOW_RESIZABLE;
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowSize(
    window: *mut SDL_Window,
    w: libc::c_int,
    h: libc::c_int,
) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_size)(window, w, h);
        return;
    }
    if window.is_null() {
        let _ = crate::core::error::set_error_message("Invalid window");
        return;
    }
    if w <= 0 {
        let _ = crate::core::error::invalid_param_error("w");
        return;
    }
    if h <= 0 {
        let _ = crate::core::error::invalid_param_error("h");
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        entry.w = w;
        entry.h = h;
        if !entry.surface.is_null() {
            let _ = recreate_surface(entry);
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetWindowTitle(window: *mut SDL_Window, title: *const libc::c_char) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().set_window_title)(window, title);
        return;
    }
    let Ok(title) = cstring_from_ptr(title) else {
        return;
    };
    let _ = with_stub_window_mut(window, |entry| {
        entry.title = title;
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ShowWindow(window: *mut SDL_Window) {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        (host_api().show_window)(window);
        return;
    }
    let _ = with_stub_window_mut(window, |entry| {
        entry.flags |= SDL_WindowFlags_SDL_WINDOW_SHOWN;
        entry.flags &= !SDL_WindowFlags_SDL_WINDOW_HIDDEN;
    });
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UpdateWindowSurface(window: *mut SDL_Window) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().update_window_surface)(window);
    }
    if SDL_GetWindowSurface(window).is_null() {
        -1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_UpdateWindowSurfaceRects(
    window: *mut SDL_Window,
    rects: *const SDL_Rect,
    numrects: libc::c_int,
) -> libc::c_int {
    if crate::video::display::current_driver_is_host() {
        crate::video::clear_real_error();
        return (host_api().update_window_surface_rects)(window, rects, numrects);
    }
    let _ = (rects, numrects);
    SDL_UpdateWindowSurface(window)
}
