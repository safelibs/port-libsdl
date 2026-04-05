use std::os::raw::c_int;
use std::ptr;

use crate::abi::generated_types::{SDL_Haptic, SDL_HapticEffect, SDL_Joystick, Uint32};
use crate::core::error::{invalid_param_error, set_error_message};

fn unavailable() -> c_int {
    set_error_message("Haptic devices are not available in this runtime")
}

#[no_mangle]
pub unsafe extern "C" fn SDL_NumHaptics() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticName(_device_index: c_int) -> *const libc::c_char {
    ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticOpen(_device_index: c_int) -> *mut SDL_Haptic {
    let _ = unavailable();
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticOpened(_device_index: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticIndex(haptic: *mut SDL_Haptic) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MouseIsHaptic() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticOpenFromMouse() -> *mut SDL_Haptic {
    let _ = unavailable();
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickIsHaptic(_joystick: *mut SDL_Joystick) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticOpenFromJoystick(
    _joystick: *mut SDL_Joystick,
) -> *mut SDL_Haptic {
    let _ = unavailable();
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticClose(_haptic: *mut SDL_Haptic) {}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticNumEffects(haptic: *mut SDL_Haptic) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticNumEffectsPlaying(haptic: *mut SDL_Haptic) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticQuery(haptic: *mut SDL_Haptic) -> libc::c_uint {
    if haptic.is_null() {
        let _ = invalid_param_error("haptic");
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticNumAxes(haptic: *mut SDL_Haptic) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticEffectSupported(
    haptic: *mut SDL_Haptic,
    _effect: *mut SDL_HapticEffect,
) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticNewEffect(
    haptic: *mut SDL_Haptic,
    _effect: *mut SDL_HapticEffect,
) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticUpdateEffect(
    haptic: *mut SDL_Haptic,
    _effect: c_int,
    _data: *mut SDL_HapticEffect,
) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticRunEffect(
    haptic: *mut SDL_Haptic,
    _effect: c_int,
    _iterations: Uint32,
) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticStopEffect(haptic: *mut SDL_Haptic, _effect: c_int) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticDestroyEffect(_haptic: *mut SDL_Haptic, _effect: c_int) {}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticGetEffectStatus(
    haptic: *mut SDL_Haptic,
    _effect: c_int,
) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticSetGain(haptic: *mut SDL_Haptic, _gain: c_int) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticSetAutocenter(
    haptic: *mut SDL_Haptic,
    _autocenter: c_int,
) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticPause(haptic: *mut SDL_Haptic) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticUnpause(haptic: *mut SDL_Haptic) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticStopAll(haptic: *mut SDL_Haptic) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticRumbleSupported(haptic: *mut SDL_Haptic) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticRumbleInit(haptic: *mut SDL_Haptic) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticRumblePlay(
    haptic: *mut SDL_Haptic,
    _strength: f32,
    _length: Uint32,
) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticRumbleStop(haptic: *mut SDL_Haptic) -> c_int {
    if haptic.is_null() {
        return invalid_param_error("haptic");
    }
    unavailable()
}
