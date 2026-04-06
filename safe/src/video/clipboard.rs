use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::SDL_bool;

#[derive(Default)]
struct ClipboardState {
    clipboard_text: String,
    primary_selection_text: String,
}

fn clipboard_state() -> &'static Mutex<ClipboardState> {
    static STATE: OnceLock<Mutex<ClipboardState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(ClipboardState::default()))
}

fn lock_clipboard_state() -> std::sync::MutexGuard<'static, ClipboardState> {
    match clipboard_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetClipboardText() -> *mut libc::c_char {
    crate::core::memory::alloc_c_string(&lock_clipboard_state().clipboard_text)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GetPrimarySelectionText() -> *mut libc::c_char {
    crate::core::memory::alloc_c_string(&lock_clipboard_state().primary_selection_text)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasClipboardText() -> SDL_bool {
    (!lock_clipboard_state().clipboard_text.is_empty()) as SDL_bool
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasPrimarySelectionText() -> SDL_bool {
    (!lock_clipboard_state().primary_selection_text.is_empty()) as SDL_bool
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetClipboardText(text: *const libc::c_char) -> libc::c_int {
    if text.is_null() {
        lock_clipboard_state().clipboard_text.clear();
        return 0;
    }
    match std::ffi::CStr::from_ptr(text).to_str() {
        Ok(value) => {
            lock_clipboard_state().clipboard_text = value.to_string();
            0
        }
        Err(_) => crate::core::error::set_error_message("Clipboard text must be valid UTF-8"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetPrimarySelectionText(text: *const libc::c_char) -> libc::c_int {
    if text.is_null() {
        lock_clipboard_state().primary_selection_text.clear();
        return 0;
    }
    match std::ffi::CStr::from_ptr(text).to_str() {
        Ok(value) => {
            lock_clipboard_state().primary_selection_text = value.to_string();
            0
        }
        Err(_) => {
            crate::core::error::set_error_message("Primary selection text must be valid UTF-8")
        }
    }
}
