use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_Keycode, SDL_Keymod, SDL_Scancode, SDL_Scancode_SDL_NUM_SCANCODES, SDL_Window, Uint8,
    SDLK_SCANCODE_MASK,
};
use crate::core::error::invalid_param_error;

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

fn real_lookup_api() -> Option<&'static KeyboardLookupApi> {
    if crate::video::real_sdl_is_available() {
        Some(lookup_api())
    } else {
        None
    }
}

fn fallback_named_scancode(name: &str) -> SDL_Scancode {
    match name {
        "RETURN" | "ENTER" => 40,
        "ESCAPE" => 41,
        "BACKSPACE" => 42,
        "TAB" => 43,
        "SPACE" => 44,
        "DELETE" => 76,
        "END" => 77,
        "DOWN" => 81,
        "KEYPAD ENTER" => 88,
        "CUT" => 123,
        "FIND" => 126,
        "KEYPAD MEMSTORE" => 208,
        "AUDIOSTOP" => 260,
        "BRIGHTNESSUP" => 276,
        "SLEEP" => 282,
        _ => match name
            .strip_prefix('F')
            .and_then(|suffix| suffix.parse::<i32>().ok())
        {
            Some(value @ 1..=12) => (value + 57) as SDL_Scancode,
            Some(value @ 13..=24) => (value + 91) as SDL_Scancode,
            _ => 0,
        },
    }
}

fn fallback_key_from_scancode(scancode: SDL_Scancode) -> SDL_Keycode {
    if scancode < 0 || scancode >= SDL_Scancode_SDL_NUM_SCANCODES {
        let _ = invalid_param_error("scancode");
        return 0;
    }
    match scancode {
        4..=29 => (b'a' + (scancode - 4) as u8) as SDL_Keycode,
        30..=38 => (b'1' + (scancode - 30) as u8) as SDL_Keycode,
        39 => b'0' as SDL_Keycode,
        40 => b'\r' as SDL_Keycode,
        41 => 27,
        42 => 8,
        43 => b'\t' as SDL_Keycode,
        44 => b' ' as SDL_Keycode,
        76 => 127,
        58..=69 | 77 | 81 | 88 | 123 | 126 | 208 | 260 | 276 | 282 => {
            (SDLK_SCANCODE_MASK | scancode as u32) as SDL_Keycode
        }
        _ => 0,
    }
}

fn fallback_scancode_from_name(name: *const libc::c_char) -> SDL_Scancode {
    if name.is_null() {
        let _ = invalid_param_error("name");
        return 0;
    }
    let Ok(text) = unsafe { std::ffi::CStr::from_ptr(name) }.to_str() else {
        let _ = invalid_param_error("name");
        return 0;
    };
    if text.is_empty() {
        let _ = invalid_param_error("name");
        return 0;
    }
    let normalized = text.trim().to_ascii_uppercase();
    let scancode = match normalized.as_str() {
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
        _ => fallback_named_scancode(&normalized),
    };
    if scancode == 0 {
        let _ = invalid_param_error("name");
    }
    scancode
}

fn fallback_scancode_from_key(key: SDL_Keycode) -> SDL_Scancode {
    if (key as u32 & SDLK_SCANCODE_MASK) != 0 {
        let scancode = (key as u32 & !SDLK_SCANCODE_MASK) as SDL_Scancode;
        if scancode > 0 && scancode < SDL_Scancode_SDL_NUM_SCANCODES {
            return scancode;
        }
        return 0;
    }
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
    } else if key == 27 {
        41
    } else if key == 8 {
        42
    } else if key == b'\t' as u32 {
        43
    } else if key == b' ' as u32 {
        44
    } else if key == 127 {
        76
    } else {
        0
    }
}

fn fallback_name_bytes_from_scancode(scancode: SDL_Scancode) -> &'static [u8] {
    if scancode < 0 || scancode >= SDL_Scancode_SDL_NUM_SCANCODES {
        let _ = invalid_param_error("scancode");
        return b"\0";
    }
    match scancode {
        4 => b"A\0",
        5 => b"B\0",
        6 => b"C\0",
        7 => b"D\0",
        8 => b"E\0",
        9 => b"F\0",
        10 => b"G\0",
        11 => b"H\0",
        12 => b"I\0",
        13 => b"J\0",
        14 => b"K\0",
        15 => b"L\0",
        16 => b"M\0",
        17 => b"N\0",
        18 => b"O\0",
        19 => b"P\0",
        20 => b"Q\0",
        21 => b"R\0",
        22 => b"S\0",
        23 => b"T\0",
        24 => b"U\0",
        25 => b"V\0",
        26 => b"W\0",
        27 => b"X\0",
        28 => b"Y\0",
        29 => b"Z\0",
        30 => b"1\0",
        31 => b"2\0",
        32 => b"3\0",
        33 => b"4\0",
        34 => b"5\0",
        35 => b"6\0",
        36 => b"7\0",
        37 => b"8\0",
        38 => b"9\0",
        39 => b"0\0",
        40 => b"Return\0",
        41 => b"Escape\0",
        42 => b"Backspace\0",
        43 => b"Tab\0",
        44 => b"Space\0",
        58 => b"F1\0",
        77 => b"End\0",
        81 => b"Down\0",
        88 => b"Keypad Enter\0",
        123 => b"Cut\0",
        126 => b"Find\0",
        208 => b"Keypad MemStore\0",
        260 => b"AudioStop\0",
        276 => b"BrightnessUp\0",
        282 => b"Sleep\0",
        _ => b"\0",
    }
}

fn fallback_name_bytes_from_key(key: SDL_Keycode) -> &'static [u8] {
    match key {
        value if value == b'\r' as SDL_Keycode => b"Return\0",
        value if value == 27 => b"Escape\0",
        value if value == 8 => b"Backspace\0",
        value if value == b'\t' as SDL_Keycode => b"Tab\0",
        value if value == b' ' as SDL_Keycode => b"Space\0",
        value if value == 127 => b"Delete\0",
        value if (b'a' as SDL_Keycode..=b'z' as SDL_Keycode).contains(&value) => {
            match value as u8 {
                b'a' => b"A\0",
                b'b' => b"B\0",
                b'c' => b"C\0",
                b'd' => b"D\0",
                b'e' => b"E\0",
                b'f' => b"F\0",
                b'g' => b"G\0",
                b'h' => b"H\0",
                b'i' => b"I\0",
                b'j' => b"J\0",
                b'k' => b"K\0",
                b'l' => b"L\0",
                b'm' => b"M\0",
                b'n' => b"N\0",
                b'o' => b"O\0",
                b'p' => b"P\0",
                b'q' => b"Q\0",
                b'r' => b"R\0",
                b's' => b"S\0",
                b't' => b"T\0",
                b'u' => b"U\0",
                b'v' => b"V\0",
                b'w' => b"W\0",
                b'x' => b"X\0",
                b'y' => b"Y\0",
                b'z' => b"Z\0",
                _ => b"\0",
            }
        }
        value if (b'0' as SDL_Keycode..=b'9' as SDL_Keycode).contains(&value) => {
            match value as u8 {
                b'0' => b"0\0",
                b'1' => b"1\0",
                b'2' => b"2\0",
                b'3' => b"3\0",
                b'4' => b"4\0",
                b'5' => b"5\0",
                b'6' => b"6\0",
                b'7' => b"7\0",
                b'8' => b"8\0",
                b'9' => b"9\0",
                _ => b"\0",
            }
        }
        _ => fallback_name_bytes_from_scancode(fallback_scancode_from_key(key)),
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
    if let Some(api) = real_lookup_api() {
        crate::video::clear_real_error();
        let key = (api.get_key_from_name)(name);
        if key != 0 {
            return key;
        }
    }
    if name.is_null() {
        return 0;
    }
    let Ok(text) = std::ffi::CStr::from_ptr(name).to_str() else {
        return 0;
    };
    if text.len() == 1 {
        let byte = text.as_bytes()[0];
        return if byte.is_ascii_uppercase() {
            byte.to_ascii_lowercase() as SDL_Keycode
        } else {
            byte as SDL_Keycode
        };
    }
    fallback_key_from_scancode(SDL_GetScancodeFromName(name))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyFromScancode(scancode: SDL_Scancode) -> SDL_Keycode {
    if let Some(api) = real_lookup_api() {
        crate::video::clear_real_error();
        let key = (api.get_key_from_scancode)(scancode);
        if key != 0 {
            return key;
        }
    }
    fallback_key_from_scancode(scancode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyName(key: SDL_Keycode) -> *const libc::c_char {
    if let Some(api) = real_lookup_api() {
        crate::video::clear_real_error();
        return (api.get_key_name)(key);
    }
    if (key as u32 & SDLK_SCANCODE_MASK) != 0 {
        return SDL_GetScancodeName((key as u32 & !SDLK_SCANCODE_MASK) as SDL_Scancode);
    }
    fallback_name_bytes_from_key(key).as_ptr().cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetScancodeFromKey(key: SDL_Keycode) -> SDL_Scancode {
    if let Some(api) = real_lookup_api() {
        crate::video::clear_real_error();
        let scancode = (api.get_scancode_from_key)(key);
        if scancode != 0 {
            return scancode;
        }
    }
    fallback_scancode_from_key(key)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetScancodeFromName(name: *const libc::c_char) -> SDL_Scancode {
    if let Some(api) = real_lookup_api() {
        crate::video::clear_real_error();
        let scancode = (api.get_scancode_from_name)(name);
        if scancode != 0 {
            return scancode;
        }
    }
    fallback_scancode_from_name(name)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetScancodeName(scancode: SDL_Scancode) -> *const libc::c_char {
    if let Some(api) = real_lookup_api() {
        crate::video::clear_real_error();
        return (api.get_scancode_name)(scancode);
    }
    fallback_name_bytes_from_scancode(scancode).as_ptr().cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ResetKeyboard() {
    reset_keyboard_state();
}
