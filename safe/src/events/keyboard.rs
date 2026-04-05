use std::sync::OnceLock;

use crate::abi::generated_types::{SDL_Keycode, SDL_Keymod, SDL_Scancode, SDL_Window, Uint8};

struct KeyboardApi {
    get_keyboard_focus: unsafe extern "C" fn() -> *mut SDL_Window,
    get_keyboard_state: unsafe extern "C" fn(*mut libc::c_int) -> *const Uint8,
    get_mod_state: unsafe extern "C" fn() -> SDL_Keymod,
    set_mod_state: unsafe extern "C" fn(SDL_Keymod),
    get_key_from_name: unsafe extern "C" fn(*const libc::c_char) -> SDL_Keycode,
    get_key_from_scancode: unsafe extern "C" fn(SDL_Scancode) -> SDL_Keycode,
    get_key_name: unsafe extern "C" fn(SDL_Keycode) -> *const libc::c_char,
    get_scancode_from_key: unsafe extern "C" fn(SDL_Keycode) -> SDL_Scancode,
    get_scancode_from_name: unsafe extern "C" fn(*const libc::c_char) -> SDL_Scancode,
    get_scancode_name: unsafe extern "C" fn(SDL_Scancode) -> *const libc::c_char,
    reset_keyboard: unsafe extern "C" fn(),
}

fn api() -> &'static KeyboardApi {
    static API: OnceLock<KeyboardApi> = OnceLock::new();
    API.get_or_init(|| KeyboardApi {
        get_keyboard_focus: crate::video::load_symbol(b"SDL_GetKeyboardFocus\0"),
        get_keyboard_state: crate::video::load_symbol(b"SDL_GetKeyboardState\0"),
        get_mod_state: crate::video::load_symbol(b"SDL_GetModState\0"),
        set_mod_state: crate::video::load_symbol(b"SDL_SetModState\0"),
        get_key_from_name: crate::video::load_symbol(b"SDL_GetKeyFromName\0"),
        get_key_from_scancode: crate::video::load_symbol(b"SDL_GetKeyFromScancode\0"),
        get_key_name: crate::video::load_symbol(b"SDL_GetKeyName\0"),
        get_scancode_from_key: crate::video::load_symbol(b"SDL_GetScancodeFromKey\0"),
        get_scancode_from_name: crate::video::load_symbol(b"SDL_GetScancodeFromName\0"),
        get_scancode_name: crate::video::load_symbol(b"SDL_GetScancodeName\0"),
        reset_keyboard: crate::video::load_symbol(b"SDL_ResetKeyboard\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyboardFocus() -> *mut SDL_Window {
    crate::video::clear_real_error();
    (api().get_keyboard_focus)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyboardState(numkeys: *mut libc::c_int) -> *const Uint8 {
    crate::video::clear_real_error();
    (api().get_keyboard_state)(numkeys)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetModState() -> SDL_Keymod {
    crate::video::clear_real_error();
    (api().get_mod_state)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetModState(modstate: SDL_Keymod) {
    crate::video::clear_real_error();
    (api().set_mod_state)(modstate);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyFromName(name: *const libc::c_char) -> SDL_Keycode {
    crate::video::clear_real_error();
    (api().get_key_from_name)(name)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyFromScancode(scancode: SDL_Scancode) -> SDL_Keycode {
    crate::video::clear_real_error();
    (api().get_key_from_scancode)(scancode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetKeyName(key: SDL_Keycode) -> *const libc::c_char {
    crate::video::clear_real_error();
    (api().get_key_name)(key)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetScancodeFromKey(key: SDL_Keycode) -> SDL_Scancode {
    crate::video::clear_real_error();
    (api().get_scancode_from_key)(key)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetScancodeFromName(name: *const libc::c_char) -> SDL_Scancode {
    crate::video::clear_real_error();
    (api().get_scancode_from_name)(name)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetScancodeName(scancode: SDL_Scancode) -> *const libc::c_char {
    crate::video::clear_real_error();
    (api().get_scancode_name)(scancode)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ResetKeyboard() {
    crate::video::clear_real_error();
    (api().reset_keyboard)();
}
