use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{SDL_Rect, SDL_Window, SDL_bool};

#[derive(Default)]
struct TextInputState {
    active: bool,
    rect: Option<SDL_Rect>,
}

fn text_input_state() -> &'static Mutex<TextInputState> {
    static STATE: OnceLock<Mutex<TextInputState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(TextInputState::default()))
}

fn lock_text_input_state() -> std::sync::MutexGuard<'static, TextInputState> {
    match text_input_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub(crate) fn reset_text_input_state() {
    *lock_text_input_state() = TextInputState::default();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ClearComposition() {}

#[no_mangle]
pub unsafe extern "C" fn SDL_HasScreenKeyboardSupport() -> SDL_bool {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IsScreenKeyboardShown(_window: *mut SDL_Window) -> SDL_bool {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IsTextInputActive() -> SDL_bool {
    lock_text_input_state().active as SDL_bool
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SetTextInputRect(rect: *const SDL_Rect) {
    let mut state = lock_text_input_state();
    state.rect = if rect.is_null() { None } else { Some(*rect) };
}

#[no_mangle]
pub unsafe extern "C" fn SDL_StartTextInput() {
    lock_text_input_state().active = true;
}

#[no_mangle]
pub unsafe extern "C" fn SDL_StopTextInput() {
    lock_text_input_state().active = false;
}
