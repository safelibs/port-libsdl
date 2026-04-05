use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_Keycode, SDL_Keymod, SDL_Scancode, SDL_Scancode_SDL_NUM_SCANCODES, SDL_Window, Uint8,
};

struct KeyboardLookupApi {
    get_key_from_name: unsafe extern "C" fn(*const libc::c_char) -> SDL_Keycode,
    get_key_from_scancode: unsafe extern "C" fn(SDL_Scancode) -> SDL_Keycode,
    get_key_name: unsafe extern "C" fn(SDL_Keycode) -> *const libc::c_char,
    get_scancode_from_key: unsafe extern "C" fn(SDL_Keycode) -> SDL_Scancode,
    get_scancode_from_name: unsafe extern "C" fn(*const libc::c_char) -> SDL_Scancode,
    get_scancode_name: unsafe extern "C" fn(SDL_Scancode) -> *const libc::c_char,
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

fn lookup_api() -> &'static KeyboardLookupApi {
    static API: OnceLock<KeyboardLookupApi> = OnceLock::new();
    API.get_or_init(|| KeyboardLookupApi {
        get_key_from_name: load_host_symbol(b"SDL_GetKeyFromName\0"),
        get_key_from_scancode: load_host_symbol(b"SDL_GetKeyFromScancode\0"),
        get_key_name: load_host_symbol(b"SDL_GetKeyName\0"),
        get_scancode_from_key: load_host_symbol(b"SDL_GetScancodeFromKey\0"),
        get_scancode_from_name: load_host_symbol(b"SDL_GetScancodeFromName\0"),
        get_scancode_name: load_host_symbol(b"SDL_GetScancodeName\0"),
    })
}

fn fallback_key_from_scancode(scancode: SDL_Scancode) -> SDL_Keycode {
    match scancode {
        4..=29 => (b'a' + (scancode - 4) as u8) as SDL_Keycode,
        30..=38 => (b'1' + (scancode - 30) as u8) as SDL_Keycode,
        39 => b'0' as SDL_Keycode,
        40 => b'\r' as SDL_Keycode,
        44 => b' ' as SDL_Keycode,
        _ => 0,
    }
}

fn fallback_scancode_from_name(name: *const libc::c_char) -> SDL_Scancode {
    if name.is_null() {
        return 0;
    }
    let Ok(text) = unsafe { std::ffi::CStr::from_ptr(name) }.to_str() else {
        return 0;
    };
    let normalized = text.trim();
    match normalized.to_ascii_uppercase().as_str() {
        "A" => 4,
        "B" => 5,
        "C" => 6,
        "D" => 7,
        "E" => 8,
        "F" => 9,
        "G" => 10,
        "H" => 11,
        "I" => 12,
        "J" => 13,
        "K" => 14,
        "L" => 15,
        "M" => 16,
        "N" => 17,
        "O" => 18,
        "P" => 19,
        "Q" => 20,
        "R" => 21,
        "S" => 22,
        "T" => 23,
        "U" => 24,
        "V" => 25,
        "W" => 26,
        "X" => 27,
        "Y" => 28,
        "Z" => 29,
        "1" => 30,
        "2" => 31,
        "3" => 32,
        "4" => 33,
        "5" => 34,
        "6" => 35,
        "7" => 36,
        "8" => 37,
        "9" => 38,
        "0" => 39,
        "RETURN" | "ENTER" => 40,
        "SPACE" => 44,
        _ => 0,
    }
}

fn fallback_scancode_from_key(key: SDL_Keycode) -> SDL_Scancode {
    let key = key as u32;
    if (b'a' as u32..=b'z' as u32).contains(&key) {
        key as SDL_Scancode - b'a' as SDL_Scancode + 4
    } else if (b'A' as u32..=b'Z' as u32).contains(&key) {
        key as SDL_Scancode - b'A' as SDL_Scancode + 4
    } else if (b'1' as u32..=b'9' as u32).contains(&key) {
        key as SDL_Scancode - b'1' as SDL_Scancode + 30
    } else if key == b'0' as u32 {
        39
    } else if key == b'\r' as u32 {
        40
    } else if key == b' ' as u32 {
        44
    } else {
        0
    }
}

struct KeyboardState {
    focus: usize,
    mod_state: SDL_Keymod,
    pressed: [Uint8; SDL_Scancode_SDL_NUM_SCANCODES as usize],
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self {
            focus: 0,
            mod_state: 0,
            pressed: [0; SDL_Scancode_SDL_NUM_SCANCODES as usize],
        }
    }
}

fn keyboard_state() -> &'static Mutex<KeyboardState> {
    static STATE: OnceLock<Mutex<KeyboardState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(KeyboardState::default()))
}

fn lock_keyboard_state() -> std::sync::MutexGuard<'static, KeyboardState> {
    match keyboard_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub(crate) fn set_keyboard_focus(window: *mut SDL_Window) {
    lock_keyboard_state().focus = window as usize;
}

pub(crate) fn clear_keyboard_focus(window: *mut SDL_Window) {
    let mut state = lock_keyboard_state();
    if state.focus == window as usize {
        state.focus = 0;
    }
}

pub(crate) fn reset_keyboard_state() {
    *lock_keyboard_state() = KeyboardState::default();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyboardFocus() -> *mut SDL_Window {
    lock_keyboard_state().focus as *mut SDL_Window
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyboardState(numkeys: *mut libc::c_int) -> *const Uint8 {
    let state = lock_keyboard_state();
    if !numkeys.is_null() {
        *numkeys = SDL_Scancode_SDL_NUM_SCANCODES as libc::c_int;
    }
    state.pressed.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetModState() -> SDL_Keymod {
    lock_keyboard_state().mod_state
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetModState(modstate: SDL_Keymod) {
    lock_keyboard_state().mod_state = modstate;
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyFromName(name: *const libc::c_char) -> SDL_Keycode {
    crate::video::clear_real_error();
    let key = (lookup_api().get_key_from_name)(name);
    if key != 0 {
        key
    } else {
        fallback_key_from_scancode(SDL_GetScancodeFromName(name))
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyFromScancode(scancode: SDL_Scancode) -> SDL_Keycode {
    crate::video::clear_real_error();
    let key = (lookup_api().get_key_from_scancode)(scancode);
    if key != 0 {
        key
    } else {
        fallback_key_from_scancode(scancode)
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyName(key: SDL_Keycode) -> *const libc::c_char {
    crate::video::clear_real_error();
    (lookup_api().get_key_name)(key)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetScancodeFromKey(key: SDL_Keycode) -> SDL_Scancode {
    crate::video::clear_real_error();
    let scancode = (lookup_api().get_scancode_from_key)(key);
    if scancode != 0 {
        scancode
    } else {
        fallback_scancode_from_key(key)
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetScancodeFromName(name: *const libc::c_char) -> SDL_Scancode {
    crate::video::clear_real_error();
    let scancode = (lookup_api().get_scancode_from_name)(name);
    if scancode != 0 {
        scancode
    } else {
        fallback_scancode_from_name(name)
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetScancodeName(scancode: SDL_Scancode) -> *const libc::c_char {
    crate::video::clear_real_error();
    (lookup_api().get_scancode_name)(scancode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ResetKeyboard() {
    reset_keyboard_state();
}
