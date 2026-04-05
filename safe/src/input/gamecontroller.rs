use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::abi::generated_types::{
    SDL_GameController, SDL_GameControllerAxis, SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_INVALID,
    SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_MAX,
    SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_AXIS,
    SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_BUTTON,
    SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_HAT,
    SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_NONE, SDL_GameControllerButton,
    SDL_GameControllerButtonBind, SDL_GameControllerButtonBind__bindgen_ty_1,
    SDL_GameControllerButtonBind__bindgen_ty_1__bindgen_ty_1,
    SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_INVALID,
    SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_MAX, SDL_GameControllerType,
    SDL_GameControllerType_SDL_CONTROLLER_TYPE_NVIDIA_SHIELD,
    SDL_GameControllerType_SDL_CONTROLLER_TYPE_PS4,
    SDL_GameControllerType_SDL_CONTROLLER_TYPE_UNKNOWN,
    SDL_GameControllerType_SDL_CONTROLLER_TYPE_VIRTUAL, SDL_Joystick, SDL_JoystickGUID,
    SDL_JoystickID, SDL_JoystickType_SDL_JOYSTICK_TYPE_GAMECONTROLLER, SDL_RWops, SDL_SensorType,
    SDL_bool, SDL_bool_SDL_FALSE, SDL_bool_SDL_TRUE, Sint16, Uint16, Uint32, Uint64, Uint8,
    SDL_ENABLE, SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT, SDL_HAT_UP, SDL_QUERY,
};
use crate::core::error::{invalid_param_error, set_error_message};
use crate::core::memory::alloc_c_string;
use crate::core::rwops::{SDL_RWclose, SDL_RWread, SDL_RWsize};
use crate::core::system::bool_to_sdl;

use super::{
    cstr_ptr, device_by_instance, device_by_instance_mut, device_index_to_instance,
    joystick_instance_from_handle, lock_input_state, mapping_for_guid, mapping_for_guid_mut,
    BindKind, ControllerHandle, MappingEntry,
};

static AXIS_NAMES: [&[u8]; SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_MAX as usize] = [
    b"leftx\0",
    b"lefty\0",
    b"rightx\0",
    b"righty\0",
    b"lefttrigger\0",
    b"righttrigger\0",
];

static BUTTON_NAMES: [&[u8]; SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_MAX as usize] = [
    b"a\0",
    b"b\0",
    b"x\0",
    b"y\0",
    b"back\0",
    b"guide\0",
    b"start\0",
    b"leftstick\0",
    b"rightstick\0",
    b"leftshoulder\0",
    b"rightshoulder\0",
    b"dpup\0",
    b"dpdown\0",
    b"dpleft\0",
    b"dpright\0",
    b"misc1\0",
    b"paddle1\0",
    b"paddle2\0",
    b"paddle3\0",
    b"paddle4\0",
    b"touchpad\0",
];

fn axis_name(axis: SDL_GameControllerAxis) -> Option<&'static [u8]> {
    usize::try_from(axis)
        .ok()
        .and_then(|index| AXIS_NAMES.get(index).copied())
}

fn button_name(button: SDL_GameControllerButton) -> Option<&'static [u8]> {
    usize::try_from(button)
        .ok()
        .and_then(|index| BUTTON_NAMES.get(index).copied())
}

fn axis_from_string(value: &str) -> SDL_GameControllerAxis {
    let value = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    AXIS_NAMES
        .iter()
        .position(|entry| {
            value.eq_ignore_ascii_case(std::str::from_utf8(entry).unwrap().trim_end_matches('\0'))
        })
        .map(|index| index as SDL_GameControllerAxis)
        .unwrap_or(SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_INVALID)
}

fn button_from_string(value: &str) -> SDL_GameControllerButton {
    BUTTON_NAMES
        .iter()
        .position(|entry| {
            value.eq_ignore_ascii_case(std::str::from_utf8(entry).unwrap().trim_end_matches('\0'))
        })
        .map(|index| index as SDL_GameControllerButton)
        .unwrap_or(SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_INVALID)
}

fn parse_bind(value: &str) -> BindKind {
    let value = value.trim();
    if value.is_empty() {
        return BindKind::None;
    }

    let normalized = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);

    if let Some(button) = normalized
        .strip_prefix('b')
        .and_then(|rest| rest.parse::<i32>().ok())
    {
        return BindKind::Button(button);
    }
    if let Some(axis) = normalized
        .strip_prefix('a')
        .and_then(|rest| rest.parse::<i32>().ok())
    {
        return BindKind::Axis(axis);
    }
    if let Some(hat) = normalized.strip_prefix('h') {
        if let Some((hat_index, hat_mask)) = hat.split_once('.') {
            if let (Ok(hat_index), Ok(hat_mask)) =
                (hat_index.parse::<i32>(), hat_mask.parse::<i32>())
            {
                return BindKind::Hat(hat_index, hat_mask);
            }
        }
    }
    BindKind::None
}

fn build_mapping_body(mapping: &MappingEntry) -> String {
    let mut fields = Vec::new();
    for (index, bind) in mapping.axis_binds.iter().enumerate() {
        if let Some(name) = axis_name(index as SDL_GameControllerAxis) {
            if !matches!(bind, BindKind::None) {
                fields.push(format!(
                    "{}:{}",
                    std::str::from_utf8(name).unwrap().trim_end_matches('\0'),
                    bind_to_string(*bind)
                ));
            }
        }
    }
    for (index, bind) in mapping.button_binds.iter().enumerate() {
        if let Some(name) = button_name(index as SDL_GameControllerButton) {
            if !matches!(bind, BindKind::None) {
                fields.push(format!(
                    "{}:{}",
                    std::str::from_utf8(name).unwrap().trim_end_matches('\0'),
                    bind_to_string(*bind)
                ));
            }
        }
    }
    fields.join(",")
}

fn build_mapping_string(guid: SDL_JoystickGUID, name: &CStr, body: &str) -> String {
    let mut guid_buffer = [0i8; 33];
    unsafe {
        crate::input::guid::SDL_JoystickGetGUIDString(
            guid,
            guid_buffer.as_mut_ptr(),
            guid_buffer.len() as c_int,
        );
    }
    let guid_string = unsafe { CStr::from_ptr(guid_buffer.as_ptr()) }.to_string_lossy();
    let mut mapping = format!("{guid_string},{},{}", name.to_string_lossy(), body);
    if !body.contains("platform:") {
        if !mapping.ends_with(',') {
            mapping.push(',');
        }
        mapping.push_str("platform:Linux");
    }
    mapping
}

fn bind_to_string(bind: BindKind) -> String {
    match bind {
        BindKind::None => String::new(),
        BindKind::Button(index) => format!("b{index}"),
        BindKind::Axis(index) => format!("a{index}"),
        BindKind::Hat(hat, mask) => format!("h{hat}.{mask}"),
    }
}

fn default_mapping_for_device(device: &super::DeviceEntry) -> Option<MappingEntry> {
    if device.joystick_type != SDL_JoystickType_SDL_JOYSTICK_TYPE_GAMECONTROLLER {
        return None;
    }

    let mut axis_binds = mapping_array!(BindKind::None; 6);
    let mut button_binds = mapping_array!(BindKind::None; 21);

    for axis in 0..device.state.axes.len().min(6) {
        axis_binds[axis] = BindKind::Axis(axis as i32);
    }
    let button_map = [
        (0, 0),
        (1, 1),
        (2, 2),
        (3, 3),
        (4, 8),
        (5, 10),
        (6, 9),
        (7, 11),
        (8, 12),
        (9, 4),
        (10, 5),
    ];
    for (controller_button, joystick_button) in button_map {
        if joystick_button < device.state.buttons.len() {
            button_binds[controller_button] = BindKind::Button(joystick_button as i32);
        }
    }
    if !device.state.hats.is_empty() {
        button_binds[11] = BindKind::Hat(0, SDL_HAT_UP as i32);
        button_binds[12] = BindKind::Hat(0, SDL_HAT_DOWN as i32);
        button_binds[13] = BindKind::Hat(0, SDL_HAT_LEFT as i32);
        button_binds[14] = BindKind::Hat(0, SDL_HAT_RIGHT as i32);
    }

    Some(MappingEntry {
        guid: device.guid,
        name: device.name.clone(),
        raw: super::make_cstring(&build_mapping_body(&MappingEntry {
            guid: device.guid,
            name: device.name.clone(),
            raw: super::make_cstring(""),
            axis_binds,
            button_binds,
        })),
        axis_binds,
        button_binds,
    })
}

fn mapping_for_device(
    state: &super::InputState,
    instance_id: SDL_JoystickID,
) -> Option<MappingEntry> {
    let device = device_by_instance(state, instance_id)?;
    mapping_for_guid(state, &device.guid)
        .cloned()
        .or_else(|| default_mapping_for_device(device))
}

fn controller_name_ptr(state: &super::InputState, instance_id: SDL_JoystickID) -> *const c_char {
    let Some(device) = device_by_instance(state, instance_id) else {
        return ptr::null();
    };
    match mapping_for_guid(state, &device.guid) {
        Some(mapping) if mapping.name.to_bytes() != b"*" => mapping.name.as_ptr(),
        _ => device.name.as_ptr(),
    }
}

fn controller_type_from_mapping_body(body: &str) -> Option<SDL_GameControllerType> {
    let start = body.find("type:")?;
    let value = &body[start + 5..];
    let value = value.split(',').next().unwrap_or_default();
    match value {
        "nvidia_shield" => Some(SDL_GameControllerType_SDL_CONTROLLER_TYPE_NVIDIA_SHIELD),
        "ps4" => Some(SDL_GameControllerType_SDL_CONTROLLER_TYPE_PS4),
        _ => None,
    }
}

fn guess_controller_type(
    device: &super::DeviceEntry,
    mapping: Option<&MappingEntry>,
) -> SDL_GameControllerType {
    if device.is_virtual {
        return SDL_GameControllerType_SDL_CONTROLLER_TYPE_VIRTUAL;
    }
    if let Some(mapping) = mapping {
        if let Some(kind) =
            controller_type_from_mapping_body(mapping.raw.to_str().unwrap_or_default())
        {
            return kind;
        }
    }
    match (device.vendor, device.product) {
        (0x0955, 0x7214) => SDL_GameControllerType_SDL_CONTROLLER_TYPE_NVIDIA_SHIELD,
        (0x054c, 0x09cc) => SDL_GameControllerType_SDL_CONTROLLER_TYPE_PS4,
        _ => SDL_GameControllerType_SDL_CONTROLLER_TYPE_UNKNOWN,
    }
}

fn bind_for_axis(mapping: &MappingEntry, axis: SDL_GameControllerAxis) -> BindKind {
    usize::try_from(axis)
        .ok()
        .and_then(|index| mapping.axis_binds.get(index).copied())
        .unwrap_or(BindKind::None)
}

fn bind_for_button(mapping: &MappingEntry, button: SDL_GameControllerButton) -> BindKind {
    usize::try_from(button)
        .ok()
        .and_then(|index| mapping.button_binds.get(index).copied())
        .unwrap_or(BindKind::None)
}

fn button_bind_state(device: &super::DeviceEntry, bind: BindKind) -> Uint8 {
    match bind {
        BindKind::None => 0,
        BindKind::Button(index) => usize::try_from(index)
            .ok()
            .and_then(|slot| device.state.buttons.get(slot).copied())
            .unwrap_or(0),
        BindKind::Axis(index) => {
            let value = usize::try_from(index)
                .ok()
                .and_then(|slot| device.state.axes.get(slot).copied())
                .unwrap_or(0);
            if value.abs() >= 16_000 {
                1
            } else {
                0
            }
        }
        BindKind::Hat(index, mask) => usize::try_from(index)
            .ok()
            .and_then(|slot| device.state.hats.get(slot).copied())
            .map(|value| if (value as i32 & mask) != 0 { 1 } else { 0 })
            .unwrap_or(0),
    }
}

fn axis_bind_state(
    device: &super::DeviceEntry,
    bind: BindKind,
    axis: SDL_GameControllerAxis,
) -> Sint16 {
    let raw = match bind {
        BindKind::None => 0,
        BindKind::Axis(index) => usize::try_from(index)
            .ok()
            .and_then(|slot| device.state.axes.get(slot).copied())
            .unwrap_or(0),
        BindKind::Button(index) => usize::try_from(index)
            .ok()
            .and_then(|slot| device.state.buttons.get(slot).copied())
            .map(|value| if value != 0 { 32767 } else { 0 })
            .unwrap_or(0),
        BindKind::Hat(index, mask) => usize::try_from(index)
            .ok()
            .and_then(|slot| device.state.hats.get(slot).copied())
            .map(|value| {
                if (value as i32 & mask) == 0 {
                    0
                } else if mask == SDL_HAT_LEFT as i32 || mask == SDL_HAT_UP as i32 {
                    -32768
                } else {
                    32767
                }
            })
            .unwrap_or(0),
    };
    if axis == 4 || axis == 5 {
        raw.max(0)
    } else {
        raw
    }
}

fn bind_to_output(bind: BindKind) -> SDL_GameControllerButtonBind {
    match bind {
        BindKind::None => SDL_GameControllerButtonBind {
            bindType: SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_NONE,
            value: SDL_GameControllerButtonBind__bindgen_ty_1 { button: 0 },
        },
        BindKind::Button(button) => SDL_GameControllerButtonBind {
            bindType: SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_BUTTON,
            value: SDL_GameControllerButtonBind__bindgen_ty_1 { button },
        },
        BindKind::Axis(axis) => SDL_GameControllerButtonBind {
            bindType: SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_AXIS,
            value: SDL_GameControllerButtonBind__bindgen_ty_1 { axis },
        },
        BindKind::Hat(hat, hat_mask) => SDL_GameControllerButtonBind {
            bindType: SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_HAT,
            value: SDL_GameControllerButtonBind__bindgen_ty_1 {
                hat: SDL_GameControllerButtonBind__bindgen_ty_1__bindgen_ty_1 { hat, hat_mask },
            },
        },
    }
}

fn parse_mapping(mapping_string: &str) -> Result<MappingEntry, c_int> {
    let mut parts = mapping_string.splitn(3, ',');
    let guid_part = parts
        .next()
        .ok_or_else(|| set_error_message("Couldn't parse controller GUID"))?;
    let name_part = parts
        .next()
        .ok_or_else(|| set_error_message("Couldn't parse controller name"))?;
    let body = parts
        .next()
        .ok_or_else(|| set_error_message("Couldn't parse controller mapping"))?;

    let guid = unsafe {
        let guid_c = super::make_cstring(guid_part);
        crate::input::guid::SDL_JoystickGetGUIDFromString(guid_c.as_ptr())
    };
    let name = super::make_cstring(name_part);
    let mut axis_binds = mapping_array!(BindKind::None; 6);
    let mut button_binds = mapping_array!(BindKind::None; 21);

    for field in body.split(',') {
        let Some((key, value)) = field.split_once(':') else {
            continue;
        };
        if key.eq_ignore_ascii_case("platform")
            || key.eq_ignore_ascii_case("crc")
            || key.eq_ignore_ascii_case("type")
            || key.eq_ignore_ascii_case("hint")
            || key.eq_ignore_ascii_case("sdk>=")
            || key.eq_ignore_ascii_case("sdk<=")
        {
            continue;
        }

        let axis = axis_from_string(key);
        if axis != SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_INVALID {
            axis_binds[axis as usize] = parse_bind(value);
            continue;
        }
        let button = button_from_string(key);
        if button != SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_INVALID {
            button_binds[button as usize] = parse_bind(value);
        }
    }

    Ok(MappingEntry {
        guid,
        name,
        raw: super::make_cstring(body),
        axis_binds,
        button_binds,
    })
}

fn controller_instance(gamecontroller: *mut SDL_GameController) -> Option<SDL_JoystickID> {
    if gamecontroller.is_null() {
        None
    } else {
        Some(unsafe { (*(gamecontroller as *mut ControllerHandle)).instance_id })
    }
}

fn write_sensor_values(values: &[f32], data: *mut f32, num_values: c_int) {
    if data.is_null() || num_values <= 0 {
        return;
    }
    let len = num_values as usize;
    unsafe {
        for index in 0..len {
            *data.add(index) = values.get(index).copied().unwrap_or(0.0);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerAddMappingsFromRW(
    rw: *mut SDL_RWops,
    freerw: c_int,
) -> c_int {
    if rw.is_null() {
        return set_error_message("Invalid RWops");
    }
    let size = SDL_RWsize(rw);
    if size < 0 {
        if freerw != 0 {
            let _ = SDL_RWclose(rw);
        }
        return set_error_message("Could not read DB");
    }

    let mut buf = vec![0u8; size as usize];
    if !buf.is_empty() && SDL_RWread(rw, buf.as_mut_ptr().cast(), buf.len(), 1) != 1 {
        if freerw != 0 {
            let _ = SDL_RWclose(rw);
        }
        return set_error_message("Could not read DB");
    }
    if freerw != 0 {
        let _ = SDL_RWclose(rw);
    }

    let mut controllers = 0;
    let text = String::from_utf8_lossy(&buf);
    for line in text.lines() {
        let trimmed = line.trim_end_matches('\r').trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(platform_pos) = trimmed.find("platform:") else {
            continue;
        };
        let platform = &trimmed[platform_pos + 9..];
        let platform = platform.split(',').next().unwrap_or_default();
        if platform.eq_ignore_ascii_case("linux")
            && SDL_GameControllerAddMapping(super::make_cstring(trimmed).as_ptr()) > 0
        {
            controllers += 1;
        }
    }
    controllers
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerAddMapping(mappingString: *const c_char) -> c_int {
    if mappingString.is_null() {
        return invalid_param_error("mappingString");
    }
    let mapping_string = CStr::from_ptr(mappingString).to_string_lossy().into_owned();
    let mapping = match parse_mapping(&mapping_string) {
        Ok(mapping) => mapping,
        Err(code) => return code,
    };

    let mut state = lock_input_state();
    if let Some(existing) = mapping_for_guid_mut(&mut state, &mapping.guid) {
        *existing = mapping;
        0
    } else {
        let index = state.mappings.len();
        state.mapping_indices.insert(mapping.guid.data, index);
        state.mappings.push(mapping);
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerNumMappings() -> c_int {
    let state = lock_input_state();
    state.mappings.len() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerMappingForIndex(mapping_index: c_int) -> *mut c_char {
    let state = lock_input_state();
    let Some(mapping) = usize::try_from(mapping_index)
        .ok()
        .and_then(|index| state.mappings.get(index))
    else {
        let _ = set_error_message("Mapping not available");
        return ptr::null_mut();
    };
    alloc_c_string(&build_mapping_string(
        mapping.guid,
        &mapping.name,
        mapping.raw.to_str().unwrap_or_default(),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerMappingForGUID(guid: SDL_JoystickGUID) -> *mut c_char {
    let state = lock_input_state();
    if let Some(mapping) = mapping_for_guid(&state, &guid) {
        return alloc_c_string(&build_mapping_string(
            guid,
            &mapping.name,
            mapping.raw.to_str().unwrap_or_default(),
        ));
    }
    if let Some(device) = state
        .devices
        .iter()
        .find(|device| device.guid.data == guid.data)
    {
        if let Some(mapping) = default_mapping_for_device(device) {
            return alloc_c_string(&build_mapping_string(
                guid,
                &mapping.name,
                mapping.raw.to_str().unwrap_or_default(),
            ));
        }
    }
    let _ = set_error_message("Mapping not available");
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerMapping(
    gamecontroller: *mut SDL_GameController,
) -> *mut c_char {
    let state = lock_input_state();
    let Some(instance_id) = controller_instance(gamecontroller) else {
        let _ = invalid_param_error("gamecontroller");
        return ptr::null_mut();
    };
    let Some(mapping) = mapping_for_device(&state, instance_id) else {
        let _ = set_error_message("Mapping not available");
        return ptr::null_mut();
    };
    alloc_c_string(&build_mapping_string(
        mapping.guid,
        &mapping.name,
        mapping.raw.to_str().unwrap_or_default(),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_IsGameController(joystick_index: c_int) -> SDL_bool {
    let mut state = lock_input_state();
    let Some(instance_id) = device_index_to_instance(&mut state, joystick_index) else {
        return SDL_bool_SDL_FALSE;
    };
    bool_to_sdl(mapping_for_device(&state, instance_id).is_some())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerNameForIndex(joystick_index: c_int) -> *const c_char {
    let mut state = lock_input_state();
    let Some(instance_id) = device_index_to_instance(&mut state, joystick_index) else {
        return ptr::null();
    };
    if mapping_for_device(&state, instance_id).is_none() {
        return ptr::null();
    }
    controller_name_ptr(&state, instance_id)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerPathForIndex(joystick_index: c_int) -> *const c_char {
    let mut state = lock_input_state();
    let Some(instance_id) = device_index_to_instance(&mut state, joystick_index) else {
        return ptr::null();
    };
    if mapping_for_device(&state, instance_id).is_none() {
        return ptr::null();
    }
    device_by_instance(&state, instance_id)
        .map(|device| cstr_ptr(device.path.as_ref()))
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerTypeForIndex(
    joystick_index: c_int,
) -> SDL_GameControllerType {
    let mut state = lock_input_state();
    let Some(instance_id) = device_index_to_instance(&mut state, joystick_index) else {
        return SDL_GameControllerType_SDL_CONTROLLER_TYPE_UNKNOWN;
    };
    let Some(device) = device_by_instance(&state, instance_id) else {
        return SDL_GameControllerType_SDL_CONTROLLER_TYPE_UNKNOWN;
    };
    let mapping = mapping_for_guid(&state, &device.guid);
    guess_controller_type(device, mapping)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerMappingForDeviceIndex(
    joystick_index: c_int,
) -> *mut c_char {
    let mut state = lock_input_state();
    let Some(instance_id) = device_index_to_instance(&mut state, joystick_index) else {
        return ptr::null_mut();
    };
    let Some(mapping) = mapping_for_device(&state, instance_id) else {
        return ptr::null_mut();
    };
    alloc_c_string(&build_mapping_string(
        mapping.guid,
        &mapping.name,
        mapping.raw.to_str().unwrap_or_default(),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerOpen(joystick_index: c_int) -> *mut SDL_GameController {
    if !super::lock_input_state().controller_initialized {
        let _ = set_error_message("Game controller subsystem isn't initialized");
        return ptr::null_mut();
    }
    if SDL_IsGameController(joystick_index) == SDL_bool_SDL_FALSE {
        let _ = set_error_message("Joystick is not a game controller");
        return ptr::null_mut();
    }
    let joystick = crate::input::joystick::SDL_JoystickOpen(joystick_index);
    if joystick.is_null() {
        return ptr::null_mut();
    }

    let mut state = lock_input_state();
    let Some(instance_id) = joystick_instance_from_handle(joystick) else {
        crate::input::joystick::SDL_JoystickClose(joystick);
        return ptr::null_mut();
    };
    let guid = device_by_instance(&state, instance_id)
        .map(|device| device.guid)
        .unwrap_or(SDL_JoystickGUID { data: [0; 16] });
    let controller = Box::into_raw(Box::new(ControllerHandle {
        joystick,
        instance_id,
        guid,
    }));
    state
        .open_controllers
        .push((controller as usize, instance_id));
    controller.cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerFromInstanceID(
    joyid: SDL_JoystickID,
) -> *mut SDL_GameController {
    let state = lock_input_state();
    state
        .open_controllers
        .iter()
        .find(|(_, instance_id)| *instance_id == joyid)
        .map(|(ptr, _)| *ptr as *mut SDL_GameController)
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerFromPlayerIndex(
    player_index: c_int,
) -> *mut SDL_GameController {
    let state = lock_input_state();
    state
        .open_controllers
        .iter()
        .find_map(|(ptr, instance_id)| {
            let device = device_by_instance(&state, *instance_id)?;
            (device.player_index == player_index).then_some(*ptr as *mut SDL_GameController)
        })
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerName(
    gamecontroller: *mut SDL_GameController,
) -> *const c_char {
    let state = lock_input_state();
    let Some(instance_id) = controller_instance(gamecontroller) else {
        return ptr::null();
    };
    if mapping_for_device(&state, instance_id).is_none() {
        return ptr::null();
    }
    controller_name_ptr(&state, instance_id)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerPath(
    gamecontroller: *mut SDL_GameController,
) -> *const c_char {
    let state = lock_input_state();
    let Some(instance_id) = controller_instance(gamecontroller) else {
        return ptr::null();
    };
    device_by_instance(&state, instance_id)
        .map(|device| cstr_ptr(device.path.as_ref()))
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetType(
    gamecontroller: *mut SDL_GameController,
) -> SDL_GameControllerType {
    let state = lock_input_state();
    let Some(instance_id) = controller_instance(gamecontroller) else {
        return SDL_GameControllerType_SDL_CONTROLLER_TYPE_UNKNOWN;
    };
    let Some(device) = device_by_instance(&state, instance_id) else {
        return SDL_GameControllerType_SDL_CONTROLLER_TYPE_UNKNOWN;
    };
    let mapping = mapping_for_device(&state, instance_id);
    guess_controller_type(device, mapping.as_ref())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetPlayerIndex(
    gamecontroller: *mut SDL_GameController,
) -> c_int {
    let state = lock_input_state();
    controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
        .map(|device| device.player_index)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerSetPlayerIndex(
    gamecontroller: *mut SDL_GameController,
    player_index: c_int,
) {
    let joystick = SDL_GameControllerGetJoystick(gamecontroller);
    if !joystick.is_null() {
        crate::input::joystick::SDL_JoystickSetPlayerIndex(joystick, player_index);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetVendor(
    gamecontroller: *mut SDL_GameController,
) -> Uint16 {
    let state = lock_input_state();
    controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
        .map(|device| device.vendor)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetProduct(
    gamecontroller: *mut SDL_GameController,
) -> Uint16 {
    let state = lock_input_state();
    controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
        .map(|device| device.product)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetProductVersion(
    gamecontroller: *mut SDL_GameController,
) -> Uint16 {
    let state = lock_input_state();
    controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
        .map(|device| device.product_version)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetFirmwareVersion(
    gamecontroller: *mut SDL_GameController,
) -> Uint16 {
    let state = lock_input_state();
    controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
        .map(|device| device.firmware_version)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetSerial(
    gamecontroller: *mut SDL_GameController,
) -> *const c_char {
    let state = lock_input_state();
    controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
        .map(|device| cstr_ptr(device.serial.as_ref()))
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetSteamHandle(
    _gamecontroller: *mut SDL_GameController,
) -> Uint64 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetAttached(
    gamecontroller: *mut SDL_GameController,
) -> SDL_bool {
    let state = lock_input_state();
    bool_to_sdl(
        controller_instance(gamecontroller)
            .and_then(|instance_id| device_by_instance(&state, instance_id))
            .is_some(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetJoystick(
    gamecontroller: *mut SDL_GameController,
) -> *mut SDL_Joystick {
    if gamecontroller.is_null() {
        ptr::null_mut()
    } else {
        (*(gamecontroller as *mut ControllerHandle)).joystick
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerEventState(state_value: c_int) -> c_int {
    let mut state = lock_input_state();
    if state_value == SDL_QUERY as c_int {
        return state.controller_event_state;
    }
    if state_value == 0 || state_value == SDL_ENABLE as c_int {
        state.controller_event_state = state_value;
        return state.controller_event_state;
    }
    state_value
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerUpdate() {
    crate::input::joystick::SDL_JoystickUpdate();
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetAxisFromString(
    str_: *const c_char,
) -> SDL_GameControllerAxis {
    if str_.is_null() {
        return SDL_GameControllerAxis_SDL_CONTROLLER_AXIS_INVALID;
    }
    axis_from_string(CStr::from_ptr(str_).to_str().unwrap_or_default())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetStringForAxis(
    axis: SDL_GameControllerAxis,
) -> *const c_char {
    axis_name(axis)
        .map(|name| name.as_ptr().cast())
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetBindForAxis(
    gamecontroller: *mut SDL_GameController,
    axis: SDL_GameControllerAxis,
) -> SDL_GameControllerButtonBind {
    let state = lock_input_state();
    let Some(instance_id) = controller_instance(gamecontroller) else {
        return bind_to_output(BindKind::None);
    };
    let Some(mapping) = mapping_for_device(&state, instance_id) else {
        return bind_to_output(BindKind::None);
    };
    bind_to_output(bind_for_axis(&mapping, axis))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerHasAxis(
    gamecontroller: *mut SDL_GameController,
    axis: SDL_GameControllerAxis,
) -> SDL_bool {
    bool_to_sdl(
        SDL_GameControllerGetBindForAxis(gamecontroller, axis).bindType
            != SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_NONE,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetAxis(
    gamecontroller: *mut SDL_GameController,
    axis: SDL_GameControllerAxis,
) -> Sint16 {
    let state = lock_input_state();
    let Some(instance_id) = controller_instance(gamecontroller) else {
        let _ = invalid_param_error("gamecontroller");
        return 0;
    };
    let Some(device) = device_by_instance(&state, instance_id) else {
        return 0;
    };
    let Some(mapping) = mapping_for_device(&state, instance_id) else {
        return 0;
    };
    axis_bind_state(device, bind_for_axis(&mapping, axis), axis)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetButtonFromString(
    str_: *const c_char,
) -> SDL_GameControllerButton {
    if str_.is_null() {
        return SDL_GameControllerButton_SDL_CONTROLLER_BUTTON_INVALID;
    }
    button_from_string(CStr::from_ptr(str_).to_str().unwrap_or_default())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetStringForButton(
    button: SDL_GameControllerButton,
) -> *const c_char {
    button_name(button)
        .map(|name| name.as_ptr().cast())
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetBindForButton(
    gamecontroller: *mut SDL_GameController,
    button: SDL_GameControllerButton,
) -> SDL_GameControllerButtonBind {
    let state = lock_input_state();
    let Some(instance_id) = controller_instance(gamecontroller) else {
        return bind_to_output(BindKind::None);
    };
    let Some(mapping) = mapping_for_device(&state, instance_id) else {
        return bind_to_output(BindKind::None);
    };
    bind_to_output(bind_for_button(&mapping, button))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerHasButton(
    gamecontroller: *mut SDL_GameController,
    button: SDL_GameControllerButton,
) -> SDL_bool {
    bool_to_sdl(
        SDL_GameControllerGetBindForButton(gamecontroller, button).bindType
            != SDL_GameControllerBindType_SDL_CONTROLLER_BINDTYPE_NONE,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetButton(
    gamecontroller: *mut SDL_GameController,
    button: SDL_GameControllerButton,
) -> Uint8 {
    let state = lock_input_state();
    let Some(instance_id) = controller_instance(gamecontroller) else {
        let _ = invalid_param_error("gamecontroller");
        return 0;
    };
    let Some(device) = device_by_instance(&state, instance_id) else {
        return 0;
    };
    let Some(mapping) = mapping_for_device(&state, instance_id) else {
        return 0;
    };
    button_bind_state(device, bind_for_button(&mapping, button))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetNumTouchpads(
    gamecontroller: *mut SDL_GameController,
) -> c_int {
    let state = lock_input_state();
    controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
        .map(|device| device.touchpads.len() as c_int)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetNumTouchpadFingers(
    gamecontroller: *mut SDL_GameController,
    touchpad: c_int,
) -> c_int {
    let state = lock_input_state();
    let Some(device) = controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
    else {
        return 0;
    };
    usize::try_from(touchpad)
        .ok()
        .and_then(|index| device.touchpads.get(index))
        .map(|touchpad| touchpad.fingers.len() as c_int)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetTouchpadFinger(
    gamecontroller: *mut SDL_GameController,
    touchpad: c_int,
    finger: c_int,
    state_out: *mut Uint8,
    x: *mut f32,
    y: *mut f32,
    pressure: *mut f32,
) -> c_int {
    if !state_out.is_null() {
        *state_out = 0;
    }
    if !x.is_null() {
        *x = 0.0;
    }
    if !y.is_null() {
        *y = 0.0;
    }
    if !pressure.is_null() {
        *pressure = 0.0;
    }
    let state = lock_input_state();
    let Some(device) = controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
    else {
        return invalid_param_error("gamecontroller");
    };
    let Some(finger_state) = usize::try_from(touchpad)
        .ok()
        .and_then(|touchpad_index| device.touchpads.get(touchpad_index))
        .and_then(|touchpad| {
            usize::try_from(finger)
                .ok()
                .and_then(|finger_index| touchpad.fingers.get(finger_index))
        })
    else {
        return set_error_message("Touchpad finger is not available");
    };
    if !state_out.is_null() {
        *state_out = finger_state.state;
    }
    if !x.is_null() {
        *x = finger_state.x;
    }
    if !y.is_null() {
        *y = finger_state.y;
    }
    if !pressure.is_null() {
        *pressure = finger_state.pressure;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerHasSensor(
    gamecontroller: *mut SDL_GameController,
    type_: SDL_SensorType,
) -> SDL_bool {
    let state = lock_input_state();
    bool_to_sdl(
        controller_instance(gamecontroller)
            .and_then(|instance_id| device_by_instance(&state, instance_id))
            .and_then(|device| device.sensors.iter().find(|sensor| sensor.type_ == type_))
            .is_some(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerSetSensorEnabled(
    gamecontroller: *mut SDL_GameController,
    type_: SDL_SensorType,
    enabled: SDL_bool,
) -> c_int {
    let mut state = lock_input_state();
    let Some(device) = controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance_mut(&mut state, instance_id))
    else {
        return invalid_param_error("gamecontroller");
    };
    let Some(sensor_index) = device
        .sensors
        .iter()
        .position(|sensor| sensor.type_ == type_)
    else {
        return set_error_message("Game controller sensor is not available");
    };
    device.sensors[sensor_index].enabled = enabled == SDL_bool_SDL_TRUE;
    super::refresh_device_features(device);
    device.sensors[sensor_index].enabled = enabled == SDL_bool_SDL_TRUE;
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerIsSensorEnabled(
    gamecontroller: *mut SDL_GameController,
    type_: SDL_SensorType,
) -> SDL_bool {
    let state = lock_input_state();
    bool_to_sdl(
        controller_instance(gamecontroller)
            .and_then(|instance_id| device_by_instance(&state, instance_id))
            .and_then(|device| device.sensors.iter().find(|sensor| sensor.type_ == type_))
            .map(|sensor| sensor.enabled)
            .unwrap_or(false),
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetSensorDataRate(
    gamecontroller: *mut SDL_GameController,
    type_: SDL_SensorType,
) -> f32 {
    let state = lock_input_state();
    controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
        .and_then(|device| device.sensors.iter().find(|sensor| sensor.type_ == type_))
        .map(|sensor| sensor.rate_hz)
        .unwrap_or(0.0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetSensorData(
    gamecontroller: *mut SDL_GameController,
    type_: SDL_SensorType,
    data: *mut f32,
    num_values: c_int,
) -> c_int {
    SDL_GameControllerGetSensorDataWithTimestamp(
        gamecontroller,
        type_,
        ptr::null_mut(),
        data,
        num_values,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetSensorDataWithTimestamp(
    gamecontroller: *mut SDL_GameController,
    type_: SDL_SensorType,
    timestamp: *mut Uint64,
    data: *mut f32,
    num_values: c_int,
) -> c_int {
    if !timestamp.is_null() {
        *timestamp = 0;
    }
    write_sensor_values(&[], data, num_values);
    let state = lock_input_state();
    let Some(device) = controller_instance(gamecontroller)
        .and_then(|instance_id| device_by_instance(&state, instance_id))
    else {
        return invalid_param_error("gamecontroller");
    };
    let Some(sensor) = device.sensors.iter().find(|sensor| sensor.type_ == type_) else {
        return set_error_message("Game controller sensor is not available");
    };
    if !sensor.enabled {
        return set_error_message("Game controller sensor is disabled");
    }
    if !timestamp.is_null() {
        *timestamp = sensor.timestamp_us;
    }
    write_sensor_values(&sensor.values, data, num_values);
    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerRumble(
    gamecontroller: *mut SDL_GameController,
    low_frequency_rumble: Uint16,
    high_frequency_rumble: Uint16,
    duration_ms: Uint32,
) -> c_int {
    crate::input::joystick::SDL_JoystickRumble(
        SDL_GameControllerGetJoystick(gamecontroller),
        low_frequency_rumble,
        high_frequency_rumble,
        duration_ms,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerRumbleTriggers(
    gamecontroller: *mut SDL_GameController,
    left_rumble: Uint16,
    right_rumble: Uint16,
    duration_ms: Uint32,
) -> c_int {
    crate::input::joystick::SDL_JoystickRumbleTriggers(
        SDL_GameControllerGetJoystick(gamecontroller),
        left_rumble,
        right_rumble,
        duration_ms,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerHasLED(
    gamecontroller: *mut SDL_GameController,
) -> SDL_bool {
    crate::input::joystick::SDL_JoystickHasLED(SDL_GameControllerGetJoystick(gamecontroller))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerHasRumble(
    gamecontroller: *mut SDL_GameController,
) -> SDL_bool {
    crate::input::joystick::SDL_JoystickHasRumble(SDL_GameControllerGetJoystick(gamecontroller))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerHasRumbleTriggers(
    gamecontroller: *mut SDL_GameController,
) -> SDL_bool {
    crate::input::joystick::SDL_JoystickHasRumbleTriggers(SDL_GameControllerGetJoystick(
        gamecontroller,
    ))
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerSetLED(
    gamecontroller: *mut SDL_GameController,
    red: Uint8,
    green: Uint8,
    blue: Uint8,
) -> c_int {
    crate::input::joystick::SDL_JoystickSetLED(
        SDL_GameControllerGetJoystick(gamecontroller),
        red,
        green,
        blue,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerSendEffect(
    gamecontroller: *mut SDL_GameController,
    data: *const std::ffi::c_void,
    size: c_int,
) -> c_int {
    crate::input::joystick::SDL_JoystickSendEffect(
        SDL_GameControllerGetJoystick(gamecontroller),
        data,
        size,
    )
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerClose(gamecontroller: *mut SDL_GameController) {
    if gamecontroller.is_null() {
        return;
    }
    let mut state = lock_input_state();
    super::close_controller_handle(&mut state, gamecontroller.cast::<ControllerHandle>());
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetAppleSFSymbolsNameForButton(
    _gamecontroller: *mut SDL_GameController,
    _button: SDL_GameControllerButton,
) -> *const c_char {
    ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_GameControllerGetAppleSFSymbolsNameForAxis(
    _gamecontroller: *mut SDL_GameController,
    _axis: SDL_GameControllerAxis,
) -> *const c_char {
    ptr::null()
}
