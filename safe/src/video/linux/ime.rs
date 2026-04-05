use std::sync::OnceLock;

use crate::abi::generated_types::{SDL_Rect, SDL_Window, SDL_bool};

struct ImeApi {
    clear_composition: unsafe extern "C" fn(),
    has_screen_keyboard_support: unsafe extern "C" fn() -> SDL_bool,
    is_screen_keyboard_shown: unsafe extern "C" fn(*mut SDL_Window) -> SDL_bool,
    is_text_input_active: unsafe extern "C" fn() -> SDL_bool,
    set_text_input_rect: unsafe extern "C" fn(*const SDL_Rect),
    start_text_input: unsafe extern "C" fn(),
    stop_text_input: unsafe extern "C" fn(),
}

fn api() -> &'static ImeApi {
    static API: OnceLock<ImeApi> = OnceLock::new();
    API.get_or_init(|| ImeApi {
        clear_composition: crate::video::load_symbol(b"SDL_ClearComposition\0"),
        has_screen_keyboard_support: crate::video::load_symbol(b"SDL_HasScreenKeyboardSupport\0"),
        is_screen_keyboard_shown: crate::video::load_symbol(b"SDL_IsScreenKeyboardShown\0"),
        is_text_input_active: crate::video::load_symbol(b"SDL_IsTextInputActive\0"),
        set_text_input_rect: crate::video::load_symbol(b"SDL_SetTextInputRect\0"),
        start_text_input: crate::video::load_symbol(b"SDL_StartTextInput\0"),
        stop_text_input: crate::video::load_symbol(b"SDL_StopTextInput\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ClearComposition() {
    crate::video::clear_real_error();
    (api().clear_composition)();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasScreenKeyboardSupport() -> SDL_bool {
    crate::video::clear_real_error();
    (api().has_screen_keyboard_support)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IsScreenKeyboardShown(window: *mut SDL_Window) -> SDL_bool {
    crate::video::clear_real_error();
    (api().is_screen_keyboard_shown)(window)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IsTextInputActive() -> SDL_bool {
    crate::video::clear_real_error();
    (api().is_text_input_active)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetTextInputRect(rect: *const SDL_Rect) {
    crate::video::clear_real_error();
    (api().set_text_input_rect)(rect);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_StartTextInput() {
    crate::video::clear_real_error();
    (api().start_text_input)();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_StopTextInput() {
    crate::video::clear_real_error();
    (api().stop_text_input)();
}
