use std::sync::OnceLock;

use crate::abi::generated_types::SDL_bool;

struct ClipboardApi {
    get_clipboard_text: unsafe extern "C" fn() -> *mut libc::c_char,
    get_primary_selection_text: unsafe extern "C" fn() -> *mut libc::c_char,
    has_clipboard_text: unsafe extern "C" fn() -> SDL_bool,
    has_primary_selection_text: unsafe extern "C" fn() -> SDL_bool,
    set_clipboard_text: unsafe extern "C" fn(*const libc::c_char) -> libc::c_int,
    set_primary_selection_text: unsafe extern "C" fn(*const libc::c_char) -> libc::c_int,
}

fn api() -> &'static ClipboardApi {
    static API: OnceLock<ClipboardApi> = OnceLock::new();
    API.get_or_init(|| ClipboardApi {
        get_clipboard_text: crate::video::load_symbol(b"SDL_GetClipboardText\0"),
        get_primary_selection_text: crate::video::load_symbol(b"SDL_GetPrimarySelectionText\0"),
        has_clipboard_text: crate::video::load_symbol(b"SDL_HasClipboardText\0"),
        has_primary_selection_text: crate::video::load_symbol(b"SDL_HasPrimarySelectionText\0"),
        set_clipboard_text: crate::video::load_symbol(b"SDL_SetClipboardText\0"),
        set_primary_selection_text: crate::video::load_symbol(b"SDL_SetPrimarySelectionText\0"),
    })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetClipboardText() -> *mut libc::c_char {
    crate::video::clear_real_error();
    (api().get_clipboard_text)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPrimarySelectionText() -> *mut libc::c_char {
    crate::video::clear_real_error();
    (api().get_primary_selection_text)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasClipboardText() -> SDL_bool {
    crate::video::clear_real_error();
    (api().has_clipboard_text)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasPrimarySelectionText() -> SDL_bool {
    crate::video::clear_real_error();
    (api().has_primary_selection_text)()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetClipboardText(text: *const libc::c_char) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_clipboard_text)(text)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetPrimarySelectionText(text: *const libc::c_char) -> libc::c_int {
    crate::video::clear_real_error();
    (api().set_primary_selection_text)(text)
}
