use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{SDL_GestureID, SDL_RWops, SDL_TouchID};

const GESTURE_MAGIC: &[u8] = b"SAFE_SDL_GESTURE_V1";

#[derive(Default)]
struct GestureState {
    recording_touch: SDL_TouchID,
}

fn gesture_state() -> &'static Mutex<GestureState> {
    static STATE: OnceLock<Mutex<GestureState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(GestureState::default()))
}

fn lock_gesture_state() -> std::sync::MutexGuard<'static, GestureState> {
    match gesture_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn write_templates(dst: *mut SDL_RWops) -> libc::c_int {
    if dst.is_null() {
        return crate::core::error::invalid_param_error("dst");
    }
    let written = unsafe {
        crate::core::rwops::SDL_RWwrite(dst, GESTURE_MAGIC.as_ptr().cast(), 1, GESTURE_MAGIC.len())
    };
    if written == GESTURE_MAGIC.len() {
        0
    } else {
        crate::core::error::set_error_message("Failed to save gesture templates")
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LoadDollarTemplates(
    _touchId: SDL_TouchID,
    src: *mut SDL_RWops,
) -> libc::c_int {
    if src.is_null() {
        return crate::core::error::invalid_param_error("src");
    }
    let mut buffer = [0u8; GESTURE_MAGIC.len()];
    let read = crate::core::rwops::SDL_RWread(src, buffer.as_mut_ptr().cast(), 1, buffer.len());
    if read == 0 {
        return 0;
    }
    if read != buffer.len() || buffer != GESTURE_MAGIC {
        return crate::core::error::set_error_message("Gesture template stream is invalid");
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RecordGesture(touchId: SDL_TouchID) -> libc::c_int {
    lock_gesture_state().recording_touch = touchId;
    1
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SaveAllDollarTemplates(dst: *mut SDL_RWops) -> libc::c_int {
    let _ = lock_gesture_state().recording_touch;
    write_templates(dst)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_SaveDollarTemplate(
    _gestureId: SDL_GestureID,
    dst: *mut SDL_RWops,
) -> libc::c_int {
    write_templates(dst)
}
