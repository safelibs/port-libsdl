use std::collections::{BTreeSet, HashMap};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::abi::generated_types::{
    SDL_Joystick, SDL_JoystickGUID, SDL_JoystickID, SDL_JoystickType, SDL_SensorID, SDL_SensorType,
    SDL_SensorType_SDL_SENSOR_ACCEL, SDL_SensorType_SDL_SENSOR_ACCEL_L,
    SDL_SensorType_SDL_SENSOR_ACCEL_R, SDL_SensorType_SDL_SENSOR_GYRO,
    SDL_SensorType_SDL_SENSOR_GYRO_L, SDL_SensorType_SDL_SENSOR_GYRO_R, Sint16, Uint16, Uint32,
    Uint64, Uint8, SDL_ENABLE, SDL_HAT_CENTERED, SDL_INIT_GAMECONTROLLER, SDL_INIT_HAPTIC,
    SDL_INIT_JOYSTICK, SDL_INIT_SENSOR, SDL_PRESSED, SDL_RELEASED,
};
use crate::core::error::{invalid_param_error, set_error_message};

const SDL_HARDWARE_BUS_VIRTUAL: u16 = 0x00ff;

#[derive(Clone, Copy)]
pub(crate) enum BindKind {
    None,
    Button(i32),
    Axis(i32),
    Hat(i32, i32),
}

#[derive(Clone)]
pub(crate) struct MappingEntry {
    pub guid: SDL_JoystickGUID,
    pub name: CString,
    pub raw: CString,
    pub axis_binds: [BindKind; 6],
    pub button_binds: [BindKind; 21],
}

#[derive(Clone, Copy)]
struct VirtualCallbacks {
    update: Option<unsafe extern "C" fn(*mut c_void)>,
    set_player_index: Option<unsafe extern "C" fn(*mut c_void, c_int)>,
    rumble: Option<unsafe extern "C" fn(*mut c_void, Uint16, Uint16) -> c_int>,
    rumble_triggers: Option<unsafe extern "C" fn(*mut c_void, Uint16, Uint16) -> c_int>,
    set_led: Option<unsafe extern "C" fn(*mut c_void, Uint8, Uint8, Uint8) -> c_int>,
    send_effect: Option<unsafe extern "C" fn(*mut c_void, *const c_void, c_int) -> c_int>,
    userdata: *mut c_void,
}

unsafe impl Send for VirtualCallbacks {}

#[derive(Clone)]
pub(crate) struct DeviceState {
    axes: Vec<Sint16>,
    pending_axes: Vec<Sint16>,
    buttons: Vec<Uint8>,
    pending_buttons: Vec<Uint8>,
    hats: Vec<Uint8>,
    pending_hats: Vec<Uint8>,
}

impl DeviceState {
    fn new(naxes: usize, nbuttons: usize, nhats: usize) -> Self {
        Self {
            axes: vec![0; naxes],
            pending_axes: vec![0; naxes],
            buttons: vec![0; nbuttons],
            pending_buttons: vec![0; nbuttons],
            hats: vec![SDL_HAT_CENTERED as Uint8; nhats],
            pending_hats: vec![SDL_HAT_CENTERED as Uint8; nhats],
        }
    }

    fn apply_pending(&mut self) {
        self.axes.clone_from(&self.pending_axes);
        self.buttons.clone_from(&self.pending_buttons);
        self.hats.clone_from(&self.pending_hats);
    }
}

#[derive(Clone)]
pub(crate) struct TouchpadFingerState {
    pub state: Uint8,
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

#[derive(Clone)]
pub(crate) struct TouchpadState {
    pub fingers: Vec<TouchpadFingerState>,
}

#[derive(Clone)]
pub(crate) struct DeviceSensorState {
    pub name: CString,
    pub type_: SDL_SensorType,
    pub non_portable_type: c_int,
    pub enabled: bool,
    pub rate_hz: f32,
    pub values: Vec<f32>,
    pub timestamp_us: Uint64,
}

pub(crate) struct DeviceEntry {
    instance_id: SDL_JoystickID,
    name: CString,
    path: Option<CString>,
    guid: SDL_JoystickGUID,
    vendor: Uint16,
    product: Uint16,
    product_version: Uint16,
    firmware_version: Uint16,
    serial: Option<CString>,
    joystick_type: SDL_JoystickType,
    player_index: c_int,
    is_virtual: bool,
    callbacks: Option<VirtualCallbacks>,
    state: DeviceState,
    touchpads: Vec<TouchpadState>,
    sensors: Vec<DeviceSensorState>,
    evdev: Option<linux::evdev::EvdevSource>,
    hint_path: Option<String>,
}

#[repr(C)]
pub(crate) struct JoystickHandle {
    pub instance_id: SDL_JoystickID,
}

#[repr(C)]
pub(crate) struct ControllerHandle {
    pub joystick: *mut SDL_Joystick,
    pub instance_id: SDL_JoystickID,
    pub guid: SDL_JoystickGUID,
}

#[repr(C)]
pub(crate) struct HapticHandle {
    pub instance_id: SDL_JoystickID,
    pub gain: c_int,
    pub autocenter: c_int,
    pub paused: bool,
    pub rumble_initialized: bool,
    pub effects: Vec<Option<crate::abi::generated_types::SDL_HapticEffect>>,
    pub effect_playing: Vec<bool>,
}

#[repr(C)]
pub(crate) struct SensorHandle {
    pub instance_id: SDL_SensorID,
    pub joystick_instance_id: SDL_JoystickID,
    pub sensor_index: usize,
}

#[derive(Default)]
pub(crate) struct InputState {
    next_instance_id: SDL_JoystickID,
    devices: Vec<DeviceEntry>,
    mappings: Vec<MappingEntry>,
    mapping_indices: HashMap<[u8; 16], usize>,
    joystick_event_state: c_int,
    controller_event_state: c_int,
    joystick_initialized: bool,
    controller_initialized: bool,
    haptic_initialized: bool,
    sensor_initialized: bool,
    open_joysticks: Vec<(usize, SDL_JoystickID)>,
    open_controllers: Vec<(usize, SDL_JoystickID)>,
    open_haptics: Vec<(usize, SDL_JoystickID)>,
    open_sensors: Vec<(usize, SDL_SensorID)>,
}

fn input_state() -> &'static Mutex<InputState> {
    static STATE: OnceLock<Mutex<InputState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(InputState {
            next_instance_id: 0,
            devices: Vec::with_capacity(16),
            mappings: Vec::with_capacity(64),
            mapping_indices: HashMap::with_capacity(64),
            joystick_event_state: SDL_ENABLE as c_int,
            controller_event_state: SDL_ENABLE as c_int,
            ..InputState::default()
        })
    })
}

pub(crate) fn lock_input_state() -> std::sync::MutexGuard<'static, InputState> {
    match input_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub(crate) fn hint_value(name: &[u8]) -> Option<String> {
    unsafe {
        let ptr = crate::core::hints::SDL_GetHint(name.as_ptr().cast());
        if ptr.is_null() {
            None
        } else {
            let value = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        }
    }
}

fn make_cstring(text: &str) -> CString {
    CString::new(text).unwrap_or_default()
}

fn normalized_axis(value: Sint16) -> f32 {
    (value as f32 + 32_768.0) / 65_535.0
}

fn signed_axis(value: Sint16) -> f32 {
    (value as f32 / 32_767.0).clamp(-1.0, 1.0)
}

pub(crate) fn sensor_instance_id(
    joystick_instance_id: SDL_JoystickID,
    sensor_index: usize,
) -> SDL_SensorID {
    joystick_instance_id
        .saturating_mul(16)
        .saturating_add(sensor_index as SDL_SensorID + 1)
}

fn default_touchpads() -> Vec<TouchpadState> {
    vec![TouchpadState {
        fingers: vec![TouchpadFingerState {
            state: SDL_RELEASED as Uint8,
            x: 0.5,
            y: 0.5,
            pressure: 0.0,
        }],
    }]
}

fn default_sensors(name_prefix: &str) -> Vec<DeviceSensorState> {
    let specs = [
        ("accelerometer", SDL_SensorType_SDL_SENSOR_ACCEL),
        ("gyro", SDL_SensorType_SDL_SENSOR_GYRO),
        ("accelerometer (L)", SDL_SensorType_SDL_SENSOR_ACCEL_L),
        ("gyro (L)", SDL_SensorType_SDL_SENSOR_GYRO_L),
        ("accelerometer (R)", SDL_SensorType_SDL_SENSOR_ACCEL_R),
        ("gyro (R)", SDL_SensorType_SDL_SENSOR_GYRO_R),
    ];
    specs
        .into_iter()
        .map(|(suffix, type_)| DeviceSensorState {
            name: make_cstring(&format!("{name_prefix} {suffix}")),
            type_,
            non_portable_type: type_,
            enabled: true,
            rate_hz: 60.0,
            values: vec![0.0; 3],
            timestamp_us: 0,
        })
        .collect()
}

fn should_expose_controller_features(joystick_type: SDL_JoystickType) -> bool {
    joystick_type == crate::abi::generated_types::SDL_JoystickType_SDL_JOYSTICK_TYPE_GAMECONTROLLER
}

pub(crate) fn refresh_device_features(device: &mut DeviceEntry) {
    if !should_expose_controller_features(device.joystick_type) {
        device.touchpads.clear();
        device.sensors.clear();
        return;
    }

    if device.touchpads.is_empty() {
        device.touchpads = default_touchpads();
    }
    if device.sensors.is_empty() {
        let base_name = device.name.to_str().unwrap_or("SDL Controller");
        device.sensors = default_sensors(base_name);
    }

    let axis0 = device.state.axes.first().copied().unwrap_or_default();
    let axis1 = device.state.axes.get(1).copied().unwrap_or_default();
    let axis2 = device.state.axes.get(2).copied().unwrap_or_default();
    let axis3 = device.state.axes.get(3).copied().unwrap_or_default();
    let axis4 = device.state.axes.get(4).copied().unwrap_or_default();
    let axis5 = device.state.axes.get(5).copied().unwrap_or_default();
    let finger_pressed = device
        .state
        .buttons
        .first()
        .copied()
        .unwrap_or(SDL_RELEASED as Uint8)
        != 0;

    if let Some(touchpad) = device.touchpads.first_mut() {
        if let Some(finger) = touchpad.fingers.first_mut() {
            finger.state = if finger_pressed {
                SDL_PRESSED as Uint8
            } else {
                SDL_RELEASED as Uint8
            };
            finger.x = normalized_axis(axis0).clamp(0.0, 1.0);
            finger.y = normalized_axis(axis1).clamp(0.0, 1.0);
            finger.pressure = if finger_pressed {
                normalized_axis(axis2).clamp(0.05, 1.0)
            } else {
                0.0
            };
        }
    }

    let timestamp_us = unsafe { crate::core::timer::SDL_GetTicks64() }
        .saturating_mul(1000)
        .max(1);
    for sensor in &mut device.sensors {
        if !sensor.enabled {
            continue;
        }
        sensor.timestamp_us = timestamp_us;
        match sensor.type_ {
            SDL_SensorType_SDL_SENSOR_ACCEL => {
                sensor.values = vec![
                    signed_axis(axis0) * 9.80665,
                    signed_axis(axis1) * 9.80665,
                    signed_axis(axis2) * 9.80665,
                ];
            }
            SDL_SensorType_SDL_SENSOR_GYRO => {
                sensor.values = vec![
                    signed_axis(axis3) * 5.0,
                    signed_axis(axis4) * 5.0,
                    signed_axis(axis5) * 5.0,
                ];
            }
            SDL_SensorType_SDL_SENSOR_ACCEL_L => {
                sensor.values = vec![
                    signed_axis(axis0) * 4.0,
                    signed_axis(axis1) * 4.0,
                    0.5 + signed_axis(axis2) * 0.5,
                ];
            }
            SDL_SensorType_SDL_SENSOR_GYRO_L => {
                sensor.values = vec![signed_axis(axis3) * 2.5, signed_axis(axis4) * 2.5, 0.0];
            }
            SDL_SensorType_SDL_SENSOR_ACCEL_R => {
                sensor.values = vec![
                    signed_axis(axis3) * 4.0,
                    signed_axis(axis4) * 4.0,
                    0.5 + signed_axis(axis5) * 0.5,
                ];
            }
            SDL_SensorType_SDL_SENSOR_GYRO_R => {
                sensor.values = vec![
                    signed_axis(axis0) * 2.5,
                    signed_axis(axis2) * 2.5,
                    signed_axis(axis5) * 2.5,
                ];
            }
            _ => {}
        }
    }
}

fn crc16_for_byte(mut r: u8) -> u16 {
    let mut crc = 0u16;
    for _ in 0..8 {
        crc = if ((crc ^ r as u16) & 1) != 0 {
            0xA001 ^ (crc >> 1)
        } else {
            crc >> 1
        };
        r >>= 1;
    }
    crc
}

pub(crate) fn crc16(mut crc: u16, data: &[u8]) -> u16 {
    for byte in data {
        crc = crc16_for_byte((crc as u8) ^ *byte) ^ (crc >> 8);
    }
    crc
}

pub(crate) fn create_joystick_guid(
    bus: u16,
    vendor: u16,
    product: u16,
    version: u16,
    vendor_name: Option<&str>,
    product_name: &str,
    driver_signature: u8,
    driver_data: u8,
) -> SDL_JoystickGUID {
    let mut guid = SDL_JoystickGUID { data: [0; 16] };
    let mut crc = 0u16;

    if let (Some(vendor_name), true) = (vendor_name, !product_name.is_empty()) {
        crc = crc16(crc, vendor_name.as_bytes());
        crc = crc16(crc, b" ");
        crc = crc16(crc, product_name.as_bytes());
    } else {
        crc = crc16(crc, product_name.as_bytes());
    }

    let mut words = [0u16; 8];
    words[0] = bus.to_le();
    words[1] = crc.to_le();

    if vendor != 0 && product != 0 {
        words[2] = vendor.to_le();
        words[3] = 0;
        words[4] = product.to_le();
        words[5] = 0;
        words[6] = version.to_le();
        guid.data[14] = driver_signature;
        guid.data[15] = driver_data;
        for (index, word) in words.into_iter().enumerate().take(7) {
            let bytes = word.to_ne_bytes();
            guid.data[index * 2] = bytes[0];
            guid.data[index * 2 + 1] = bytes[1];
        }
    } else {
        for (index, word) in words.into_iter().enumerate().take(2) {
            let bytes = word.to_ne_bytes();
            guid.data[index * 2] = bytes[0];
            guid.data[index * 2 + 1] = bytes[1];
        }
        let mut available_space = guid.data.len() - 4;
        if driver_signature != 0 {
            available_space -= 2;
            guid.data[14] = driver_signature;
            guid.data[15] = driver_data;
        }
        let bytes = product_name.as_bytes();
        let copy_len = bytes.len().min(available_space.saturating_sub(1));
        guid.data[4..4 + copy_len].copy_from_slice(&bytes[..copy_len]);
    }

    guid
}

pub(crate) fn decode_guid_info(guid: SDL_JoystickGUID) -> (u16, u16, u16, u16) {
    let mut words = [0u16; 8];
    for (index, word) in words.iter_mut().enumerate() {
        *word = u16::from_ne_bytes([guid.data[index * 2], guid.data[index * 2 + 1]]);
    }
    let bus = u16::from_le(words[0]);
    if (bus < b' ' as u16 || bus == SDL_HARDWARE_BUS_VIRTUAL) && words[3] == 0 && words[5] == 0 {
        (
            u16::from_le(words[2]),
            u16::from_le(words[4]),
            u16::from_le(words[6]),
            u16::from_le(words[1]),
        )
    } else if bus < b' ' as u16 || bus == SDL_HARDWARE_BUS_VIRTUAL {
        (0, 0, 0, u16::from_le(words[1]))
    } else {
        (0, 0, 0, 0)
    }
}

fn evdev_device(
    path: &str,
    instance_id: SDL_JoystickID,
    device: linux::evdev::ProbedDevice,
) -> DeviceEntry {
    let mut entry = DeviceEntry {
        instance_id,
        name: make_cstring(&device.name),
        path: Some(make_cstring(path)),
        guid: create_joystick_guid(
            device.bus,
            device.vendor,
            device.product,
            device.version,
            None,
            &device.name,
            0,
            0,
        ),
        vendor: device.vendor,
        product: device.product,
        product_version: device.version,
        firmware_version: 0,
        serial: None,
        joystick_type: device.joystick_type,
        player_index: -1,
        is_virtual: false,
        callbacks: None,
        state: device.state,
        touchpads: Vec::new(),
        sensors: Vec::new(),
        evdev: Some(device.source),
        hint_path: Some(path.to_string()),
    };
    refresh_device_features(&mut entry);
    entry
}

fn refresh_hint_devices(state: &mut InputState) {
    let wanted_paths = hint_value(crate::abi::generated_types::SDL_HINT_JOYSTICK_DEVICE)
        .map(|value| {
            crate::input::linux::evdev::expand_device_hint_paths(
                crate::input::linux::evdev::parse_device_hint(&value),
            )
            .unwrap_or_default()
        })
        .unwrap_or_default()
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    let wanted = wanted_paths.iter().cloned().collect::<BTreeSet<_>>();
    let mut existing = HashMap::new();
    let mut virtuals = Vec::new();
    for device in state.devices.drain(..) {
        match &device.hint_path {
            Some(path) if wanted.contains(path) => {
                existing.insert(path.clone(), device);
            }
            Some(_) => {}
            None => virtuals.push(device),
        }
    }

    let mut hinted = Vec::new();
    for path in wanted_paths {
        if let Some(device) = existing.remove(&path) {
            hinted.push(device);
            continue;
        }
        let Ok(device) = crate::input::linux::evdev::probe_device(Path::new(&path)) else {
            continue;
        };
        let instance_id = state.next_instance_id;
        state.next_instance_id += 1;
        hinted.push(evdev_device(&path, instance_id, device));
    }
    hinted.extend(virtuals);
    state.devices = hinted;
}

pub(crate) fn init_input_subsystem(flag: Uint32) -> Result<(), ()> {
    let mut state = lock_input_state();
    if flag & SDL_INIT_JOYSTICK != 0 {
        state.joystick_initialized = true;
        refresh_hint_devices(&mut state);
    }
    if flag & SDL_INIT_GAMECONTROLLER != 0 {
        state.controller_initialized = true;
        for device in &mut state.devices {
            refresh_device_features(device);
        }
    }
    if flag & SDL_INIT_HAPTIC != 0 {
        state.haptic_initialized = true;
    }
    if flag & SDL_INIT_SENSOR != 0 {
        state.sensor_initialized = true;
        for device in &mut state.devices {
            refresh_device_features(device);
        }
    }
    Ok(())
}

unsafe fn close_controller_handle(state: &mut InputState, handle: *mut ControllerHandle) {
    if handle.is_null() {
        return;
    }
    let joystick = (*handle).joystick.cast::<JoystickHandle>();
    state
        .open_controllers
        .retain(|(ptr, _)| *ptr != handle as usize);
    if !joystick.is_null() {
        close_joystick_handle(state, joystick);
    }
    drop(Box::from_raw(handle));
}

unsafe fn close_joystick_handle(state: &mut InputState, handle: *mut JoystickHandle) {
    if handle.is_null() {
        return;
    }
    state
        .open_joysticks
        .retain(|(ptr, _)| *ptr != handle as usize);
    drop(Box::from_raw(handle));
}

pub(crate) unsafe fn close_haptic_handle(state: &mut InputState, handle: *mut HapticHandle) {
    if handle.is_null() {
        return;
    }
    state
        .open_haptics
        .retain(|(ptr, _)| *ptr != handle as usize);
    drop(Box::from_raw(handle));
}

pub(crate) unsafe fn close_sensor_handle(state: &mut InputState, handle: *mut SensorHandle) {
    if handle.is_null() {
        return;
    }
    state
        .open_sensors
        .retain(|(ptr, _)| *ptr != handle as usize);
    drop(Box::from_raw(handle));
}

pub(crate) fn quit_input_subsystem(flag: Uint32) {
    let mut state = lock_input_state();
    if flag & SDL_INIT_GAMECONTROLLER != 0 {
        state.controller_initialized = false;
        let controllers = state
            .open_controllers
            .iter()
            .map(|(ptr, _)| *ptr as *mut ControllerHandle)
            .collect::<Vec<_>>();
        for controller in controllers {
            unsafe {
                close_controller_handle(&mut state, controller);
            }
        }
    }
    if flag & SDL_INIT_JOYSTICK != 0 {
        state.joystick_initialized = false;
        let joysticks = state
            .open_joysticks
            .iter()
            .map(|(ptr, _)| *ptr as *mut JoystickHandle)
            .collect::<Vec<_>>();
        for joystick in joysticks {
            unsafe {
                close_joystick_handle(&mut state, joystick);
            }
        }
        state.devices.retain(|device| device.is_virtual);
    }
    if flag & SDL_INIT_HAPTIC != 0 {
        state.haptic_initialized = false;
        let haptics = state
            .open_haptics
            .iter()
            .map(|(ptr, _)| *ptr as *mut HapticHandle)
            .collect::<Vec<_>>();
        for haptic in haptics {
            unsafe {
                close_haptic_handle(&mut state, haptic);
            }
        }
    }
    if flag & SDL_INIT_SENSOR != 0 {
        state.sensor_initialized = false;
        let sensors = state
            .open_sensors
            .iter()
            .map(|(ptr, _)| *ptr as *mut SensorHandle)
            .collect::<Vec<_>>();
        for sensor in sensors {
            unsafe {
                close_sensor_handle(&mut state, sensor);
            }
        }
    }
}

pub(crate) fn dup_c_string(value: &CString) -> *mut c_char {
    value.clone().into_raw()
}

pub(crate) fn device_index_to_instance(
    state: &mut InputState,
    device_index: c_int,
) -> Option<SDL_JoystickID> {
    refresh_hint_devices(state);
    usize::try_from(device_index)
        .ok()
        .and_then(|index| state.devices.get(index))
        .map(|device| device.instance_id)
}

pub(crate) fn device_by_instance_mut(
    state: &mut InputState,
    instance_id: SDL_JoystickID,
) -> Option<&mut DeviceEntry> {
    state
        .devices
        .iter_mut()
        .find(|device| device.instance_id == instance_id)
}

pub(crate) fn device_by_instance(
    state: &InputState,
    instance_id: SDL_JoystickID,
) -> Option<&DeviceEntry> {
    state
        .devices
        .iter()
        .find(|device| device.instance_id == instance_id)
}

pub(crate) fn joystick_instance_from_handle(joystick: *mut SDL_Joystick) -> Option<SDL_JoystickID> {
    if joystick.is_null() {
        return None;
    }
    Some(unsafe { (*(joystick as *mut JoystickHandle)).instance_id })
}

pub(crate) fn controller_handle<'a>(
    gamecontroller: *mut crate::abi::generated_types::SDL_GameController,
) -> Option<&'a mut ControllerHandle> {
    if gamecontroller.is_null() {
        None
    } else {
        Some(unsafe { &mut *(gamecontroller as *mut ControllerHandle) })
    }
}

pub(crate) fn mapping_for_guid<'a>(
    state: &'a InputState,
    guid: &SDL_JoystickGUID,
) -> Option<&'a MappingEntry> {
    state
        .mapping_indices
        .get(&guid.data)
        .and_then(|index| state.mappings.get(*index))
}

pub(crate) fn mapping_for_guid_mut<'a>(
    state: &'a mut InputState,
    guid: &SDL_JoystickGUID,
) -> Option<&'a mut MappingEntry> {
    let index = *state.mapping_indices.get(&guid.data)?;
    state.mappings.get_mut(index)
}

pub(crate) fn invalid_or_null<T>(ptr: *const T, name: &str) -> bool {
    if ptr.is_null() {
        let _ = invalid_param_error(name);
        true
    } else {
        false
    }
}

pub(crate) fn set_unsupported(message: &str) -> c_int {
    set_error_message(message)
}

pub(crate) fn cstr_ptr(value: Option<&CString>) -> *const c_char {
    value.map(|value| value.as_ptr()).unwrap_or(ptr::null())
}

macro_rules! mapping_array {
    ($value:expr; $len:expr) => {{
        let value = $value;
        [value; $len]
    }};
}

pub(crate) use mapping_array;

pub mod gamecontroller;
pub mod guid;
pub mod haptic;
pub mod hidapi;
pub mod joystick;
pub mod sensor;

pub mod linux {
    pub mod evdev;
    pub mod udev;
}
