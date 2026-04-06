use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_Cursor, SDL_Surface, SDL_SystemCursor, SDL_Window, SDL_bool, Uint32, Uint8, SDL_QUERY,
};

struct CursorRecord {
    is_default: bool,
    _system_cursor: Option<SDL_SystemCursor>,
}

struct MouseState {
    cursors: HashMap<usize, Box<CursorRecord>>,
    current_cursor: usize,
    default_cursor: usize,
    visible: libc::c_int,
    relative_mode: bool,
    x: libc::c_int,
    y: libc::c_int,
    rel_x: libc::c_int,
    rel_y: libc::c_int,
    buttons: Uint32,
    focus_window: usize,
    grabbed_window: usize,
}

impl Default for MouseState {
    fn default() -> Self {
        let mut cursors = HashMap::new();
        let mut default_cursor = Box::new(CursorRecord {
            is_default: true,
            _system_cursor: Some(0),
        });
        let default_ptr = default_cursor.as_mut() as *mut CursorRecord as usize;
        cursors.insert(default_ptr, default_cursor);
        Self {
            cursors,
            current_cursor: default_ptr,
            default_cursor: default_ptr,
            visible: 1,
            relative_mode: false,
            x: 0,
            y: 0,
            rel_x: 0,
            rel_y: 0,
            buttons: 0,
            focus_window: 0,
            grabbed_window: 0,
        }
    }
}

fn mouse_state() -> &'static Mutex<MouseState> {
    static STATE: OnceLock<Mutex<MouseState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(MouseState::default()))
}

fn lock_mouse_state() -> std::sync::MutexGuard<'static, MouseState> {
    match mouse_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn alloc_cursor(system_cursor: Option<SDL_SystemCursor>) -> *mut SDL_Cursor {
    let mut state = lock_mouse_state();
    let mut record = Box::new(CursorRecord {
        is_default: false,
        _system_cursor: system_cursor,
    });
    let ptr = record.as_mut() as *mut CursorRecord as *mut SDL_Cursor;
    state.cursors.insert(ptr as usize, record);
    ptr
}

pub(crate) fn set_mouse_focus(window: *mut SDL_Window) {
    lock_mouse_state().focus_window = window as usize;
}

pub(crate) fn set_grabbed_window(window: *mut SDL_Window) {
    lock_mouse_state().grabbed_window = window as usize;
}

pub(crate) fn clear_window_references(window: *mut SDL_Window) {
    let mut state = lock_mouse_state();
    let target = window as usize;
    if state.focus_window == target {
        state.focus_window = 0;
    }
    if state.grabbed_window == target {
        state.grabbed_window = 0;
    }
}

pub(crate) fn reset_mouse_state() {
    *lock_mouse_state() = MouseState::default();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CaptureMouse(_enabled: SDL_bool) -> libc::c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateColorCursor(
    surface: *mut SDL_Surface,
    _hot_x: libc::c_int,
    _hot_y: libc::c_int,
) -> *mut SDL_Cursor {
    if surface.is_null() {
        let _ = crate::core::error::invalid_param_error("surface");
        return std::ptr::null_mut();
    }
    alloc_cursor(None)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateCursor(
    data: *const Uint8,
    mask: *const Uint8,
    _w: libc::c_int,
    _h: libc::c_int,
    _hot_x: libc::c_int,
    _hot_y: libc::c_int,
) -> *mut SDL_Cursor {
    if data.is_null() || mask.is_null() {
        let _ = crate::core::error::set_error_message("Cursor data is invalid");
        return std::ptr::null_mut();
    }
    alloc_cursor(None)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_CreateSystemCursor(id: SDL_SystemCursor) -> *mut SDL_Cursor {
    alloc_cursor(Some(id))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreeCursor(cursor: *mut SDL_Cursor) {
    if cursor.is_null() {
        return;
    }
    let mut state = lock_mouse_state();
    let cursor_key = cursor as usize;
    if cursor_key == state.default_cursor {
        return;
    }
    if state.current_cursor == cursor_key {
        state.current_cursor = state.default_cursor;
    }
    state.cursors.remove(&cursor_key);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetCursor() -> *mut SDL_Cursor {
    lock_mouse_state().current_cursor as *mut SDL_Cursor
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetDefaultCursor() -> *mut SDL_Cursor {
    lock_mouse_state().default_cursor as *mut SDL_Cursor
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetGlobalMouseState(
    x: *mut libc::c_int,
    y: *mut libc::c_int,
) -> Uint32 {
    let state = lock_mouse_state();
    if !x.is_null() {
        *x = state.x;
    }
    if !y.is_null() {
        *y = state.y;
    }
    state.buttons
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetGrabbedWindow() -> *mut SDL_Window {
    lock_mouse_state().grabbed_window as *mut SDL_Window
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetMouseFocus() -> *mut SDL_Window {
    lock_mouse_state().focus_window as *mut SDL_Window
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetMouseState(x: *mut libc::c_int, y: *mut libc::c_int) -> Uint32 {
    let state = lock_mouse_state();
    if !x.is_null() {
        *x = state.x;
    }
    if !y.is_null() {
        *y = state.y;
    }
    state.buttons
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRelativeMouseMode() -> SDL_bool {
    lock_mouse_state().relative_mode as SDL_bool
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetRelativeMouseState(
    x: *mut libc::c_int,
    y: *mut libc::c_int,
) -> Uint32 {
    let mut state = lock_mouse_state();
    if !x.is_null() {
        *x = state.rel_x;
    }
    if !y.is_null() {
        *y = state.rel_y;
    }
    state.rel_x = 0;
    state.rel_y = 0;
    state.buttons
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetCursor(cursor: *mut SDL_Cursor) {
    let mut state = lock_mouse_state();
    let cursor_key = if cursor.is_null() {
        state.default_cursor
    } else {
        cursor as usize
    };
    if state.cursors.contains_key(&cursor_key) {
        state.current_cursor = cursor_key;
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetRelativeMouseMode(enabled: SDL_bool) -> libc::c_int {
    lock_mouse_state().relative_mode = enabled != 0;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ShowCursor(toggle: libc::c_int) -> libc::c_int {
    let mut state = lock_mouse_state();
    let previous = state.visible;
    if toggle != SDL_QUERY {
        state.visible = if toggle == 0 { 0 } else { 1 };
    }
    if toggle == SDL_QUERY {
        state.visible
    } else {
        previous
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WarpMouseGlobal(x: libc::c_int, y: libc::c_int) -> libc::c_int {
    let mut state = lock_mouse_state();
    state.rel_x += x - state.x;
    state.rel_y += y - state.y;
    state.x = x;
    state.y = y;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WarpMouseInWindow(
    window: *mut SDL_Window,
    x: libc::c_int,
    y: libc::c_int,
) {
    let mut state = lock_mouse_state();
    state.focus_window = window as usize;
    state.rel_x += x - state.x;
    state.rel_y += y - state.y;
    state.x = x;
    state.y = y;
}
