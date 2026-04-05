use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::abi::generated_types::{
    SDL_Joystick, SDL_JoystickGUID, SDL_JoystickID,
    SDL_JoystickPowerLevel_SDL_JOYSTICK_POWER_UNKNOWN, SDL_JoystickType,
    SDL_JoystickType_SDL_JOYSTICK_TYPE_UNKNOWN, SDL_VirtualJoystickDesc, SDL_bool, Sint16, Uint16,
    Uint32, Uint8, SDL_ENABLE, SDL_HAT_CENTERED, SDL_PRESSED, SDL_QUERY,
    SDL_VIRTUAL_JOYSTICK_DESC_VERSION,
};
use crate::core::error::{invalid_param_error, set_error_message};
use crate::core::system::bool_to_sdl;

use super::{
    create_joystick_guid, cstr_ptr, device_by_instance, device_by_instance_mut,
    device_index_to_instance, joystick_instance_from_handle, lock_input_state, make_cstring,
    DeviceEntry, DeviceState, JoystickHandle, VirtualCallbacks, SDL_HARDWARE_BUS_VIRTUAL,
};

fn invalid_device_index() -> c_int {
    set_error_message("Invalid joystick device index")
}

fn invalid_joystick_handle() -> c_int {
    invalid_param_error("joystick")
}

fn device_index_to_device_index(
    state: &mut super::InputState,
    device_index: c_int,
) -> Option<usize> {
    if !state.joystick_initialized {
        return None;
    }
    super::refresh_hint_devices(state);
    usize::try_from(device_index)
        .ok()
        .filter(|&index| index < state.devices.len())
}

fn device_path_string(device: &DeviceEntry) -> *const c_char {
    cstr_ptr(device.path.as_ref())
}

fn handle_device<'a>(
    state: &'a super::InputState,
    joystick: *mut SDL_Joystick,
) -> Option<&'a DeviceEntry> {
    let instance_id = joystick_instance_from_handle(joystick)?;
    device_by_instance(state, instance_id)
}

fn handle_device_mut<'a>(
    state: &'a mut super::InputState,
    joystick: *mut SDL_Joystick,
) -> Option<&'a mut DeviceEntry> {
    let instance_id = joystick_instance_from_handle(joystick)?;
    device_by_instance_mut(state, instance_id)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LockJoysticks() {}

#[no_mangle]
pub unsafe extern "C" fn SDL_UnlockJoysticks() {}

#[no_mangle]
pub unsafe extern "C" fn SDL_NumJoysticks() -> c_int {
    let mut state = lock_input_state();
    if !state.joystick_initialized {
        return 0;
    }
    super::refresh_hint_devices(&mut state);
    state.devices.len() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickNameForIndex(device_index: c_int) -> *const c_char {
    let mut state = lock_input_state();
    let Some(index) = device_index_to_device_index(&mut state, device_index) else {
        let _ = invalid_device_index();
        return ptr::null();
    };
    state.devices[index].name.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickPathForIndex(device_index: c_int) -> *const c_char {
    let mut state = lock_input_state();
    let Some(index) = device_index_to_device_index(&mut state, device_index) else {
        let _ = invalid_device_index();
        return ptr::null();
    };
    device_path_string(&state.devices[index])
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetDevicePlayerIndex(device_index: c_int) -> c_int {
    let mut state = lock_input_state();
    let Some(index) = device_index_to_device_index(&mut state, device_index) else {
        return -1;
    };
    state.devices[index].player_index
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetDeviceGUID(device_index: c_int) -> SDL_JoystickGUID {
    let mut state = lock_input_state();
    let Some(index) = device_index_to_device_index(&mut state, device_index) else {
        return SDL_JoystickGUID { data: [0; 16] };
    };
    state.devices[index].guid
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetDeviceVendor(device_index: c_int) -> Uint16 {
    let mut state = lock_input_state();
    let Some(index) = device_index_to_device_index(&mut state, device_index) else {
        return 0;
    };
    state.devices[index].vendor
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetDeviceProduct(device_index: c_int) -> Uint16 {
    let mut state = lock_input_state();
    let Some(index) = device_index_to_device_index(&mut state, device_index) else {
        return 0;
    };
    state.devices[index].product
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetDeviceProductVersion(device_index: c_int) -> Uint16 {
    let mut state = lock_input_state();
    let Some(index) = device_index_to_device_index(&mut state, device_index) else {
        return 0;
    };
    state.devices[index].product_version
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetDeviceType(device_index: c_int) -> SDL_JoystickType {
    let mut state = lock_input_state();
    let Some(index) = device_index_to_device_index(&mut state, device_index) else {
        return SDL_JoystickType_SDL_JOYSTICK_TYPE_UNKNOWN;
    };
    state.devices[index].joystick_type
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetDeviceInstanceID(device_index: c_int) -> SDL_JoystickID {
    let mut state = lock_input_state();
    device_index_to_instance(&mut state, device_index).unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickOpen(device_index: c_int) -> *mut SDL_Joystick {
    let mut state = lock_input_state();
    if !state.joystick_initialized {
        let _ = set_error_message("Joystick subsystem isn't initialized");
        return ptr::null_mut();
    }

    let Some(instance_id) = device_index_to_instance(&mut state, device_index) else {
        let _ = invalid_device_index();
        return ptr::null_mut();
    };

    let handle = Box::into_raw(Box::new(JoystickHandle { instance_id }));
    state.open_joysticks.push((handle as usize, instance_id));
    handle.cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickFromInstanceID(
    instance_id: SDL_JoystickID,
) -> *mut SDL_Joystick {
    let state = lock_input_state();
    state
        .open_joysticks
        .iter()
        .find(|(_, open_id)| *open_id == instance_id)
        .map(|(ptr, _)| *ptr as *mut SDL_Joystick)
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickFromPlayerIndex(player_index: c_int) -> *mut SDL_Joystick {
    let state = lock_input_state();
    state
        .open_joysticks
        .iter()
        .find_map(|(ptr, instance_id)| {
            let device = device_by_instance(&state, *instance_id)?;
            (device.player_index == player_index).then_some(*ptr as *mut SDL_Joystick)
        })
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickAttachVirtual(
    type_: SDL_JoystickType,
    naxes: c_int,
    nbuttons: c_int,
    nhats: c_int,
) -> c_int {
    let desc = SDL_VirtualJoystickDesc {
        version: SDL_VIRTUAL_JOYSTICK_DESC_VERSION as Uint16,
        type_: type_ as Uint16,
        naxes: naxes.max(0) as Uint16,
        nbuttons: nbuttons.max(0) as Uint16,
        nhats: nhats.max(0) as Uint16,
        vendor_id: 0,
        product_id: 0,
        padding: 0,
        button_mask: 0,
        axis_mask: 0,
        name: ptr::null(),
        userdata: ptr::null_mut(),
        Update: None,
        SetPlayerIndex: None,
        Rumble: None,
        RumbleTriggers: None,
        SetLED: None,
        SendEffect: None,
    };
    SDL_JoystickAttachVirtualEx(&desc)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickAttachVirtualEx(
    desc: *const SDL_VirtualJoystickDesc,
) -> c_int {
    if desc.is_null() {
        return crate::core::error::invalid_param_error("desc");
    }
    let desc = &*desc;
    if desc.version != SDL_VIRTUAL_JOYSTICK_DESC_VERSION as Uint16 {
        return set_error_message("Unsupported virtual joystick description version");
    }

    let mut state = lock_input_state();
    super::refresh_hint_devices(&mut state);
    let device_index = state.devices.len() as c_int;
    let instance_id = state.next_instance_id;
    state.next_instance_id += 1;

    let name = if desc.name.is_null() {
        make_cstring("Virtual Joystick")
    } else {
        CStr::from_ptr(desc.name).to_owned()
    };
    let name_str = name.to_string_lossy().into_owned();
    let vendor = desc.vendor_id;
    let product = desc.product_id;
    let callbacks = VirtualCallbacks {
        update: desc.Update,
        set_player_index: desc.SetPlayerIndex,
        rumble: desc.Rumble,
        rumble_triggers: desc.RumbleTriggers,
        set_led: desc.SetLED,
        send_effect: desc.SendEffect,
        userdata: desc.userdata,
    };

    state.devices.push(DeviceEntry {
        instance_id,
        name,
        path: None,
        guid: create_joystick_guid(
            SDL_HARDWARE_BUS_VIRTUAL,
            vendor,
            product,
            0,
            None,
            &name_str,
            0,
            0,
        ),
        vendor,
        product,
        product_version: 0,
        firmware_version: 0,
        serial: None,
        joystick_type: desc.type_ as SDL_JoystickType,
        player_index: -1,
        is_virtual: true,
        callbacks: Some(callbacks),
        state: DeviceState::new(
            desc.naxes as usize,
            desc.nbuttons as usize,
            desc.nhats as usize,
        ),
        touchpads: Vec::new(),
        sensors: Vec::new(),
        evdev: None,
        hint_path: None,
    });
    if let Some(device) = state.devices.last_mut() {
        super::refresh_device_features(device);
    }

    device_index
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickDetachVirtual(device_index: c_int) -> c_int {
    let mut state = lock_input_state();
    let Some(index) = device_index_to_device_index(&mut state, device_index) else {
        return invalid_device_index();
    };
    if !state.devices[index].is_virtual {
        return set_error_message("Joystick isn't virtual");
    }
    state.devices.remove(index);
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickIsVirtual(device_index: c_int) -> SDL_bool {
    let mut state = lock_input_state();
    let Some(index) = device_index_to_device_index(&mut state, device_index) else {
        return crate::abi::generated_types::SDL_bool_SDL_FALSE;
    };
    bool_to_sdl(state.devices[index].is_virtual)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickSetVirtualAxis(
    joystick: *mut SDL_Joystick,
    axis: c_int,
    value: Sint16,
) -> c_int {
    let mut state = lock_input_state();
    let Some(device) = handle_device_mut(&mut state, joystick) else {
        return invalid_joystick_handle();
    };
    if !device.is_virtual {
        return set_error_message("Joystick isn't virtual");
    }
    let Some(slot) = usize::try_from(axis)
        .ok()
        .and_then(|index| device.state.pending_axes.get_mut(index))
    else {
        return crate::core::error::invalid_param_error("axis");
    };
    *slot = value;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickSetVirtualButton(
    joystick: *mut SDL_Joystick,
    button: c_int,
    value: Uint8,
) -> c_int {
    let mut state = lock_input_state();
    let Some(device) = handle_device_mut(&mut state, joystick) else {
        return invalid_joystick_handle();
    };
    if !device.is_virtual {
        return set_error_message("Joystick isn't virtual");
    }
    let Some(slot) = usize::try_from(button)
        .ok()
        .and_then(|index| device.state.pending_buttons.get_mut(index))
    else {
        return crate::core::error::invalid_param_error("button");
    };
    *slot = if value == SDL_PRESSED as Uint8 {
        SDL_PRESSED as Uint8
    } else {
        0
    };
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickSetVirtualHat(
    joystick: *mut SDL_Joystick,
    hat: c_int,
    value: Uint8,
) -> c_int {
    let mut state = lock_input_state();
    let Some(device) = handle_device_mut(&mut state, joystick) else {
        return invalid_joystick_handle();
    };
    if !device.is_virtual {
        return set_error_message("Joystick isn't virtual");
    }
    let Some(slot) = usize::try_from(hat)
        .ok()
        .and_then(|index| device.state.pending_hats.get_mut(index))
    else {
        return crate::core::error::invalid_param_error("hat");
    };
    *slot = value;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickName(joystick: *mut SDL_Joystick) -> *const c_char {
    let state = lock_input_state();
    let Some(device) = handle_device(&state, joystick) else {
        let _ = invalid_joystick_handle();
        return ptr::null();
    };
    device.name.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickPath(joystick: *mut SDL_Joystick) -> *const c_char {
    let state = lock_input_state();
    let Some(device) = handle_device(&state, joystick) else {
        let _ = invalid_joystick_handle();
        return ptr::null();
    };
    device_path_string(device)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetPlayerIndex(joystick: *mut SDL_Joystick) -> c_int {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| device.player_index)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickSetPlayerIndex(
    joystick: *mut SDL_Joystick,
    player_index: c_int,
) {
    let mut state = lock_input_state();
    if let Some(device) = handle_device_mut(&mut state, joystick) {
        device.player_index = player_index;
        if let Some(callbacks) = device.callbacks {
            if let Some(callback) = callbacks.set_player_index {
                callback(callbacks.userdata, player_index);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetGUID(joystick: *mut SDL_Joystick) -> SDL_JoystickGUID {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| device.guid)
        .unwrap_or(SDL_JoystickGUID { data: [0; 16] })
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetVendor(joystick: *mut SDL_Joystick) -> Uint16 {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| device.vendor)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetProduct(joystick: *mut SDL_Joystick) -> Uint16 {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| device.product)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetProductVersion(joystick: *mut SDL_Joystick) -> Uint16 {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| device.product_version)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetFirmwareVersion(joystick: *mut SDL_Joystick) -> Uint16 {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| device.firmware_version)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetSerial(joystick: *mut SDL_Joystick) -> *const c_char {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| cstr_ptr(device.serial.as_ref()))
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetType(joystick: *mut SDL_Joystick) -> SDL_JoystickType {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| device.joystick_type)
        .unwrap_or(SDL_JoystickType_SDL_JOYSTICK_TYPE_UNKNOWN)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetAttached(joystick: *mut SDL_Joystick) -> SDL_bool {
    let state = lock_input_state();
    bool_to_sdl(handle_device(&state, joystick).is_some())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickInstanceID(joystick: *mut SDL_Joystick) -> SDL_JoystickID {
    let Some(instance_id) = joystick_instance_from_handle(joystick) else {
        let _ = invalid_joystick_handle();
        return -1;
    };
    instance_id
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickNumAxes(joystick: *mut SDL_Joystick) -> c_int {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| device.state.axes.len() as c_int)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickNumBalls(joystick: *mut SDL_Joystick) -> c_int {
    if joystick.is_null() {
        let _ = invalid_joystick_handle();
        -1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickNumHats(joystick: *mut SDL_Joystick) -> c_int {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| device.state.hats.len() as c_int)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickNumButtons(joystick: *mut SDL_Joystick) -> c_int {
    let state = lock_input_state();
    handle_device(&state, joystick)
        .map(|device| device.state.buttons.len() as c_int)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickUpdate() {
    let mut state = lock_input_state();
    if !state.joystick_initialized {
        return;
    }
    super::refresh_hint_devices(&mut state);
    for device in &mut state.devices {
        if let Some(evdev) = device.evdev.as_mut() {
            let _ = super::linux::evdev::poll_device(evdev, &mut device.state);
        }
        if let Some(callbacks) = device.callbacks {
            if let Some(callback) = callbacks.update {
                callback(callbacks.userdata);
            }
        }
        device.state.apply_pending();
        super::refresh_device_features(device);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickEventState(state_value: c_int) -> c_int {
    let mut state = lock_input_state();
    if state_value == SDL_QUERY as c_int {
        return state.joystick_event_state;
    }
    if state_value == 0 || state_value == SDL_ENABLE as c_int {
        state.joystick_event_state = state_value;
        return state.joystick_event_state;
    }
    state_value
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetAxis(joystick: *mut SDL_Joystick, axis: c_int) -> Sint16 {
    let state = lock_input_state();
    let Some(device) = handle_device(&state, joystick) else {
        let _ = invalid_joystick_handle();
        return 0;
    };
    usize::try_from(axis)
        .ok()
        .and_then(|index| device.state.axes.get(index).copied())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetAxisInitialState(
    joystick: *mut SDL_Joystick,
    axis: c_int,
    state_out: *mut Sint16,
) -> SDL_bool {
    if state_out.is_null() {
        return crate::abi::generated_types::SDL_bool_SDL_FALSE;
    }
    let state = lock_input_state();
    let Some(device) = handle_device(&state, joystick) else {
        let _ = invalid_joystick_handle();
        return crate::abi::generated_types::SDL_bool_SDL_FALSE;
    };
    let Some(value) = usize::try_from(axis)
        .ok()
        .and_then(|index| device.state.axes.get(index).copied())
    else {
        return crate::abi::generated_types::SDL_bool_SDL_FALSE;
    };
    *state_out = value;
    crate::abi::generated_types::SDL_bool_SDL_TRUE
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetHat(joystick: *mut SDL_Joystick, hat: c_int) -> Uint8 {
    let state = lock_input_state();
    let Some(device) = handle_device(&state, joystick) else {
        let _ = invalid_joystick_handle();
        return SDL_HAT_CENTERED as Uint8;
    };
    usize::try_from(hat)
        .ok()
        .and_then(|index| device.state.hats.get(index).copied())
        .unwrap_or(SDL_HAT_CENTERED as Uint8)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetBall(
    joystick: *mut SDL_Joystick,
    ball: c_int,
    dx: *mut c_int,
    dy: *mut c_int,
) -> c_int {
    if joystick.is_null() {
        return invalid_joystick_handle();
    }
    if ball != 0 {
        return crate::core::error::invalid_param_error("ball");
    }
    if !dx.is_null() {
        *dx = 0;
    }
    if !dy.is_null() {
        *dy = 0;
    }
    set_error_message("Joystick has no trackballs")
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickGetButton(
    joystick: *mut SDL_Joystick,
    button: c_int,
) -> Uint8 {
    let state = lock_input_state();
    let Some(device) = handle_device(&state, joystick) else {
        let _ = invalid_joystick_handle();
        return 0;
    };
    usize::try_from(button)
        .ok()
        .and_then(|index| device.state.buttons.get(index).copied())
        .unwrap_or(0)
}

fn with_virtual_callback_result(
    joystick: *mut SDL_Joystick,
    callback: impl FnOnce(VirtualCallbacks) -> Option<c_int>,
    missing_message: &str,
) -> c_int {
    let state = lock_input_state();
    let Some(device) = handle_device(&state, joystick) else {
        return invalid_joystick_handle();
    };
    let Some(callbacks) = device.callbacks else {
        return set_error_message(missing_message);
    };
    callback(callbacks).unwrap_or_else(|| set_error_message(missing_message))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickRumble(
    joystick: *mut SDL_Joystick,
    low_frequency_rumble: Uint16,
    high_frequency_rumble: Uint16,
    _duration_ms: Uint32,
) -> c_int {
    with_virtual_callback_result(
        joystick,
        |callbacks| {
            callbacks.rumble.map(|callback| {
                callback(
                    callbacks.userdata,
                    low_frequency_rumble,
                    high_frequency_rumble,
                )
            })
        },
        "Joystick rumble is not supported",
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickRumbleTriggers(
    joystick: *mut SDL_Joystick,
    left_rumble: Uint16,
    right_rumble: Uint16,
    _duration_ms: Uint32,
) -> c_int {
    with_virtual_callback_result(
        joystick,
        |callbacks| {
            callbacks
                .rumble_triggers
                .map(|callback| callback(callbacks.userdata, left_rumble, right_rumble))
        },
        "Joystick trigger rumble is not supported",
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickHasLED(joystick: *mut SDL_Joystick) -> SDL_bool {
    let state = lock_input_state();
    bool_to_sdl(
        handle_device(&state, joystick)
            .and_then(|device| device.callbacks)
            .and_then(|callbacks| callbacks.set_led)
            .is_some(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickHasRumble(joystick: *mut SDL_Joystick) -> SDL_bool {
    let state = lock_input_state();
    bool_to_sdl(
        handle_device(&state, joystick)
            .and_then(|device| device.callbacks)
            .and_then(|callbacks| callbacks.rumble)
            .is_some(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickHasRumbleTriggers(joystick: *mut SDL_Joystick) -> SDL_bool {
    let state = lock_input_state();
    bool_to_sdl(
        handle_device(&state, joystick)
            .and_then(|device| device.callbacks)
            .and_then(|callbacks| callbacks.rumble_triggers)
            .is_some(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickSetLED(
    joystick: *mut SDL_Joystick,
    red: Uint8,
    green: Uint8,
    blue: Uint8,
) -> c_int {
    with_virtual_callback_result(
        joystick,
        |callbacks| {
            callbacks
                .set_led
                .map(|callback| callback(callbacks.userdata, red, green, blue))
        },
        "Joystick LED is not supported",
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickSendEffect(
    joystick: *mut SDL_Joystick,
    data: *const std::ffi::c_void,
    size: c_int,
) -> c_int {
    with_virtual_callback_result(
        joystick,
        |callbacks| {
            callbacks
                .send_effect
                .map(|callback| callback(callbacks.userdata, data, size))
        },
        "Joystick effects are not supported",
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickClose(joystick: *mut SDL_Joystick) {
    if joystick.is_null() {
        return;
    }
    let mut state = lock_input_state();
    state
        .open_joysticks
        .retain(|(ptr, _)| *ptr != joystick as usize);
    drop(Box::from_raw(joystick.cast::<JoystickHandle>()));
}

#[no_mangle]
pub unsafe extern "C" fn SDL_JoystickCurrentPowerLevel(
    joystick: *mut SDL_Joystick,
) -> crate::abi::generated_types::SDL_JoystickPowerLevel {
    if joystick.is_null() {
        let _ = invalid_joystick_handle();
        return SDL_JoystickPowerLevel_SDL_JOYSTICK_POWER_UNKNOWN;
    }
    SDL_JoystickPowerLevel_SDL_JOYSTICK_POWER_UNKNOWN
}
