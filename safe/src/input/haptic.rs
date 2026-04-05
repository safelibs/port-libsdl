use std::os::raw::c_int;
use std::ptr;

use crate::abi::generated_types::{
    SDL_Haptic, SDL_HapticEffect, SDL_Joystick, Uint16, Uint32, SDL_HAPTIC_AUTOCENTER,
    SDL_HAPTIC_GAIN, SDL_HAPTIC_LEFTRIGHT, SDL_HAPTIC_PAUSE, SDL_HAPTIC_STATUS,
};
use crate::core::error::{invalid_param_error, set_error_message};

use super::{device_by_instance, joystick_instance_from_handle, lock_input_state, HapticHandle};

const HAPTIC_QUERY_MASK: u32 = SDL_HAPTIC_LEFTRIGHT
    | SDL_HAPTIC_GAIN
    | SDL_HAPTIC_AUTOCENTER
    | SDL_HAPTIC_STATUS
    | SDL_HAPTIC_PAUSE;

fn device_supports_haptics(device: &super::DeviceEntry) -> bool {
    device
        .callbacks
        .map(|callbacks| callbacks.rumble.is_some() || callbacks.send_effect.is_some())
        .unwrap_or(false)
}

fn haptic_instance_ids(
    state: &super::InputState,
) -> Vec<crate::abi::generated_types::SDL_JoystickID> {
    state
        .devices
        .iter()
        .filter(|device| device_supports_haptics(device))
        .map(|device| device.instance_id)
        .collect()
}

fn handle_mut<'a>(haptic: *mut SDL_Haptic) -> Option<&'a mut HapticHandle> {
    if haptic.is_null() {
        None
    } else {
        Some(unsafe { &mut *(haptic as *mut HapticHandle) })
    }
}

fn drive_rumble(
    instance_id: crate::abi::generated_types::SDL_JoystickID,
    low: Uint16,
    high: Uint16,
) -> c_int {
    let state = lock_input_state();
    let Some(device) = device_by_instance(&state, instance_id) else {
        return set_error_message("Haptic device is no longer attached");
    };
    let Some(callbacks) = device.callbacks else {
        return set_error_message("Haptic effects are not supported");
    };
    let Some(callback) = callbacks.rumble else {
        return set_error_message("Haptic rumble is not supported");
    };
    unsafe { callback(callbacks.userdata, low, high) }
}

fn stop_all_effects(handle: &mut HapticHandle) -> c_int {
    for playing in &mut handle.effect_playing {
        *playing = false;
    }
    drive_rumble(handle.instance_id, 0, 0)
}

fn scaled_rumble(value: Uint16, gain: c_int) -> Uint16 {
    let gain = gain.clamp(0, 100) as u32;
    (((value as u32) * gain) / 100) as Uint16
}

fn effect_slot_mut<'a>(
    handle: &'a mut HapticHandle,
    effect: c_int,
) -> Option<(&'a mut Option<SDL_HapticEffect>, &'a mut bool)> {
    let index = usize::try_from(effect).ok()?;
    Some((
        handle.effects.get_mut(index)?,
        handle.effect_playing.get_mut(index)?,
    ))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_NumHaptics() -> c_int {
    let mut state = lock_input_state();
    if !state.haptic_initialized {
        return 0;
    }
    super::refresh_hint_devices(&mut state);
    haptic_instance_ids(&state).len() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticName(device_index: c_int) -> *const libc::c_char {
    let mut state = lock_input_state();
    if !state.haptic_initialized {
        return ptr::null();
    }
    super::refresh_hint_devices(&mut state);
    let Some(instance_id) = usize::try_from(device_index)
        .ok()
        .and_then(|index| haptic_instance_ids(&state).get(index).copied())
    else {
        return ptr::null();
    };
    device_by_instance(&state, instance_id)
        .map(|device| device.name.as_ptr())
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticOpen(device_index: c_int) -> *mut SDL_Haptic {
    let mut state = lock_input_state();
    if !state.haptic_initialized {
        let _ = set_error_message("Haptic subsystem isn't initialized");
        return ptr::null_mut();
    }
    super::refresh_hint_devices(&mut state);
    let Some(instance_id) = usize::try_from(device_index)
        .ok()
        .and_then(|index| haptic_instance_ids(&state).get(index).copied())
    else {
        let _ = set_error_message("Invalid haptic device index");
        return ptr::null_mut();
    };
    let handle = Box::into_raw(Box::new(HapticHandle {
        instance_id,
        gain: 100,
        autocenter: 0,
        paused: false,
        rumble_initialized: false,
        effects: vec![None; 16],
        effect_playing: vec![false; 16],
    }));
    state.open_haptics.push((handle as usize, instance_id));
    handle.cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticOpened(device_index: c_int) -> c_int {
    let mut state = lock_input_state();
    if !state.haptic_initialized {
        return 0;
    }
    super::refresh_hint_devices(&mut state);
    let Some(instance_id) = usize::try_from(device_index)
        .ok()
        .and_then(|index| haptic_instance_ids(&state).get(index).copied())
    else {
        return 0;
    };
    i32::from(
        state
            .open_haptics
            .iter()
            .any(|(_, open_instance_id)| *open_instance_id == instance_id),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticIndex(haptic: *mut SDL_Haptic) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    let state = lock_input_state();
    haptic_instance_ids(&state)
        .iter()
        .position(|instance_id| *instance_id == handle.instance_id)
        .map(|index| index as c_int)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_MouseIsHaptic() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticOpenFromMouse() -> *mut SDL_Haptic {
    let _ = set_error_message("Mouse haptics are not supported");
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickIsHaptic(joystick: *mut SDL_Joystick) -> c_int {
    let state = lock_input_state();
    joystick_instance_from_handle(joystick)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
        .map(device_supports_haptics)
        .unwrap_or(false) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticOpenFromJoystick(
    joystick: *mut SDL_Joystick,
) -> *mut SDL_Haptic {
    let Some(instance_id) = joystick_instance_from_handle(joystick) else {
        let _ = invalid_param_error("joystick");
        return ptr::null_mut();
    };
    let mut state = lock_input_state();
    if !state.haptic_initialized {
        let _ = set_error_message("Haptic subsystem isn't initialized");
        return ptr::null_mut();
    }
    let Some(device) = device_by_instance(&state, instance_id) else {
        let _ = set_error_message("Joystick is no longer attached");
        return ptr::null_mut();
    };
    if !device_supports_haptics(device) {
        let _ = set_error_message("Joystick is not haptic");
        return ptr::null_mut();
    }
    let handle = Box::into_raw(Box::new(HapticHandle {
        instance_id,
        gain: 100,
        autocenter: 0,
        paused: false,
        rumble_initialized: false,
        effects: vec![None; 16],
        effect_playing: vec![false; 16],
    }));
    state.open_haptics.push((handle as usize, instance_id));
    handle.cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticClose(haptic: *mut SDL_Haptic) {
    if haptic.is_null() {
        return;
    }
    let mut state = lock_input_state();
    super::close_haptic_handle(&mut state, haptic.cast::<HapticHandle>());
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticNumEffects(haptic: *mut SDL_Haptic) -> c_int {
    handle_mut(haptic)
        .map(|handle| handle.effects.len() as c_int)
        .unwrap_or_else(|| invalid_param_error("haptic"))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticNumEffectsPlaying(haptic: *mut SDL_Haptic) -> c_int {
    handle_mut(haptic)
        .map(|handle| {
            handle
                .effect_playing
                .iter()
                .filter(|playing| **playing)
                .count() as c_int
        })
        .unwrap_or_else(|| invalid_param_error("haptic"))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticQuery(haptic: *mut SDL_Haptic) -> libc::c_uint {
    if handle_mut(haptic).is_none() {
        let _ = invalid_param_error("haptic");
        return 0;
    }
    HAPTIC_QUERY_MASK
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticNumAxes(haptic: *mut SDL_Haptic) -> c_int {
    handle_mut(haptic)
        .map(|_| 2)
        .unwrap_or_else(|| invalid_param_error("haptic"))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticEffectSupported(
    haptic: *mut SDL_Haptic,
    effect: *mut SDL_HapticEffect,
) -> c_int {
    if handle_mut(haptic).is_none() {
        return invalid_param_error("haptic");
    }
    if effect.is_null() {
        return invalid_param_error("effect");
    }
    i32::from((*effect).type_ as u32 == SDL_HAPTIC_LEFTRIGHT)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticNewEffect(
    haptic: *mut SDL_Haptic,
    effect: *mut SDL_HapticEffect,
) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    if effect.is_null() {
        return invalid_param_error("effect");
    }
    if (*effect).type_ as u32 != SDL_HAPTIC_LEFTRIGHT {
        return set_error_message("Only left/right haptic effects are supported");
    }
    let Some(index) = handle.effects.iter().position(|slot| slot.is_none()) else {
        return set_error_message("No free haptic effect slots");
    };
    handle.effects[index] = Some(*effect);
    handle.effect_playing[index] = false;
    index as c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticUpdateEffect(
    haptic: *mut SDL_Haptic,
    effect: c_int,
    data: *mut SDL_HapticEffect,
) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    if data.is_null() {
        return invalid_param_error("data");
    }
    if (*data).type_ as u32 != SDL_HAPTIC_LEFTRIGHT {
        return set_error_message("Only left/right haptic effects are supported");
    }
    let Some((slot, _)) = effect_slot_mut(handle, effect) else {
        return set_error_message("Invalid haptic effect");
    };
    *slot = Some(*data);
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticRunEffect(
    haptic: *mut SDL_Haptic,
    effect: c_int,
    _iterations: Uint32,
) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    let gain = handle.gain;
    let paused = handle.paused;
    let instance_id = handle.instance_id;
    let Some((slot, playing)) = effect_slot_mut(handle, effect) else {
        return set_error_message("Invalid haptic effect");
    };
    let Some(effect) = slot.as_ref() else {
        return set_error_message("Invalid haptic effect");
    };
    if effect.type_ as u32 != SDL_HAPTIC_LEFTRIGHT {
        return set_error_message("Only left/right haptic effects are supported");
    }
    *playing = true;
    if paused {
        return 0;
    }
    let low = scaled_rumble(effect.leftright.large_magnitude, gain);
    let high = scaled_rumble(effect.leftright.small_magnitude, gain);
    drive_rumble(instance_id, low, high)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticStopEffect(haptic: *mut SDL_Haptic, effect: c_int) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    let instance_id = handle.instance_id;
    let Some((slot, playing)) = effect_slot_mut(handle, effect) else {
        return set_error_message("Invalid haptic effect");
    };
    if slot.is_none() {
        return set_error_message("Invalid haptic effect");
    }
    *playing = false;
    drive_rumble(instance_id, 0, 0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticDestroyEffect(haptic: *mut SDL_Haptic, effect: c_int) {
    if let Some(handle) = handle_mut(haptic) {
        if let Some((slot, playing)) = effect_slot_mut(handle, effect) {
            *slot = None;
            *playing = false;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticGetEffectStatus(
    haptic: *mut SDL_Haptic,
    effect: c_int,
) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    effect_slot_mut(handle, effect)
        .map(|(_, playing)| i32::from(*playing))
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticSetGain(haptic: *mut SDL_Haptic, gain: c_int) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    handle.gain = gain.clamp(0, 100);
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticSetAutocenter(
    haptic: *mut SDL_Haptic,
    autocenter: c_int,
) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    handle.autocenter = autocenter.clamp(0, 100);
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticPause(haptic: *mut SDL_Haptic) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    handle.paused = true;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticUnpause(haptic: *mut SDL_Haptic) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    handle.paused = false;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticStopAll(haptic: *mut SDL_Haptic) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    stop_all_effects(handle)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticRumbleSupported(haptic: *mut SDL_Haptic) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    let state = lock_input_state();
    device_by_instance(&state, handle.instance_id)
        .map(|device| {
            i32::from(
                device
                    .callbacks
                    .and_then(|callbacks| callbacks.rumble)
                    .is_some(),
            )
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticRumbleInit(haptic: *mut SDL_Haptic) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    if SDL_HapticRumbleSupported(haptic) == 0 {
        return set_error_message("Haptic rumble is not supported");
    }
    handle.rumble_initialized = true;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticRumblePlay(
    haptic: *mut SDL_Haptic,
    strength: f32,
    _length: Uint32,
) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    if !handle.rumble_initialized {
        return set_error_message("Haptic rumble has not been initialized");
    }
    let magnitude = ((strength.clamp(0.0, 1.0) * 65_535.0) as u32).min(u16::MAX as u32) as Uint16;
    drive_rumble(handle.instance_id, magnitude, magnitude)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_HapticRumbleStop(haptic: *mut SDL_Haptic) -> c_int {
    let Some(handle) = handle_mut(haptic) else {
        return invalid_param_error("haptic");
    };
    drive_rumble(handle.instance_id, 0, 0)
}
